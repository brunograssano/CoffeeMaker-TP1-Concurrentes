pub(crate) mod orders_reader {
    use std::{ error::Error, collections::VecDeque };
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use std::sync::{ Arc, RwLock };
    use serde::Deserialize;
    use std_semaphore::Semaphore;

    use crate::order::order::{ Ingredient, Order };

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
        orders: Vec<JsonOrder>,
        order_list_clone: Arc<RwLock<VecDeque<Order>>>,
        order_to_take: Arc<Semaphore>
    ) {
        for order in orders {
            let mut ingredients = Vec::new();
            if 0 < order.ground_coffee {
                ingredients.push(Ingredient::GroundCoffee(order.ground_coffee));
            }
            if 0 < order.cacao {
                ingredients.push(Ingredient::Cacao(order.cacao));
            }
            if 0 < order.hot_water {
                ingredients.push(Ingredient::HotWater(order.hot_water));
            }
            if 0 < order.milk_foam {
                ingredients.push(Ingredient::MilkFoam(order.milk_foam));
            }
            if let Ok(mut queue) = order_list_clone.write() {
                queue.push_back(Order {
                    ingredients,
                });
                order_to_take.release();
            } else {
                println!("[ERROR] Error while taking the queue lock");
                return;
            }
            println!("[INFO] Added order");
        }
        println!("[INFO] There are no orders left");
    }

    pub fn read_and_add_orders<P: AsRef<Path>>(
        order_list_clone: Arc<RwLock<VecDeque<Order>>>,
        order_to_take: Arc<Semaphore>,
        path: P
    ) {
        let result = read_orders_from_file(path);
        match result {
            Ok(orders) => add_orders_to_list(orders, order_list_clone, order_to_take),
            Err(err) => println!("[ERROR] Error while reading the orders from the file"),
        }
    }
}