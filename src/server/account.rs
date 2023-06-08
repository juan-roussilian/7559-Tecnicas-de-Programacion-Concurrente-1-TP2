use chrono::{DateTime, Local};
use crate::errors::ServerError;
pub struct Account{
    pub id: usize,
    points: usize,
    last_updated_on: DateTime<Local>,
    is_reserved: bool
}

impl Account{
    pub fn new(id:usize, points:usize)->Self{
        Account{id,points, last_updated_on: Local::now(), is_reserved:false}
    }
    pub fn points(&self)->usize {
        self.points
    }
    pub fn add_points(&mut self, points: usize){
        self.points += points;
    }
    pub fn update_points(&mut self,points:usize)-> Result<(), ServerError>{
        self.points = points;
        Ok(())
    }
    pub fn substract_points(&mut self,points: usize) -> Result<(), ServerError>{
        Ok(())
    }
    pub fn reserve(&mut self){

    }
    pub fn is_reserved(&mut self) -> bool{
        self.is_reserved
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_account_should_not_be_reserverd() {
        let mut account: Account = Account::new(1,0);
        assert_eq!(account.is_reserved(), false)
    }
    #[test]
    fn account_points_for_empty_account_after_adding_5_points_should_return_5(){
        let mut account: Account = Account::new(1,0);
        account.add_points(5);
        assert_eq!(account.points(), 5)
    }

    #[test]
    fn accounts_point_for_account_with_points_after_adding_100_points_should_return_correct_amount(){
        let mut account: Account = Account::new(1,200);
        let correct_amount = account.points() + 100;
        account.add_points(100);
        assert_eq!(account.points(), correct_amount)
    }
    #[test]
    fn account_points_after_updating_account_points_with_new_points_number_should_return_new_value(){
        let mut account: Account = Account::new(1,200);
        let updated_points_amount = 1000;
        account.update_points(1000).expect("[Error]Failed to substract points");
        assert_eq!(account.points(), updated_points_amount)
    }

    #[test]
    fn account_points_after_substracting_10_points_to_not_reserved_account_with_20_points_should_return_10(){
        let mut account: Account = Account::new(1,20);
        let correct_amount = account.points() - 10;
        account.substract_points(10).expect("[Error]Failed to substract points");
        assert_eq!(account.points(), correct_amount)
    }
    #[test]
    fn substracting_more_points_than_available_should_return_error_(){
        let mut account: Account = Account::new(1,50);
        assert!(account.substract_points(100).is_err())
    }

    #[test]
    fn substracting_points_to_reserved_account_should_return_error(){
        let mut account: Account = Account::new(1,20);
        account.reserve();
        assert!(account.substract_points(10).is_err());
    }

    #[test]
    fn updating_points_in_a_reserved_account_should_return_error(){
        let mut account: Account = Account::new(1,20);
        account.reserve();
        assert!(account.update_points(10).is_err());
    }

}

    