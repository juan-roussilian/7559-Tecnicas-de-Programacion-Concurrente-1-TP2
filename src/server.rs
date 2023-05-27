use crate::{
    connection_protocol::ConnectionProtocol,
    errors::ServerError,
    local_connection_messages::{CoffeeMakerRequest, CoffeeMakerResponse, MessageType},
};
use async_trait::async_trait;

#[async_trait]
pub trait LocalServerClient {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError>;
}

pub struct LocalServer {
    pub connection: Box<dyn ConnectionProtocol>,
}

impl LocalServer {
    async fn handle_request(
        &self,
        message_type: MessageType,
        account_id: usize,
        points: usize,
    ) -> Result<(), ServerError> {
        let req = CoffeeMakerRequest {
            message_type,
            account_id,
            points,
        };
        let serialized = bincode::serialize(&req);
        if serialized.is_err() {
            return ServerError::SerializationError;
        }
        let serialized = serialized.unwrap();
        self.connection.send(serialized).await?;
        let encoded = self.connection.recv().await?;
        let decoded: CoffeeMakerResponse = bincode::deserialize(&encoded[..]);
        if decoded.is_err() {
            return ServerError::SerializationError;
        }
        let decoded = decoded;
        decoded.status
    }
}

#[async_trait]
impl LocalServerClient for LocalServer {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        self.handle_request(MessageType::AddPoints, account_id, points)
            .await
    }
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        self.handle_request(MessageType::RequestPoints, account_id, points)
            .await
    }
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        self.handle_request(MessageType::TakePoints, account_id, points)
            .await
    }
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError> {
        self.handle_request(MessageType::CancelPointsRequest, account_id, 0)
            .await
    }
}
