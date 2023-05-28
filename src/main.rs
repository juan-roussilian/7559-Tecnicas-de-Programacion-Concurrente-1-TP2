use coffee_maker::main_coffee;

pub mod coffee_maker;
pub mod connection_protocol;
pub mod errors;
pub mod local_connection_messages;
pub mod local_server_client;
pub mod logger;
pub mod messages;
pub mod order;
pub mod orders_reader;
pub mod randomizer;

fn main() {
    main_coffee();
}
