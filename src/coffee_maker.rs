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
        replenisher::replenisher::{ replenish_from_container, replenish_from_external_source },
    };

    pub struct CoffeeMaker {
        order_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<u64>>>>,
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        dispensers: Vec<Arc<Dispenser>>,
    }

    impl CoffeeMaker {
        pub fn new() -> CoffeeMaker {
            let resources = Arc::new(get_map_of_ingredients());
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

            CoffeeMaker {
                order_list,
                orders_to_take,
                resources,
                replenisher_cond,
                ingredients_cond,
                dispensers,
            }
        }

        fn create_replenishers(
            &self
        ) -> Result<Vec<JoinHandle<Result<(), ReplenisherError>>>, CoffeeMakerError> {
            let mut replenisher_threads = Vec::new();
            let cold_milk = self.resources
                .get(&Ingredient::ColdMilk)
                .ok_or(CoffeeMakerError::IngredientNotInMap)?
                .clone();
            let milk_foam = self.resources
                .get(&Ingredient::MilkFoam)
                .ok_or(CoffeeMakerError::IngredientNotInMap)?
                .clone();

            let ground_coffee = self.resources
                .get(&Ingredient::GroundCoffee)
                .ok_or(CoffeeMakerError::IngredientNotInMap)?
                .clone();

            let grains_to_grind = self.resources
                .get(&Ingredient::GrainsToGrind)
                .ok_or(CoffeeMakerError::IngredientNotInMap)?
                .clone();

            let water = self.resources
                .get(&Ingredient::HotWater)
                .ok_or(CoffeeMakerError::IngredientNotInMap)?
                .clone();

            let finish_clone = Arc::new(RwLock::new(false));
            let replenisher_clone = self.replenisher_cond.clone();
            let ingredients_clone = self.ingredients_cond.clone();

            let milk_replenisher = thread::spawn(move || {
                let origin = (Ingredient::ColdMilk, cold_milk);
                let destination = (Ingredient::MilkFoam, milk_foam);
                replenish_from_container(
                    origin,
                    destination,
                    replenisher_clone,
                    ingredients_clone,
                    finish_clone,
                    E_STORAGE
                )
            });

            let finish_clone = Arc::new(RwLock::new(false));
            let replenisher_clone = self.replenisher_cond.clone();
            let ingredients_clone = self.ingredients_cond.clone();

            let coffee_replenisher = thread::spawn(move || {
                let origin = (Ingredient::GrainsToGrind, grains_to_grind);
                let destination = (Ingredient::GroundCoffee, ground_coffee);
                replenish_from_container(
                    origin,
                    destination,
                    replenisher_clone,
                    ingredients_clone,
                    finish_clone,
                    M_STORAGE
                )
            });

            let finish_clone = Arc::new(RwLock::new(false));
            let replenisher_clone = self.replenisher_cond.clone();
            let ingredients_clone = self.ingredients_cond.clone();

            let water_replenisher = thread::spawn(move || {
                let container = (Ingredient::HotWater, water);
                replenish_from_external_source(
                    container,
                    replenisher_clone,
                    ingredients_clone,
                    finish_clone,
                    A_STORAGE
                )
            });

            replenisher_threads.push(milk_replenisher);
            replenisher_threads.push(coffee_replenisher);
            replenisher_threads.push(water_replenisher);
            Ok(replenisher_threads)
        }

        pub fn manage_orders(&self) -> Result<(), CoffeeMakerError> {
            let orders_list_clone = self.order_list.clone();
            let orders_to_take_clone = self.orders_to_take.clone();
            let finish = Arc::new(RwLock::new(false));

            let reader = thread::spawn(move || {
                read_and_add_orders(orders_list_clone, orders_to_take_clone, "orders.json");
            });

            let replenisher_threads = self.create_replenishers()?;

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

    fn get_map_of_ingredients() -> HashMap<Ingredient, Arc<Mutex<u64>>> {
        let mut resources = HashMap::new();
        resources.insert(Ingredient::ColdMilk, Arc::new(Mutex::new(L_STORAGE)));
        resources.insert(Ingredient::MilkFoam, Arc::new(Mutex::new(E_STORAGE)));
        resources.insert(Ingredient::Cacao, Arc::new(Mutex::new(C_STORAGE)));
        resources.insert(Ingredient::HotWater, Arc::new(Mutex::new(A_STORAGE)));
        resources.insert(Ingredient::GrainsToGrind, Arc::new(Mutex::new(G_STORAGE)));
        resources.insert(Ingredient::GroundCoffee, Arc::new(Mutex::new(M_STORAGE)));
        resources
    }
}