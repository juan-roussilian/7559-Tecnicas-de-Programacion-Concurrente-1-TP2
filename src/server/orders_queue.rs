use std::collections::HashMap;

use lib::local_connection_messages::{CoffeeMakerRequest, MessageType};

/// Representa a la cola de pedidos de las cafeteras. Estas van a ser procesadas por el OrdersManager
pub struct OrdersQueue {
    adding_orders: Vec<(CoffeeMakerRequest, usize)>,
    request_points_orders: Vec<(CoffeeMakerRequest, usize)>,
}

impl OrdersQueue {
    pub fn new() -> OrdersQueue {
        OrdersQueue {
            adding_orders: Vec::new(),
            request_points_orders: Vec::new(),
        }
    }

    pub fn add(&mut self, order: CoffeeMakerRequest, coffee_maker_id: usize) {
        match order.message_type {
            MessageType::AddPoints => self.adding_orders.push((order, coffee_maker_id)),
            MessageType::RequestPoints => self.request_points_orders.push((order, coffee_maker_id)),
            _ => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.adding_orders.is_empty() && self.request_points_orders.is_empty()
    }

    /// Retorna los pedidos de suma reduciendolos en caso de que sean varios sobre la misma cuenta
    pub fn get_and_clear_adding_orders(&mut self) -> Vec<CoffeeMakerRequest> {
        let mut reduced = HashMap::new();
        for req in &self.adding_orders {
            *reduced.entry(req.0.account_id).or_insert(0) += req.0.points;
        }
        self.adding_orders.clear();
        reduced
            .into_iter()
            .map(|(account_id, points)| CoffeeMakerRequest {
                account_id,
                points,
                message_type: MessageType::AddPoints,
            })
            .collect()
    }

    pub fn get_and_clear_request_points_orders(&mut self) -> Vec<(CoffeeMakerRequest, usize)> {
        let mut orders = Vec::new();
        for req in self.request_points_orders.iter() {
            orders.push(*req);
        }
        self.request_points_orders.clear();
        orders
    }
}

impl Default for OrdersQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use lib::local_connection_messages::CoffeeMakerRequest;

    use super::*;

    #[test]
    fn should_add_both_types_of_orders_to_the_queue() {
        let mut orders = OrdersQueue::new();

        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::RequestPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );
        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::AddPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );

        assert!(!orders.is_empty());
        assert_eq!(1, orders.adding_orders.len());
        assert_eq!(1, orders.request_points_orders.len());
    }

    #[test]
    fn should_reduce_same_account_adding_orders() {
        let mut orders = OrdersQueue::new();

        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::AddPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );
        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::AddPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );

        assert!(!orders.is_empty());
        assert_eq!(2, orders.adding_orders.len());
        let adding_orders = orders.get_and_clear_adding_orders();
        assert_eq!(1, adding_orders.len());
        assert_eq!(20, adding_orders[0].points);
    }

    #[test]
    fn should_clear_and_return_substract_orders() {
        let mut orders = OrdersQueue::new();

        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::RequestPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );
        orders.add(
            CoffeeMakerRequest {
                message_type: MessageType::RequestPoints,
                account_id: 0,
                points: 10,
            },
            0,
        );

        assert!(!orders.is_empty());
        assert_eq!(2, orders.request_points_orders.len());
        let substract_orders = orders.get_and_clear_request_points_orders();
        assert_eq!(2, substract_orders.len());
        assert!(orders.request_points_orders.is_empty());
    }
}
