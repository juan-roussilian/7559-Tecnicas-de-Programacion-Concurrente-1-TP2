use std::sync::Arc;

use async_std::sync::Mutex;
use lib::{
    common_errors::ConnectionError,
    connection_protocol::ConnectionProtocol,
    local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse, ResponseStatus},
    serializer::{deserialize, serialize},
};
use log::info;

pub async fn receive_messages_from_coffee_maker(
    connection: Arc<Mutex<Box<dyn ConnectionProtocol + Send>>>,
) -> Result<(), ConnectionError> {
    let mut connection = connection.lock().await;
    loop {
        let mut encoded = connection.recv().await?;
        let decoded: CoffeeMakerRequest = deserialize(&mut encoded)?;
        info!("{:?}", decoded);

        // TODO HANDLE REQUEST and get response status

        let response = CoffeeMakerResponse {
            message_type: decoded.message_type,
            status: ResponseStatus::Ok,
        };
        let serialized = serialize(&response)?;
        connection.send(&serialized).await?;
    }
}
