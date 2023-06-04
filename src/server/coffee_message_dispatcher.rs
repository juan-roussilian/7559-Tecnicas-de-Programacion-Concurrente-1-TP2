use crate::orders_queue::OrdersQueue;
use lib::local_connection_messages::{
    CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct CoffeeMessageDispatcher {
    orders: Arc<Mutex<OrdersQueue>>,
    machine_request_receiver: Receiver<(CoffeeMakerRequest, usize)>,
    machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
}

impl CoffeeMessageDispatcher {
    pub fn new(
        orders: Arc<Mutex<OrdersQueue>>,
        machine_request_receiver: Receiver<(CoffeeMakerRequest, usize)>,
    ) -> Self {
        Self {
            orders,
            machine_request_receiver,
            machine_response_senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn dispatch_coffee_requests(
        &mut self,
        orders_request_sender: Sender<CoffeeMakerRequest>,
        orders_response_sender: Sender<(CoffeeMakerResponse, usize)>,
        orders_response_receiver: Receiver<(CoffeeMakerResponse, usize)>,
    ) {
        let handle = thread::spawn(move || {
            Self::send_coffee_responses(
                self.machine_response_senders.clone(),
                orders_response_receiver,
            );
        });

        loop {
            let new_request = self.machine_request_receiver.recv().unwrap();

            match new_request.0.message_type {
                MessageType::AddPoints => {
                    let mut orders = self.orders.lock().unwrap();
                    orders.add(new_request.0);
                    drop(orders);

                    orders_response_sender
                        .send((
                            CoffeeMakerResponse {
                                message_type: new_request.0.message_type,
                                status: ResponseStatus::Ok,
                            },
                            new_request.1,
                        ))
                        .unwrap();
                }

                MessageType::RequestPoints => {
                    let mut orders = self.orders.lock().unwrap();
                    orders.add(new_request.0);
                    // OrdersManager will be the one that sends the CoffeeMakerResponse through orders_request_sender channel in this case
                }

                _ => {
                    orders_request_sender.send(new_request.0.clone()).unwrap();
                    orders_response_sender
                        .send((
                            CoffeeMakerResponse {
                                message_type: new_request.0.message_type,
                                status: ResponseStatus::Ok,
                            },
                            new_request.1,
                        ))
                        .unwrap();
                }
            }
        }

        // how do we reach this?
        handle.join().unwrap();
    }

    fn send_coffee_responses(
        machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
        orders_response_receiver: Receiver<(CoffeeMakerResponse, usize)>,
    ) {
        loop {
            let (response, machine_id) = orders_response_receiver.recv().unwrap();
            let mut machine_senders_guard = machine_response_senders.lock().unwrap();

            match machine_senders_guard.get(&machine_id) {
                Some(sender) => {
                    sender.send(response).unwrap();
                }
                None => {
                    // TODO: What happens if we are receiving messages from a non-registered coffeemaker?
                    // Maybe we should check if the key exists way before this, when we get a request.
                }
            }
            drop(machine_senders_guard)
        }
    }
}