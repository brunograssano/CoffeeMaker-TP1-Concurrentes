//! Punto de entrada a la aplicacion. Maneja la cafetera.

use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread::{self, JoinHandle},
};

use crate::{
    constants::{
        A_WATER_STORAGE, C_CACAO_STORAGE, E_FOAM_STORAGE, G_GRAINS_STORAGE, L_MILK_STORAGE,
        M_COFFEE_STORAGE, N_DISPENSERS,
    },
    container::Container,
    container_source_replenisher::ContainerReplenisher,
    dispenser::Dispenser,
    errors::CoffeeMakerError,
    external_source_replenisher::ExternalReplenisher,
    order::{Ingredient, TOTAL_INGREDIENTS},
    orders_queue::OrdersQueue,
    orders_reader::read_and_add_orders,
    statistics::StatisticsPrinter,
};

pub struct CoffeeMaker {
    orders_queue: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    dispensers: Vec<Arc<Dispenser>>,
    container_replenishers: Vec<Arc<ContainerReplenisher>>,
    water_replenisher: Arc<ExternalReplenisher>,
    statistics_printer: Arc<StatisticsPrinter>,
}

impl CoffeeMaker {
    pub fn new() -> CoffeeMaker {
        // Initialize containers and resources hash map
        let mut resources = HashMap::with_capacity(TOTAL_INGREDIENTS);
        let cold_milk = Arc::new(Mutex::new(Container::new(L_MILK_STORAGE)));
        let milk_foam = Arc::new(Mutex::new(Container::new(E_FOAM_STORAGE)));
        let hot_water = Arc::new(Mutex::new(Container::new(A_WATER_STORAGE)));
        let grains_to_grind = Arc::new(Mutex::new(Container::new(G_GRAINS_STORAGE)));
        let ground_coffee = Arc::new(Mutex::new(Container::new(M_COFFEE_STORAGE)));
        resources.insert(Ingredient::ColdMilk, cold_milk.clone());
        resources.insert(Ingredient::MilkFoam, milk_foam.clone());
        resources.insert(Ingredient::HotWater, hot_water.clone());
        resources.insert(Ingredient::GrainsToGrind, grains_to_grind.clone());
        resources.insert(Ingredient::GroundCoffee, ground_coffee.clone());
        resources.insert(
            Ingredient::Cacao,
            Arc::new(Mutex::new(Container::new(C_CACAO_STORAGE))),
        );

        // Initialize dispenser shared data
        let resources = Arc::new(resources);
        let orders_queue = Arc::new(Mutex::new(OrdersQueue::new()));
        let orders_cond = Arc::new(Condvar::new());
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());
        let orders_processed = Arc::new(RwLock::new(0));

        // Initialize dispensers
        let dispensers = (0..N_DISPENSERS)
            .map(|id| {
                Arc::new(Dispenser::new(
                    id,
                    orders_queue.clone(),
                    orders_cond.clone(),
                    replenisher_cond.clone(),
                    ingredients_cond.clone(),
                    resources.clone(),
                    orders_processed.clone(),
                ))
            })
            .collect::<Vec<Arc<Dispenser>>>();

        // Initialize containers
        let mut container_replenishers = Vec::new();
        container_replenishers.push(Arc::new(ContainerReplenisher::new(
            (Ingredient::GrainsToGrind, grains_to_grind.clone()),
            (Ingredient::GroundCoffee, ground_coffee.clone()),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            M_COFFEE_STORAGE,
        )));

        container_replenishers.push(Arc::new(ContainerReplenisher::new(
            (Ingredient::ColdMilk, cold_milk.clone()),
            (Ingredient::MilkFoam, milk_foam.clone()),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            E_FOAM_STORAGE,
        )));

        let water_replenisher = Arc::new(ExternalReplenisher::new(
            (Ingredient::HotWater, hot_water.clone()),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            A_WATER_STORAGE,
        ));

