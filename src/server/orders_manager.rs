use std::sync::{Arc, Condvar};

use lib::common_errors::ConnectionError;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

use crate::accounts_manager::AccountsManager;
use crate::errors::ServerError;
use crate::memory_accounts_manager::MemoryAccountsManager;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::ServerMessage;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,

    token_receiver: Receiver<ServerMessage>,
    to_next_sender: Sender<ServerMessage>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
}

impl OrdersManager {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        token_receiver: Receiver<ServerMessage>,
        to_next_sender: Sender<ServerMessage>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>,
        accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
    ) -> OrdersManager {
        OrdersManager {
            orders,
            token_receiver,
            to_next_sender,
            request_points_channel,
            result_take_points_channel,
            accounts_manager,
        }
    }

    pub fn handle_orders(&mut self) -> Result<(), ServerError> {
        loop {
            let token = self.token_receiver.recv()?;
            let adding_orders;
            let request_points_orders;
            {
                let mut orders = self.orders.lock()?;
                if orders.is_empty() {
                    self.to_next_sender.send(token)?;
                    continue;
                }
                adding_orders = orders.get_and_clear_adding_orders();
                request_points_orders = orders.get_and_clear_request_points_orders();
            }
            let mut accounts = self.accounts_manager.lock()?;
            for order in adding_orders {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
                accounts.add_points(order.account_id, order.points, Some(timestamp));
                // Agregar al token
            }

            let mut total_request_orders = 0;
            for (order, coffee_maker_id) in request_points_orders {
                let result = accounts.request_points(order.account_id, order.points);
                
                let status = match result {
                    Ok(()) => {
                        total_request_orders += 1;
                        ResponseStatus::Ok
                    },
                    Err(ServerError::NotEnoughPointsInAccount) => {ResponseStatus::Err(ConnectionError::NotEnoughPoints)}
                    Err(ServerError::AccountNotFound) => {ResponseStatus::Err(ConnectionError::AccountNotFound)}
                    _ => {ResponseStatus::Err(ConnectionError::UnexpectedError)}
                };
                let result = self.request_points_channel.send((
                    CoffeeMakerResponse {
                        message_type: MessageType::RequestPoints,
                        status,
                    },
                    coffee_maker_id,
                ));
                if result.is_err() {
                    return Err(ServerError::ChannelError);
                }
            }

            for _ in 0..total_request_orders {
                // TODO agregar timeout al channel este
                let result = self.result_take_points_channel.recv()?;
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
                accounts.substract_points(result.account_id, result.points, Some(timestamp));
                // Agregar al token
            }
            self.to_next_sender.send(token)?;
        }
    }
}
