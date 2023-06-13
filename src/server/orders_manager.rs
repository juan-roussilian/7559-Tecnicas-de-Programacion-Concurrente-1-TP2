use std::sync::Arc;

use lib::common_errors::CoffeeSystemError;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use log::{debug, error /*, info */};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
//use std::thread;

use crate::accounts_manager::AccountsManager;
use crate::constants::{COFFEE_RESULT_TIMEOUT_IN_MS, POST_INITIAL_TIMEOUT_COFFEE_RESULT_IN_MS};
use crate::errors::ServerError;
use crate::memory_accounts_manager::MemoryAccountsManager;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::{recreate_token, AccountAction, ServerMessage, TokenData};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Ejecuta los pedidos de las cafeteras, guarda en la base de datos y le responde al dispatcher en las restas
/// Se ejecuta el algoritmo cada vez que recibe el token.
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
        // Uncomment to see accounts data
        // let accounts_manager_clone = self.accounts_manager.clone();
        // let _account_print_handle = thread::spawn(move || loop {
        //     thread::sleep(Duration::from_secs(5));
        //     let accounts = accounts_manager_clone.lock().unwrap();
        //     info!("{:?}", accounts);
        // });

        loop {
            let mut timeout = Duration::from_millis(COFFEE_RESULT_TIMEOUT_IN_MS);
            let mut token = self.token_receiver.recv()?;
            debug!("[ORDERS MANAGER] I have the token");
            let adding_orders;
            let request_points_orders;
            {
                let mut orders = self.orders.lock()?;
                if orders.is_empty() {
                    self.to_next_sender
                        .send(recreate_token(self.my_id, token))?;
                    debug!("[ORDERS MANAGER] I don't need the token");
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
                        ResponseStatus::Err(CoffeeSystemError::NotEnoughPoints)
                    }
                    Err(ServerError::AccountNotFound) => {
                        ResponseStatus::Err(CoffeeSystemError::AccountNotFound)
                    }
                    Err(ServerError::AccountIsReserved) => {
                        ResponseStatus::Err(CoffeeSystemError::AccountIsReserved)
                    }
                    _ => ResponseStatus::Err(CoffeeSystemError::UnexpectedError),
                };
                self.request_points_channel.send((
                    CoffeeMakerResponse {
                        message_type: MessageType::RequestPoints,
                        status,
                    },
                    coffee_maker_id,
                ))?;
            }

            let mut there_was_a_timeout = false;
            for _ in 0..total_request_orders {
                let result = self.result_take_points_channel.recv_timeout(timeout);
                match result {
                    Ok(result) => {
                        self.handle_result_of_substract_order(result, &mut accounts, &mut token)?;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        there_was_a_timeout = true;
                        timeout = Duration::from_millis(POST_INITIAL_TIMEOUT_COFFEE_RESULT_IN_MS);
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        return Err(ServerError::ChannelError);
                    }
                }
            }
            if there_was_a_timeout {
                accounts.clear_reservations();
            }
            self.to_next_sender
                .send(recreate_token(self.my_id, token))?;
            debug!("[ORDERS MANAGER] Passed the token to next connection");
        }
    }

    fn handle_result_of_substract_order(
        &self,
        result: CoffeeMakerRequest,
        accounts: &mut std::sync::MutexGuard<'_, MemoryAccountsManager>,
        token: &mut TokenData,
    ) -> Result<(), ServerError> {
        match result.message_type {
            MessageType::CancelPointsRequest => {
                if accounts.cancel_requested_points(result.account_id).is_err() {
                    error!(
                        "Error canceling points request from account {}",
                        result.account_id
                    );
                }
            }
            MessageType::TakePoints => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
                if let Err(e) =
                    accounts.substract_points(result.account_id, result.points, Some(timestamp))
                {
                    error!(
                        "Error taking {} points from account {}, {:?}",
                        result.points, result.account_id, e
                    );
                    return Ok(());
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
        Ok(())
    }
}
