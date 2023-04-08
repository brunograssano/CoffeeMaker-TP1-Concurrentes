pub mod coffee_maker {
    use std::{ thread::{ JoinHandle, self }, collections::VecDeque, sync::{ Arc, RwLock } };

    use std_semaphore::Semaphore;
    use crate::{
        orders_reader::orders_reader::read_and_add_orders,
        order::order::Order,
        dispenser::dispenser::handle_orders,
    };

    const N_DISPENSERS: usize = 10;

    pub struct CoffeeMaker {
        order_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        finish: Arc<RwLock<bool>>,
    }

    impl CoffeeMaker {
        pub fn new() -> CoffeeMaker {
            CoffeeMaker {
                order_list: Arc::new(RwLock::new(VecDeque::new())),
                orders_to_take: Arc::new(Semaphore::new(0)),
                finish: Arc::new(RwLock::new(false)),
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
                    thread::spawn(move || {
                        handle_orders(id, orders_list_clone, orders_to_take_clone, finish_clone);
                    })
                })
                .collect();

            for dispenser in dispenser_threads {
                dispenser.join().expect("Error en join");
            }
        }
    }
}