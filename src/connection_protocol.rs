use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::TcpStream,
    task,
};
use async_trait::async_trait;
use log::{error, info};

use crate::errors::ServerError;

#[async_trait]
pub trait ConnectionProtocol {
    async fn send(&mut self, data: &Vec<u8>) -> Result<(), ServerError>;
    async fn recv(&mut self) -> Result<Vec<u8>, ServerError>;
}

pub struct TcpConnection {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl TcpConnection {
    pub fn new() -> Result<TcpConnection, ServerError> {
        // TODO revisar
        let result = task::block_on(TcpStream::connect("127.0.0.1:12345"));
        match result {
            Err(e) => {
                error!("[TCP CONNECTION] Error connecting to server, {}", e);
                Err(ServerError::ConnectionLost)
            }
            Ok(stream) => {
                info!("[TCP CONNECTION] Established connection to local server");
                Ok(TcpConnection {
                    writer: stream.clone(),
                    reader: BufReader::new(stream),
                })
            }
        }
    }
}

#[async_trait]
impl ConnectionProtocol for TcpConnection {
    async fn send(&mut self, data: &Vec<u8>) -> Result<(), ServerError> {
        match self.writer.write_all(data).await {
            Ok(()) => Ok(()),
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error sending message to server, {}",
                    error
                );
                Err(ServerError::ConnectionLost)
            }
        }
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ServerError> {
        let mut buffer = String::new();
        match self.reader.read_line(&mut buffer).await {
            Ok(read) => {
                if read == 0 {
                    return Err(ServerError::ConnectionLost);
                }
                Ok(buffer.as_bytes().to_vec())
            }
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error receiving message from server, {}",
                    error
                );
                Err(ServerError::ConnectionLost)
            }
        }
    }
}
