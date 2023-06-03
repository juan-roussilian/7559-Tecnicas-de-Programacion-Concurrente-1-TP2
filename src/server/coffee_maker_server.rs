use async_std::task;
use lib::common_errors::ConnectionError;
use lib::local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse};
use log::error;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
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
        new_response_senders: Sender<(Sender<CoffeeMakerResponse>, usize)>,
    ) -> Result<(), ServerError> {
        let mut curr_machine_id = 0;
        loop {
            let (curr_machine_response_sender, curr_machine_response_receiver) = mpsc::channel();
            if let Err(e) =
                new_response_senders.send((curr_machine_response_sender, curr_machine_id))
            {
                error!("trying to send on a channel without receiver");
                return Err(ServerError::ListenerError);
            }

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
