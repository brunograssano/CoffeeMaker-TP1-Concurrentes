pub mod orders_reader;
pub mod order;
pub mod dispenser;
pub mod coffee_maker;
pub mod errors;
pub mod replenisher;
pub mod constants;

use coffee_maker::coffee_maker::CoffeeMaker;

fn main() {
    let coffee_maker = CoffeeMaker::new();
    coffee_maker.manage_orders();
}