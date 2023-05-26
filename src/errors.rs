#[derive(Debug, PartialEq, Eq)]
pub enum CoffeeMakerError {
    /// Ocurrio un error en la lectura del archivo, ya sea porque no existe o tiene un formato equivocado
    FileReaderError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServerError {
    AccountNotFound,
    NotEnoughPoints,
    ConnectionLost,
}
