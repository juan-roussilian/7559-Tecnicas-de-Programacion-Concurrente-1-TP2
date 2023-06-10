use crate::{errors::ServerError, server_messages::UpdatedAccount};

pub trait AccountsManager {
    fn add_points(
        &mut self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError>;
    fn substract_points(
        &self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError>;
    fn update(
        &self,
        account_id: usize,
        points: usize,
        operation_time: u128,
    ) -> Result<(), ServerError>;
    fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    fn cancel_requested_points(&self, account_id: usize) -> Result<(), ServerError>;
    fn get_most_recent_update(&self) -> u128;
    fn get_accounts_updated_after(&self, timestamp: u128) -> Vec<UpdatedAccount>;
    fn clear_reservations(&self);
}
