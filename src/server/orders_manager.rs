use std::sync::Arc;

use std::sync::mpsc::{ Receiver, Sender };
use std::sync::{ Condvar, Mutex };

use lib::local_connection_messages::{
    CoffeeMakerRequest,
    CoffeeMakerResponse,
    MessageType,
    ResponseStatus,
};
use log::error;

use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::{ AccountChange, ServerMessage };

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,
    token_receiver: Receiver<ServerMessage>,
    to_next_sender: Sender<ServerMessage>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
}

impl OrdersManager {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        token_receiver: Receiver<ServerMessage>,
        to_next_sender: Sender<ServerMessage>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>
    ) -> OrdersManager {
        OrdersManager {
            orders,
            token_receiver,
            to_next_sender,
            request_points_channel,
            result_take_points_channel,
        }
    }

    pub fn handle_orders(&mut self) -> Result<(), ServerError> {
        loop {
            let token = self.token_receiver.recv();
            if token.is_err() {
                error!("[ORDERS MANAGER] Token channel closed");
                return Err(ServerError::ChannelError);
            }
            let token = token.unwrap();
            let mut orders = self.orders.lock()?;
            if orders.is_empty() {
                self.to_next_sender.send(token);
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
                        status: ResponseStatus::Ok /* obtener el status */,
                    },
                    0 /* obtener el id */,
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
            self.to_next_sender.send(token);
        }
    }
}
