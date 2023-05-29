use lib::{
    common_errors::ConnectionError, connection_protocol::ConnectionProtocol,
    local_connection_messages::CoffeeMakerRequest,
};
use log::info;

pub struct CoffeeMakerConnection {
    connection: Box<dyn ConnectionProtocol>,
}

impl CoffeeMakerConnection {
    pub fn new(conn: Box<dyn ConnectionProtocol>) -> CoffeeMakerConnection {
        CoffeeMakerConnection { connection: conn }
    }

    pub async fn receive_messages(&mut self) -> Result<(), ConnectionError> {
        loop {
            info!("Recv from coffee");
            let encoded = self.connection.recv().await?;
            let decoded: CoffeeMakerRequest = bincode::deserialize(&encoded[..])?;
            info!("{:?}", decoded);

            //self.connection.send().await?;
        }
    }
}
