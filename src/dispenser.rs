pub mod dispenser {
    use std::{ sync::{ Arc, RwLock }, collections::VecDeque };

    use std_semaphore::Semaphore;

    use crate::{ order::order::Order, errors::DispenserError };

    pub fn handle_orders(
        id: usize,
        orders_list: Arc<RwLock<VecDeque<Order>>>,
        orders_to_take: Arc<Semaphore>,
        finish: Arc<RwLock<bool>>
    ) -> Result<(), DispenserError> {
        loop {
            orders_to_take.acquire();

            let order = {
                let mut orders = orders_list.write()?;
                orders.pop_front().ok_or(DispenserError::EmptyQueueWhenNotExpected)?
            };

            println!("[DISPENSER {}] Takes order {}", id, order.id);

            if *finish.read()? {
                return Ok(());
            }
        }
    }
}