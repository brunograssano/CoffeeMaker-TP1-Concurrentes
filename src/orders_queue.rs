use std::collections::VecDeque;

use crate::order::Order;

pub struct OrdersQueue {
    orders: VecDeque<Order>,
    pub finished: bool,
}

impl OrdersQueue {
    pub fn new() -> OrdersQueue {
        OrdersQueue {
            orders: VecDeque::new(),
            finished: false,
        }
    }

    pub fn push(&mut self, order: Order) {
        self.orders.push_back(order);
    }

    pub fn pop(&mut self) -> Option<Order> {
        self.orders.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
}
