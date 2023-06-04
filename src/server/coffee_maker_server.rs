use async_std::task;
use lib::common_errors::ConnectionError;
use lib::local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::{
    coffee_maker_connection::receive_messages_from_coffee_maker,
    connection_server::{ConnectionServer, TcpConnectionServer},
    errors::ServerError,
};

pub struct CoffeeMakerServer {
    listener: Box<dyn ConnectionServer>,
    coffee_machines_connections: Vec<JoinHandle<Result<(), ConnectionError>>>,
}

fn id_to_coffee_port(id: usize) -> String {
    let port = id + 20000;
    port.to_string()
}

impl CoffeeMakerServer {
    pub fn new(id: usize) -> Result<CoffeeMakerServer, ServerError> {
        let listener: Box<dyn ConnectionServer> =
            Box::new(TcpConnectionServer::new(&id_to_coffee_port(id))?);
        Ok(CoffeeMakerServer {
            listener,
            coffee_machines_connections: Vec::new(),
        })
    }

    pub fn listen(
        &mut self,
        coffee_request_sender: Sender<(CoffeeMakerRequest, usize)>,
        machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
    ) -> Result<(), ServerError> {
        let mut curr_machine_id = 0;
        loop {
            let (curr_machine_response_sender, curr_machine_response_receiver) = mpsc::channel();

            let mut machine_senders_guard = machine_response_senders.clone().lock().unwrap();
            machine_senders_guard.insert(curr_machine_id, curr_machine_response_sender);
            drop(machine_senders_guard);

            let curr_machine_request_sender = coffee_request_sender.clone();
            let mut new_conn_result = task::block_on(self.listener.listen())?;
            let handle = thread::spawn(move || {
                receive_messages_from_coffee_maker(
                    &mut new_conn_result,
                    curr_machine_id,
                    curr_machine_request_sender,
                    curr_machine_response_receiver,
                )
            });
            self.coffee_machines_connections.push(handle);
            curr_machine_id += 1;
        }
    }
}
