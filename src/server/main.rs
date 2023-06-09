use std::env;

use errors::ServerError;
use lib::logger::set_logger_config;
use local_server::LocalServer;
use log::error;
use server_args::ServerArgs;

pub mod coffee_maker_connection;
pub mod coffee_maker_server;
pub mod coffee_message_dispatcher;
pub mod connection_server;
pub mod errors;
pub mod local_server;
pub mod orders_manager;
pub mod orders_queue;
pub mod previous_connection;
pub mod server_args;
pub mod server_messages;

pub mod connection_status;
pub mod next_connection;

pub mod account;
pub mod accounts_manager;
pub mod address_resolver;
pub mod constants;
pub mod memory_accounts_manager;
pub mod offline_substract_orders_cleaner;

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
