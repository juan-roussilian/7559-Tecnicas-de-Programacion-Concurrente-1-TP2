use std::collections::{HashMap, HashSet};

use lib::local_connection_messages::MessageType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    pub message_type: ServerMessageType,
    pub sender_id: usize,
    pub passed_by: HashSet<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ServerMessageType {
    NewConnection(Diff),
    CloseConnection,
    Token(TokenData),
    MaybeWeLostTheTokenTo(ServerId),
}

type ServerId = usize;
pub type TokenData = HashMap<usize, Vec<AccountAction>>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AccountAction {
    pub message_type: MessageType,
    pub account_id: usize,
    pub points: usize,
    pub last_updated_on: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Diff {
    pub last_update: u128,
    pub changes: Vec<UpdatedAccount>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdatedAccount {
    pub id: usize,
    pub amount: usize,
    pub last_updated_on: u128,
}

pub fn create_new_connection_message(sender_id: usize, most_recent_update: u128) -> ServerMessage {
    let diff = Diff {
        last_update: most_recent_update,
        changes: Vec::new(),
    };
    create_server_message(sender_id, ServerMessageType::NewConnection(diff))
}

pub fn create_token_message(sender_id: usize) -> ServerMessage {
    create_server_message(sender_id, ServerMessageType::Token(HashMap::new()))
}

pub fn create_maybe_we_lost_the_token_message(sender_id: usize, to_id: usize) -> ServerMessage {
    create_server_message(sender_id, ServerMessageType::MaybeWeLostTheTokenTo(to_id))
}

pub fn create_close_connection_message(sender_id: usize) -> ServerMessage {
    create_server_message(sender_id, ServerMessageType::CloseConnection)
}

pub fn recreate_token(sender_id: usize, token_data: TokenData) -> ServerMessage {
    create_server_message(sender_id, ServerMessageType::Token(token_data))
}

fn create_server_message(sender_id: usize, message_type: ServerMessageType) -> ServerMessage {
    ServerMessage {
        message_type,
        sender_id,
        passed_by: HashSet::new(),
    }
}
