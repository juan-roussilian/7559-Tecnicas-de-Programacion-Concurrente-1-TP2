use crate::{
    coffee_maker_connection::CoffeeMakerConnection,
    connection_server::{ConnectionServer, TcpConnectionServer},
    errors::ServerError,
};

pub struct CoffeeMakerServer {
    listener: Box<dyn ConnectionServer>,
    coffee_machines: Vec<CoffeeMakerConnection>,
}

impl CoffeeMakerServer {
    pub fn new() -> Result<CoffeeMakerServer, ServerError> {
        let listener: Box<dyn ConnectionServer> = Box::new(TcpConnectionServer::new()?);
        Ok(CoffeeMakerServer {
            listener,
            coffee_machines: Vec::new(),
        })
    }

    pub async fn listen(&self) -> Result<(), ServerError> {
        loop {
            let new_conn_result = self.listener.listen().await?;
            let mut new_coffee_maker_conn = CoffeeMakerConnection::new(new_conn_result);
            new_coffee_maker_conn.receive_messages().await;
        }
    }
}
