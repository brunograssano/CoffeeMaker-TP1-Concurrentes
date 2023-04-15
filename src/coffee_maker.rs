use std::{
    collections::HashMap,
    sync::{ Arc, Condvar, Mutex, RwLock },
    thread::{ self, JoinHandle },
};

use crate::{
    constants::{ A_STORAGE, C_STORAGE, E_STORAGE, G_STORAGE, L_STORAGE, M_STORAGE, N_DISPENSERS },
    container::Container,
    container_source_replenisher::ContainerReplenisher,
    dispenser::Dispenser,
    errors::CoffeeMakerError,
    external_source_replenisher::ExternalReplenisher,
    order::{ Ingredient, TOTAL_INGREDIENTS },
    orders_queue::OrdersQueue,
    orders_reader::read_and_add_orders,
    statistics::StatisticsPrinter,
};

pub struct CoffeeMaker {
    order_list: Arc<Mutex<OrdersQueue>>,
    orders_to_take: Arc<Condvar>,
    dispensers: Vec<Arc<Dispenser>>,
    container_replenishers: Vec<Arc<ContainerReplenisher>>,
    water_replenisher: Arc<ExternalReplenisher>,
    statistics_printer: Arc<StatisticsPrinter>,
}

impl CoffeeMaker {
    pub fn new() -> CoffeeMaker {
        let mut resources = HashMap::with_capacity(TOTAL_INGREDIENTS);
        let cold_milk = Arc::new(Mutex::new(Container::new(L_STORAGE)));
        let milk_foam = Arc::new(Mutex::new(Container::new(E_STORAGE)));
        let hot_water = Arc::new(Mutex::new(Container::new(A_STORAGE)));
        let grains_to_grind = Arc::new(Mutex::new(Container::new(G_STORAGE)));
        let ground_coffee = Arc::new(Mutex::new(Container::new(M_STORAGE)));
        resources.insert(Ingredient::ColdMilk, cold_milk.clone());
        resources.insert(Ingredient::MilkFoam, milk_foam.clone());
        resources.insert(Ingredient::HotWater, hot_water.clone());
        resources.insert(Ingredient::GrainsToGrind, grains_to_grind.clone());
        resources.insert(Ingredient::GroundCoffee, ground_coffee.clone());
        resources.insert(Ingredient::Cacao, Arc::new(Mutex::new(Container::new(C_STORAGE))));

        let resources = Arc::new(resources);
        let order_list = Arc::new(Mutex::new(OrdersQueue::new()));
        let orders_to_take = Arc::new(Condvar::new());
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());

        let orders_processed = Arc::new(RwLock::new(0));
        let dispensers = (0..N_DISPENSERS)
            .map(|id| {
                Arc::new(
                    Dispenser::new(
                        id,
                        order_list.clone(),
                        orders_to_take.clone(),
                        replenisher_cond.clone(),
                        ingredients_cond.clone(),
                        resources.clone(),
                        orders_processed.clone()
                    )
                )
            })
            .collect::<Vec<Arc<Dispenser>>>();

        let mut container_replenishers = Vec::new();

        container_replenishers.push(
            Arc::new(
                ContainerReplenisher::new(
                    (Ingredient::GrainsToGrind, grains_to_grind.clone()),
                    (Ingredient::GroundCoffee, ground_coffee.clone()),
                    replenisher_cond.clone(),
                    ingredients_cond.clone(),
                    M_STORAGE
                )
            )
        );

        container_replenishers.push(
            Arc::new(
                ContainerReplenisher::new(
                    (Ingredient::ColdMilk, cold_milk.clone()),
                    (Ingredient::MilkFoam, milk_foam.clone()),
                    replenisher_cond.clone(),
                    ingredients_cond.clone(),
                    E_STORAGE
                )
            )
        );

        let water_replenisher = Arc::new(
            ExternalReplenisher::new(
                (Ingredient::HotWater, hot_water.clone()),
                replenisher_cond.clone(),
                ingredients_cond.clone(),
                A_STORAGE
            )
        );

