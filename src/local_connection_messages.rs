use crate::errors::ServerError;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CoffeeMakerRequest {
    pub message_type: MessageType,
    pub account_id: usize,
    pub points: usize,
}

#[derive(Deserialize)]
pub struct CoffeeMakerResponse {
    pub message_type: MessageType,
    pub status: ResponseStatus,
}

#[derive(Deserialize)]
pub enum ResponseStatus {
    Ok,
    Err(ServerError),
}

#[derive(Serialize, Deserialize)]
pub enum MessageType {
    AddPoints,
    RequestPoints,
    TakePoints,
    CancelPointsRequest,
}
