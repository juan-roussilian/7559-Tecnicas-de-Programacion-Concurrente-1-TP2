use std::sync::{mpsc::Sender, Arc, Condvar, Mutex};

use lib::{
    common_errors::ConnectionError,
    local_connection_messages::{CoffeeMakerResponse, MessageType, ResponseStatus},
};

use crate::{connection_status::ConnectionStatus, errors::ServerError, orders_queue::OrdersQueue};

pub fn clean_substract_orders_if_offline(
    orders: Arc<Mutex<OrdersQueue>>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    connected_cond: Arc<Condvar>,
) -> Result<(), ServerError> {
    let response = CoffeeMakerResponse {
        message_type: MessageType::RequestPoints,
        status: ResponseStatus::Err(ConnectionError::UnexpectedError),
    };
    loop {
        let _ = connected_cond.wait_while(connection_status.lock()?, |connection_status| {
            connection_status.is_online()
        });

        let discarded_orders = orders.lock()?.get_and_clear_request_points_orders();

        for order in discarded_orders.iter() {
            request_points_channel.send((response, order.1))?;
        }
    }
}
