pub mod local_server_client;
pub mod randomizer;

pub mod actor_messages;
pub mod errors;
pub mod order;
pub mod orders_reader;

pub mod coffee_args;
pub mod coffee_maker;
pub mod constants;

use std::env;

use actix::Actor;
use actix_rt::System;

use actor_messages::OpenFile;
use coffee_args::CoffeeArgs;
use coffee_maker::CoffeeMaker;
use constants::{DEFAULT_ORDERS_FILE, SUCCESS_CHANCE};
use errors::CoffeeMakerError;
use lib::logger::set_logger_config;
use log::error;
use orders_reader::OrdersReader;
use randomizer::RealRandomizer;

fn get_args() -> Result<CoffeeArgs, CoffeeMakerError> {
    let args: Vec<String> = env::args().collect();
    let mut orders_file_path = String::from(DEFAULT_ORDERS_FILE);
    let server_ip_and_port;
    if args.len() == 3 {
        server_ip_and_port = args[1].clone();
        orders_file_path = args[2].clone();
    } else if args.len() == 2 {
        server_ip_and_port = args[1].clone();
    } else {
        return Err(CoffeeMakerError::ArgsMissing);
    }
    Ok(CoffeeArgs {
        orders_file_path,
        server_ip_and_port,
    })
}

pub fn main() {
    let system = System::new();
    set_logger_config();
    let args = get_args();
    if args.is_err() {
        error!("Error setting args. Use [IP:PORT] [FILE - OPTIONAL]");
        return;
    }
    let args = args.unwrap();
    system.block_on(async {
        let reader = OrdersReader::new(args.orders_file_path);
        let reader_addr = reader.start();
        let coffee_maker = CoffeeMaker::new(
            reader_addr.clone(),
            args.server_ip_and_port,
            Box::new(RealRandomizer::new(SUCCESS_CHANCE)),
        );
        if coffee_maker.is_err() {
            System::current().stop();
            return;
        }
        let coffee_maker = coffee_maker.unwrap();
        let coffee_maker_addr = coffee_maker.start();
        if reader_addr.try_send(OpenFile(coffee_maker_addr)).is_err() {
            error!("[COFFEE MAKER] Unable to send OpenFile message to file reader");
            System::current().stop();
        }
    });

    system.run().unwrap();
}
