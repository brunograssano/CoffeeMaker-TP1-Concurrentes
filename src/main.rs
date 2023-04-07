pub mod orders_reader;
pub mod order;

use crate::orders_reader::orders_reader::read_orders_from_file;

fn main() {
    let orders = read_orders_from_file("orders.json").unwrap();
    println!("{:#?}", orders);
}