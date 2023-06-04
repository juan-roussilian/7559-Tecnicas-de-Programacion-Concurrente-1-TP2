pub trait AccountsManager{
    fn add_points(account_id: u32, points: u32);
    fn request_points(account_id: u32, points: u32)-> Result<(), ServerError>;
    fn cancel_requested_points(account_id: u32)-> Result<(), ServerError>;
    fn substract_points(account_id: u32, points: u32)-> Result<(), ServerError>;
    fn update_account(account_id:u32, points:u32) -> Result<(), ServerError>;
}