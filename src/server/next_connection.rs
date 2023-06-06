use std::sync::mpsc::Receiver;

use crate::server_messages::ServerMessage;

pub struct NextConnection {
    id: usize,
    next_conn_receiver: Receiver<ServerMessage>,
}

impl NextConnection {
    pub fn new(id: usize, next_conn_receiver: Receiver<ServerMessage>) -> NextConnection {
        NextConnection {
            id,
            next_conn_receiver,
        }
    }

    pub fn connect_to_next(&mut self) {}
}
