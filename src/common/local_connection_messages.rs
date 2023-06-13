use serde::{Deserialize, Serialize};

use crate::common_errors::CoffeeSystemError;

/// Representa un pedido desde la cafetera hacia el servidor local.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CoffeeMakerRequest {
    pub message_type: MessageType,
    pub account_id: usize,
    pub points: usize,
}

/// Representa una respuesta desde el servidor local hacia la cafetera.
#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct CoffeeMakerResponse {
    pub message_type: MessageType,
    pub status: ResponseStatus,
}

/// Enumera los estados posibles de una CoffeeMakerResponse
#[derive(Deserialize, Serialize, Copy, Clone)]
pub enum ResponseStatus {
    Ok,
    Err(CoffeeSystemError),
}

/// Enumera los distintos tipos de mensajes en la comunicación Cafetera<->Servidor
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    AddPoints,
    RequestPoints,
    TakePoints,
    CancelPointsRequest,
}
