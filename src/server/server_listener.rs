use std::thread::{self, JoinHandle};

use async_std::task;
use lib::common_errors::ConnectionError;

use crate::{
    connection_server::{ConnectionServer, TcpConnectionServer},
    errors::ServerError,
    previous_connection::PrevConnection,
};

pub struct LocalServer {
    id: usize,
    listener: Box<dyn ConnectionServer>,
    next_connection: ConnectionStatus,
    prev_connection: ConnectionStatus,
}

#[derive(Debug, PartialEq, Eq)]
enum ConnectionStatus {
    Connected,
    Disconnected,
}

fn id_to_server_port(id: usize) -> String {
    let port = id + 10000;
    port.to_string()
}

impl LocalServer {
    pub fn new(id: usize) -> Result<LocalServer, ServerError> {
        let listener: Box<dyn ConnectionServer> =
            Box::new(TcpConnectionServer::new(&id_to_server_port(id))?);
        Ok(LocalServer {
            listener,
            id,
            prev_connection: ConnectionStatus::Disconnected,
            next_connection: ConnectionStatus::Disconnected,
        })
    }

    pub fn listen(&mut self) -> Result<(), ServerError> {
        let mut curr_prev_handle: Option<JoinHandle<Result<(), ConnectionError>>> = None;
        //let (curr_machine_response_sender, curr_machine_response_receiver) = mpsc::channel();
        loop {
            let mut new_connection = task::block_on(self.listener.listen())?;
            let previous = PrevConnection::new(new_connection);

            // un channel hacia el next
            let new_prev_handle = thread::spawn(move || previous.listen());
            if self.prev_connection == ConnectionStatus::Connected {
                if let Some(handle) = curr_prev_handle {
                    handle.join();
                }
            }

            curr_prev_handle = Some(new_prev_handle);

            self.prev_connection = ConnectionStatus::Connected;
        }
    }

    // todo crear next en nuevo hilo que este manejando los intentos de conexiones y envio de mensajes a siguiente
}
