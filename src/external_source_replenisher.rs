pub(crate) mod external_source_replenisher {
    use std::{ sync::{ Condvar, Arc, RwLock, Mutex }, thread, time::Duration };

    use crate::{
        order::order::Ingredient,
        errors::ReplenisherError,
        constants::constants::{ REPLENISH_LIMIT, MINIMUM_WAIT_TIME_REPLENISHER },
    };

    pub struct ExternalReplenisher {
        ingredient: Ingredient,
        container_lock: Arc<Mutex<u64>>,
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        finish: Arc<RwLock<bool>>,
        max_storage_of_container: u64,
    }

    impl ExternalReplenisher {
        pub fn new(
            container: (Ingredient, Arc<Mutex<u64>>),
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
        pub fn replenish_container(&self) -> Result<(), ReplenisherError> {
            loop {
                if let Ok(lock) = self.container_lock.lock() {
                    let mut dest_remaining = self.replenisher_cond
                        .wait_while(lock, |remaining| { *remaining > REPLENISH_LIMIT })
                        .map_err(|_| { ReplenisherError::LockError })?;

                    if *self.finish.read()? {
                        // TODO
                        return Ok(());
                    }

                    let replenish_quantity = self.max_storage_of_container - *dest_remaining;
                    *dest_remaining = self.max_storage_of_container;
                    thread::sleep(
                        Duration::from_millis(MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity)
                    );
                    self.ingredients_cond.notify_all();
                    println!(
                        "[REPLENISHER] Replenished {:?} with {} from external source",
                        self.ingredient,
                        replenish_quantity
                    );
                } else {
                    println!("[ERROR] Error while taking the resource {:?} lock", self.ingredient);
                    return Err(ReplenisherError::LockError);
                }
            }
        }
    }
}