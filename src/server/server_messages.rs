use std::{ time::{ Instant, Duration }, collections::{ HashMap, HashSet } };

use chrono::{ DateTime, Local };
use lib::local_connection_messages::CoffeeMakerRequest;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    pub message_type: ServerMessageType,
    pub sender_id: usize,
    pub passed_by: HashSet<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessageType {
    NewConnection(Diff),
    CloseConnection,
    Token(TokenData),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenData {
    changes: HashMap<usize, Vec<CoffeeMakerRequest>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Diff {
    last_update: Duration,
    changes: Vec<UpdatedAccount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatedAccount {
    pub id: usize,
    pub amount: usize,
    pub last_updated_on: Duration,
}
