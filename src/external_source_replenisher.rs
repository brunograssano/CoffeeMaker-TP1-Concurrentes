//! Reponedor de la cafetera a partir de una fuente externa. Por ejemplo el agua.
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use log::{error, info};

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

/// Representa a un reponedor de un contenedor a partir de una fuente externa. Esta fuente no se agota
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
                    container.remaining > MAX_OF_INGREDIENT_IN_AN_ORDER && !container.finished
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
        sync::sleep(Duration::from_millis(
            MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity,
        ));
        info!(
            "[REPLENISHER] Replenished {:?} with {} from external source",
            self.ingredient, replenish_quantity
        );
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::constants::A_WATER_STORAGE;

    use super::*;

    #[test]
    fn should_replenish_the_container_when_awaken() {
        let hot_water = Arc::new(Mutex::new(Container::new(A_WATER_STORAGE)));
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());
        let water_replenisher = Arc::new(ExternalReplenisher::new(
            (Ingredient::HotWater, hot_water.clone()),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            A_WATER_STORAGE,
        ));
        let water_clone = water_replenisher.clone();
        let handle = thread::spawn(move || water_clone.replenish_container());

        {
            let mut container = hot_water.lock().expect("Lock error in test");
            container.remaining = 0;
        }
        replenisher_cond.notify_all();
        {
            let container = ingredients_cond
                .wait_while(hot_water.lock().expect("Lock error in test"), |container| {
                    container.remaining < A_WATER_STORAGE
                })
                .expect("Test error when returning from condvar");
            assert_eq!(container.remaining, A_WATER_STORAGE);
        }
        water_replenisher.finish();
        _ = handle.join().expect("Error when joining thread");
    }
}
