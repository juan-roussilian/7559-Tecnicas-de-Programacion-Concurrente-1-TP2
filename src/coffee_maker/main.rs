/// Modulo que representa al actor que realizara la comunicacion entre la cafetera y el servidor local.
pub mod local_server_client;
/// Modulo que devuelve exito o error utilizando un numero generado al azar y un porcentaje de exito.
pub mod randomizer;
/// Modulo de mensajes que utilizan los actores que componen la cafetera.
pub mod actor_messages;
/// Modulo de errores que utiliza unicamente la cafetera.
pub mod errors;
/// Modulo que representa un pedido de los clientes de la cafeteria.
pub mod order;
/// Modulo de que representa al actor que procesa archivos de pedidos de clientes y los convierte en structs order.
pub mod orders_reader;
/// Modulo que representa los parametros que puede recibir el binario para su ejecucion
pub mod coffee_args;
/// Modulo que representa al actor cafetera, el cual recibe las ordenes e interactua con el actor cliente de local server
pub mod coffee_maker;
/// Modulo donde se encuentran las constantes definidas para el funcionamiento correcto de la cafetera.
pub mod constants;

use std::{collections::HashMap, env};

use actix::Actor;
use actix_rt::System;

use actor_messages::OpenFile;
use coffee_args::CoffeeArgs;
use coffee_maker::CoffeeMaker;
use constants::{DEFAULT_ORDERS_FILE, DISPENSERS, SUCCESS_CHANCE};
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
        let mut coffee_addresses = HashMap::new();
        for id in 0..DISPENSERS {
            let coffee_maker = CoffeeMaker::new(
                reader_addr.clone(),
                &args.server_ip_and_port,
                Box::new(RealRandomizer::new(SUCCESS_CHANCE)),
                id,
            );
            match coffee_maker {
                Err(_) => {
                    System::current().stop();
                    return;
                }
                Ok(coffee_maker) => {
                    let coffee_maker_addr = coffee_maker.start();
                    coffee_addresses.insert(id, coffee_maker_addr);
                }
            }
        }
        if reader_addr.try_send(OpenFile(coffee_addresses)).is_err() {
            error!("[COFFEE MAKER] Unable to send OpenFile message to file reader");
            System::current().stop();
        }
    });

    system.run().unwrap();
}
