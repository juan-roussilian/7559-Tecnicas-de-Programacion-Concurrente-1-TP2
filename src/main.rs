use coffee_maker::main_coffee;

pub mod coffee_maker;
pub mod connection_protocol;
pub mod errors;
pub mod local_connection_messages;
pub mod logger;
pub mod messages;
pub mod order;
pub mod orders_reader;
pub mod randomizer;
pub mod server;

fn main() {
    main_coffee();
}