        CoffeeMaker {
            orders_queue,
            orders_cond,
            dispensers,
            container_replenishers,
            water_replenisher,
            statistics_printer: Arc::new(StatisticsPrinter::new(
                orders_processed.clone(),
                resources.clone(),
            )),
        }
    }

    pub fn manage_orders(&self, path: String) {
        let reader = self.create_reader_thread(path);
        let replenisher_threads = self.create_container_replenisher_threads();
        let water_replenisher_thread = self.create_water_replenisher_thread();
        let statistics_thread = self.create_statistics_thread();
        let dispenser_threads = self.create_dispenser_threads();
        wait_for_reader(reader);
        wait_for_dispensers(dispenser_threads);
        self.wait_for_replenishers(replenisher_threads, water_replenisher_thread);
        self.wait_for_statistics_thread(statistics_thread);
    }

    fn create_reader_thread(&self, path: String) -> JoinHandle<Result<(), CoffeeMakerError>> {
        let orders_queue_clone = self.orders_queue.clone();
        let orders_cond_clone = self.orders_cond.clone();
        thread::spawn(move || read_and_add_orders(orders_queue_clone, orders_cond_clone, path))
    }

    fn create_container_replenisher_threads(
        &self,
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
        statistics_thread: JoinHandle<Result<(), CoffeeMakerError>>,
    ) {
        self.statistics_printer.finish();
        if let Err(err) = statistics_thread.join() {
            println!("[ERROR ON STATISTICS THREAD] {:?}", err);
        }
    }

    fn wait_for_replenishers(
        &self,
        replenisher_threads: Vec<JoinHandle<Result<(), CoffeeMakerError>>>,
        water_replenisher_thread: JoinHandle<Result<(), CoffeeMakerError>>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_finish_correctly_if_the_file_does_not_exists() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("not-a-file.json"));
        assert_eq!(
            0,
            *coffee_maker
                .statistics_printer
                .processed
                .read()
                .expect("Fail test")
        );
    }

    #[test]
    fn should_finish_correctly_if_there_are_no_orders_on_the_file() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("tests/no_orders.json"));
        assert_eq!(
            0,
            *coffee_maker
                .statistics_printer
                .processed
                .read()
                .expect("Fail test")
        );
    }

    #[test]
    fn should_process_an_order_and_finish() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("tests/simple_order.json"));

        let processed = *coffee_maker
            .statistics_printer
            .processed
            .read()
            .expect("Fail test");
        assert_eq!(1, processed);

        let resources = &coffee_maker.statistics_printer.resources;

        let cacao = resources.get(&Ingredient::Cacao).expect("Fail test");
        let milk_foam = resources.get(&Ingredient::MilkFoam).expect("Fail test");
        let ground_coffee = resources.get(&Ingredient::GroundCoffee).expect("Fail test");
        let water = resources.get(&Ingredient::HotWater).expect("Fail test");
        let grains = resources
            .get(&Ingredient::GrainsToGrind)
            .expect("Fail test");
        let cold_milk = resources.get(&Ingredient::ColdMilk).expect("Fail test");

        let cacao = cacao.lock().expect("Fail test");
        let milk_foam = milk_foam.lock().expect("Fail test");
        let ground_coffee = ground_coffee.lock().expect("Fail test");
        let water = water.lock().expect("Fail test");
        let grains = grains.lock().expect("Fail test");
        let cold_milk = cold_milk.lock().expect("Fail test");

        assert_eq!(C_CACAO_STORAGE - 60, cacao.remaining);
        assert_eq!(60, cacao.consumed);

        assert_eq!(E_FOAM_STORAGE - 70, milk_foam.remaining);
        assert_eq!(70, milk_foam.consumed);

        assert_eq!(M_COFFEE_STORAGE - 100, ground_coffee.remaining);
        assert_eq!(100, ground_coffee.consumed);

        assert_eq!(A_WATER_STORAGE - 150, water.remaining);
        assert_eq!(150, water.consumed);

        assert_eq!(G_GRAINS_STORAGE, grains.remaining);
        assert_eq!(0, grains.consumed);

        assert_eq!(L_MILK_STORAGE, cold_milk.remaining);
        assert_eq!(0, cold_milk.consumed);
    }

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000
    #[test]
    fn should_process_three_big_orders_and_replenish_the_containers() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("tests/replenish_containers.json"));

        let processed = *coffee_maker
            .statistics_printer
            .processed
            .read()
            .expect("Fail test");
        assert_eq!(3, processed);

        let resources = &coffee_maker.statistics_printer.resources;

        let cacao = resources.get(&Ingredient::Cacao).expect("Fail test");
        let milk_foam = resources.get(&Ingredient::MilkFoam).expect("Fail test");
        let ground_coffee = resources.get(&Ingredient::GroundCoffee).expect("Fail test");
        let water = resources.get(&Ingredient::HotWater).expect("Fail test");
        let grains = resources
            .get(&Ingredient::GrainsToGrind)
            .expect("Fail test");
        let cold_milk = resources.get(&Ingredient::ColdMilk).expect("Fail test");

        let cacao = cacao.lock().expect("Fail test");
        let milk_foam = milk_foam.lock().expect("Fail test");
        let ground_coffee = ground_coffee.lock().expect("Fail test");
        let water = water.lock().expect("Fail test");
        let grains = grains.lock().expect("Fail test");
        let cold_milk = cold_milk.lock().expect("Fail test");

        assert_eq!(C_CACAO_STORAGE - 1800, cacao.remaining);
        assert_eq!(1800, cacao.consumed);

        assert_eq!(E_FOAM_STORAGE - 2000, milk_foam.remaining);
        assert_eq!(6000, milk_foam.consumed);

        assert_eq!(M_COFFEE_STORAGE - 2000, ground_coffee.remaining);
        assert_eq!(6000, ground_coffee.consumed);

        assert_eq!(A_WATER_STORAGE - 2000, water.remaining);
        assert_eq!(6000, water.consumed);

        assert_eq!(G_GRAINS_STORAGE - 4000, grains.remaining);
        assert_eq!(4000, grains.consumed);

        assert_eq!(L_MILK_STORAGE - 4000, cold_milk.remaining);
        assert_eq!(4000, cold_milk.consumed);
    }

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000
    #[test]
    fn should_process_multiple_orders_and_finish() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("tests/multiple_orders.json"));

        let processed = *coffee_maker
            .statistics_printer
            .processed
            .read()
            .expect("Fail test");
        assert_eq!(41, processed);

        let resources = &coffee_maker.statistics_printer.resources;

        let cacao = resources.get(&Ingredient::Cacao).expect("Fail test");
        let milk_foam = resources.get(&Ingredient::MilkFoam).expect("Fail test");
        let ground_coffee = resources.get(&Ingredient::GroundCoffee).expect("Fail test");
        let water = resources.get(&Ingredient::HotWater).expect("Fail test");
        let grains = resources
            .get(&Ingredient::GrainsToGrind)
            .expect("Fail test");
        let cold_milk = resources.get(&Ingredient::ColdMilk).expect("Fail test");

        let cacao = cacao.lock().expect("Fail test");
        let milk_foam = milk_foam.lock().expect("Fail test");
        let ground_coffee = ground_coffee.lock().expect("Fail test");
        let water = water.lock().expect("Fail test");
        let grains = grains.lock().expect("Fail test");
        let cold_milk = cold_milk.lock().expect("Fail test");

        assert_eq!(C_CACAO_STORAGE - 410, cacao.remaining);
        assert_eq!(410, cacao.consumed);

        assert_eq!(E_FOAM_STORAGE - 410, milk_foam.remaining);
        assert_eq!(410, milk_foam.consumed);

        assert_eq!(M_COFFEE_STORAGE - 410, ground_coffee.remaining);
        assert_eq!(410, ground_coffee.consumed);

        assert_eq!(A_WATER_STORAGE - 410, water.remaining);
        assert_eq!(410, water.consumed);

        assert_eq!(G_GRAINS_STORAGE, grains.remaining);
        assert_eq!(0, grains.consumed);

        assert_eq!(L_MILK_STORAGE, cold_milk.remaining);
        assert_eq!(0, cold_milk.consumed);
    }

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000.
    /// Hay 8 ordenes en el archivo, el cacao se acaba y se terminan salteando 2 ordenes
    #[test]
    fn should_skip_an_order_if_there_is_not_enough_of_an_ingredient() {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.manage_orders(String::from("tests/skip_orders.json"));

        let processed = *coffee_maker
            .statistics_printer
            .processed
            .read()
            .expect("Fail test");
        assert_eq!(6, processed);

        let resources = &coffee_maker.statistics_printer.resources;

        let cacao = resources.get(&Ingredient::Cacao).expect("Fail test");
        let milk_foam = resources.get(&Ingredient::MilkFoam).expect("Fail test");
        let ground_coffee = resources.get(&Ingredient::GroundCoffee).expect("Fail test");
        let water = resources.get(&Ingredient::HotWater).expect("Fail test");
        let grains = resources
            .get(&Ingredient::GrainsToGrind)
            .expect("Fail test");
        let cold_milk = resources.get(&Ingredient::ColdMilk).expect("Fail test");

        let cacao = cacao.lock().expect("Fail test");
        let milk_foam = milk_foam.lock().expect("Fail test");
        let ground_coffee = ground_coffee.lock().expect("Fail test");
        let water = water.lock().expect("Fail test");
        let grains = grains.lock().expect("Fail test");
        let cold_milk = cold_milk.lock().expect("Fail test");

        assert_eq!(0, cacao.remaining);
        assert_eq!(C_CACAO_STORAGE, cacao.consumed);

        assert_eq!(E_FOAM_STORAGE - 1000, milk_foam.remaining);
        assert_eq!(1000, milk_foam.consumed);

        assert_eq!(M_COFFEE_STORAGE - 1000, ground_coffee.remaining);
        assert_eq!(1000, ground_coffee.consumed);

        assert_eq!(A_WATER_STORAGE - 1000, water.remaining);
        assert_eq!(1000, water.consumed);

        assert_eq!(G_GRAINS_STORAGE, grains.remaining);
        assert_eq!(0, grains.consumed);

        assert_eq!(L_MILK_STORAGE, cold_milk.remaining);
        assert_eq!(0, cold_milk.consumed);
    }
}
