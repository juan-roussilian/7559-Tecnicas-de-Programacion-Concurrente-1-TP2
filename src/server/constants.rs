/// Indica el tiempo de espera inicial para reintentar la conexion en ms
pub const INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT: u64 = 500;

/// Indica el tiempo de espera maximo antes de reiniciar los intentos de conexion en ms. 3600000 es una hora en
pub const MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT: u64 = 3600000;
