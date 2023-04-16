pub mod coffee_maker;
pub mod constants;
pub mod container;
pub mod container_source_replenisher;
pub mod dispenser;
pub mod errors;
pub mod external_source_replenisher;
pub mod order;
pub mod orders_queue;
pub mod orders_reader;
pub mod statistics;

use coffee_maker::CoffeeMaker;
use std::env;

fn main() {
    set_logger_config();
    let path = get_orders_path();
    let coffee_maker = CoffeeMaker::new();
    coffee_maker.manage_orders(path);
}

fn get_orders_path() -> String {
    let args: Vec<String> = env::args().collect();
    let mut path = "orders.json";
    if args.len() == 2 {
        path = &args[1];
    }
    String::from(path)
}

fn set_logger_config() {
    if env::var("RUST_LOG").is_err() {
        if let Err(err) = simple_logger::init_with_level(log::Level::Error) {
            println!("Error setting logger to default value: {:?}", err);
        }
    } else if let Err(err) = simple_logger::init_with_env() {
        println!("Error setting logger: {:?}", err);
    }
}
