use std::{
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use log::{error, info};

use crate::{
    constants::{MINIMUM_WAIT_TIME_REPLENISHER, REPLENISH_LIMIT},
    container::Container,
    errors::CoffeeMakerError,
    order::Ingredient,
};

pub struct ExternalReplenisher {
    ingredient: Ingredient,
    container_lock: Arc<Mutex<Container>>,
    replenisher_cond: Arc<Condvar>,
    ingredients_cond: Arc<Condvar>,
    max_storage_of_container: u64,
}

impl ExternalReplenisher {
    pub fn new(
        container: (Ingredient, Arc<Mutex<Container>>),
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        max_storage_of_container: u64,
    ) -> ExternalReplenisher {
        let (ingredient, container_lock) = container;
        ExternalReplenisher {
            ingredient,
            container_lock,
            replenisher_cond,
            ingredients_cond,
            max_storage_of_container,
        }
    }

    pub fn finish(&self) {
        if let Ok(mut container) = self.container_lock.lock() {
            container.finished = true;
            self.replenisher_cond.notify_all();
            return;
        }
        error!("Error setting replenisher to finish");
    }

    pub fn replenish_container(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let mut container = self
                .replenisher_cond
                .wait_while(self.container_lock.lock()?, |container| {
                    container.remaining > REPLENISH_LIMIT && !container.finished
                })
                .map_err(|_| CoffeeMakerError::LockError)?;

            if container.finished {
                return Ok(());
            }
            self.replenish(&mut container);
            self.ingredients_cond.notify_all();
        }
    }

    fn replenish(&self, container: &mut std::sync::MutexGuard<Container>) {
        let replenish_quantity = self.max_storage_of_container - container.remaining;
        container.remaining += replenish_quantity;
        thread::sleep(Duration::from_millis(
            MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity,
        ));
        info!(
            "[REPLENISHER] Replenished {:?} with {} from external source",
            self.ingredient, replenish_quantity
        );
    }
}
