use std::{ sync::{ Condvar, Arc, RwLock, Mutex }, thread, time::Duration };

use log::{ info, error };

use crate::{
    order::Ingredient,
    errors::CoffeeMakerError,
    constants::{ REPLENISH_LIMIT, MINIMUM_WAIT_TIME_REPLENISHER },
};

pub struct ExternalReplenisher {
    ingredient: Ingredient,
    container_lock: Arc<Mutex<(u64, u64)>>,
    replenisher_cond: Arc<Condvar>,
    ingredients_cond: Arc<Condvar>,
    finish: Arc<RwLock<bool>>,
    max_storage_of_container: u64,
}

impl ExternalReplenisher {
    pub fn new(
        container: (Ingredient, Arc<Mutex<(u64, u64)>>),
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        max_storage_of_container: u64
    ) -> ExternalReplenisher {
        let (ingredient, container_lock) = container;
        ExternalReplenisher {
            ingredient,
            container_lock,
            replenisher_cond,
            ingredients_cond,
            max_storage_of_container,
            finish: Arc::new(RwLock::new(false)),
        }
    }

    pub fn finish(&self) {
        if let Ok(mut finish) = self.finish.write() {
            *finish = true;
            self.replenisher_cond.notify_all();
            return;
        }
        error!("Error setting replenisher to finish");
    }

    pub fn replenish_container(&self) -> Result<(), CoffeeMakerError> {
        loop {
            if let Ok(lock) = self.container_lock.lock() {
                let mut mutex = self.replenisher_cond
                    .wait_while(lock, |(remaining, _)| {
                        let mut finish = true;
                        if let Ok(finish_result) = self.finish.read() {
                            finish = *finish_result;
                        }
                        *remaining > REPLENISH_LIMIT && !finish
                    })
                    .map_err(|_| { CoffeeMakerError::LockError })?;

                if *self.finish.read()? {
                    return Ok(());
                }
                let (mut dest_remaining, _) = *mutex;
                let replenish_quantity = self.max_storage_of_container - dest_remaining;
                dest_remaining += replenish_quantity;
                (*mutex).0 = dest_remaining;
                thread::sleep(
                    Duration::from_millis(MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity)
                );
                self.ingredients_cond.notify_all();
                info!(
                    "[REPLENISHER] Replenished {:?} with {} from external source",
                    self.ingredient,
                    replenish_quantity
                );
            } else {
                error!("[REPLENISHER] Error while taking the resource {:?} lock", self.ingredient);
                return Err(CoffeeMakerError::LockError);
            }
        }
    }
}