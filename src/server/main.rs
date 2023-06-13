use std::env;

use errors::ServerError;
use lib::logger::set_logger_config;
use local_server::LocalServer;
use log::error;
use server_args::ServerArgs;
/// Modulo utilizado para representar una cuenta de un cliente de la cafeteria
pub mod account;
/// Abstraccion utilizada para representar una manejador de cuentas de un cliente de la cafeteria
pub mod accounts_manager;
/// Modulo de funciones tipo helper para mapear id de servidores a conexiones IPs
pub mod address_resolver;
/// Modulo que realiza la comunicacion con la cafetera
pub mod coffee_maker_connection;
/// Modulo que crea hilos para las conexiones con cada cafetera
pub mod coffee_maker_server;
/// Modulo que despacha mensajes entrantes de todas las cafeteras hacia el OrdersManager de ser posible
pub mod coffee_message_dispatcher;
/// Modulo de conexion entre servidores
pub mod connection_server;
/// Modulo utilizado por los servidores para realizar consultas y alterar los estados de los peers vecinos del ring
pub mod connection_status;
/// Modulo donde se encuentran las constantes definidas para el funcionamiento correcto del servidor
pub mod constants;
/// Modulo de errores que utiliza unicamente el servidor
pub mod errors;
/// Modulo que representa al servidor
pub mod local_server;
/// Modulo que contiene una implementacion implementacion de manejador de cuentas en memoria
pub mod memory_accounts_manager;
/// Modulo que representa la conexion de un servidor con el peer siguiente del token ring
pub mod next_connection;
/// Modulo que representa un limpiador de ordenes de resta de puntos en caso de que un servidor este offline
pub mod offline_substract_orders_cleaner;
/// Modulo que maneja las ordenes de la cafetera
pub mod orders_manager;
/// Modulo que representa una cola de ordenes
pub mod orders_queue;
/// Modulo que representa la conexion de un servidor con el peer anterior del token ring
pub mod previous_connection;
/// Modulo que representa los parametros que recibe el servidor al ejecutarse
pub mod server_args;
/// Modulo que contiene los posibles mensajes que pueden intercambiar los servidores pares
pub mod server_messages;

fn get_args() -> Result<ServerArgs, ServerError> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 {
        let parsed_id: Result<usize, _> = args[1].clone().trim().parse();
        let parsed_peer_count: Result<usize, _> = args[2].clone().trim().parse();

        return match (parsed_id, parsed_peer_count) {
            (Ok(id), Ok(peer_server_count)) => Ok(ServerArgs {
                id,
                peer_server_count,
            }),
            (_, _) => Err(ServerError::ArgsFormat),
        };
    }
    Err(ServerError::ArgsMissing)
}

fn main() {
    set_logger_config();
    let server_args_res = get_args();
    if server_args_res.is_err() {
        error!("Error setting args. Use [ID] [PEER_COUNT]");
        return;
    }
    let server_args = server_args_res.unwrap();

    let result = LocalServer::new(server_args.id, server_args.peer_server_count);
    if result.is_err() {
        error!("Error booting up local server, stopping...");
        return;
    }
    let mut server = result.unwrap();
    server.start_server();
}
