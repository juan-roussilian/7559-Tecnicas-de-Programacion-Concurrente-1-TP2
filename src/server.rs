use crate::errors::ServerError;

pub trait Server {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError>;
}

pub struct LocalServer {}
