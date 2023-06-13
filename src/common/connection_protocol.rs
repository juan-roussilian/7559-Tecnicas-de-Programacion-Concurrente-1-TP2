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
/// Trait que representa la capa de conexión del sistema. Quienes la implementen podrán tanto enviar
/// como recibir mensajes del protocolo.
pub trait ConnectionProtocol {
    async fn send(&mut self, data: &[u8]) -> Result<(), CoffeeSystemError>;
    async fn recv(&mut self) -> Result<String, CoffeeSystemError>;
}

/// Representa una conexión TCP, ya sea entre servidores o entre un servidor y una cafetera.
pub struct TcpConnection {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    addr: SocketAddr,
}

impl TcpConnection {
    /// Devuelve un nuevo cliente TcpConnection a partir de una dirección de servidor IP:PUERTO en caso de éxito, o
    /// error de no poder establecer la conexión.
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

    /// Devuelve un nuevo TcpConnection a modo de servidor.
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
    /// Envía un array de bytes a través de la conexión TCP, normalmente la serialización de
    /// un struct de Request/Response o de mensaje entre servidores. Devuelve un error en caso de
    /// haber perdido la conexión.
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
    /// Devuelve un buffer en forma de string del cuál podremos deserializar mensajes entrantes a
    /// la conexión TCP. Devuelve un error en caso de haber perdido la conexión o en el caso de que
    /// haya sido cerrada intencionalmente del otro lado.
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
