use lib::local_connection_messages::{CoffeeMakerRequest, MessageType};

pub struct OrdersQueue {
    adding_orders: Vec<CoffeeMakerRequest>,
    request_points_orders: Vec<CoffeeMakerRequest>,
}

impl OrdersQueue {
    pub fn new() -> OrdersQueue {
        OrdersQueue {
            adding_orders: Vec::new(),
            request_points_orders: Vec::new(),
        }
    }

    pub fn add(&mut self, order: CoffeeMakerRequest) {
        match order.message_type {
            MessageType::AddPoints => self.adding_orders.push(order),
            MessageType::RequestPoints => self.request_points_orders.push(order),
            _ => {}
        }
    }
}
