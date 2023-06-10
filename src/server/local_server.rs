use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use async_std::task;
use lib::common_errors::ConnectionError;
use log::error;

use crate::{
    address_resolver::id_to_server_port,
    coffee_maker_server::CoffeeMakerServer,
    coffee_message_dispatcher::CoffeeMessageDispatcher,
    connection_server::{ConnectionServer, TcpConnectionServer},
    connection_status::ConnectionStatus,
    errors::ServerError,
    memory_accounts_manager::MemoryAccountsManager,
    next_connection::NextConnection,
    offline_substract_orders_cleaner::SubstractOrdersCleaner,
    orders_manager::OrdersManager,
    orders_queue::OrdersQueue,
    previous_connection::PrevConnection,
    server_messages::{ServerMessage, TokenData},
};

pub struct LocalServer {
    id: usize,
    listener: Box<dyn ConnectionServer>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    to_next_conn_sender: Sender<ServerMessage>,
    to_orders_manager_sender: Sender<TokenData>,
    have_token: Arc<Mutex<bool>>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
    next_conn_handle: Option<JoinHandle<Result<(), ServerError>>>,
    orders_manager_handle: Option<JoinHandle<Result<(), ServerError>>>,
    dispatcher_handle: Option<JoinHandle<Result<(), ServerError>>>,
    coffee_handle: Option<JoinHandle<Result<(), ServerError>>>,
}

impl LocalServer {
    pub fn new(id: usize, peer_count: usize) -> Result<LocalServer, ServerError> {
        let listener: Box<dyn ConnectionServer> =
            Box::new(TcpConnectionServer::new(&id_to_server_port(id))?);
        let (to_next_conn_sender, next_conn_receiver) = mpsc::channel();
        let (to_orders_manager_sender, orders_manager_receiver) = mpsc::channel();

        let (request_points_result_sender, request_points_result_receiver) = mpsc::channel();
        let (result_points_sender, result_points_receiver) = mpsc::channel();
        let (orders_from_coffee_sender, orders_from_coffee_receiver) = mpsc::channel();

        let connection_status = Arc::new(Mutex::new(ConnectionStatus::new()));
        let have_token = Arc::new(Mutex::new(false));

        let orders = Arc::new(Mutex::new(OrdersQueue::new()));
        let orders_clone = orders.clone();
        let request_points_channel_clone = request_points_result_sender.clone();

        let accounts_manager = Arc::new(Mutex::new(MemoryAccountsManager::new()));

        let mut orders_manager = OrdersManager::new(
            id,
            orders.clone(),
            orders_manager_receiver,
            to_next_conn_sender.clone(),
            request_points_result_sender.clone(),
            result_points_receiver,
            accounts_manager.clone(),
        );

        let machine_response_senders = Arc::new(Mutex::new(HashMap::new()));
        let mut coffee_message_dispatcher = CoffeeMessageDispatcher::new(
            connection_status.clone(),
            orders,
            orders_from_coffee_receiver,
            machine_response_senders.clone(),
        );

        let offline_cleaner =
            SubstractOrdersCleaner::new(orders_clone, request_points_channel_clone);

        let mut next_connection = NextConnection::new(
            id,
            peer_count,
            next_conn_receiver,
            connection_status.clone(),
            have_token.clone(),
            accounts_manager.clone(),
            offline_cleaner,
        );

        let coffee_server =
            CoffeeMakerServer::new(id, orders_from_coffee_sender, machine_response_senders);
        if coffee_server.is_err() {
            error!("Error booting up coffee maker server, stopping...");
            return Err(ServerError::CoffeeServerStartError);
        }
        let mut coffee_server = coffee_server.unwrap();
        let coffee_handle = thread::spawn(move || coffee_server.listen());
        let dispatcher_handle = thread::spawn(move || {
            coffee_message_dispatcher.dispatch_coffee_requests(
                result_points_sender,
                request_points_result_sender,
                request_points_result_receiver,
            )
        });
        let orders_manager_handle = thread::spawn(move || orders_manager.handle_orders());
        let next_conn_handle = thread::spawn(move || next_connection.handle_message_to_next());

        Ok(LocalServer {
            listener,
            id,
            connection_status,
            to_next_conn_sender,
            to_orders_manager_sender,
            have_token,
            accounts_manager,
            next_conn_handle: Some(next_conn_handle),
            dispatcher_handle: Some(dispatcher_handle),
            orders_manager_handle: Some(orders_manager_handle),
            coffee_handle: Some(coffee_handle),
        })
    }

    pub fn start_server(&mut self) {
        if self.listen().is_err() {
            error!("Error on local server listener");
        }
        self.coffee_handle.take().map(JoinHandle::join);
        self.dispatcher_handle.take().map(JoinHandle::join);
        self.next_conn_handle.take().map(JoinHandle::join);
        self.orders_manager_handle.take().map(JoinHandle::join);
    }

    fn listen(&mut self) -> Result<(), ServerError> {
        let mut curr_prev_handle: Option<JoinHandle<Result<(), ConnectionError>>> = None;
        loop {
            let new_connection = task::block_on(self.listener.listen())?;
            let to_next_channel = self.to_next_conn_sender.clone();
            let to_orders_manager_channel = self.to_orders_manager_sender.clone();
            let mut previous = PrevConnection::new(
                new_connection,
                to_next_channel,
                to_orders_manager_channel,
                self.connection_status.clone(),
                self.id,
                self.have_token.clone(),
                self.accounts_manager.clone(),
            );

            let new_prev_handle = thread::spawn(move || previous.listen());
            if self.connection_status.lock()?.is_prev_online() {
                if let Some(handle) = curr_prev_handle {
                    if handle.join().is_err() {
                        error!("[LOCAL SERVER LISTENER] Error joining old previous connection");
                    }
                }
            }

            curr_prev_handle = Some(new_prev_handle);

            self.connection_status.lock()?.set_prev_online();
        }
    }

    // todo crear next en nuevo hilo que este manejando los intentos de conexiones y envio de mensajes a siguiente
}
