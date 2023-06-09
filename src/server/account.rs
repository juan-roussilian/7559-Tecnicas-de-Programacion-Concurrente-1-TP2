use crate::errors::ServerError;
use std::time::{SystemTime, UNIX_EPOCH};
pub struct Account {
    pub id: usize,
    points: usize,
    last_updated_on: u128,
    is_reserved: bool,
}

impl Account {
    pub fn new(id: usize, points: usize) -> Option<Self> {
        if let Ok(current_timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
            Some(Account {
                id,
                points,
                last_updated_on: current_timestamp.as_nanos(),
                is_reserved: false,
            })
        } else {
            None
        }
    }

    pub fn points(&self) -> usize {
        self.points
    }

    pub fn is_reserved(&mut self) -> bool {
        self.is_reserved
    }

    pub fn add_points(
        &mut self,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        match operation_time {
            Some(timestamp) => {
                if self.last_updated_on < timestamp {
                    self.points += points;
                    Ok(())
                } else {
                    Err(ServerError::OperationIsOutdated)
                }
            }
            None => {
                self.points += points;
                Ok(())
            }
        }
    }
    pub fn substract_points(
        &mut self,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if points > self.points {
            return Err(ServerError::NotEnoughPointsInAccount);
        }

        match operation_time {
            Some(timestamp) => {
                if self.last_updated_on < timestamp {
                    self.points -= points;
                    self.is_reserved = false;
                    Ok(())
                } else {
                    Err(ServerError::OperationIsOutdated)
                }
            }
            None => {
                self.points -= points;
                self.is_reserved = false;
                Ok(())
            }
        }
    }
    pub fn cancel_reservation(&mut self) {
        self.is_reserved = false;
    }
    pub fn reserve(&mut self) -> Result<(), ServerError> {
        if self.is_reserved {
            Err(ServerError::AccountIsReserved)
        } else {
            self.is_reserved = true;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_points_for_empty_account_after_adding_5_points_should_return_5() {
        if let Some(mut account) = Account::new(1, 0) {
            let _result = account.add_points(5, None);
            assert_eq!(account.points(), 5)
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn accounts_point_for_account_with_points_after_adding_100_points_should_return_correct_amount()
    {
        if let Some(mut account) = Account::new(1, 200) {
            let correct_amount = account.points() + 100;
            let _result = account.add_points(100, None);
            assert_eq!(account.points(), correct_amount)
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn account_points_after_substracting_10_points_to_account_with_20_points_should_return_10() {
        if let Some(mut account) = Account::new(1, 20) {
            let correct_amount = account.points() - 10;
            account
                .substract_points(10, None)
                .expect("[Error]Failed to substract points");
            assert_eq!(account.points(), correct_amount)
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn reserved_account_should_be_unreserved_after_substracting() {
        if let Some(mut account) = Account::new(1, 20) {
            let _result = account.reserve();
            account
                .substract_points(10, None)
                .expect("[Error]Failed to substract points");
            assert!(!account.is_reserved())
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn substracting_more_points_than_available_should_return_error_() {
        if let Some(mut account) = Account::new(1, 50) {
            assert!(account.substract_points(100, None).is_err())
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn trying_to_reserve_unreserved_account_should_return_ok() {
        if let Some(mut account) = Account::new(1, 50) {
            assert!(!account.reserve().is_err());
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn trying_to_reserve_reserved_account_should_return_error() {
        if let Some(mut account) = Account::new(1, 50) {
            account
                .reserve()
                .expect("[Err] Account was already reserved");
            assert!(account.reserve().is_err());
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }
}
