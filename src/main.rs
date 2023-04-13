pub mod orders_reader;
pub mod order;
pub mod dispenser;
pub mod coffee_maker;
pub mod errors;
pub mod constants;
pub mod external_source_replenisher;
pub mod container_source_replenisher;
pub mod statistics;

use coffee_maker::CoffeeMaker;

fn main() {
    let coffee_maker = CoffeeMaker::new();
    coffee_maker.manage_orders();
}