        CoffeeMaker {
            order_list,
            orders_to_take,
            dispensers,
            container_replenishers,
            water_replenisher,
            statistics_printer: Arc::new(
                StatisticsPrinter::new(orders_processed.clone(), resources.clone())
            ),
        }
    }

    pub fn manage_orders(&self) {
        let reader = self.create_reader_thread();
        let replenisher_threads = self.create_container_replenisher_threads();
        let water_replenisher_thread = self.create_water_replenisher_thread();
        let statistics_thread = self.create_statistics_thread();
        let dispenser_threads = self.create_dispenser_threads();

        wait_for_reader(reader);
        wait_for_dispensers(dispenser_threads);
        self.wait_for_replenishers(replenisher_threads, water_replenisher_thread);
        self.wait_for_statistics_thread(statistics_thread);
    }

    fn create_reader_thread(&self) -> JoinHandle<Result<(), CoffeeMakerError>> {
        let orders_list_clone = self.order_list.clone();
        let orders_to_take_clone = self.orders_to_take.clone();

        thread::spawn(move || {
            read_and_add_orders(orders_list_clone, orders_to_take_clone, "orders.json")
        })
    }

    fn create_container_replenisher_threads(
        &self
    ) -> Vec<JoinHandle<Result<(), CoffeeMakerError>>> {
        self.container_replenishers
            .iter()
            .map(|replenisher| {
                let replenisher_clone = replenisher.clone();
                thread::spawn(move || replenisher_clone.replenish_container())
            })
            .collect()
    }

    fn create_water_replenisher_thread(&self) -> JoinHandle<Result<(), CoffeeMakerError>> {
        let water_replenisher_clone = self.water_replenisher.clone();
        thread::spawn(move || water_replenisher_clone.replenish_container())
    }

    fn create_statistics_thread(&self) -> JoinHandle<Result<(), CoffeeMakerError>> {
        let statistics_printer_clone = self.statistics_printer.clone();
        thread::spawn(move || statistics_printer_clone.process_statistics())
    }

    fn create_dispenser_threads(&self) -> Vec<JoinHandle<Result<(), CoffeeMakerError>>> {
        self.dispensers
            .iter()
            .map(|dispenser| {
                let dispenser_clone = dispenser.clone();
                thread::spawn(move || dispenser_clone.handle_orders())
            })
            .collect()
    }

    fn wait_for_statistics_thread(
        &self,
        statistics_thread: JoinHandle<Result<(), CoffeeMakerError>>
    ) {
        self.statistics_printer.finish();
        if let Err(err) = statistics_thread.join() {
            println!("[ERROR ON STATISTICS THREAD] {:?}", err);
        }
    }

    fn wait_for_replenishers(
        &self,
        replenisher_threads: Vec<JoinHandle<Result<(), CoffeeMakerError>>>,
        water_replenisher_thread: JoinHandle<Result<(), CoffeeMakerError>>
    ) {
        self.signal_replenishers_to_finish();

        for replenisher in replenisher_threads {
            if let Err(err) = replenisher.join() {
                println!("[ERROR ON REPLENISHER] {:?}", err);
            }
        }

        if let Err(err) = water_replenisher_thread.join() {
            println!("[ERROR ON REPLENISHER] {:?}", err);
        }
    }

    fn signal_replenishers_to_finish(&self) {
        for replenisher in &self.container_replenishers {
            replenisher.finish();
        }
        self.water_replenisher.finish();
    }
}

fn wait_for_reader(reader: JoinHandle<Result<(), CoffeeMakerError>>) {
    if let Err(err) = reader.join() {
        println!("[ERROR ON READER] {:?}", err);
    }
}

fn wait_for_dispensers(dispenser_threads: Vec<JoinHandle<Result<(), CoffeeMakerError>>>) {
    for dispenser in dispenser_threads {
        if let Err(err) = dispenser.join() {
            println!("[ERROR ON DISPENSER] {:?}", err);
        }
    }
}