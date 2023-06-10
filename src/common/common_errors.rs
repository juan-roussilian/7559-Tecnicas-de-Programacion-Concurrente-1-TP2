use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConnectionError {
    AccountNotFound,
    NotEnoughPoints,
    ConnectionLost,
    ConnectionClosed,
    SerializationError,
    UnexpectedError,
}

impl From<Box<bincode::ErrorKind>> for ConnectionError {
    fn from(_: Box<bincode::ErrorKind>) -> Self {
        ConnectionError::SerializationError
    }
}

impl<T> From<std::sync::PoisonError<T>> for ConnectionError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ConnectionError::UnexpectedError
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for ConnectionError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        ConnectionError::UnexpectedError
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(_: std::io::Error) -> Self {
        ConnectionError::ConnectionLost
    }
}
