/// Indica el tiempo de espera inicial para reintentar la conexion en ms
pub const INITIAL_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT: u64 = 500;

/// Indica el tiempo de espera maximo antes de reiniciar los intentos de conexion en ms. 3600000 es una hora en
pub const MAX_WAIT_IN_MS_FOR_CONNECTION_ATTEMPT: u64 = 3600000;

/// Es el tiempo de timeout que tiene el sender hacia la next connection.
/// Si no recibe nada en este tiempo revisa si esta conectado. Si lo esta se toma como que hay demora.
pub const TO_NEXT_CONN_CHANNEL_TIMEOUT_IN_MS: u64 = 5000;
