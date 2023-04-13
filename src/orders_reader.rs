use log::{ info, error, debug };
use std::{ error::Error, collections::VecDeque };
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{ Arc, Mutex, Condvar };
use serde::Deserialize;

use rand::{ thread_rng };
use rand::seq::SliceRandom;

use crate::errors::CoffeeMakerError;
use crate::order::{ Ingredient, Order };

#[derive(Deserialize, Debug)]
struct JsonOrder {
    ground_coffee: u64,
    hot_water: u64,
    cacao: u64,
    milk_foam: u64,
}

#[derive(Deserialize)]
struct OrdersConfiguration {
    orders: Vec<JsonOrder>,
}

fn read_orders_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<JsonOrder>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let orders_config: OrdersConfiguration = serde_json::from_reader(reader)?;
    Ok(orders_config.orders)
}

fn add_orders_to_list(
    json_orders: Vec<JsonOrder>,
    orders_queue_lock: Arc<Mutex<VecDeque<Order>>>,
    order_semaphore: Arc<Condvar>
) -> Result<(), CoffeeMakerError> {
    let mut id = 0;
    for order in json_orders {
        let ingredients = get_ingredients_from_order(order);
        if let Ok(mut orders_queue) = orders_queue_lock.lock() {
            orders_queue.push_back(Order::new(id, ingredients));
            debug!("[READER] Added order {}", id);
            id += 1;
            order_semaphore.notify_one();
        } else {
            error!("[READER] Error while taking the queue lock");
            return Err(CoffeeMakerError::LockError);
        }
    }
    info!("[READER] No more orders left");
    Ok(())
}

fn get_ingredients_from_order(order: JsonOrder) -> Vec<(Ingredient, u64)> {
    let mut ingredients = Vec::new();
    if 0 < order.ground_coffee {
        ingredients.push((Ingredient::GroundCoffee, order.ground_coffee));
    }
    if 0 < order.cacao {
        ingredients.push((Ingredient::Cacao, order.cacao));
    }
    if 0 < order.hot_water {
        ingredients.push((Ingredient::HotWater, order.hot_water));
    }
    if 0 < order.milk_foam {
        ingredients.push((Ingredient::MilkFoam, order.milk_foam));
    }
    ingredients.shuffle(&mut thread_rng());
    ingredients
}

pub fn read_and_add_orders<P: AsRef<Path>>(
    order_list: Arc<Mutex<VecDeque<Order>>>,
    order_semaphore: Arc<Condvar>,
    path: P
) -> Result<(), CoffeeMakerError> {
    let result = read_orders_from_file(path);
    match result {
        Ok(json_orders) => add_orders_to_list(json_orders, order_list, order_semaphore),
        Err(_) => Err(CoffeeMakerError::FileReaderError),
    }
}