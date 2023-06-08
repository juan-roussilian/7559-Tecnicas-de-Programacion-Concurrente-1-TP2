use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    time::Duration,
};

use async_std::task;
use lib::{
    connection_protocol::{ConnectionProtocol, TcpConnection},
    serializer::serialize,
};
use log::{error, info};

use crate::{
    address_resolver::id_to_address,
    connection_status::ConnectionStatus,
    constants::{
        INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT, MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT,
        TO_NEXT_CONN_CHANNEL_TIMEOUT_IN_MS,
    },
    errors::ServerError,
    server_messages::{
        create_close_connection_message, create_new_connection_message, create_token_message, Diff,
        ServerMessage, ServerMessageType, TokenData,
    },
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
    next_id: usize,
    last_token: Option<ServerMessage>,
    have_token: Arc<Mutex<bool>>,
}

impl NextConnection {
    pub fn new(
        id: usize,
        peer_count: usize,
        next_conn_receiver: Receiver<ServerMessage>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
        have_token: Arc<Mutex<bool>>,
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
            next_id: id,
            last_token: None,
            have_token,
        }
    }

    fn attempt_connections(
        &mut self,
        start: usize,
        stop: usize,
        message: ServerMessage,
    ) -> Result<(), ServerError> {
        for id in start..stop {
            let result = TcpConnection::new_client_connection(id_to_address(id));
            if let Ok(connection) = result {
                self.next_id = id;
                self.connection = Some(Box::new(connection));
                self.connection_status.lock()?.set_next_online();
                if self.send_message(message.clone()).is_err() {
                    continue;
                }
                return Ok(());
            }
        }
        Err(ServerError::ConnectionLost)
    }

    fn connect_to_next(&mut self, message: ServerMessage) -> Result<(), ServerError> {
        let peer_count = self.peer_count;
        let my_id = self.id;
        if self
            .attempt_connections(my_id + 1, peer_count, message.clone())
            .is_ok()
        {
            return Ok(());
        }
        if self.attempt_connections(0, my_id, message.clone()).is_ok() {
            return Ok(());
        }
        if my_id == 0 && self.initial_connection {
            self.initial_connection = false;
            return self.attempt_connections(0, 1, message);
        }
        self.connection_status.lock()?.set_next_offline();
        return Err(ServerError::ConnectionLost);
    }

    fn try_to_connect_wait_if_offline(&mut self) {
        let mut wait = INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT;
        let message = create_new_connection_message(self.id);
        loop {
            if self.connect_to_next(message.clone()).is_ok() {
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
        let timeout = Duration::from_millis(TO_NEXT_CONN_CHANNEL_TIMEOUT_IN_MS);
        self.try_to_connect_wait_if_offline();
        if self.id == 0 {
            if self.send_message(create_token_message(self.id)).is_err() {
                error!("Failed to send initial token");
                return Err(ServerError::ConnectionLost);
            }
            info!("Sent initial token to {}", self.next_id);
        }
        loop {
            if !self.connection_status.lock()?.is_online() {
                self.try_to_connect_wait_if_offline();
            }
            let result = self.next_conn_receiver.recv_timeout(timeout);
            if let Err(e) = result {
                match e {
                    RecvTimeoutError::Timeout => {
                        // Cubre el caso en que la red no quedo propiamente formada.
                        // Nos damos cuenta cuando no estamos escuchando mensajes de nadie (prev offline)
                        // pero nosotros nos creemos conectados.
                        // Ej. Red con nodos 0, 1, 2, 3. 2 esta offline y se logra conectar con 0 (justo con el 3 no pudo)
                        // 1 cierra la conexion con 3. La red quedo con un nodo apuntando al equivocado.
                        // Al detectar que no recibimos mensajes y no tenemos intenamos unirnos nuevamente.
                        // Con algo mas de logica podemos manejar particiones...
                        let mut connected = self.connection_status.lock()?;
                        if !connected.is_prev_online() {
                            connected.set_next_offline();
                            continue;
                        }
                    }
                    RecvTimeoutError::Disconnected => {
                        error!("[TO NEXT CONNECTION] Channel error, stopping...");
                        return Err(ServerError::ChannelError);
                    }
                }
            }
            let mut message = result.unwrap();
            match &mut message.message_type {
                ServerMessageType::NewConnection(diff) => {
                    if is_in_between(self.id, message.sender_id, self.next_id, self.peer_count) {
                        self.add_data_to_diff(diff);
                        let result = self.connect_to_new_conn(message.sender_id);
                        if result.is_err() {
                            continue;
                        }
                        let new_conn = result.unwrap();
                        if self
                            .send_message(create_close_connection_message(self.id))
                            .is_err()
                        {
                            error!(
                                "[SENDER {}] Failed to notify {} of close connection",
                                self.id, self.next_id
                            );
                        }
                        self.next_id = message.sender_id;
                        self.connection = Some(new_conn);
                    }
                    message.passed_by.insert(self.id);
                    if self.send_message(message).is_err() {
                        error!(
                            "[SENDER {}] Failed to send to {} new connection message",
                            self.id, self.next_id
                        );
                    }
                }
                ServerMessageType::Token(_) => {
                    // enviar el token al siguiente
                    // si tenemos cambios de una perdida anterior donde justo teniamos el token agregarlos y limpiarlo

                    // si falla reintentar conectarnos con el/los siguiente/s
                    // si fallan todas las reconexiones, perdimos la conexion y el token no es valido
                    // guardar los cambios hechos en otro lugar (solo las sumas) para appendearlos al proximo token cuando recuperemos la conexion
                    // hacemos continue, reintentamos hasta poder
                    // si perdimos la conexion marcamos que ya no tenemos el token

                    // si no fallan todas las reconexiones (ej logramos conectarnos al siguiente del siguiente)
                    // le mandamos el token, no se perdio

                    // marcamos en un mutex que ya no tenemos el token

                    self.last_token = Some(message.clone());
                    _ = self.send_message(message);
                }
                ServerMessageType::MaybeWeLostTheTokenTo(lost_id) => {
                    let lost_id = *lost_id;
                    // si el que perdio la conexion es al que apuntamos
                    // SOLO si es al que apuntamos, que nos llegue este mensaje es que se perdio el token
                    // (llego al final de la carrera - no estaba el token circulando porque se perdio)
                    // nos conectamos con el siguiente y mandarle mensaje token guardado
                    if self.next_id == lost_id {
                        if let Some(token) = self.last_token.as_ref() {
                            if self.connect_to_next(token.clone()).is_err() {
                                error!("Error passing the token to the next, we lost connection");
                                continue;
                            }
                        }
                    }
                    message.passed_by.insert(self.id);
                    if self.send_message(message.clone()).is_err() {
                        let mut in_order = (self.next_id..lost_id).collect::<Vec<_>>();
                        if lost_id < self.next_id {
                            let ring_return = (0..lost_id).collect::<Vec<_>>();
                            in_order = (self.next_id..self.peer_count).collect::<Vec<_>>();
                            in_order.extend(ring_return);
                        }

                        for id in in_order {
                            let result = TcpConnection::new_client_connection(id_to_address(id));
                            if let Ok(connection) = result {
                                self.next_id = id;
                                self.connection = Some(Box::new(connection));
                                self.connection_status.lock()?.set_next_online();
                                if self.send_message(message.clone()).is_ok() {
                                    break;
                                }
                            }
                        }

                        if self.connection.is_some() {
                            continue;
                        }

                        // Yo perdi la conexion
                        if !self.connection_status.lock()?.is_prev_online() {
                            continue;
                        }

                        if let Some(token) = self.last_token.as_ref() {
                            if self.connect_to_next(token.clone()).is_err() {
                                error!("Error passing the token to the next, we lost connection");
                            }
                        }
                    }
                    // mas que verlo como un lost connection verlo como un maybe we lost the token, circula por la red en forma de carrera (si esta el token)
                }
                _ => {}
            }
        }
    }

    fn send_message(&mut self, message: ServerMessage) -> Result<(), ServerError> {
        let message = serialize(&message)?;
        if let Some(connection) = self.connection.as_mut() {
            if task::block_on(connection.send(&message[..])).is_err() {
                self.connection = None;
                self.connection_status.lock()?.set_next_offline();
                return Err(ServerError::ConnectionLost);
            }
            return Ok(());
        }
        Err(ServerError::ConnectionLost)
    }

    fn add_data_to_diff(&self, diff: &mut Diff) {
        // TODO llamar al accounts manager para que nos de o agregue al diff todo lo mas nuevo
    }

    fn connect_to_new_conn(
        &mut self,
        sender_id: usize,
    ) -> Result<Box<dyn ConnectionProtocol>, ServerError> {
        let result = TcpConnection::new_client_connection(id_to_address(sender_id));
        if let Ok(connection) = result {
            return Ok(Box::new(connection));
        }
        Err(ServerError::ConnectionLost)
    }
}

fn is_in_between(my_id: usize, sender_id: usize, next_id: usize, peer_count: usize) -> bool {
    // caso cerramos circulo o caso en orden
    (next_id < my_id && my_id < sender_id) || (my_id < sender_id && sender_id < next_id)
}
