use std::{ sync::{ mpsc::{ self, Sender }, Arc, Mutex }, thread::{ self, JoinHandle } };

use async_std::task;
use lib::common_errors::ConnectionError;

use crate::{
    address_resolver::id_to_server_port,
    coffee_message_dispatcher::CoffeeMessageDispatcher,
    connection_server::{ ConnectionServer, TcpConnectionServer },
    connection_status::ConnectionStatus,
    errors::ServerError,
    next_connection::NextConnection,
    orders_manager::OrdersManager,
    orders_queue::OrdersQueue,
    previous_connection::PrevConnection,
    server_messages::ServerMessage,
};

pub struct LocalServer {
    id: usize,
    listener: Box<dyn ConnectionServer>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    to_next_conn_sender: Sender<ServerMessage>,
    to_orders_manager_sender: Sender<ServerMessage>,
    next_connection: Arc<NextConnection>,
    orders_manager: Arc<OrdersManager>,
    coffee_message_dispatcher: Arc<CoffeeMessageDispatcher>,
}

impl LocalServer {
    pub fn new(id: usize, peer_count: usize) -> Result<LocalServer, ServerError> {
        let listener: Box<dyn ConnectionServer> = Box::new(
            TcpConnectionServer::new(&id_to_server_port(id))?
        );
        let (to_next_conn_sender, next_conn_receiver) = mpsc::channel();
        let (to_orders_manager_sender, orders_manager_receiver) = mpsc::channel();

        let (request_points_result_sender, request_points_result_receiver) = mpsc::channel();
        let (result_points_sender, result_points_receiver) = mpsc::channel();
        let (orders_from_coffee_sender, orders_from_coffee_receiver) = mpsc::channel();

        let connection_status = Arc::new(Mutex::new(ConnectionStatus::new()));
        let next_connection = Arc::new(
            NextConnection::new(id, peer_count, next_conn_receiver, connection_status.clone())
        );

        let orders = Arc::new(Mutex::new(OrdersQueue::new()));

        let orders_manager = Arc::new(
            OrdersManager::new(
                orders.clone(),
                orders_manager_receiver,
                to_next_conn_sender.clone(),
                request_points_result_sender,
                result_points_receiver
            )
        );

        let coffee_message_dispatcher = Arc::new(
            CoffeeMessageDispatcher::new(
                connection_status.clone(),
                orders,
                orders_from_coffee_receiver
            )
        );

        Ok(LocalServer {
            listener,
            id,
            connection_status,
            next_connection,
            to_next_conn_sender,
            to_orders_manager_sender,
            orders_manager,
            coffee_message_dispatcher,
        })
    }

    pub fn listen(&mut self) -> Result<(), ServerError> {
        let mut curr_prev_handle: Option<JoinHandle<Result<(), ConnectionError>>> = None;
        loop {
            let new_connection = task::block_on(self.listener.listen())?;
            let to_next_channel = self.to_next_conn_sender.clone();
            let to_orders_manager_channel = self.to_orders_manager_sender.clone();
            let mut previous = PrevConnection::new(
                new_connection,
                to_next_channel,
                to_orders_manager_channel
            );

            let new_prev_handle = thread::spawn(move || previous.listen());
            if self.connection_status.lock()?.is_prev_online() {
                if let Some(handle) = curr_prev_handle {
                    handle.join();
                }
            }

            curr_prev_handle = Some(new_prev_handle);

            self.connection_status.lock()?.set_prev_online();
        }
    }

    // todo crear next en nuevo hilo que este manejando los intentos de conexiones y envio de mensajes a siguiente
}
