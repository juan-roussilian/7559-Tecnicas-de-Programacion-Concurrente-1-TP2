use crate::errors::ServerError;

pub trait AccountsManager{
    fn new(&self)-> Self;
    fn add_points(&mut self, account_id: usize, points: usize);
    fn request_points(&self, account_id: usize, points: usize)-> Result<(), ServerError>;
    fn cancel_requested_points(&self, account_id: usize)-> Result<(), ServerError>;
    fn substract_points(&self, account_id: usize, points: usize)-> Result<(), ServerError>;
    fn update_account(&self, account_id:usize, points:usize) -> Result<(), ServerError>;
}