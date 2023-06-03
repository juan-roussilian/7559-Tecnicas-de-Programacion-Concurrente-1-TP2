use std::sync::Arc;

use std::sync::{Condvar, Mutex};

use crate::orders_queue::OrdersQueue;

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    // channel para recibir las respuestas y responder al request points
}

impl OrdersManager {
    pub fn new(orders: Arc<Mutex<OrdersQueue>>, orders_cond: Arc<Condvar>) -> OrdersManager {
        OrdersManager {
            orders,
            orders_cond,
        }
    }

    pub fn handle_orders(&mut self) {}
}
