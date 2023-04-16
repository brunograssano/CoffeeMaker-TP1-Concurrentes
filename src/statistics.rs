use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

use log::error;

use crate::{
    constants::{C_STORAGE, L_STORAGE, M_STORAGE, STATISTICS_WAIT_IN_MS, X_PERCENTAGE_OF_CAPACITY},
    container::Container,
    errors::CoffeeMakerError,
    order::Ingredient,
};

pub struct StatisticsPrinter {
    processed: Arc<RwLock<u64>>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
    finish: Arc<Mutex<bool>>,
}

impl StatisticsPrinter {
    pub fn new(
        processed: Arc<RwLock<u64>>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
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
        let mut statistics = format!(
            "[STATISTICS] Orders processed={} | Ingredient=(remaining, consumed) |",
            orders_processed
        );
        self.add_resources_to_statistics_string(&mut statistics)?;
        println!("{}", statistics);
        Ok(())
    }

    fn add_resources_to_statistics_string(
        &self,
        statistics: &mut String,
    ) -> Result<(), CoffeeMakerError> {
        for (ingredient, container_lock) in self.resources.iter() {
            let container = container_lock
                .lock()
                .map_err(|_| CoffeeMakerError::LockError)?;
            statistics.push_str(&format!(
                " {:?}=({},{}) ",
                ingredient, container.remaining, container.consumed
            ));
            print_warning_if_below_x_level(&ingredient, container.remaining);
        }
        Ok(())
    }

    fn get_orders_processed(&self) -> Result<u64, CoffeeMakerError> {
        let processed = *self
            .processed
            .read()
            .map_err(|_| CoffeeMakerError::LockError)?;
        Ok(processed)
    }
}

fn print_warning_if_below_x_level(ingredient: &Ingredient, remaining: u64) {
    match ingredient {
        Ingredient::Cacao => handle_warning_level(ingredient, remaining, C_STORAGE),
        Ingredient::ColdMilk => handle_warning_level(ingredient, remaining, L_STORAGE),
        Ingredient::GrainsToGrind => handle_warning_level(ingredient, remaining, M_STORAGE),
        _ => {}
    }
}

fn handle_warning_level(ingredient: &Ingredient, remaining: u64, initial_level: u64) {
    if remaining < (initial_level * X_PERCENTAGE_OF_CAPACITY) / 100 {
        println!(
            "[WARNING] {:?} container below {}% capacity",
            ingredient, X_PERCENTAGE_OF_CAPACITY
        )
    }
}
