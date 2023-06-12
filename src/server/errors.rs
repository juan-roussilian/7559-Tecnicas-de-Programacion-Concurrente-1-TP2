use log::error;
use std::time::SystemTimeError;

#[derive(Debug)]
pub enum ServerError {
    ListenerError,
    AcceptError,
    ArgsMissing,
    ArgsFormat,
    LockError,
    ChannelError,
    AccountNotFound,
    NotEnoughPointsInAccount,
    ConnectionLost,
    SerializationError,
    OperationIsOutdated,
    AccountIsReserved,
    CoffeeServerStartError,
    TimestampError,
}

impl<T> From<std::sync::PoisonError<T>> for ServerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServerError::LockError
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for ServerError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        ServerError::ChannelError
    }
}

impl From<std::sync::mpsc::RecvError> for ServerError {
    fn from(_: std::sync::mpsc::RecvError) -> Self {
        ServerError::ChannelError
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(_: serde_json::Error) -> Self {
        ServerError::SerializationError
    }
}

impl From<SystemTimeError> for ServerError {
    fn from(_: SystemTimeError) -> Self {
        ServerError::TimestampError
    }
}
