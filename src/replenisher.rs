pub mod replenisher {
    use std::{ sync::{ Condvar, Arc, RwLock, Mutex }, cmp::min, thread, time::Duration };

    use crate::{
        order::order::Ingredient,
        errors::ReplenisherError,
        constants::constants::{ REPLENISH_LIMIT, MINIMUM_WAIT_TIME_REPLENISHER },
    };

    pub fn replenish_from_container(
        origin: (Ingredient, Arc<Mutex<u64>>),
        destination: (Ingredient, Arc<Mutex<u64>>),
        replenisher_cond: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        finish: Arc<RwLock<bool>>,
        max_storage_of_dest_container: u64
    ) -> Result<(), ReplenisherError> {
        let (origin_ingredient, origin_lock) = origin;
        let (destination_ingredient, destination_lock) = destination;
        loop {
            if let Ok(lock) = destination_lock.lock() {
                let mut dest_remaining = replenisher_cond
                    .wait_while(lock, |remaining| { *remaining > REPLENISH_LIMIT })
                    .map_err(|_| { ReplenisherError::LockError })?;

                if *finish.read()? {
                    // TODO
                    return Ok(());
                }

                let replenish_quantity = {
                    let mut origin_remaining = origin_lock
                        .lock()
                        .map_err(|_| { ReplenisherError::LockError })?;
                    let replenish_quantity = min(
                        max_storage_of_dest_container - *dest_remaining,
                        *origin_remaining
                    );

                    // TODO Que pasa si se vacia el contenedor de origen?
                    *origin_remaining -= replenish_quantity;
                    replenish_quantity
                };
                *dest_remaining += replenish_quantity;
                thread::sleep(
                    Duration::from_millis(MINIMUM_WAIT_TIME_REPLENISHER + replenish_quantity)
                );
                ingredients_cond.notify_all();
                println!(
                    "[REPLENISHER] Replenished {:?} with {} of {:?}",
                    destination_ingredient,
                    replenish_quantity,
                    origin_ingredient
                );
            } else {
                println!(
                    "[ERROR] Error while taking the resource {:?} lock",
                    destination_ingredient
                );
                return Err(ReplenisherError::LockError);
            }
        }
    }
}