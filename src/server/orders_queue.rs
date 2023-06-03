use std::collections::HashMap;

use lib::local_connection_messages::{CoffeeMakerRequest, MessageType};

pub struct OrdersQueue {
    adding_orders: Vec<CoffeeMakerRequest>,
    request_points_orders: Vec<CoffeeMakerRequest>,
    have_token: bool,
}

impl OrdersQueue {
    pub fn new() -> OrdersQueue {
        OrdersQueue {
            adding_orders: Vec::new(),
            request_points_orders: Vec::new(),
            have_token: false,
        }
    }

    pub fn add(&mut self, order: CoffeeMakerRequest) {
        match order.message_type {
            MessageType::AddPoints => self.adding_orders.push(order),
            MessageType::RequestPoints => self.request_points_orders.push(order),
            _ => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.adding_orders.is_empty() && self.request_points_orders.is_empty()
    }

    pub fn get_and_clear_adding_orders(&mut self) -> Vec<CoffeeMakerRequest> {
        let mut reduced = HashMap::new();
        for req in &self.adding_orders {
            *reduced.entry(req.account_id).or_insert(req.points) += req.points;
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

    pub fn get_and_clear_request_points_orders(&mut self) -> Vec<CoffeeMakerRequest> {
        let mut orders = Vec::new();
        for req in self.request_points_orders.iter() {
            orders.push(*req);
        }
        self.request_points_orders.clear();
        orders
    }

    pub fn have_token(&self) -> bool {
        self.have_token
    }

    pub fn received_token(&mut self) {
        self.have_token = true;
    }

    pub fn clear_token(&mut self) {
        self.have_token = false;
    }
}
