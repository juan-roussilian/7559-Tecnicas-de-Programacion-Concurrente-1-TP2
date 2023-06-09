use std::sync::{Arc, Condvar};

use lib::common_errors::ConnectionError;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};

use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::ServerMessage;
use crate::memory_accounts_manager::MemoryAccountsManager;
use crate::accounts_manager::AccountsManager;

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,

    token_receiver: Receiver<ServerMessage>,
    to_next_sender: Sender<ServerMessage>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>
}

impl OrdersManager {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        token_receiver: Receiver<ServerMessage>,
        to_next_sender: Sender<ServerMessage>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>,
        accounts_manager: Arc<Mutex<MemoryAccountsManager>>
    ) -> OrdersManager {
        OrdersManager {
            orders,
            token_receiver,
            to_next_sender,
            request_points_channel,
            result_take_points_channel,
            accounts_manager
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
                accounts.add_points(order.account_id, order.points, None);
                // Agregar al token
            }

            let mut total_request_orders = 0;
            for order in request_points_orders {
                if let Ok(()) = accounts.request_points(order.0.account_id, order.0.points){
                    total_request_orders += 1;
                }
                let result = self.request_points_channel.send((
                    CoffeeMakerResponse {
                        message_type: MessageType::RequestPoints,
                        status: ResponseStatus::Ok, /* obtener el status */
                    },
                    0, /* obtener el id */
                ));
                if result.is_err() {
                    return Err(ServerError::ChannelError);
                }
            }

            for _ in 0..total_request_orders {
                let result = self.result_take_points_channel.recv();
                if result.is_err() {
                    return Err(ServerError::ChannelError);
                }
                let result = result.unwrap();
                accounts.substract_points(result.account_id,result.points, None);
                // Agregar al token
            }
            self.to_next_sender.send(token)?;
        }
    
    }
}
