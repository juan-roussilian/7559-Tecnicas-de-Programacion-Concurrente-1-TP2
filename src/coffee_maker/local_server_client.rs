#[cfg(test)]
use mockall::automock;

use std::sync::Arc;

use async_std::sync::Mutex;
use async_trait::async_trait;
use lib::{
    common_errors::CoffeeSystemError,
    connection_protocol::{ConnectionProtocol, TcpConnection},
    local_connection_messages::{
        CoffeeMakerRequest, CoffeeMakerResponse, MessageType, ResponseStatus,
    },
    serializer::{deserialize, serialize},
};

/// Interfaz de las operaciones que se puede hacer con el servidor local
#[cfg_attr(test, automock)]
#[async_trait]
pub trait LocalServerClient {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), CoffeeSystemError>;
    async fn request_points(
        &self,
        account_id: usize,
        points: usize,
    ) -> Result<(), CoffeeSystemError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), CoffeeSystemError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), CoffeeSystemError>;
}

/// Conexion con el servidor local, arma los mensajes, los envia y espera
pub struct LocalServer {
    connection: Arc<Mutex<Box<dyn ConnectionProtocol + Send + Sync>>>,
}

impl LocalServer {
    pub fn new(server_addr: &String) -> Result<LocalServer, CoffeeSystemError> {
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
) -> Result<(), CoffeeSystemError> {
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
    /// Metodo mediante el cual la cafetera le pide al servidor que sume puntos a una cuenta
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), CoffeeSystemError> {
        handle_request(
            self.connection.clone(),
            MessageType::AddPoints,
            account_id,
            points,
        )
        .await
    }

    /// Metodo mediante el cual la cafetera le pide al servidor que reserve los puntos de una
    async fn request_points(
        &self,
        account_id: usize,
        points: usize,
    ) -> Result<(), CoffeeSystemError> {
        handle_request(
            self.connection.clone(),
            MessageType::RequestPoints,
            account_id,
            points,
        )
        .await
    }

    /// Metodo mediante el cual la cafetera le pide al servidor que reste puntos a una cuenta
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), CoffeeSystemError> {
        handle_request(
            self.connection.clone(),
            MessageType::TakePoints,
            account_id,
            points,
        )
        .await
    }
    /// Metodo mediante el cual la cafetera le pide al servidor que cancele la reserva que realizo sobre los puntos de una cuenta
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), CoffeeSystemError> {
        handle_request(
            self.connection.clone(),
            MessageType::CancelPointsRequest,
            account_id,
            0,
        )
        .await
    }
}
