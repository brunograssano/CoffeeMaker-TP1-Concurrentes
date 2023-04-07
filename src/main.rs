pub mod orders_reader;
pub mod order;

use std::{
    thread::{ JoinHandle, self },
    collections::VecDeque,
    sync::{ Arc, RwLock },
    time::Duration,
};

use std_semaphore::Semaphore;
use crate::order::order::Order;
use crate::orders_reader::orders_reader::read_and_add_orders;

const N_DISPENSERS: usize = 10;

fn main() {
    let order_list = Arc::new(RwLock::new(VecDeque::new()));
    let order_to_take = Arc::new(Semaphore::new(0));
    let order_list_clone = order_list.clone();
    let order_to_take_clone = order_to_take.clone();
    thread::spawn(move || {
        read_and_add_orders(order_list_clone, order_to_take_clone, "orders.json");
    });

    // let dispenser_threads: Vec<JoinHandle<()>> = (0..N_DISPENSERS)
    //     .map(|id| { thread::spawn(move || {}) })
    //     .collect();

    // for dispenser in dispenser_threads {
    //     dispenser.join().expect("Error en join");
    // }
    loop {
        order_to_take.acquire();
        thread::sleep(Duration::from_millis(2000));
        println!("[MAIN] Se agarro el semaforo");
    }
}