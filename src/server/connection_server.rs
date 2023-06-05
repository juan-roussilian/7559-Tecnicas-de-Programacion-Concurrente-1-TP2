use async_std::{net::TcpListener, task};
use async_trait::async_trait;

use lib::connection_protocol::{ConnectionProtocol, TcpConnection};
use log::{error, info};

use crate::errors::ServerError;

#[async_trait]
pub trait ConnectionServer {
    async fn listen(&self) -> Result<Box<dyn ConnectionProtocol + Send>, ServerError>;
}

pub struct TcpConnectionServer {
    listener: TcpListener,
}

impl TcpConnectionServer {
    pub fn new(port: &str) -> Result<TcpConnectionServer, ServerError> {
        let listener = task::block_on(TcpListener::bind("127.0.0.1:".to_owned() + port));
        if let Err(e) = listener {
            error!("[SERVER] Error binding to port, {}", e);
            return Err(ServerError::ListenerError);
        }
        info!("[SERVER] Bind to port successful");
        let listener = listener.unwrap();
        Ok(TcpConnectionServer { listener })
    }
}

#[async_trait]
impl ConnectionServer for TcpConnectionServer {
    async fn listen(&self) -> Result<Box<dyn ConnectionProtocol + Send>, ServerError> {
        let result = self.listener.accept().await;
        match result {
            Ok((tcp_stream, addr)) => {
                info!("[SERVER] Accepted connection from {}", addr.ip());
                let conn = TcpConnection::new_server_connection(tcp_stream);
                Ok(Box::new(conn))
            }
            Err(e) => {
                error!("[COFFEE MAKER SERVER] Error accepting connection {}", e);
                Err(ServerError::AcceptError)
            }
        }
    }
}
