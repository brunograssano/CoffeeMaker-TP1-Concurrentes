pub mod orders_reader;
pub mod order;
pub mod dispenser;
pub mod coffee_maker;
pub mod errors;

use coffee_maker::CoffeeMaker::CoffeeMaker;

fn main() {
    let coffee_maker = CoffeeMaker::new();
    coffee_maker.manage_orders();
}