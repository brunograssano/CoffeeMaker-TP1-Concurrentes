pub mod coffee_maker {
    use std::{
        thread::{ JoinHandle, self },
        collections::{ VecDeque, HashMap },
        sync::{ Arc, RwLock, Condvar, Mutex },
    };

    use std_semaphore::Semaphore;
    use crate::{
        orders_reader::orders_reader::read_and_add_orders,
        order::order::{ Order, Ingredient },
        dispenser::dispenser::{ Dispenser },
        constants::constants::{
            L_STORAGE,
            E_STORAGE,
            C_STORAGE,
            A_STORAGE,
            G_STORAGE,
            M_STORAGE,
            N_DISPENSERS,
        },
        errors::{ CoffeeMakerError, DispenserError, ReplenisherError },
        container_source_replenisher::container_source_replenisher::ContainerReplenisher,
        external_source_replenisher::external_source_replenisher::ExternalReplenisher,
    };

    pub struct CoffeeMaker {
        order_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<u64>>>>,
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        dispensers: Vec<Arc<Dispenser>>,
        container_replenishers: Vec<Arc<ContainerReplenisher>>,
        water_replenisher: Arc<ExternalReplenisher>,
    }

    impl CoffeeMaker {
        pub fn new() -> CoffeeMaker {
            let mut resources = HashMap::new();
            let cold_milk = Arc::new(Mutex::new(L_STORAGE));
            let milk_foam = Arc::new(Mutex::new(E_STORAGE));
            let hot_water = Arc::new(Mutex::new(A_STORAGE));
            let grains_to_grind = Arc::new(Mutex::new(G_STORAGE));
            let ground_coffee = Arc::new(Mutex::new(M_STORAGE));
            resources.insert(Ingredient::ColdMilk, cold_milk.clone());
            resources.insert(Ingredient::MilkFoam, milk_foam.clone());
            resources.insert(Ingredient::HotWater, hot_water.clone());
            resources.insert(Ingredient::GrainsToGrind, grains_to_grind.clone());
            resources.insert(Ingredient::GroundCoffee, ground_coffee.clone());
            resources.insert(Ingredient::Cacao, Arc::new(Mutex::new(C_STORAGE)));

            let resources = Arc::new(resources);
            let order_list = Arc::new(RwLock::new(VecDeque::new()));
            let orders_to_take = Arc::new(Semaphore::new(0));
            let replenisher_cond = Arc::new(Condvar::new());
            let ingredients_cond = Arc::new(Condvar::new());

            let dispensers = (0..N_DISPENSERS)
                .map(|id| {
                    let orders_list_clone = order_list.clone();
                    let orders_to_take_clone = orders_to_take.clone();
                    let replenisher_clone = replenisher_cond.clone();
                    let ingredients_clone = ingredients_cond.clone();
                    let resources_clone = resources.clone();
                    Arc::new(
                        Dispenser::new(
                            id,
                            orders_list_clone,
                            orders_to_take_clone,
                            replenisher_clone,
                            ingredients_clone,
                            resources_clone
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
                resources,
                replenisher_cond,
                ingredients_cond,
                dispensers,
                container_replenishers,
                water_replenisher,
            }
        }

        pub fn manage_orders(&self) -> Result<(), CoffeeMakerError> {
            let orders_list_clone = self.order_list.clone();
            let orders_to_take_clone = self.orders_to_take.clone();
            let finish = Arc::new(RwLock::new(false));

            let reader = thread::spawn(move || {
                read_and_add_orders(orders_list_clone, orders_to_take_clone, "orders.json");
            });

            let _replenisher_threads: Vec<
                JoinHandle<Result<(), ReplenisherError>>
            > = self.container_replenishers
                .iter()
                .map(|replenisher| {
                    let replenisher_clone = replenisher.clone();
                    thread::spawn(move || { replenisher_clone.replenish_container() })
                })
                .collect();

            let water_replenisher_clone = self.water_replenisher.clone();
            let _water_replenisher_thread = thread::spawn(move || {
                water_replenisher_clone.replenish_container()
            });

            let dispenser_threads: Vec<JoinHandle<Result<(), DispenserError>>> = self.dispensers
                .iter()
                .map(|dispenser| {
                    let dispenser_clone = dispenser.clone();
                    thread::spawn(move || { dispenser_clone.handle_orders() })
                })
                .collect();

            reader.join().map_err(|_| { CoffeeMakerError::JoinError })?;
            for dispenser in dispenser_threads {
                dispenser.join().expect("Error en join");
            }
            Ok(())
        }
    }
}