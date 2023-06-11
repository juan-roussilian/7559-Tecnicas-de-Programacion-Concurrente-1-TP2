use std::{
    collections::HashSet,
    sync::{mpsc::Sender, Arc, Mutex},
};

use async_std::task;
use lib::{
    common_errors::ConnectionError, connection_protocol::ConnectionProtocol,
    local_connection_messages::MessageType, serializer::deserialize,
};
use log::{debug, info, warn};

use crate::{
    accounts_manager::AccountsManager,
    connection_status::ConnectionStatus,
    memory_accounts_manager::MemoryAccountsManager,
    server_messages::{
        create_maybe_we_lost_the_token_message, Diff, ServerMessage, ServerMessageType, TokenData,
    },
};

pub struct PrevConnection {
    connection: Box<dyn ConnectionProtocol + Send>,
    to_next_sender: Sender<ServerMessage>,
    to_orders_manager_sender: Sender<TokenData>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    listening_to_id: Option<usize>,
    my_id: usize,
    have_token: Arc<Mutex<bool>>,
    accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
}

impl PrevConnection {
    pub fn new(
        connection: Box<dyn ConnectionProtocol + Send>,
        to_next_sender: Sender<ServerMessage>,
        to_orders_manager_sender: Sender<TokenData>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
        my_id: usize,
        have_token: Arc<Mutex<bool>>,
        accounts_manager: Arc<Mutex<MemoryAccountsManager>>,
    ) -> PrevConnection {
        PrevConnection {
            connection,
            to_next_sender,
            to_orders_manager_sender,
            connection_status,
            listening_to_id: None,
            my_id,
            have_token,
            accounts_manager,
        }
    }

    pub fn listen(&mut self) -> Result<(), ConnectionError> {
        loop {
            let encoded = task::block_on(self.connection.recv());
            if let Err(e) = encoded {
                //if e == ConnectionError::ConnectionLost { // TODO revisar
                warn!("[PREVIOUS CONNECTION] Previous connection died");
                self.connection_status.lock()?.set_prev_offline();
                if !*self.have_token.lock()? {
                    warn!(
                        "[PREVIOUS CONNECTION] I don't have the token, maybe we lost the token, sending message"
                    );
                    let to_id = self.listening_to_id.unwrap_or(self.my_id);
                    self.to_next_sender
                        .send(create_maybe_we_lost_the_token_message(self.my_id, to_id))?;
                } else {
                    info!("[PREVIOUS CONNECTION] Previous connection died but i have the token");
                }
                return Err(ConnectionError::ConnectionLost);
                //}
                //return Ok(()); // Closed connection
            }

            let mut encoded = encoded.unwrap();
            let mut message: ServerMessage = deserialize(&mut encoded)?;

            match &mut message.message_type {
                ServerMessageType::NewConnection(diff) => {
                    info!(
                        "[PREVIOUS CONNECTION] Received new connection message from {}",
                        message.sender_id
                    );
                    self.set_listening_to_id(&message.passed_by, message.sender_id);
                    if message.sender_id == self.my_id {
                        self.update_myself_by_diff(diff);
                        continue;
                    }
                    if message.passed_by.contains(&self.my_id) {
                        debug!(
                            "[PREVIOUS CONNECTION] I have already seen this message, dropping..."
                        );
                        continue;
                    }
                    self.to_next_sender.send(message)?;
                }
                ServerMessageType::CloseConnection => {
                    info!(
                        "[PREVIOUS CONNECTION] Received close connection from {}",
                        message.sender_id
                    );
                    self.connection_status.lock()?.set_prev_offline();
                    return Ok(());
                }
                ServerMessageType::Token(data) => {
                    self.set_listening_to_id(&message.passed_by, message.sender_id);
                    debug!(
                        "[PREVIOUS CONNECTION] Received the token from {}",
                        message.sender_id
                    );
                    *self.have_token.lock()? = true;
                    self.receive_update_of_other_nodes_and_clean_my_updates(data);
                    self.to_orders_manager_sender.send(data.to_owned())?;
                }
                ServerMessageType::MaybeWeLostTheTokenTo(lost_id) => {
                    self.set_listening_to_id(&message.passed_by, message.sender_id);
                    debug!(
                        "[PREVIOUS CONNECTION] Received maybe we lost the token to {} from {}",
                        lost_id, message.sender_id
                    );
                    if *self.have_token.lock()? {
                        info!("[PREVIOUS CONNECTION] I have the token, we did't lost it");
                        continue;
                    }
                    if message.sender_id == self.my_id || message.passed_by.contains(&self.my_id) {
                        debug!(
                            "[PREVIOUS CONNECTION] I have already seen this message, dropping..."
                        );
                        continue;
                    }
                    warn!("[PREVIOUS CONNECTION] I don't have the token, maybe we lost it");
                    self.to_next_sender.send(message)?;
                }
            }
        }
    }

    fn set_listening_to_id(&mut self, passed_by: &HashSet<usize>, sender: usize) {
        if self.listening_to_id.is_none() && passed_by.is_empty() {
            info!("[PREVIOUS CONNECTION] My previous connection is {}", sender);
            self.listening_to_id = Some(sender);
        }
    }

    fn update_myself_by_diff(&mut self, diff: &Diff) {
        info!("[PREVIOUS CONNECTION] It's from me, updating database...");
        debug!(
            "[PREVIOUS CONNECTION] Updating myself with diff data {:?}",
            diff
        );
        for update in &diff.changes {
            if let Ok(mut guard) = self.accounts_manager.lock() {
                guard.update(update.id, update.amount, update.last_updated_on);
            }
        }
    }

    fn receive_update_of_other_nodes_and_clean_my_updates(&mut self, data: &mut TokenData) {
        data.remove(&self.my_id);
        for (server_id, changes) in data {
            debug!(
                "[PREVIOUS CONNECTION] There are updates from server {}",
                server_id
            );
            debug!("[PREVIOUS CONNECTION] List of changes {:?}", changes);
            for update in changes {
                if update.message_type == MessageType::AddPoints {
                    if let Ok(mut guard) = self.accounts_manager.lock() {
                        guard.add_points(
                            update.account_id,
                            update.points,
                            Some(update.last_updated_on),
                        );
                    }
                } else if update.message_type == MessageType::TakePoints {
                    if let Ok(mut guard) = self.accounts_manager.lock() {
                        guard.substract_points(
                            update.account_id,
                            update.points,
                            Some(update.last_updated_on),
                        );
                    }
                }
            }
        }
    }
}
