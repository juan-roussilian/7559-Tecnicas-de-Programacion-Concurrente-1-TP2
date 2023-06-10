use crate::connection_status::ConnectionStatus;
use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;
use lib::common_errors::ConnectionError;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use log::{error, info};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct CoffeeMessageDispatcher {
    is_connected: Arc<Mutex<ConnectionStatus>>,
    orders: Arc<Mutex<OrdersQueue>>,
    machine_request_receiver: Receiver<(CoffeeMakerRequest, usize)>,
    machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
}

impl CoffeeMessageDispatcher {
    pub fn new(
        is_connected: Arc<Mutex<ConnectionStatus>>,
        orders: Arc<Mutex<OrdersQueue>>,
        machine_request_receiver: Receiver<(CoffeeMakerRequest, usize)>,
        machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
    ) -> Self {
        Self {
            is_connected,
            orders,
            machine_request_receiver,
            machine_response_senders,
        }
    }

    pub fn dispatch_coffee_requests(
        &mut self,
        orders_request_sender: Sender<CoffeeMakerRequest>,
        orders_response_sender: Sender<(CoffeeMakerResponse, usize)>,
        orders_response_receiver: Receiver<(CoffeeMakerResponse, usize)>,
    ) -> Result<(), ServerError> {
        let senders_clone = self.machine_response_senders.clone();
        let handle = thread::spawn(move || {
            Self::send_coffee_responses(senders_clone, orders_response_receiver);
        });

        loop {
            let new_request = self.machine_request_receiver.recv()?;

            match new_request.0.message_type {
                MessageType::AddPoints => {
                    {
                        let orders = self.orders.lock();
                        if orders.is_err() {
                            return Err(ServerError::LockError);
                        }
                        let mut orders = orders.unwrap();

                        orders.add(new_request.0, new_request.1);
                    }

                    orders_response_sender.send((
                        CoffeeMakerResponse {
                            message_type: new_request.0.message_type,
                            status: ResponseStatus::Ok,
                        },
                        new_request.1,
                    ))?;
                }

                MessageType::RequestPoints => {
                    let is_now_connected = self.is_connected.lock();
                    if is_now_connected.is_err() {
                        return Err(ServerError::LockError);
                    }
                    let is_now_connected = is_now_connected.unwrap().is_online();

                    if !is_now_connected {
                        orders_response_sender.send((
                            CoffeeMakerResponse {
                                message_type: new_request.0.message_type,
                                status: ResponseStatus::Err(ConnectionError::ConnectionLost),
                            },
                            new_request.1,
                        ))?;
                    }

                    let orders = self.orders.lock();
                    if orders.is_err() {
                        return Err(ServerError::LockError);
                    }
                    let mut orders = orders.unwrap();
                    orders.add(new_request.0, new_request.1); // TODO agregar de que cafetera
                                                              // OrdersManager will be the one that sends the CoffeeMakerResponse through orders_request_sender channel in this case
                }

                _ => {
                    orders_request_sender.send(new_request.0)?;
                    orders_response_sender.send((
                        CoffeeMakerResponse {
                            message_type: new_request.0.message_type,
                            status: ResponseStatus::Ok,
                        },
                        new_request.1,
                    ))?;
                }
            }
        }
    }

    fn send_coffee_responses(
        machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
        orders_response_receiver: Receiver<(CoffeeMakerResponse, usize)>,
    ) {
        loop {
            let next_response = orders_response_receiver.recv();
            if next_response.is_err() {
                return; // the sender has disconnected, no more responses.
            }
            let (response, machine_id) = next_response.unwrap();

            let machine_senders_guard = machine_response_senders.lock();
            if machine_senders_guard.is_err() {
                error!("Unable to lock senders for sending response")
            }
            let machine_senders = machine_senders_guard.unwrap();
            if let Some(sender) = machine_senders.get(&machine_id) {
                if sender.send(response).is_err() {
                    info!("Trying to send response through closed coffee maker channel")
                }
            }
        }
    }
}
