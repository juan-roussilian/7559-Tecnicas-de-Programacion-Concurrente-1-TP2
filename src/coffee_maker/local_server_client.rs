#[cfg(test)]
use mockall::automock;

use std::sync::Arc;

use async_std::sync::Mutex;
use async_trait::async_trait;
use lib::{
    common_errors::ConnectionError,
    connection_protocol::{ConnectionProtocol, TcpConnection},
    local_connection_messages::{
        CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
    },
    serializer::{deserialize, serialize},
};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait LocalServerClient {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ConnectionError>;
    async fn request_points(&self, account_id: usize, points: usize)
        -> Result<(), ConnectionError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ConnectionError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ConnectionError>;
}

pub struct LocalServer {
    connection: Arc<Mutex<Box<dyn ConnectionProtocol + Send + Sync>>>,
}

impl LocalServer {
    pub fn new(server_addr: &String) -> Result<LocalServer, ConnectionError> {
        let protocol = TcpConnection::new_client_connection(server_addr)?;
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
) -> Result<(), ConnectionError> {
    let req = CoffeeMakerRequest {
        message_type,
        account_id,
        points,
    };
    let serialized = serialize(&req)?;
    let mut connection = connection.lock().await;
    connection.send(&serialized).await?;
    let mut encoded = connection.recv().await?;
    let decoded: CoffeeMakerResponse = deserialize(&mut encoded)?;
    match decoded.status {
        ResponseStatus::Ok => Ok(()),
        ResponseStatus::Err(error) => Err(error),
    }
}

#[async_trait]
impl LocalServerClient for LocalServer {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ConnectionError> {
        handle_request(
            self.connection.clone(),
            MessageType::AddPoints,
            account_id,
            points,
        )
        .await
    }
    async fn request_points(
        &self,
        account_id: usize,
        points: usize,
    ) -> Result<(), ConnectionError> {
        handle_request(
            self.connection.clone(),
            MessageType::RequestPoints,
            account_id,
            points,
        )
        .await
    }
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ConnectionError> {
        handle_request(
            self.connection.clone(),
            MessageType::TakePoints,
            account_id,
            points,
        )
        .await
    }
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ConnectionError> {
        handle_request(
            self.connection.clone(),
            MessageType::CancelPointsRequest,
            account_id,
            0,
        )
        .await
    }
}
