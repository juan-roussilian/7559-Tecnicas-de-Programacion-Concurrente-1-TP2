use async_std::task;
use lib::common_errors::CoffeeSystemError;
use lib::local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::address_resolver::id_to_coffee_port;
use crate::{
    coffee_maker_connection::receive_messages_from_coffee_maker,
    connection_server::{ConnectionServer, TcpConnectionServer},
    errors::ServerError,
};

/// Representa un servidor que escucha nuevas conexiones de cafeteras. Además de su listener,
/// contiene el Sender channel de CoffeeMakerRequest que clonará para cada cafetera conectada,
/// y un mutex de un diccionario donde nos guardaremos el Sender channel de CoffeeMakerResponse
/// para cada id de cafetera.
pub struct CoffeeMakerServer {
    listener: TcpConnectionServer,
    coffee_machines_connections: Vec<JoinHandle<Result<(), CoffeeSystemError>>>,
    coffee_request_sender: Sender<(CoffeeMakerRequest, usize)>,
    machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
}

impl CoffeeMakerServer {
    /// Devuelve un nuevo CoffeeMakerServer, o error en caso de no poder abrir un nuevo listener.
    pub fn new(
        id: usize,
        coffee_request_sender: Sender<(CoffeeMakerRequest, usize)>,
        machine_response_senders: Arc<Mutex<HashMap<usize, Sender<CoffeeMakerResponse>>>>,
    ) -> Result<CoffeeMakerServer, ServerError> {
        let listener = TcpConnectionServer::new(&id_to_coffee_port(id))?;
        Ok(CoffeeMakerServer {
            listener,
            coffee_machines_connections: Vec::new(),
            coffee_request_sender,
            machine_response_senders,
        })
    }

    /// Escucha por nuevas conexiones entrantes de cafeteras, y para cada una de ellas las registra en el diccionario interno.
    /// Además levanta un hilo donde se llama a receive_messages_from_coffee_maker() para esa cafetera en específico.
    pub fn listen(&mut self) -> Result<(), ServerError> {
        let mut curr_machine_id = 0;
        loop {
            let (curr_machine_response_sender, curr_machine_response_receiver) = mpsc::channel();

            {
                let mut machine_senders_guard = self.machine_response_senders.lock().unwrap();
                machine_senders_guard.insert(curr_machine_id, curr_machine_response_sender);
            }

            let curr_machine_request_sender = self.coffee_request_sender.clone();
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
