//! Dispenser de la cafetera. Procesa los pedidos.
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex, MutexGuard, RwLock},
    thread,
    time::Duration,
};

use log::{debug, info};

use crate::{
    container::Container,
    errors::CoffeeMakerError,
    order::{Ingredient, Order},
    orders_queue::OrdersQueue,
};

/// Representa a un dispenser de la cafetera.
/// Tiene referencias a la cola de pedidos (junto con su variable condicional),
/// reponedores de ingredientes (junto con su variable condicional), los recursos, y el contador de ordenes procesadas
pub struct Dispenser {
    id: usize,
    orders_queue: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    replenisher: Arc<Condvar>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
    ingredients_cond: Arc<Condvar>,
    orders_processed: Arc<RwLock<u64>>,
}

impl Dispenser {
    pub fn new(
        id: usize,
        orders_queue: Arc<Mutex<OrdersQueue>>,
        orders_cond: Arc<Condvar>,
        replenisher: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
        orders_processed: Arc<RwLock<u64>>,
    ) -> Dispenser {
        Dispenser {
            id,
            orders_queue,
            orders_cond,
            replenisher,
            ingredients_cond,
            resources,
            orders_processed,
        }
    }

    pub fn handle_orders(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let order = {
                let mut orders = self
                    .orders_cond
                    .wait_while(self.orders_queue.lock()?, |queue| {
                        queue.is_empty() && !queue.finished
                    })?;

                if orders.is_empty() && orders.finished {
                    return Ok(());
                }

                orders
                    .pop()
                    .ok_or(CoffeeMakerError::EmptyQueueWhenNotExpected)?
            };

            debug!("[DISPENSER {}] Takes order {}", self.id, order.id);
            self.process_order(order)?;
        }
    }

    fn process_order(&self, order: Order) -> Result<(), CoffeeMakerError> {
        for (ingredient, quantity_required) in order.ingredients {
            let resource_lock = self.get_resource_lock(&ingredient)?;

            let mut container = self
                .ingredients_cond
                .wait_while(resource_lock.lock()?, |container| {
                    self.should_wake_replenisher(container, quantity_required, &ingredient)
                })
                .map_err(|_| CoffeeMakerError::LockError)?;
            if container.remaining < quantity_required {
                info!(
                    "[DISPENSER {}] Skipped order {}, not enough {:?}",
                    self.id, order.id, ingredient
                );
                return Ok(());
            }
            self.consume_ingredient(&mut container, quantity_required, &ingredient);
        }
        self.increase_processed_orders()?;
        Ok(())
    }

    fn should_wake_replenisher(
        &self,
        container: &Container,
        quantity_required: u64,
        ingredient: &Ingredient,
    ) -> bool {
        if container.finished || has_no_replenisher(ingredient) {
            return false;
        }
        let need_more_resource = container.remaining < quantity_required;
        if need_more_resource {
            info!(
                "[DISPENSER {}] Not enough {:?} for this order, waking up replenisher",
                self.id, ingredient
            );
            self.replenisher.notify_all();
        }
        need_more_resource
    }

    fn get_resource_lock(
        &self,
        ingredient: &Ingredient,
    ) -> Result<&Arc<Mutex<Container>>, CoffeeMakerError> {
        let resource_lock = self
            .resources
            .get(ingredient)
            .ok_or(CoffeeMakerError::IngredientNotInMap)?;
        Ok(resource_lock)
    }

    fn increase_processed_orders(&self) -> Result<(), CoffeeMakerError> {
        let mut processed = self
            .orders_processed
            .write()
            .map_err(|_| CoffeeMakerError::LockError)?;
        *processed += 1;
        Ok(())
    }

    fn consume_ingredient(
        &self,
        mutex: &mut MutexGuard<Container>,
        quantity_required: u64,
        ingredient: &Ingredient,
    ) {
        debug!(
            "[DISPENSER {}] Uses {} of {:?}, there is {}",
            self.id, quantity_required, ingredient, mutex.remaining
        );
        mutex.remaining -= quantity_required;
        mutex.consumed += quantity_required;
        thread::sleep(Duration::from_millis(quantity_required));
        debug!(
            "[DISPENSER {}] Remains {} of {:?}",
            self.id, mutex.remaining, ingredient
        );
    }
}

fn has_no_replenisher(ingredient: &Ingredient) -> bool {
    *ingredient == Ingredient::Cacao
}
