use std::sync::mpsc::{Receiver, Sender};

use async_std::task;
use lib::{
    common_errors::ConnectionError,
    connection_protocol::ConnectionProtocol,
    local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse},
    serializer::{deserialize, serialize},
};
use log::{error, info};

pub fn receive_messages_from_coffee_maker(
    connection: &mut Box<dyn ConnectionProtocol + Send>,
    machine_id: usize,
    request_sender: Sender<(CoffeeMakerRequest, usize)>,
    response_receiver: Receiver<CoffeeMakerResponse>,
) -> Result<(), ConnectionError> {
    loop {
        let mut encoded = task::block_on(async { connection.recv().await })?;
        let decoded: CoffeeMakerRequest = deserialize(&mut encoded)?;
        info!("{:?}", decoded);

        if let Err(e) = request_sender.send((decoded, machine_id)) {
            error!("trying to send on a channel without receiver!");
            return Err(ConnectionError::ConnectionClosed);
        }

        let response = response_receiver.recv();

        match response {
            Err(_) => {
                error!("Trying to receive on a channel without sender");
                return Err(ConnectionError::ConnectionClosed);
            }
            Ok(res) => {
                let serialized = serialize(&res)?;
                task::block_on(async { connection.send(&serialized).await })?;
            }
        }
    }
}
