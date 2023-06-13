use std::sync::{mpsc::Sender, Arc, Mutex};

use lib::{
    common_errors::CoffeeSystemError,
    local_connection_messages::{CoffeeMakerResponse, MessageType, ResponseStatus},
};

use crate::{errors::ServerError, orders_queue::OrdersQueue};

/// Limpador de ordenes de resta en caso de perdida de conexion
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
    /// Metodo para eliminar las ordenes de resta de puntos cuando no se tiene conexion por un tiempo
    pub fn clean_substract_orders_if_offline(&self) -> Result<(), ServerError> {
        let response = CoffeeMakerResponse {
            message_type: MessageType::RequestPoints,
            status: ResponseStatus::Err(CoffeeSystemError::UnexpectedError),
        };

        let discarded_orders = self.orders.lock()?.get_and_clear_request_points_orders();

        for order in discarded_orders.iter() {
            self.request_points_channel.send((response, order.1))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib::local_connection_messages::CoffeeMakerRequest;

    use super::*;

    #[test]
    fn should_clean_the_saved_substract_orders() {
        let orders = Arc::new(Mutex::new(OrdersQueue::new()));
        let (request_points_result_sender, request_points_result_receiver) = mpsc::channel();

        let cleaner = SubstractOrdersCleaner::new(orders.clone(), request_points_result_sender);
        {
            let mut queue = orders.lock().expect("Lock error in test");
            queue.add(
                CoffeeMakerRequest {
                    message_type: MessageType::RequestPoints,
                    account_id: 0,
                    points: 10,
                },
                0,
            );
            queue.add(
                CoffeeMakerRequest {
                    message_type: MessageType::RequestPoints,
                    account_id: 0,
                    points: 10,
                },
                0,
            );
        }
        assert!(cleaner.clean_substract_orders_if_offline().is_ok());
        assert!(orders.lock().expect("Lock error in test").is_empty());
        assert!(request_points_result_receiver.try_recv().is_ok());
        assert!(request_points_result_receiver.try_recv().is_ok());
    }
}
