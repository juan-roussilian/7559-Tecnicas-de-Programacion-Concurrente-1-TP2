use log::error;

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
}

impl<T> From<std::sync::PoisonError<T>> for ServerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServerError::LockError
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for ServerError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        error!("Send error on channel, closed");
        ServerError::ChannelError
    }
}

impl From<std::sync::mpsc::RecvError> for ServerError {
    fn from(_: std::sync::mpsc::RecvError) -> Self {
        error!("Recv error on channel, closed");
        ServerError::ChannelError
    }
}

impl From<Box<bincode::ErrorKind>> for ServerError {
    fn from(_: Box<bincode::ErrorKind>) -> Self {
        error!("Error serializing message");
        ServerError::SerializationError
    }
}
