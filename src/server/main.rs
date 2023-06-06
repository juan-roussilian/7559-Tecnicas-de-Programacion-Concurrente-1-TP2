use std::env;

use actix_rt::System;
use coffee_maker_server::CoffeeMakerServer;
use errors::ServerError;
use lib::logger::set_logger_config;
use log::error;
use server_args::ServerArgs;

pub mod coffee_maker_connection;
pub mod coffee_maker_server;
pub mod coffee_message_dispatcher;
pub mod connection_server;
pub mod errors;
pub mod orders_manager;
pub mod orders_queue;
pub mod previous_connection;
pub mod server_args;
pub mod local_server;
pub mod server_messages;

pub mod next_connection;
pub mod connection_status;

pub mod accounts_manager;
pub mod account;
pub mod memory_accounts_manager;

fn get_args() -> Result<ServerArgs, ServerError> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 {
        let parsed_id: Result<usize, _> = args[1].clone().trim().parse();
        let parsed_peer_count: Result<usize, _> = args[2].clone().trim().parse();

        match (parsed_id, parsed_peer_count) {
            (Ok(id), Ok(peer_server_count)) =>
                Ok(ServerArgs {
                    id,
                    peer_server_count,
                }),
            (_, _) => Err(ServerError::ArgsFormat),
        }
    } else {
        return Err(ServerError::ArgsMissing);
    }
}

fn main() {
    set_logger_config();
    let system = System::new();
    let server_args_res = get_args();

    let server_args;

    match server_args_res {
        Err(_) => {
            error!("Error setting args. Use [ID] [PEER_COUNT]");
            return;
        }
        Ok(args) => {
            server_args = args;
        }
    }

    system.block_on(async {
        let coffee_server = CoffeeMakerServer::new(server_args.id);
        if coffee_server.is_err() {
            error!("Error booting up coffee maker server, stopping...");
            System::current().stop();
            return;
        }
        let mut coffee_server = coffee_server.unwrap();
        //coffee_server.listen();
    });

    system.run().unwrap();
}
