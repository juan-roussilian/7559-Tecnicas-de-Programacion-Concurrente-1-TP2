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
}

impl<T> From<std::sync::PoisonError<T>> for ServerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServerError::LockError
    }
}
