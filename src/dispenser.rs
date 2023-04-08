pub mod dispenser {
    use std::{ sync::{ Arc, RwLock, Condvar, Mutex }, collections::{ VecDeque, HashMap } };

    use std_semaphore::Semaphore;

    use crate::{ order::order::{ Order, Ingredient }, errors::DispenserError };

    pub fn handle_orders(
        id: usize,
        orders_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        finish: Arc<RwLock<bool>>,
        replenisher: Arc<Condvar>,
        ingredients_available: Arc<Condvar>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<u64>>>>
    ) -> Result<(), DispenserError> {
        loop {
            orders_to_take.acquire();

            let order = {
                let mut orders = orders_list.write()?;
                orders.pop_front().ok_or(DispenserError::EmptyQueueWhenNotExpected)?
            };

            println!("[DISPENSER {}] Takes order {}", id, order.id);

            for (ingredient, quantity_required) in order.ingredients {
                let resource_lock = resources
                    .get(&ingredient)
                    .ok_or(DispenserError::IngredientNotInMap)?;
                if let Ok(lock) = resource_lock.lock() {
                    println!("[DISPENSER {}] Takes access to container of {:?}", id, ingredient);
                    let mut in_container = ingredients_available
                        .wait_while(lock, |quantity_in_container| {
                            let need_to_wake_up_replenisher =
                                *quantity_in_container < quantity_required;
                            if need_to_wake_up_replenisher {
                                println!(
                                    "[DISPENSER {}] Not enough {:?} for this order, waking up replenisher",
                                    id,
                                    ingredient
                                );
                                replenisher.notify_all();
                            }
                            need_to_wake_up_replenisher
                        })
                        .map_err(|_| { DispenserError::LockError })?;
                    println!(
                        "[DISPENSER {}] Uses {} of {:?}, there is {}",
                        id,
                        quantity_required,
                        ingredient,
                        *in_container
                    );
                    *in_container -= quantity_required;
                    println!("[DISPENSER {}] Remains {} of {:?}", id, *in_container, ingredient);
                } else {
                    println!("[ERROR] Error while taking the resource {:?} lock", ingredient);
                    return Err(DispenserError::LockError);
                }
            }

            if *finish.read()? {
                return Ok(());
            }
        }
    }
}