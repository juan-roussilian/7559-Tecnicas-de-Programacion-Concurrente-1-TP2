pub mod local_server_client;
pub mod randomizer;

pub mod actor_messages;
pub mod errors;
pub mod order;
pub mod orders_reader;

pub mod coffee_maker;
pub mod constants;

use actix::Actor;
use actix_rt::System;

use actor_messages::OpenFile;
use coffee_maker::CoffeeMaker;
use constants::SUCCESS_CHANCE;
use lib::logger::set_logger_config;
use log::error;
use orders_reader::OrdersReader;
use randomizer::RealRandomizer;

pub fn main() {
    let system = System::new();
    set_logger_config();
    system.block_on(async {
        let reader = OrdersReader::new(String::from("tests/orders.csv"));
        let reader_addr = reader.start();
        let coffee_maker = CoffeeMaker::new(
            reader_addr.clone(),
            8080,
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
