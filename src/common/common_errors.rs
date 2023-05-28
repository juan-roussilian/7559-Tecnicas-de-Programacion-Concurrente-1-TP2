use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub enum ServerError {
    AccountNotFound,
    NotEnoughPoints,
    ConnectionLost,
    SerializationError,
}

impl From<Box<bincode::ErrorKind>> for ServerError {
    fn from(_: Box<bincode::ErrorKind>) -> Self {
        ServerError::SerializationError
    }
}
