use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Duration,
};

use async_std::task;
use lib::connection_protocol::{ConnectionProtocol, TcpConnection};
use log::error;

use crate::{
    address_resolver::id_to_address,
    connection_status::ConnectionStatus,
    constants::{INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT, MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT},
    errors::ServerError,
    server_messages::ServerMessage,
};

use self::sync::sleep;

mod sync {
    use std::thread;
    use std::time::Duration;

    #[cfg(not(test))]
    pub(crate) fn sleep(d: Duration) {
        thread::sleep(d);
    }

    #[cfg(test)]
    pub(crate) fn sleep(_: Duration) {
        thread::yield_now();
    }
}

pub struct NextConnection {
    id: usize,
    peer_count: usize,
    next_conn_receiver: Receiver<ServerMessage>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    connection: Option<Box<dyn ConnectionProtocol>>,
    initial_connection: bool,
}

impl NextConnection {
    pub fn new(
        id: usize,
        peer_count: usize,
        next_conn_receiver: Receiver<ServerMessage>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
    ) -> NextConnection {
        let mut initial_connection = false;
        if id == 0 {
            initial_connection = true;
        }
        NextConnection {
            id,
            peer_count,
            next_conn_receiver,
            connection_status,
            connection: None,
            initial_connection,
        }
    }

    fn attempt_connections(&mut self, start: usize, stop: usize) -> Result<(), ServerError> {
        for id in start..stop {
            let result = TcpConnection::new_client_connection(id_to_address(id));
            if let Ok(connection) = result {
                self.connection = Some(Box::new(connection));
                self.connection_status.lock()?.set_next_online();
                return Ok(());
            }
        }
        Err(ServerError::ConnectionLost)
    }

    fn connect_to_next(&mut self) -> Result<(), ServerError> {
        let peer_count = self.peer_count;
        let my_id = self.id;
        if self.attempt_connections(my_id + 1, peer_count).is_ok() {
            return Ok(());
        }
        if self.attempt_connections(0, my_id).is_ok() {
            return Ok(());
        }
        if my_id == 0 && self.initial_connection {
            self.initial_connection = false;
            return self.attempt_connections(0, 1);
        }
        self.connection_status.lock()?.set_next_offline();
        return Err(ServerError::ConnectionLost);
    }

    fn try_to_connect_wait_if_offline(&mut self) {
        let mut wait = INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT;
        loop {
            if self.connect_to_next().is_ok() {
                return;
            }
            sleep(Duration::from_millis(wait));
            wait *= 2;
            if wait >= MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT {
                wait = INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT;
            }
        }
    }

    pub fn handle_message_to_next(&mut self) -> Result<(), ServerError> {
        self.try_to_connect_wait_if_offline();
        if self.id == 0 {
            // send initial token to connection
        }
        loop {
            if !self.connection_status.lock()?.is_online() {
                self.try_to_connect_wait_if_offline();
                // todo send new connection message to next server
            }
            let result = self.next_conn_receiver.recv();
            if result.is_err() {
                error!("[TO NEXT CONNECTION] Channel error, stopping...");
                return Err(ServerError::ChannelError);
            }
            let message = result.unwrap();
            // TODO si el message es de nueva conexion, revisar de quien es, si el nuevo debe de ser next agregarle lo de diff,
            // cerrar conexion vieja y reconectar

            if let Some(connection) = self.connection.as_mut() {
                // TODO handle message conversion
                if task::block_on(connection.send(&[])).is_err() {
                    self.connection = None;
                    self.connection_status.lock()?.set_next_offline();
                }
            }
        }
    }
}
