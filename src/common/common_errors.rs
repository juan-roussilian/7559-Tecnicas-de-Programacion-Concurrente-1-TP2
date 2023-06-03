use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum ConnectionError {
    AccountNotFound,
    NotEnoughPoints,
    ConnectionLost,
    ConnectionClosed,
    SerializationError,
}

impl From<Box<bincode::ErrorKind>> for ConnectionError {
    fn from(_: Box<bincode::ErrorKind>) -> Self {
        ConnectionError::SerializationError
    }
}
