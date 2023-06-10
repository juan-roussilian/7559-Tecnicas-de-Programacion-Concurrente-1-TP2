use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::TcpStream,
    task,
};
use async_trait::async_trait;
use log::{error, info};

use crate::common_errors::ConnectionError;

#[async_trait]
pub trait ConnectionProtocol {
    async fn send(&mut self, data: &[u8]) -> Result<(), ConnectionError>;
    async fn recv(&mut self) -> Result<Vec<u8>, ConnectionError>;
}

pub struct TcpConnection {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl TcpConnection {
    pub fn new_client_connection(server_addr: String) -> Result<TcpConnection, ConnectionError> {
        let result = task::block_on(TcpStream::connect(&server_addr));
        match result {
            Err(e) => {
                error!(
                    "[TCP CONNECTION] Error connecting to server {}, {}",
                    server_addr, e
                );
                Err(ConnectionError::ConnectionLost)
            }
            Ok(stream) => {
                info!(
                    "[TCP CONNECTION] Established connection to local server {}",
                    server_addr
                );
                Ok(TcpConnection {
                    writer: stream.clone(),
                    reader: BufReader::new(stream),
                })
            }
        }
    }

    pub fn new_server_connection(stream: TcpStream) -> TcpConnection {
        TcpConnection {
            writer: stream.clone(),
            reader: BufReader::new(stream),
        }
    }
}

#[async_trait]
impl ConnectionProtocol for TcpConnection {
    async fn send(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        match self.writer.write_all(data).await {
            Ok(()) => Ok(()),
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error sending message to server, {}",
                    error
                );
                Err(ConnectionError::ConnectionLost)
            }
        }
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ConnectionError> {
        let mut buffer = Vec::new();
        match self.reader.read_until(b';', &mut buffer).await {
            Ok(read) => {
                if read == 0 {
                    info!("[TCP CONNECTION] Closed connection");
                    return Err(ConnectionError::ConnectionClosed);
                }
                Ok(buffer)
            }
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error receiving message from server, {}",
                    error
                );
                Err(ConnectionError::ConnectionLost)
            }
        }
    }
}
