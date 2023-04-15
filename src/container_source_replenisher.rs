use std::{ sync::{ Condvar, Arc, RwLock, Mutex }, cmp::min, thread, time::Duration };

use log::{ error, debug };

use crate::{
    order::Ingredient,
    errors::CoffeeMakerError,
    constants::{ REPLENISH_LIMIT, MINIMUM_WAIT_TIME_REPLENISHER },
    container::Container,
};

pub struct ContainerReplenisher {
    source_ingredient: Ingredient,
    destination_ingredient: Ingredient,
    source_container_lock: Arc<Mutex<Container>>,
    dest_container_lock: Arc<Mutex<Container>>,
    replenisher_cond: Arc<Condvar>,
    ingredients_cond: Arc<Condvar>,
    finish: Arc<RwLock<bool>>,
    max_storage_of_container: u64,
}

impl ContainerReplenisher {
    pub fn new(
        source: (Ingredient, Arc<Mutex<Container>>),
        dest: (Ingredient, Arc<Mutex<Container>>),
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        max_storage_of_container: u64
    ) -> ContainerReplenisher {
        let (source_ingredient, source_container_lock) = source;
        let (destination_ingredient, dest_container_lock) = dest;
        ContainerReplenisher {
            source_ingredient,
            destination_ingredient,
            source_container_lock,
            dest_container_lock,
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
            if let Ok(lock) = self.dest_container_lock.lock() {
                let mut mutex = self.replenisher_cond
                    .wait_while(lock, |container| {
                        let mut finish = true;
                        if let Ok(finish_result) = self.finish.read() {
                            finish = *finish_result;
                        }
                        container.remaining > REPLENISH_LIMIT && !finish
                    })
                    .map_err(|_| { CoffeeMakerError::LockError })?;

                if *self.finish.read()? {
                    return Ok(());
                }
                self.replenish(&mut mutex)?;
                self.ingredients_cond.notify_all();
            } else {
                error!(
                    "[ERROR] Error while taking the resource {:?} lock",
                    self.destination_ingredient
                );
                return Err(CoffeeMakerError::LockError);
            }
        }
    }

    fn replenish(
        &self,
        mutex: &mut std::sync::MutexGuard<Container>
    ) -> Result<(), CoffeeMakerError> {
        let replenish_quantity = self.take_resource_from_source(mutex.remaining)?;
        mutex.remaining += replenish_quantity;
        thread::sleep(Duration::from_millis(MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity));
        debug!(
            "[REPLENISHER] Replenished {:?} with {} of {:?}",
            self.destination_ingredient,
            replenish_quantity,
            self.source_ingredient
        );
        Ok(())
    }

    fn take_resource_from_source(&self, dest_remaining: u64) -> Result<u64, CoffeeMakerError> {
        let mut mutex = self.source_container_lock
            .lock()
            .map_err(|_| { CoffeeMakerError::LockError })?;
        let replenish_quantity = min(
            self.max_storage_of_container - dest_remaining,
            mutex.remaining
        );

        // TODO Que pasa si se vacia el contenedor de origen?
        mutex.remaining -= replenish_quantity;
        mutex.consumed += replenish_quantity;
        Ok(replenish_quantity)
    }
}