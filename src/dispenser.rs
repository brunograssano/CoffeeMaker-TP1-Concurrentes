use std::{
    sync::{ Arc, RwLock, Condvar, Mutex },
    collections::{ VecDeque, HashMap },
    time::Duration,
    thread,
};

use std_semaphore::Semaphore;

use crate::{ order::{ Order, Ingredient }, errors::CoffeeMakerError };

pub struct Dispenser {
    id: usize,
    orders_list: Arc<RwLock<VecDeque<Order>>>,
    orders_to_take: Arc<Semaphore>,
    finish: Arc<RwLock<bool>>,
    replenisher: Arc<Condvar>,
    ingredients_available: Arc<Condvar>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<(u64, u64)>>>>,
    orders_processed: Arc<RwLock<u64>>,
}

impl Dispenser {
    pub fn new(
        id: usize,
        orders_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        replenisher: Arc<Condvar>,
        ingredients_available: Arc<Condvar>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<(u64, u64)>>>>,
        orders_processed: Arc<RwLock<u64>>
    ) -> Dispenser {
        Dispenser {
            id,
            orders_list,
            orders_to_take,
            replenisher,
            ingredients_available,
            resources,
            orders_processed,
            finish: Arc::new(RwLock::new(false)),
        }
    }

    pub fn handle_orders(&self) -> Result<(), CoffeeMakerError> {
        loop {
            self.orders_to_take.acquire();

            let order = {
                let mut orders = self.orders_list.write()?;
                orders.pop_front().ok_or(CoffeeMakerError::EmptyQueueWhenNotExpected)?
            };

            println!("[DISPENSER {}] Takes order {}", self.id, order.id);

            for (ingredient, quantity_required) in order.ingredients {
                let resource_lock = self.resources
                    .get(&ingredient)
                    .ok_or(CoffeeMakerError::IngredientNotInMap)?;
                if let Ok(lock) = resource_lock.lock() {
                    println!(
                        "[DISPENSER {}] Takes access to container of {:?}",
                        self.id,
                        ingredient
                    );
                    let mut mutex = self.ingredients_available
                        .wait_while(lock, |(quantity_in_container, _)| {
                            let need_to_wake_up_replenisher =
                                *quantity_in_container < quantity_required;
                            if need_to_wake_up_replenisher {
                                println!(
                                    "[DISPENSER {}] Not enough {:?} for this order, waking up replenisher",
                                    self.id,
                                    ingredient
                                );
                                self.replenisher.notify_all();
                            }
                            need_to_wake_up_replenisher
                        })
                        .map_err(|_| { CoffeeMakerError::LockError })?;
                    let (mut remaining, mut consumed) = *mutex;
                    println!(
                        "[DISPENSER {}] Uses {} of {:?}, there is {}",
                        self.id,
                        quantity_required,
                        ingredient,
                        remaining
                    );
                    remaining -= quantity_required;
                    consumed += quantity_required;
                    *mutex = (remaining, consumed);
                    thread::sleep(Duration::from_millis(quantity_required));
                    println!("[DISPENSER {}] Remains {} of {:?}", self.id, remaining, ingredient);
                } else {
                    println!("[ERROR] Error while taking the resource {:?} lock", ingredient);
                    return Err(CoffeeMakerError::LockError);
                }
            }

            {
                let mut processed = self.orders_processed
                    .write()
                    .map_err(|_| { CoffeeMakerError::LockError })?;
                *processed += 1;
            }

            if *self.finish.read()? {
                return Ok(());
            }
        }
    }
}