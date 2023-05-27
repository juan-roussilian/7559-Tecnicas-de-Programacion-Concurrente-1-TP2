use async_trait::async_trait;

use crate::errors::ServerError;

#[async_trait]
pub trait ConnectionProtocol {
    async fn send(&self, data: Vec<u8>) -> Result<(), ServerError>;
    async fn recv(&self) -> Result<Vec<u8>, ServerError>;
}

pub struct TcpConnection {}

impl TcpConnection {
    pub fn new() -> TcpConnection {
        TcpConnection {}
    }
}

#[async_trait]
impl ConnectionProtocol for TcpConnection {
    async fn send(&self, data: Vec<u8>) -> Result<(), ServerError> {
        Ok(())
    }
    async fn recv(&self) -> Result<Vec<u8>, ServerError> {
        Ok(vec![])
    }
}
