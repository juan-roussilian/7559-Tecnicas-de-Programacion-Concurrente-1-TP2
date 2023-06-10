use std::sync::mpsc::{Receiver, Sender};

use async_std::task;
use lib::{
    common_errors::ConnectionError,
    connection_protocol::ConnectionProtocol,
    local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse},
    serializer::{deserialize, serialize},
};
use log::{debug, error};

pub fn receive_messages_from_coffee_maker(
    connection: &mut Box<dyn ConnectionProtocol + Send>,
    machine_id: usize,
    request_sender: Sender<(CoffeeMakerRequest, usize)>,
    response_receiver: Receiver<CoffeeMakerResponse>,
) -> Result<(), ConnectionError> {
    loop {
        let mut encoded = task::block_on(connection.recv())?;
        let decoded: CoffeeMakerRequest = deserialize(&mut encoded)?;
        debug!(
            "[COFFEE MAKER {}] Received {:?} message",
            machine_id, decoded
        );
        if request_sender.send((decoded, machine_id)).is_err() {
            error!(
                "[COFFEE MAKER {}] Trying to send on a channel without receiver",
                machine_id
            );
            return Err(ConnectionError::ConnectionClosed);
        }

        let response = response_receiver.recv();

        match response {
            Err(_) => {
                error!(
                    "[COFFEE MAKER {}] Trying to receive on a channel without sender",
                    machine_id
                );
                return Err(ConnectionError::ConnectionClosed);
            }
            Ok(res) => {
                let serialized = serialize(&res)?;
                task::block_on(connection.send(&serialized))?;
            }
        }
    }
}
