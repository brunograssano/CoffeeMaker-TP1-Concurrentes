//! Reponedor de un contenedor a partir de otro contenedor
use std::{
    cmp::min,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use log::{debug, error};

use crate::{
    constants::{MAX_OF_INGREDIENT_IN_AN_ORDER, MINIMUM_WAIT_TIME_REPLENISHER},
    container::Container,
    errors::CoffeeMakerError,
    order::Ingredient,
};

mod sync {
    use std::thread;
    use std::time::Duration;

    #[cfg(not(test))]
    pub(crate) fn sleep(d: Duration) {
        thread::sleep(d);
    }

    #[cfg(test)]
    pub(crate) fn sleep(_: Duration) {
        thread::yield_now();
    }
}

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
        sync::sleep(Duration::from_millis(
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

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::constants::{E_FOAM_STORAGE, L_MILK_STORAGE};

    use super::*;

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000
    #[test]
    fn should_replenish_the_container_taking_resource_from_second_container_when_awaken() {
        let cold_milk = Arc::new(Mutex::new(Container::new(L_MILK_STORAGE)));
        let milk_foam = Arc::new(Mutex::new(Container::new(E_FOAM_STORAGE)));
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());

        let milk_replenisher = Arc::new(ContainerReplenisher::new(
            (Ingredient::ColdMilk, cold_milk.clone()),
            (Ingredient::MilkFoam, milk_foam.clone()),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            E_FOAM_STORAGE,
        ));
        let milk_clone = milk_replenisher.clone();

        let handle = thread::spawn(move || milk_clone.replenish_container());

        {
            let mut container = milk_foam.lock().expect("Lock error in test");
            container.remaining = 0;
        }
        replenisher_cond.notify_all();
        {
            let container = ingredients_cond
                .wait_while(milk_foam.lock().expect("Lock error in test"), |container| {
                    container.remaining < E_FOAM_STORAGE
                })
                .expect("Test error when returning from condvar");
            assert_eq!(container.remaining, E_FOAM_STORAGE);
        }
        {
            let container = cold_milk.lock().expect("Lock error in test");
            assert_eq!(container.remaining, 0);
            assert_eq!(container.consumed, L_MILK_STORAGE);
        }

        milk_replenisher.finish();
        _ = handle.join().expect("Error when joining thread");
    }
}
