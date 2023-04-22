//! Representacion de la cola de pedidos
use std::collections::VecDeque;

use crate::order::Order;

/// Cola de pedidos a realizar. Se le agrega el campo `finished` para indicar que no se van a estar cargando más pedidos a la cola.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_an_empty_order_queue() {
        let queue = OrdersQueue::new();
        assert_eq!(false, queue.finished);
        assert_eq!(true, queue.is_empty());
    }

    #[test]
    fn should_add_an_order_to_the_queue() {
        let mut queue = OrdersQueue::new();
        queue.push(Order::new(1, Vec::new()));
        assert_eq!(false, queue.finished);
        assert_eq!(false, queue.is_empty());
    }

    #[test]
    fn should_pop_an_order_from_the_queue() {
        let mut queue = OrdersQueue::new();
        queue.push(Order::new(1, Vec::new()));
        let order = queue.pop();
        assert_eq!(true, order.is_some());
        assert_eq!(true, queue.is_empty());
    }

    #[test]
    fn should_pop_and_return_none_from_the_queue() {
        let mut queue = OrdersQueue::new();
        let order = queue.pop();
        assert_eq!(true, order.is_none());
        assert_eq!(true, queue.is_empty());
    }
}
