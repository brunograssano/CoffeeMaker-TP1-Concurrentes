pub(crate) mod container_source_replenisher {
    use std::{ sync::{ Condvar, Arc, RwLock, Mutex }, cmp::min, thread, time::Duration };

    use crate::{
        order::order::Ingredient,
        errors::ReplenisherError,
        constants::constants::{ REPLENISH_LIMIT, MINIMUM_WAIT_TIME_REPLENISHER },
    };

    pub struct ContainerReplenisher {
        source_ingredient: Ingredient,
        destination_ingredient: Ingredient,
        source_container_lock: Arc<Mutex<u64>>,
        dest_container_lock: Arc<Mutex<u64>>,
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        finish: Arc<RwLock<bool>>,
        max_storage_of_container: u64,
    }

    impl ContainerReplenisher {
        pub fn new(
            source: (Ingredient, Arc<Mutex<u64>>),
            dest: (Ingredient, Arc<Mutex<u64>>),
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
        pub fn replenish_container(&self) -> Result<(), ReplenisherError> {
            loop {
                if let Ok(lock) = self.dest_container_lock.lock() {
                    let mut dest_remaining = self.replenisher_cond
                        .wait_while(lock, |remaining| { *remaining > REPLENISH_LIMIT })
                        .map_err(|_| { ReplenisherError::LockError })?;

                    if *self.finish.read()? {
                        // TODO
                        return Ok(());
                    }

                    let replenish_quantity = {
                        let mut source_remaining = self.source_container_lock
                            .lock()
                            .map_err(|_| { ReplenisherError::LockError })?;
                        let replenish_quantity = min(
                            self.max_storage_of_container - *dest_remaining,
                            *source_remaining
                        );

                        // TODO Que pasa si se vacia el contenedor de origen?
                        *source_remaining -= replenish_quantity;
                        replenish_quantity
                    };
                    *dest_remaining += replenish_quantity;
                    thread::sleep(
                        Duration::from_millis(MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity)
                    );
                    self.ingredients_cond.notify_all();
                    println!(
                        "[REPLENISHER] Replenished {:?} with {} of {:?}",
                        self.destination_ingredient,
                        replenish_quantity,
                        self.source_ingredient
                    );
                } else {
                    println!(
                        "[ERROR] Error while taking the resource {:?} lock",
                        self.destination_ingredient
                    );
                    return Err(ReplenisherError::LockError);
                }
            }
        }
    }
}