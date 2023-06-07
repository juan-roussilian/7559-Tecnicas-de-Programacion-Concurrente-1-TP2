use std::{
    collections::HashSet,
    sync::{mpsc::Sender, Arc, Mutex},
};

use async_std::task;
use lib::{
    common_errors::ConnectionError, connection_protocol::ConnectionProtocol,
    local_connection_messages::MessageType, serializer::deserialize,
};

use crate::{
    connection_status::ConnectionStatus,
    server_messages::{Diff, ServerMessage, ServerMessageType, TokenData},
};

pub struct PrevConnection {
    connection: Box<dyn ConnectionProtocol + Send>,
    to_next_sender: Sender<ServerMessage>,
    to_orders_manager_sender: Sender<ServerMessage>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    listening_to_id: Option<usize>,
    my_id: usize,
}

impl PrevConnection {
    pub fn new(
        connection: Box<dyn ConnectionProtocol + Send>,
        to_next_sender: Sender<ServerMessage>,
        to_orders_manager_sender: Sender<ServerMessage>,
        connection_status: Arc<Mutex<ConnectionStatus>>,
        my_id: usize,
    ) -> PrevConnection {
        PrevConnection {
            connection,
            to_next_sender,
            to_orders_manager_sender,
            connection_status,
            listening_to_id: None,
            my_id,
        }
    }

    fn create_lost_connection_message(&self) -> ServerMessage {
        let to_id = self.listening_to_id.unwrap_or(self.my_id);
        ServerMessage {
            sender_id: self.my_id,
            message_type: ServerMessageType::LostConnection(to_id),
            passed_by: HashSet::new(),
        }
    }

    pub fn listen(&mut self) -> Result<(), ConnectionError> {
        loop {
            let encoded = task::block_on(self.connection.recv());
            if encoded.is_err() {
                self.connection_status.lock()?.set_prev_offline();
                self.to_next_sender
                    .send(self.create_lost_connection_message())?;
                return Err(ConnectionError::ConnectionLost);
            }
            let mut encoded = encoded.unwrap();
            let mut message: ServerMessage = deserialize(&mut encoded)?;

            match &mut message.message_type {
                ServerMessageType::NewConnection(diff) => {
                    self.set_listening_to_id(&message.passed_by, message.sender_id);
                    if message.sender_id == self.my_id {
                        self.update_myself_by_diff(diff);
                        continue;
                    }
                    self.to_next_sender.send(message)?;
                }
                ServerMessageType::CloseConnection => {
                    self.connection_status.lock()?.set_prev_offline();
                    return Ok(());
                }
                ServerMessageType::Token(data) => {
                    self.receive_update_of_other_nodes_and_clean_my_updates(data);
                    self.to_orders_manager_sender.send(message)?;
                }
                ServerMessageType::LostConnection(_) => {
                    self.to_orders_manager_sender.send(message)?;
                }
            }
        }
    }

    fn set_listening_to_id(&mut self, passed_by: &HashSet<usize>, sender: usize) {
        if self.listening_to_id.is_none() && passed_by.is_empty() {
            self.listening_to_id = Some(sender);
        }
    }

    fn update_myself_by_diff(&mut self, diff: &Diff) {
        for update in &diff.changes {
            // update accounts
        }
    }

    fn receive_update_of_other_nodes_and_clean_my_updates(&mut self, data: &mut TokenData) {
        data.remove(&self.my_id);
        for changes in data.values() {
            for update in changes {
                if update.message_type == MessageType::AddPoints {
                    // update account
                } else if update.message_type == MessageType::TakePoints {
                    // update account
                }
            }
        }
    }
}
