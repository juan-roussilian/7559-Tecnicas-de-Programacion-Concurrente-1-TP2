use std::sync::Arc;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Condvar, Mutex};

use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};

use crate::broadcaster::broadcast_message;
use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;
use crate::server_messages::{AccountChange, ServerMessage};

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
    result_take_points_channel: Receiver<CoffeeMakerRequest>,
}

impl OrdersManager {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        orders_cond: Arc<Condvar>,
        request_points_channel: Sender<(CoffeeMakerResponse, usize)>,
        result_take_points_channel: Receiver<CoffeeMakerRequest>,
    ) -> OrdersManager {
        OrdersManager {
            orders,
            orders_cond,
            request_points_channel,
            result_take_points_channel,
        }
    }

    pub fn handle_orders(&mut self) -> Result<(), ServerError> {
        loop {
            let mut orders = self.orders_cond.wait_while(self.orders.lock()?, |orders| {
                orders.is_empty() && !orders.have_token()
            })?;
            let adding_orders = orders.get_and_clear_adding_orders();
            for order in adding_orders {
                // TODO agregar puntos a la db local
                broadcast_message(ServerMessage::AddPoints(AccountChange {
                    account_id: order.account_id,
                    points: order.points,
                }));
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
                broadcast_message(ServerMessage::TakePoints(AccountChange {
                    account_id: result.account_id,
                    points: result.points,
                }));
            }
            self.orders_cond.notify_all();
        }
    }
}
