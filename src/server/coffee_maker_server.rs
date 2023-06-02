use std::sync::Arc;

use async_std::{
    sync::Mutex,
    task::{self, JoinHandle},
};
use lib::common_errors::ConnectionError;

use crate::{
    coffee_maker_connection::receive_messages_from_coffee_maker,
    connection_server::{ConnectionServer, TcpConnectionServer},
    errors::ServerError,
};

pub struct CoffeeMakerServer {
    listener: Box<dyn ConnectionServer>,
    coffee_machines_connections: Vec<JoinHandle<Result<(), ConnectionError>>>,
}

impl CoffeeMakerServer {
    pub fn new() -> Result<CoffeeMakerServer, ServerError> {
        let listener: Box<dyn ConnectionServer> = Box::new(TcpConnectionServer::new()?);
        Ok(CoffeeMakerServer {
            listener,
            coffee_machines_connections: Vec::new(),
        })
    }

    pub async fn listen(&mut self) -> Result<(), ServerError> {
        loop {
            let new_conn_result = self.listener.listen().await?;
            let connection = Arc::new(Mutex::new(new_conn_result));
            let future_handle = task::spawn(receive_messages_from_coffee_maker(connection));
            self.coffee_machines_connections.push(future_handle);
        }
    }
}