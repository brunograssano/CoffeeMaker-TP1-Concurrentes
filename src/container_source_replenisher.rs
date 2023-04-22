use std::{
    cmp::min,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use log::{debug, error};

use crate::{
    constants::{MAX_OF_INGREDIENT_IN_AN_ORDER, MINIMUM_WAIT_TIME_REPLENISHER},
    container::Container,
    errors::CoffeeMakerError,
    order::Ingredient,
};

/// Representa a un reponedor de un contenedor a partir de otro contenedor. El contenedor usado como fuente puede agotarse
pub struct ContainerReplenisher {
    source_ingredient: Ingredient,
    dest_ingredient: Ingredient,
    source_container_lock: Arc<Mutex<Container>>,
    dest_container_lock: Arc<Mutex<Container>>,
    replenisher_cond: Arc<Condvar>,
    ingredients_cond: Arc<Condvar>,
    max_storage_of_dest_container: u64,
}

impl ContainerReplenisher {
    pub fn new(
        source: (Ingredient, Arc<Mutex<Container>>),
        dest: (Ingredient, Arc<Mutex<Container>>),
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        max_storage_of_container: u64,
    ) -> ContainerReplenisher {
        let (source_ingredient, source_container_lock) = source;
        let (dest_ingredient, dest_container_lock) = dest;
        ContainerReplenisher {
            source_ingredient,
            dest_ingredient,
            source_container_lock,
            dest_container_lock,
            replenisher_cond,
            ingredients_cond,
            max_storage_of_dest_container: max_storage_of_container,
        }
    }

    pub fn finish(&self) {
        if let Ok(mut container) = self.dest_container_lock.lock() {
            container.finished = true;
            self.replenisher_cond.notify_all();
            return;
        }
        error!("Error setting replenisher to finish");
    }

    pub fn replenish_container(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let mut dest_container = self
                .replenisher_cond
                .wait_while(self.dest_container_lock.lock()?, |container| {
                    container.remaining > MAX_OF_INGREDIENT_IN_AN_ORDER && !container.finished
                })
                .map_err(|_| CoffeeMakerError::LockError)?;

            if dest_container.finished {
                return Ok(());
            }
            self.replenish(&mut dest_container)?;
            self.ingredients_cond.notify_all();
        }
    }

    fn replenish(
        &self,
        dest_container: &mut std::sync::MutexGuard<Container>,
    ) -> Result<(), CoffeeMakerError> {
        let (replenish_quantity, source_is_empty) =
            self.take_resource_from_source(dest_container.remaining)?;
        dest_container.remaining += replenish_quantity;
        dest_container.finished = source_is_empty;
        thread::sleep(Duration::from_millis(
            MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity,
        ));
        debug!(
            "[REPLENISHER] Replenished {:?} with {} of {:?}",
            self.dest_ingredient, replenish_quantity, self.source_ingredient
        );
        Ok(())
    }

    fn take_resource_from_source(
        &self,
        dest_remaining: u64,
    ) -> Result<(u64, bool), CoffeeMakerError> {
        let mut source_container = self
            .source_container_lock
            .lock()
            .map_err(|_| CoffeeMakerError::LockError)?;
        let replenish_quantity = min(
            self.max_storage_of_dest_container - dest_remaining,
            source_container.remaining,
        );

        source_container.remaining -= replenish_quantity;
        source_container.consumed += replenish_quantity;

        let source_is_empty = source_container.is_empty();
        Ok((replenish_quantity, source_is_empty))
    }
}
