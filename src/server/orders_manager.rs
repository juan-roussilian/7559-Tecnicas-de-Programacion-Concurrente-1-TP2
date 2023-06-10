use std::sync::Arc;

use lib::common_errors::ConnectionError;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use log::error;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

use crate::accounts_manager::AccountsManager;
use crate::errors::ServerError;
use crate::memory_accounts_manager::MemoryAccountsManager;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::{recreate_token, AccountAction, ServerMessage, TokenData};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OrdersManager {
    my_id: usize,
    orders: Arc<Mutex<OrdersQueue>>,
    token_receiver: Receiver<TokenData>,
    to_next_sender: Sender<ServerMessage>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
}

impl OrdersManager {
    pub fn new(
        my_id: usize,
        orders: Arc<Mutex<OrdersQueue>>,
        token_receiver: Receiver<TokenData>,
        to_next_sender: Sender<ServerMessage>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>,
        accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
    ) -> OrdersManager {
        OrdersManager {
            my_id,
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
            let mut token = self.token_receiver.recv()?;

            let adding_orders;
            let request_points_orders;
            {
                let mut orders = self.orders.lock()?;
                if orders.is_empty() {
                    self.to_next_sender
                        .send(recreate_token(self.my_id, token))?;
                    continue;
                }
                adding_orders = orders.get_and_clear_adding_orders();
                request_points_orders = orders.get_and_clear_request_points_orders();
            }
            let mut accounts = self.accounts_manager.lock()?;
            for order in adding_orders {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
                if accounts
                    .add_points(order.account_id, order.points, Some(timestamp))
                    .is_err()
                {
                    error!(
                        "Error adding {} points to account {}",
                        order.points, order.account_id
                    );
                }
                let action = AccountAction {
                    message_type: MessageType::AddPoints,
                    account_id: order.account_id,
                    points: order.points,
                    last_updated_on: timestamp,
                };
                token.entry(self.my_id).or_insert(vec![]).push(action);
            }

            let mut total_request_orders = 0;
            for (order, coffee_maker_id) in request_points_orders {
                let result = accounts.request_points(order.account_id, order.points);

                let status = match result {
                    Ok(()) => {
                        total_request_orders += 1;
                        ResponseStatus::Ok
                    }
                    Err(ServerError::NotEnoughPointsInAccount) => {
                        ResponseStatus::Err(ConnectionError::NotEnoughPoints)
                    }
                    Err(ServerError::AccountNotFound) => {
                        ResponseStatus::Err(ConnectionError::AccountNotFound)
                    }
                    _ => ResponseStatus::Err(ConnectionError::UnexpectedError),
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
                match result.message_type {
                    MessageType::CancelPointsRequest => {
                        if accounts.cancel_requested_points(result.account_id).is_err() {
                            {
                                error!(
                                    "Error canceling points request from account {}",
                                    result.account_id
                                );
                                continue;
                            }
                        }
                    }
                    MessageType::TakePoints => {
                        if accounts
                            .substract_points(result.account_id, result.points, Some(timestamp))
                            .is_err()
                        {
                            error!(
                                "Error substracting {} points from account {}",
                                result.points, result.account_id
                            );
                            continue;
                        }
                        let action = AccountAction {
                            message_type: MessageType::TakePoints,
                            account_id: result.account_id,
                            points: result.points,
                            last_updated_on: timestamp,
                        };
                        token.entry(self.my_id).or_insert(vec![]).push(action);
                    }
                    _ => {}
                }
            }
            self.to_next_sender
                .send(recreate_token(self.my_id, token))?;
        }
    }
}
