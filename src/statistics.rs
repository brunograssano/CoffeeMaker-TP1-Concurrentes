use std::{ sync::{ Arc, Mutex, RwLock }, collections::HashMap, time::Duration, thread };

use crate::{ order::Ingredient, errors::CoffeeMakerError, constants::STATISTICS_WAIT_IN_MS };

pub struct StatisticsPrinter {
    processed: Arc<RwLock<u64>>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<(u64, u64)>>>>,
    finish: Arc<RwLock<bool>>,
}

impl StatisticsPrinter {
    pub fn new(
        processed: Arc<RwLock<u64>>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<(u64, u64)>>>>
    ) -> StatisticsPrinter {
        StatisticsPrinter {
            processed,
            resources,
            finish: Arc::new(RwLock::new(false)),
        }
    }

    pub fn print_statistics(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let orders_processed = {
                *self.processed.read().map_err(|_| { CoffeeMakerError::LockError })?
            };
            let mut statistics =
                format!("[STATISTICS] Orders processed={} | Ingredient=(remaining, consumed) |", orders_processed);
            for (ingredient, container_lock) in self.resources.iter() {
                let (remaining, consumed) = {
                    *container_lock.lock().map_err(|_| { CoffeeMakerError::LockError })?
                };
                statistics.push_str(&format!(" {:?}=({},{}) ", ingredient, remaining, consumed));
            }

            println!("{}", statistics);

            thread::sleep(Duration::from_millis(STATISTICS_WAIT_IN_MS));
        }
    }
}