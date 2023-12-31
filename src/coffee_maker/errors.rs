#[derive(Debug, PartialEq, Eq)]
pub enum CoffeeMakerError {
    /// Ocurrio un error al intentar abrir el archivo
    FileReaderNotFoundError,

    /// Ocurrio un error al leer del archivo. Puede darse si tiene un formato equivocado
    FileReaderFormatError,

    /// Faltan argumentos al iniciar la aplicacion
    ArgsMissing,
}

impl From<std::num::ParseIntError> for CoffeeMakerError {
    fn from(_: std::num::ParseIntError) -> Self {
        CoffeeMakerError::FileReaderFormatError
    }
}
