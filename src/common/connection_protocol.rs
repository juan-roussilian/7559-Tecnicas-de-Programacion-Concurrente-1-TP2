use std::net::SocketAddr;

use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::TcpStream,
    task,
};
use async_trait::async_trait;
use log::{error, info};

use crate::common_errors::CoffeeSystemError;

#[async_trait]
pub trait ConnectionProtocol {
    async fn send(&mut self, data: &[u8]) -> Result<(), CoffeeSystemError>;
    async fn recv(&mut self) -> Result<String, CoffeeSystemError>;
}

pub struct TcpConnection {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    addr: SocketAddr,
}

impl TcpConnection {
    pub fn new_client_connection(server_addr: &String) -> Result<TcpConnection, CoffeeSystemError> {
        let result = task::block_on(TcpStream::connect(&server_addr));
        match result {
            Err(e) => {
                error!(
                    "[TCP CONNECTION] Error connecting to server {}, {}",
                    server_addr, e
                );
                Err(CoffeeSystemError::ConnectionLost)
            }
            Ok(stream) => {
                info!(
                    "[TCP CONNECTION] Established connection to local server {}",
                    server_addr
                );
                Ok(TcpConnection {
                    writer: stream.clone(),
                    addr: stream.peer_addr()?,
                    reader: BufReader::new(stream),
                })
            }
        }
    }

    pub fn new_server_connection(stream: TcpStream, addr: SocketAddr) -> TcpConnection {
        TcpConnection {
            writer: stream.clone(),
            reader: BufReader::new(stream),
            addr,
        }
    }
}

#[async_trait]
impl ConnectionProtocol for TcpConnection {
    async fn send(&mut self, data: &[u8]) -> Result<(), CoffeeSystemError> {
        match self.writer.write_all(data).await {
            Ok(()) => Ok(()),
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error sending message to server {} {}, {}",
                    self.addr.ip(),
                    self.addr.port(),
                    error
                );
                Err(CoffeeSystemError::ConnectionLost)
            }
        }
    }
    async fn recv(&mut self) -> Result<String, CoffeeSystemError> {
        let mut buffer = String::new();
        match self.reader.read_line(&mut buffer).await {
            Ok(read) => {
                if read == 0 {
                    info!(
                        "[TCP CONNECTION] Closed connection {} {}",
                        self.addr.ip(),
                        self.addr.port()
                    );
                    return Err(CoffeeSystemError::ConnectionClosed);
                }
                Ok(buffer)
            }
            Err(error) => {
                error!(
                    "[TCP CONNECTION] Error receiving message from server {} {}, {}",
                    self.addr.ip(),
                    self.addr.port(),
                    error
                );
                Err(CoffeeSystemError::ConnectionLost)
            }
        }
    }
}
