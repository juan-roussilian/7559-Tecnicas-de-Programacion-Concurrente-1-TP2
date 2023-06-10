use async_std::task;
use lib::{
    connection_protocol::{ConnectionProtocol, TcpConnection},
    local_connection_messages::MessageType,
    serializer::serialize,
};
use log::{debug, error, info, warn};
use std::{
    sync::{
        mpsc::{Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::{
    accounts_manager::AccountsManager,
    address_resolver::id_to_address,
    connection_status::ConnectionStatus,
    constants::{
        CLEAN_ORDERS_TIME_IN_MS, INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT,
        MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT, TO_NEXT_CONN_CHANNEL_TIMEOUT_IN_MS,
    },
    errors::ServerError,
    memory_accounts_manager::MemoryAccountsManager,
    offline_substract_orders_cleaner::SubstractOrdersCleaner,
    server_messages::{
        create_close_connection_message, create_new_connection_message, create_token_message, Diff,
        ServerMessage, ServerMessageType,
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
    connection: Option<TcpConnection>,
    initial_connection: bool,
    next_id: usize,
    last_token: Option<ServerMessage>,
    have_token: Arc<Mutex<bool>>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
    offline_cleaner: SubstractOrdersCleaner,
}

impl NextConnection {
    pub fn new(
        id: usize,
        peer_count: usize,
        next_conn_receiver: Receiver<ServerMessage>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
        have_token: Arc<Mutex<bool>>,
        accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
        offline_cleaner: SubstractOrdersCleaner,
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
            accounts_manager,
            offline_cleaner,
        }
    }

    fn attempt_connections(
        &mut self,
        start: usize,
        stop: usize,
        message: ServerMessage,
    ) -> Result<(), ServerError> {
        for id in start..stop {
            let result = TcpConnection::new_client_connection(&id_to_address(id));
            if let Ok(connection) = result {
                self.next_id = id;
                self.connection = Some(connection);
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
        Err(ServerError::ConnectionLost)
    }

    fn try_to_connect_wait_if_offline(&mut self) -> Result<(), ServerError> {
        let mut cleaned_orders = false;
        let mut wait = INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT;
        let most_recent_update = self.accounts_manager.lock()?.get_most_recent_update();
        let message = create_new_connection_message(self.id, most_recent_update);
        loop {
            if self.connect_to_next(message.clone()).is_ok() {
                return Ok(());
            }
            sleep(Duration::from_millis(wait));
            wait *= 2;
            if wait >= MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT {
                wait = INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT;
            }
            if wait >= CLEAN_ORDERS_TIME_IN_MS && !cleaned_orders {
                self.offline_cleaner.clean_substract_orders_if_offline()?;
                cleaned_orders = false;
            }
        }
    }

    pub fn handle_message_to_next(&mut self) -> Result<(), ServerError> {
        let timeout = Duration::from_millis(TO_NEXT_CONN_CHANNEL_TIMEOUT_IN_MS);
        let mut pending_sums = vec![];
        if self.id == 0 {
            self.try_to_connect_wait_if_offline()?;
            if self.send_message(create_token_message(self.id)).is_err() {
                error!("Failed to send initial token");
                return Err(ServerError::ConnectionLost);
            }
            info!("Sent initial token to {}", self.next_id);
        }
        loop {
            if !self.connection_status.lock()?.is_online() {
                self.try_to_connect_wait_if_offline()?;
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
                        }
                        continue;
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
                    if is_in_between(self.id, message.sender_id, self.next_id) {
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
                ServerMessageType::Token(token_data) => {
                    let mut token_data_copy = token_data.clone();

                    if !pending_sums.is_empty() {
                        token_data
                            .entry(self.id)
                            .or_insert(pending_sums.clone())
                            .append(&mut pending_sums);
                    }

                    let token_backup = Some(message.clone());
                    // enviar el token al siguiente
                    if self.send_message(message.clone()).is_err() {
                        // si tenemos cambios de una perdida anterior donde justo teniamos el token agregarlos y limpiarlo
                        // si falla reintentar conectarnos con el/los siguiente/s
                        if self.connect_to_next(message).is_err() {
                            // si fallan todas las reconexiones, perdimos la conexion y el token no es valido
                            // guardar los cambios hechos en otro lugar (solo las sumas) para appendearlos al proximo token cuando recuperemos la conexion
                            // hacemos continue, reintentamos hasta poder
                            if let Some(requests) = token_data_copy.remove(&self.id) {
                                let mut sums = requests
                                    .into_iter()
                                    .filter(|req| req.message_type == MessageType::AddPoints)
                                    .collect::<Vec<_>>();
                                pending_sums.append(&mut sums);
                            }
                            // marcamos en un mutex que ya no tenemos el token, estamos sin conexion
                            *self.have_token.lock()? = false;
                            continue;
                        }
                    }
                    // marcamos en un mutex que ya no tenemos el token
                    *self.have_token.lock()? = false;
                    // si no fallan todas las reconexiones (ej logramos conectarnos al siguiente del siguiente)
                    // le mandamos el token, no se perdio
                    self.last_token = token_backup;
                    pending_sums.clear();
                }
                ServerMessageType::MaybeWeLostTheTokenTo(lost_id) => {
                    let lost_id = *lost_id;
                    // si el que perdio la conexion es al que apuntamos
                    // SOLO si es al que apuntamos, que nos llegue este mensaje es que se perdio el token
                    // (llego al final de la carrera - no estaba el token circulando porque se perdio)
                    // nos conectamos con el siguiente y mandarle mensaje token guardado
                    if *self.have_token.lock()? {
                        info!("[SENDER {}] I have the token, we did't lost it", self.id);
                        continue;
                    }

                    if self.next_id == lost_id {
                        warn!(
                            "[SENDER {}] We lost the token, sending copy to next possible connection",
                            self.id
                        );
                        if let Some(token) = self.last_token.as_ref() {
                            if self.connect_to_next(token.clone()).is_err() {
                                error!(
                                    "[SENDER {}] Error passing the token to the next, we lost connection",
                                    self.id
                                );
                                continue;
                            }
                            info!(
                                "[SENDER {}] We managed to send the copy of the token to {}",
                                self.id, self.next_id
                            );
                            continue;
                        }
                    }
                    message.passed_by.insert(self.id);
                    if self.send_message(message.clone()).is_err() {
                        error!(
                            "[SENDER {}] Next is offline, trying to contact nodes after me and initial lost server",
                            self.id
                        );
                        let mut in_order = (self.next_id..lost_id).collect::<Vec<_>>();
                        if lost_id < self.next_id {
                            let ring_return = (0..lost_id).collect::<Vec<_>>();
                            in_order = (self.next_id..self.peer_count).collect::<Vec<_>>();
                            in_order.extend(ring_return);
                        }

                        for id in in_order {
                            let result = TcpConnection::new_client_connection(&id_to_address(id));
                            if let Ok(connection) = result {
                                self.next_id = id;
                                self.connection = Some(connection);
                                self.connection_status.lock()?.set_next_online();
                                if self.send_message(message.clone()).is_ok() {
                                    break;
                                }
                            }
                        }

                        if self.connection.is_some() {
                            info!(
                                "[SENDER {}] Sent Maybe We Lost The token to {} in between me and lost node",
                                self.id,
                                self.next_id
                            );
                            continue;
                        }

                        // Yo perdi la conexion
                        if !self.connection_status.lock()?.is_prev_online() {
                            error!("[SENDER {}] I lost connection", self.id);
                            continue;
                        }

                        if let Some(token) = self.last_token.as_ref() {
                            warn!(
                                "[SENDER {}] The token was lost between {} and {}, sending copy to next possible connection",
                                self.id,
                                self.id,
                                lost_id
                            );
                            if self.connect_to_next(token.clone()).is_err() {
                                error!(
                                    "[SENDER {}] Error passing the token to the next, we lost connection",
                                    self.id
                                );
                                continue;
                            }
                            info!(
                                "[SENDER {}] Managed to send a copy of the token to next connection {}",
                                self.id,
                                self.next_id
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn send_message(&mut self, message: ServerMessage) -> Result<(), ServerError> {
        let message = serialize(&message)?;
        if let Some(connection) = self.connection.as_mut() {
            thread::sleep(Duration::from_millis(1000));
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
        info!(
            "[SENDER {}] New connection is in between, adding diff data to update server",
            self.id
        );
        if let Ok(accounts) = self.accounts_manager.lock() {
            let update = accounts.get_accounts_updated_after(diff.last_update);
            diff.changes = update;
            return;
        }
        error!(
            "[SENDER {}] Error adding update to new connection message",
            self.id
        );
    }

    fn connect_to_new_conn(&mut self, sender_id: usize) -> Result<TcpConnection, ServerError> {
        let result = TcpConnection::new_client_connection(&id_to_address(sender_id));
        if let Ok(connection) = result {
            return Ok(connection);
        }
        Err(ServerError::ConnectionLost)
    }
}

fn is_in_between(my_id: usize, sender_id: usize, next_id: usize) -> bool {
    // caso cerramos circulo o caso en orden
    // (next_id < my_id && my_id < sender_id) || (my_id < sender_id && sender_id < next_id)
    (sender_id < next_id || next_id <= my_id) && my_id < sender_id
}
