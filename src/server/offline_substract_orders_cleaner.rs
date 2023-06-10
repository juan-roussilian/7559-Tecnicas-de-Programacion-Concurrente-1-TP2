use std::sync::{mpsc::Sender, Arc, Mutex};

use lib::{
    common_errors::ConnectionError,
    local_connection_messages::{CoffeeMakerResponse, MessageType, ResponseStatus},
};

use crate::{errors::ServerError, orders_queue::OrdersQueue};

pub struct SubstractOrdersCleaner {
    orders: Arc<Mutex<OrdersQueue>>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
}

impl SubstractOrdersCleaner {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    ) -> SubstractOrdersCleaner {
        SubstractOrdersCleaner {
            orders,
            request_points_channel,
        }
    }
    pub fn clean_substract_orders_if_offline(&self) -> Result<(), ServerError> {
        let response = CoffeeMakerResponse {
            message_type: MessageType::RequestPoints,
            status: ResponseStatus::Err(ConnectionError::UnexpectedError),
        };

        let discarded_orders = self.orders.lock()?.get_and_clear_request_points_orders();

        for order in discarded_orders.iter() {
            self.request_points_channel.send((response, order.1))?;
        }
        Ok(())
    }
}
