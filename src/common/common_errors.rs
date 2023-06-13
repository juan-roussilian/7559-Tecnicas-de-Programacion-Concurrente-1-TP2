use serde::{Deserialize, Serialize};

/// CoffeeSystemError representa los distintos tipos de error que pueden surgir durante la comunicación CoffeeMaker<->Server
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CoffeeSystemError {
    AccountNotFound,
    NotEnoughPoints,
    ConnectionLost,
    ConnectionClosed,
    SerializationError,
    UnexpectedError,
    AccountIsReserved,
}

impl From<serde_json::Error> for CoffeeSystemError {
    fn from(_: serde_json::Error) -> Self {
        CoffeeSystemError::SerializationError
    }
}

impl<T> From<std::sync::PoisonError<T>> for CoffeeSystemError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        CoffeeSystemError::UnexpectedError
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for CoffeeSystemError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        CoffeeSystemError::UnexpectedError
    }
}

impl From<std::io::Error> for CoffeeSystemError {
    fn from(_: std::io::Error) -> Self {
        CoffeeSystemError::ConnectionLost
    }
}
