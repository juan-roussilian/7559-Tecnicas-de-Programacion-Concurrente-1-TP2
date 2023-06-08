use std::sync::{Arc, Condvar};

use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use lib::common_errors::ConnectionError;

use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use crate::connection_status::ConnectionStatus;

use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::ServerMessage;

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    connected: Condvar,
    token_receiver: Receiver<ServerMessage>,
    to_next_sender: Sender<ServerMessage>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
}

impl OrdersManager {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
        connected: Condvar,
        token_receiver: Receiver<ServerMessage>,
        to_next_sender: Sender<ServerMessage>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>,
    ) -> OrdersManager {
        OrdersManager {
            orders,
            connection_status,
            connected,
            token_receiver,
            to_next_sender,
            request_points_channel,
            result_take_points_channel,
        }
    }

    pub fn handle_orders(&mut self) -> Result<(), ServerError> {

        let orders_clone = self.orders.clone();
        let connection_status_clone = self.connection_status.clone();
        let connection_condvar_clone = &self.connected;
        let request_points_channel_clone = self.request_points_channel.clone();

        let handle = thread::spawn(move || {
            let _ = connection_condvar_clone.wait_while(connection_status_clone.lock()?, |connection_status| {
                connection_status.is_online()
            });

            let discarded_orders = orders_clone.lock().unwrap().get_and_clear_request_points_orders();

            let response = CoffeeMakerResponse{ message_type: MessageType::RequestPoints, status: ResponseStatus::Err(ConnectionError::UnexpectedError)};
            for order in discarded_orders.iter(){
                request_points_channel_clone.send((response, order.1))?;
            }
        });

        loop {
            let token = self.token_receiver.recv()?;
            let mut orders = self.orders.lock()?;
            if orders.is_empty() {
                self.to_next_sender.send(token)?;
                continue;
            }
            let adding_orders = orders.get_and_clear_adding_orders();
            for order in adding_orders {
                // TODO agregar puntos a la db local
                // Agregar al token
            }

            let request_points_orders = orders.get_and_clear_request_points_orders();
            let mut total_request_orders = 0;
            for order in request_points_orders {
                // TODO ver si alcanzan los puntos (si hay 2 o mas sobre la misma cuenta ir acumulando en el gestor de puntos?)
                // if alcanzan los puntos {
                total_request_orders += 1;
                //}
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
                // TODO restar los puntos locales si corresponde
                // Agregar al token
            }
            self.to_next_sender.send(token)?;
        }
    }
}
