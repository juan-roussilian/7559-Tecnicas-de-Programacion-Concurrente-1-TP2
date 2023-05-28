#[cfg(test)]
use mockall::automock;

use std::sync::Arc;

use crate::{
    connection_protocol::{ConnectionProtocol, TcpConnection},
    errors::ServerError,
    local_connection_messages::{
        CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
    },
};
use async_std::sync::Mutex;
use async_trait::async_trait;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait LocalServerClient {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError>;
}

pub struct LocalServer {
    connection: Arc<Mutex<Box<dyn ConnectionProtocol + Send + Sync>>>,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, ServerError> {
        let protocol = TcpConnection::new()?;
        Ok(LocalServer {
            connection: Arc::new(Mutex::new(Box::new(protocol))),
        })
    }
}

async fn handle_request(
    connection: Arc<Mutex<Box<dyn ConnectionProtocol + Send + Sync>>>,
    message_type: MessageType,
    account_id: usize,
    points: usize,
) -> Result<(), ServerError> {
    let req = CoffeeMakerRequest {
        message_type,
        account_id,
        points,
    };
    let serialized = bincode::serialize(&req)?;
    let mut connection = connection.lock().await;
    connection.send(&serialized).await?;
    let encoded = connection.recv().await?;
    let decoded: CoffeeMakerResponse = bincode::deserialize(&encoded[..])?;
    match decoded.status {
        ResponseStatus::Ok => Ok(()),
        ResponseStatus::Err(error) => Err(error),
    }
}

#[async_trait]
impl LocalServerClient for LocalServer {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        handle_request(
            self.connection.clone(),
            MessageType::AddPoints,
            account_id,
            points,
        )
        .await
    }
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        let connection = &self.connection;
        handle_request(
            connection.clone(),
            MessageType::RequestPoints,
            account_id,
            points,
        )
        .await
    }
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        handle_request(
            self.connection.clone(),
            MessageType::TakePoints,
            account_id,
            points,
        )
        .await
    }
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError> {
        handle_request(
            self.connection.clone(),
            MessageType::CancelPointsRequest,
            account_id,
            0,
        )
        .await
    }
}
