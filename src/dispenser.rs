use std::{
    sync::{ Arc, RwLock, Condvar, Mutex },
    collections::{ VecDeque, HashMap },
    time::Duration,
    thread,
};

use log::{ debug, info, error };

use crate::{
    order::{ Order, Ingredient },
    errors::CoffeeMakerError,
    orders_queue::OrdersQueue,
    container::Container,
};

pub struct Dispenser {
    id: usize,
    orders_list: Arc<Mutex<OrdersQueue>>,
    orders_to_take: Arc<Condvar>,
    replenisher: Arc<Condvar>,
    ingredients_available: Arc<Condvar>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
    orders_processed: Arc<RwLock<u64>>,
}

impl Dispenser {
    pub fn new(
        id: usize,
        orders_list: Arc<Mutex<OrdersQueue>>,
        orders_to_take: Arc<Condvar>,
        replenisher: Arc<Condvar>,
        ingredients_available: Arc<Condvar>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
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
        }
    }

    pub fn handle_orders(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let order = {
                let mut orders = self.orders_to_take.wait_while(self.orders_list.lock()?, |queue| {
                    queue.is_empty() && !queue.finished
                })?;

                if orders.is_empty() && orders.finished {
                    println!("{}", self.id);
                    return Ok(());
                }

                orders.pop().ok_or(CoffeeMakerError::EmptyQueueWhenNotExpected)?
            };

            debug!("[DISPENSER {}] Takes order {}", self.id, order.id);
            self.process_order(order)?;
            self.increase_processed_orders()?;
        }
    }

    fn process_order(&self, order: Order) -> Result<(), CoffeeMakerError> {
        for (ingredient, quantity_required) in order.ingredients {
            let resource_lock = self.get_resource_lock(&ingredient)?;
            if let Ok(lock) = resource_lock.lock() {
                debug!("[DISPENSER {}] Takes access to container of {:?}", self.id, ingredient);
                let mut mutex = self.ingredients_available
                    .wait_while(lock, |container| {
                        let need_to_wake_up_replenisher = container.remaining < quantity_required;
                        if need_to_wake_up_replenisher {
                            info!(
                                "[DISPENSER {}] Not enough {:?} for this order, waking up replenisher",
                                self.id,
                                ingredient
                            );
                            self.replenisher.notify_all();
                        }
                        need_to_wake_up_replenisher
                    })
                    .map_err(|_| { CoffeeMakerError::LockError })?;
                self.consume_ingredient(&mut mutex, quantity_required, &ingredient);
            } else {
                error!("[ERROR] Error while taking the resource {:?} lock", ingredient);
                return Err(CoffeeMakerError::LockError);
            }
        }
        Ok(())
    }

    fn get_resource_lock(
        &self,
        ingredient: &Ingredient
    ) -> Result<&Arc<Mutex<Container>>, CoffeeMakerError> {
        let resource_lock = self.resources
            .get(ingredient)
            .ok_or(CoffeeMakerError::IngredientNotInMap)?;
        Ok(resource_lock)
    }

    fn increase_processed_orders(&self) -> Result<(), CoffeeMakerError> {
        let mut processed = self.orders_processed
            .write()
            .map_err(|_| { CoffeeMakerError::LockError })?;
        *processed += 1;
        Ok(())
    }

    fn consume_ingredient(
        &self,
        mutex: &mut std::sync::MutexGuard<Container>,
        quantity_required: u64,
        ingredient: &Ingredient
    ) {
        debug!(
            "[DISPENSER {}] Uses {} of {:?}, there is {}",
            self.id,
            quantity_required,
            ingredient,
            mutex.remaining
        );
        mutex.remaining -= quantity_required;
        mutex.consumed += quantity_required;
        thread::sleep(Duration::from_millis(quantity_required));
        debug!("[DISPENSER {}] Remains {} of {:?}", self.id, mutex.remaining, ingredient);
    }
}