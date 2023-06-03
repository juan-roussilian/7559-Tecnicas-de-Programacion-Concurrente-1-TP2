use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountChange {
    pub account_id: usize,
    pub points: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    AddPoints(AccountChange),
    TakePoints(AccountChange),
}
