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

    pub fn process_statistics(&self) -> Result<(), CoffeeMakerError> {
        loop {
            if *self.finish.lock()? {
                self.print_statistics()?;
                return Ok(());
            }

            self.print_statistics()?;

            thread::sleep(Duration::from_millis(STATISTICS_WAIT_IN_MS));
        }
    }

    fn print_statistics(&self) -> Result<(), CoffeeMakerError> {
        let orders_processed = self.get_orders_processed()?;
        let mut statistics =
            format!("[STATISTICS] Orders processed={} | Ingredient=(remaining, consumed) |", orders_processed);
        self.add_resources_to_statistics_string(&mut statistics)?;
        println!("{}", statistics);
        Ok(())
    }

    fn add_resources_to_statistics_string(
        &self,
        statistics: &mut String
    ) -> Result<(), CoffeeMakerError> {
        for (ingredient, container_lock) in self.resources.iter() {
            let container = container_lock.lock().map_err(|_| { CoffeeMakerError::LockError })?;
            statistics.push_str(
                &format!(" {:?}=({},{}) ", ingredient, container.remaining, container.consumed)
            );
        }
        Ok(())
    }

    fn get_orders_processed(&self) -> Result<u64, CoffeeMakerError> {
        let processed = *self.processed.read().map_err(|_| { CoffeeMakerError::LockError })?;
        Ok(processed)
    }
}