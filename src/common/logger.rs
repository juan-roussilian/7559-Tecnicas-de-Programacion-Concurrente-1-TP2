use std::env;

/// Configura el nivel de logs en base a las variables de entorno. Devuelve error en caso de fallar
/// con el nivel establecido o con el nivel default.
pub fn set_logger_config() {
    if env::var("RUST_LOG").is_err() {
        if let Err(err) = simple_logger::init_with_level(log::Level::Debug) {
            println!("Error setting logger to default value. Error: {:?}", err);
        }
    } else if let Err(err) = simple_logger::init_with_env() {
        println!("Error setting logger: {:?}", err);
    }
}
