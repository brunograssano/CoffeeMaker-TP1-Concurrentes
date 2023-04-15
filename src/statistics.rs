use std::{ sync::{ Arc, Mutex, RwLock }, collections::HashMap, time::Duration, thread };

use log::error;

use crate::{
    order::Ingredient,
    errors::CoffeeMakerError,
    constants::STATISTICS_WAIT_IN_MS,
    container::Container,
};

pub struct StatisticsPrinter {
    processed: Arc<RwLock<u64>>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
    finish: Arc<Mutex<bool>>,
}

impl StatisticsPrinter {
    pub fn new(
        processed: Arc<RwLock<u64>>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>
    ) -> StatisticsPrinter {
        StatisticsPrinter {
            processed,
            resources,
            finish: Arc::new(Mutex::new(false)),
        }
    }

    pub fn finish(&self) {
        if let Ok(mut finish) = self.finish.lock() {
            *finish = true;
            return;
        }
        error!("Error setting statistics thread to finish");
    }

    pub fn print_statistics(&self) -> Result<(), CoffeeMakerError> {
        loop {
            if *self.finish.lock()? {
                return Ok(());
            }

            let orders_processed = {
                *self.processed.read().map_err(|_| { CoffeeMakerError::LockError })?
            };
            let mut statistics =
                format!("[STATISTICS] Orders processed={} | Ingredient=(remaining, consumed) |", orders_processed);
            for (ingredient, container_lock) in self.resources.iter() {
                let container = container_lock.lock().map_err(|_| { CoffeeMakerError::LockError })?;
                statistics.push_str(
                    &format!(" {:?}=({},{}) ", ingredient, container.remaining, container.consumed)
                );
            }

            println!("{}", statistics);

            thread::sleep(Duration::from_millis(STATISTICS_WAIT_IN_MS));
        }
    }
}