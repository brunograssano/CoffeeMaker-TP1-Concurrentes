use log::{debug, error, info};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::errors::CoffeeMakerError;
use crate::order::{Ingredient, Order};

use crate::orders_queue::OrdersQueue;

/// Representacion de un pedido cuando viene en el archivo JSON. Tiene los ingredientes que se pueden usar y las cantidades de cada uno.
#[derive(Deserialize, Debug)]
struct JsonOrder {
    ground_coffee: u64,
    hot_water: u64,
    cacao: u64,
    milk_foam: u64,
}

/// Representa la lista de pedidos en el archivo JSON
#[derive(Deserialize)]
struct OrdersConfiguration {
    orders: Vec<JsonOrder>,
}

fn read_orders_from_file(path: String) -> Result<Vec<JsonOrder>, Box<dyn Error>> {
    let file = File::open(&Path::new(&path))?;
    let reader = BufReader::new(file);
    let orders_config: OrdersConfiguration = serde_json::from_reader(reader)?;
    Ok(orders_config.orders)
}

fn add_orders_to_list(
    json_orders: Vec<JsonOrder>,
    orders_queue_lock: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
) -> Result<(), CoffeeMakerError> {
    let mut id = 0;
    for order in json_orders {
        let ingredients = get_ingredients_from_order(order);
        if let Ok(mut queue) = orders_queue_lock.lock() {
            queue.push(Order::new(id, ingredients));
            debug!("[READER] Added order {}", id);
            id += 1;
            orders_cond.notify_all();
        } else {
            error!("[READER] Error while taking the queue lock");
            return Err(CoffeeMakerError::LockError);
        }
    }
    info!("[READER] No more orders left");
    if let Ok(mut queue) = orders_queue_lock.lock() {
        queue.finished = true;
        orders_cond.notify_all();
        return Ok(());
    }
    Err(CoffeeMakerError::LockError)
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

pub fn read_and_add_orders(
    order_list: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    path: String,
) -> Result<(), CoffeeMakerError> {
    let result = read_orders_from_file(path);
    match result {
        Ok(json_orders) => add_orders_to_list(json_orders, order_list, orders_cond),
        Err(_) => handle_error_with_file(order_list, orders_cond),
    }
}

fn handle_error_with_file(
    orders_queue_lock: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
) -> Result<(), CoffeeMakerError> {
    if let Ok(mut queue) = orders_queue_lock.lock() {
        queue.finished = true;
        orders_cond.notify_all();
    }
    Err(CoffeeMakerError::FileReaderError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_the_ingredients_from_the_json_order() {
        let ingredients = get_ingredients_from_order(JsonOrder {
            ground_coffee: 10,
            hot_water: 20,
            cacao: 30,
            milk_foam: 40,
        });
        assert_eq!(false, ingredients.is_empty());
        assert_eq!(4, ingredients.len());
        let mut quantities = [0; 4];
        for (i, quantity) in ingredients {
            match i {
                Ingredient::Cacao => {
                    quantities[0] = quantity;
                }
                Ingredient::HotWater => {
                    quantities[1] = quantity;
                }
                Ingredient::GroundCoffee => {
                    quantities[2] = quantity;
                }
                Ingredient::MilkFoam => {
                    quantities[3] = quantity;
                }
                _ => panic!("Failed to get ingredients from json order"),
            }
        }
        assert_eq!([30, 20, 10, 40], quantities);
    }

    #[test]
    fn should_get_the_ingredients_from_the_json_order_when_there_are_some_missing() {
        let ingredients = get_ingredients_from_order(JsonOrder {
            ground_coffee: 10,
            hot_water: 0,
            cacao: 30,
            milk_foam: 0,
        });
        assert_eq!(false, ingredients.is_empty());
        assert_eq!(2, ingredients.len());
        let mut quantities = [0; 2];
        for (i, quantity) in ingredients {
            match i {
                Ingredient::Cacao => {
                    quantities[0] = quantity;
                }
                Ingredient::GroundCoffee => {
                    quantities[1] = quantity;
                }
                _ => panic!("Failed to get ingredients from json order"),
            }
        }
        assert_eq!([30, 10], quantities);
    }

    #[test]
    fn should_add_the_orders_to_the_queue() {
        let mut json_orders = Vec::new();
        json_orders.push(JsonOrder {
            ground_coffee: 10,
            hot_water: 0,
            cacao: 30,
            milk_foam: 0,
        });
        json_orders.push(JsonOrder {
            ground_coffee: 100,
            hot_water: 200,
            cacao: 300,
            milk_foam: 400,
        });

        let queue = OrdersQueue::new();
        let mutex = Arc::new(Mutex::new(queue));
        let cond = Arc::new(Condvar::new());
        let result = add_orders_to_list(json_orders, mutex.clone(), cond);
        assert!(result.is_ok());

        let mut queue = mutex.lock().expect("Test error");
        assert!(queue.finished);
        assert!(queue.pop().is_some());
        assert!(queue.pop().is_some());
        assert!(queue.pop().is_none());
    }

    #[test]
    fn should_return_file_error_if_the_file_does_not_exists() {
        let result = read_and_add_orders(
            Arc::new(Mutex::new(OrdersQueue::new())),
            Arc::new(Condvar::new()),
            String::from("not-a-file.json"),
        );
        assert!(result.is_err());
        assert_eq!(
            CoffeeMakerError::FileReaderError,
            result.err().expect("Fail test")
        )
    }

    #[test]
    fn should_return_file_error_if_the_files_format_is_wrong() {
        let result = read_and_add_orders(
            Arc::new(Mutex::new(OrdersQueue::new())),
            Arc::new(Condvar::new()),
            String::from("tests/wrong_format.json"),
        );
        assert!(result.is_err());
        assert_eq!(
            CoffeeMakerError::FileReaderError,
            result.err().expect("Fail test")
        )
    }
}
