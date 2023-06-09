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
    pub fn update_points(
        &mut self,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if !self.is_reserved {
            match operation_time {
                Some(timestamp) => {
                    if self.last_updated_on < timestamp {
                        self.points = points;
                        Ok(())
                    } else {
                        Err(ServerError::OperationIsOutdated)
                    }
                }
                None => {
                    self.points = points;
                    Ok(())
                }
            }
        } else {
            Err(ServerError::AccountIsReserved)
        }
    }
    pub fn substract_points(
        &mut self,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if !self.is_reserved {
            match operation_time {
                Some(timestamp) => {
                    if self.last_updated_on < timestamp {
                        self.points -= points;
                        Ok(())
                    } else {
                        Err(ServerError::OperationIsOutdated)
                    }
                }
                None => {
                    self.points -= points;
                    Ok(())
                }
            }
        } else {
            Err(ServerError::AccountIsReserved)
        }
    }
    pub fn reserve(&mut self) {
        self.is_reserved = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_points_for_empty_account_after_adding_5_points_should_return_5() {
        if let Some(mut account) = Account::new(1, 0) {
            account.add_points(5, None);
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
            account.add_points(100, None);
            assert_eq!(account.points(), correct_amount)
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }
    #[test]
    fn account_points_after_updating_account_points_with_new_points_number_should_return_new_value()
    {
        if let Some(mut account) = Account::new(1, 200) {
            let updated_points_amount = 1000;
            account
                .update_points(1000, None)
                .expect("[Error]Failed to substract points");
            assert_eq!(account.points(), updated_points_amount)
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn account_points_after_substracting_10_points_to_not_reserved_account_with_20_points_should_return_10(
    ) {
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
    fn substracting_more_points_than_available_should_return_error_() {
        if let Some(mut account) = Account::new(1, 50) {
            assert!(account.substract_points(100, None).is_err())
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn substracting_points_to_reserved_account_should_return_error() {
        if let Some(mut account) = Account::new(1, 50) {
            account.reserve();
            assert!(account.substract_points(10, None).is_err());
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }

    #[test]
    fn updating_points_in_a_reserved_account_should_return_error() {
        if let Some(mut account) = Account::new(1, 50) {
            account.reserve();
            assert!(account.update_points(10, None).is_err());
        } else {
            panic!("[Error] System time is somehow older than UNIX EPOCH")
        }
    }
}
