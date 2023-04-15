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

fn main() {
    let coffee_maker = CoffeeMaker::new();
    coffee_maker.manage_orders();
}
