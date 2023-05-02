//! Dispenser de la cafetera. Procesa los pedidos.
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex, MutexGuard, RwLock},
    time::Duration,
};

use log::{debug, info};

use crate::{
    container::Container,
    errors::CoffeeMakerError,
    order::{Ingredient, Order},
    orders_queue::OrdersQueue,
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

/// Representa a un dispenser de la cafetera.
/// Tiene referencias a la cola de pedidos (junto con su variable condicional),
/// reponedores de ingredientes (junto con su variable condicional), los recursos, y el contador de ordenes procesadas
pub struct Dispenser {
    id: usize,
    orders_queue: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    replenisher: Arc<Condvar>,
    resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
    ingredients_cond: Arc<Condvar>,
    orders_processed: Arc<RwLock<u64>>,
}

impl Dispenser {
    pub fn new(
        id: usize,
        orders_queue: Arc<Mutex<OrdersQueue>>,
        orders_cond: Arc<Condvar>,
        replenisher: Arc<Condvar>,
        ingredients_cond: Arc<Condvar>,
        resources: Arc<HashMap<Ingredient, Arc<Mutex<Container>>>>,
        orders_processed: Arc<RwLock<u64>>,
    ) -> Dispenser {
        Dispenser {
            id,
            orders_queue,
            orders_cond,
            replenisher,
            ingredients_cond,
            resources,
            orders_processed,
        }
    }

    pub fn handle_orders(&self) -> Result<(), CoffeeMakerError> {
        loop {
            let order = {
                let mut orders = self
                    .orders_cond
                    .wait_while(self.orders_queue.lock()?, |queue| {
                        queue.is_empty() && !queue.finished
                    })?;

                if orders.is_empty() && orders.finished {
                    return Ok(());
                }

                orders
                    .pop()
                    .ok_or(CoffeeMakerError::EmptyQueueWhenNotExpected)?
            };

            debug!("[DISPENSER {}] Takes order {}", self.id, order.id);
            self.process_order(order)?;
        }
    }

    fn process_order(&self, order: Order) -> Result<(), CoffeeMakerError> {
        for (ingredient, quantity_required) in order.ingredients {
            let resource_lock = self.get_resource_lock(&ingredient)?;

            let mut container = self
                .ingredients_cond
                .wait_while(resource_lock.lock()?, |container| {
                    self.should_wake_replenisher(container, quantity_required, &ingredient)
                })
                .map_err(|_| CoffeeMakerError::LockError)?;
            if container.remaining < quantity_required {
                info!(
                    "[DISPENSER {}] Skipped order {}, not enough {:?}",
                    self.id, order.id, ingredient
                );
                return Ok(());
            }
            self.consume_ingredient(&mut container, quantity_required, &ingredient);
        }
        self.increase_processed_orders()?;
        Ok(())
    }

    fn should_wake_replenisher(
        &self,
        container: &Container,
        quantity_required: u64,
        ingredient: &Ingredient,
    ) -> bool {
        if container.finished || has_no_replenisher(ingredient) {
            return false;
        }
        let need_more_resource = container.remaining < quantity_required;
        if need_more_resource {
            info!(
                "[DISPENSER {}] Not enough {:?} for this order, waking up replenisher",
                self.id, ingredient
            );
            self.replenisher.notify_all();
        }
        need_more_resource
    }

    fn get_resource_lock(
        &self,
        ingredient: &Ingredient,
    ) -> Result<&Arc<Mutex<Container>>, CoffeeMakerError> {
        let resource_lock = self
            .resources
            .get(ingredient)
            .ok_or(CoffeeMakerError::IngredientNotInMap)?;
        Ok(resource_lock)
    }

    fn increase_processed_orders(&self) -> Result<(), CoffeeMakerError> {
        let mut processed = self
            .orders_processed
            .write()
            .map_err(|_| CoffeeMakerError::LockError)?;
        *processed += 1;
        Ok(())
    }

    fn consume_ingredient(
        &self,
        mutex: &mut MutexGuard<Container>,
        quantity_required: u64,
        ingredient: &Ingredient,
    ) {
        debug!(
            "[DISPENSER {}] Uses {} of {:?}, there is {}",
            self.id, quantity_required, ingredient, mutex.remaining
        );
        mutex.remaining -= quantity_required;
        mutex.consumed += quantity_required;
        sync::sleep(Duration::from_millis(quantity_required));
        debug!(
            "[DISPENSER {}] Remains {} of {:?}",
            self.id, mutex.remaining, ingredient
        );
    }
}

fn has_no_replenisher(ingredient: &Ingredient) -> bool {
    *ingredient == Ingredient::Cacao
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::{
            A_WATER_STORAGE, C_CACAO_STORAGE, E_FOAM_STORAGE, G_GRAINS_STORAGE, L_MILK_STORAGE,
            M_COFFEE_STORAGE,
        },
        order::TOTAL_INGREDIENTS,
    };

    use super::*;

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000
    #[test]
    fn should_process_an_order() {
        let mut resources = HashMap::with_capacity(TOTAL_INGREDIENTS);
        let cold_milk = Arc::new(Mutex::new(Container::new(L_MILK_STORAGE)));
        let milk_foam = Arc::new(Mutex::new(Container::new(E_FOAM_STORAGE)));
        let hot_water = Arc::new(Mutex::new(Container::new(A_WATER_STORAGE)));
        let grains_to_grind = Arc::new(Mutex::new(Container::new(G_GRAINS_STORAGE)));
        let ground_coffee = Arc::new(Mutex::new(Container::new(M_COFFEE_STORAGE)));
        resources.insert(Ingredient::ColdMilk, cold_milk.clone());
        resources.insert(Ingredient::MilkFoam, milk_foam.clone());
        resources.insert(Ingredient::HotWater, hot_water.clone());
        resources.insert(Ingredient::GrainsToGrind, grains_to_grind.clone());
        resources.insert(Ingredient::GroundCoffee, ground_coffee.clone());
        resources.insert(
            Ingredient::Cacao,
            Arc::new(Mutex::new(Container::new(C_CACAO_STORAGE))),
        );

        // Initialize dispenser shared data
        let resources = Arc::new(resources);
        let orders_queue = Arc::new(Mutex::new(OrdersQueue::new()));
        let orders_cond = Arc::new(Condvar::new());
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());
        let orders_processed = Arc::new(RwLock::new(0));

        let dispenser = Arc::new(Dispenser::new(
            1,
            orders_queue.clone(),
            orders_cond.clone(),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            resources.clone(),
            orders_processed.clone(),
        ));

        let result = dispenser.process_order(Order::new(
            1,
            vec![(Ingredient::HotWater, 100), (Ingredient::GroundCoffee, 100)],
        ));

        assert!(result.is_ok());
        assert_eq!(
            1,
            *orders_processed
                .read()
                .expect("Error reading processed orders in test")
        );

        let container = hot_water.lock().expect("Error in hot water lock in test");
        assert_eq!(A_WATER_STORAGE - 100, container.remaining);
        assert_eq!(100, container.consumed);

        let container = ground_coffee.lock().expect("Error in coffee lock in test");
        assert_eq!(M_COFFEE_STORAGE - 100, container.remaining);
        assert_eq!(100, container.consumed);
    }

    /// Las cantidades de los ingredientes fueron calculadas con valores iniciales de 5000
    #[test]
    fn should_skip_an_order_if_there_is_no_resource_left() {
        let mut resources = HashMap::with_capacity(TOTAL_INGREDIENTS);
        let cold_milk = Arc::new(Mutex::new(Container::new(L_MILK_STORAGE)));
        let milk_foam = Arc::new(Mutex::new(Container::new(E_FOAM_STORAGE)));
        let hot_water = Arc::new(Mutex::new(Container::new(A_WATER_STORAGE)));
        let grains_to_grind = Arc::new(Mutex::new(Container::new(G_GRAINS_STORAGE)));
        let ground_coffee = Arc::new(Mutex::new(Container::new(M_COFFEE_STORAGE)));
        let cacao = Arc::new(Mutex::new(Container::new(C_CACAO_STORAGE)));
        resources.insert(Ingredient::ColdMilk, cold_milk.clone());
        resources.insert(Ingredient::MilkFoam, milk_foam.clone());
        resources.insert(Ingredient::HotWater, hot_water.clone());
        resources.insert(Ingredient::GrainsToGrind, grains_to_grind.clone());
        resources.insert(Ingredient::GroundCoffee, ground_coffee.clone());
        resources.insert(Ingredient::Cacao, cacao.clone());

        // Initialize dispenser shared data
        let resources = Arc::new(resources);
        let orders_queue = Arc::new(Mutex::new(OrdersQueue::new()));
        let orders_cond = Arc::new(Condvar::new());
        let replenisher_cond = Arc::new(Condvar::new());
        let ingredients_cond = Arc::new(Condvar::new());
        let orders_processed = Arc::new(RwLock::new(0));

        let dispenser = Arc::new(Dispenser::new(
            1,
            orders_queue.clone(),
            orders_cond.clone(),
            replenisher_cond.clone(),
            ingredients_cond.clone(),
            resources.clone(),
            orders_processed.clone(),
        ));
        {
            let mut container = cacao.lock().expect("Error in cacao lock in test");
            container.remaining = 0;
            container.consumed = C_CACAO_STORAGE;
        }

        let result = dispenser.process_order(Order::new(
            1,
            vec![
                (Ingredient::HotWater, 100),
                (Ingredient::Cacao, 100),
                (Ingredient::MilkFoam, 100),
            ],
        ));

        assert!(result.is_ok());
        assert_eq!(
            0,
            *orders_processed
                .read()
                .expect("Error reading processed orders in test")
        );

        let container = hot_water.lock().expect("Error in hot water lock in test");
        assert_eq!(A_WATER_STORAGE - 100, container.remaining);
        assert_eq!(100, container.consumed);

        let container = cacao.lock().expect("Error in cacao lock in test");
        assert_eq!(0, container.remaining);
        assert_eq!(C_CACAO_STORAGE, container.consumed);

        let container = milk_foam.lock().expect("Error in milk foam lock in test");
        assert_eq!(E_FOAM_STORAGE, container.remaining);
        assert_eq!(0, container.consumed);
    }
}
