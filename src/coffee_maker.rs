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
        dispenser::dispenser::handle_orders,
    };

    const N_DISPENSERS: usize = 10;
    const G_STORAGE: u64 = 5000;
    const M_STORAGE: u64 = 5000;
    const L_STORAGE: u64 = 5000;
    const E_STORAGE: u64 = 5000;
    const C_STORAGE: u64 = 5000;
    const A_STORAGE: u64 = 5000;

    pub struct CoffeeMaker {
        order_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        finish: Arc<RwLock<bool>>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<u64>>>>,
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
    }

    impl CoffeeMaker {
        pub fn new() -> CoffeeMaker {
            let mut resources = HashMap::new();
            resources.insert(Ingredient::ColdMilk, Arc::new(Mutex::new(L_STORAGE)));
            resources.insert(Ingredient::MilkFoam, Arc::new(Mutex::new(E_STORAGE)));
            resources.insert(Ingredient::Cacao, Arc::new(Mutex::new(C_STORAGE)));
            resources.insert(Ingredient::HotWater, Arc::new(Mutex::new(A_STORAGE)));
            resources.insert(Ingredient::GrainsToGrind, Arc::new(Mutex::new(G_STORAGE)));
            resources.insert(Ingredient::GroundCoffee, Arc::new(Mutex::new(M_STORAGE)));

            CoffeeMaker {
                order_list: Arc::new(RwLock::new(VecDeque::new())),
                orders_to_take: Arc::new(Semaphore::new(0)),
                finish: Arc::new(RwLock::new(false)),
                resources: Arc::new(resources),
                replenisher_cond: Arc::new(Condvar::new()),
                ingredients_cond: Arc::new(Condvar::new()),
            }
        }

        pub fn manage_orders(&self) {
            let orders_list_clone = self.order_list.clone();
            let orders_to_take_clone = self.orders_to_take.clone();
            let finish_clone = Arc::new(RwLock::new(false));

            thread::spawn(move || {
                read_and_add_orders(orders_list_clone, orders_to_take_clone, "orders.json");
            });

            let dispenser_threads: Vec<JoinHandle<()>> = (0..N_DISPENSERS)
                .map(|id| {
                    let orders_list_clone = self.order_list.clone();
                    let orders_to_take_clone = self.orders_to_take.clone();
                    let finish_clone = self.finish.clone();
                    let replenisher_clone = self.replenisher_cond.clone();
                    let ingredients_clone = self.ingredients_cond.clone();
                    let resources_clone = self.resources.clone();
                    thread::spawn(move || {
                        handle_orders(
                            id,
                            orders_list_clone,
                            orders_to_take_clone,
                            finish_clone,
                            replenisher_clone,
                            ingredients_clone,
                            resources_clone
                        );
                    })
                })
                .collect();

            for dispenser in dispenser_threads {
                dispenser.join().expect("Error en join");
            }
        }
    }
}