use actix_rt::System;
use coffee_maker_server::CoffeeMakerServer;
use lib::logger::set_logger_config;

pub mod coffee_maker_connection;
pub mod coffee_maker_server;
pub mod connection_server;
pub mod errors;

fn main() {
    let system = System::new();
    set_logger_config();
    system.block_on(async {
        let coffee_server = CoffeeMakerServer::new();
        let mut coffee_server = coffee_server.unwrap();
        coffee_server.listen().await;
    });

    system.run().unwrap();
}
