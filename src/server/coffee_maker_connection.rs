use lib::{
    common_errors::ConnectionError,
    connection_protocol::ConnectionProtocol,
    local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse, ResponseStatus},
    serializer::{deserialize, serialize},
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
            let mut encoded = self.connection.recv().await?;
            let decoded: CoffeeMakerRequest = deserialize(&mut encoded)?;
            info!("{:?}", decoded);
            let response = CoffeeMakerResponse {
                message_type: decoded.message_type,
                status: ResponseStatus::Ok,
            };
            let serialized = serialize(&response)?;
            self.connection.send(&serialized).await?;
        }
    }
}
