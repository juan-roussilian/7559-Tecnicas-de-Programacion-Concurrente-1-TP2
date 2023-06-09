use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::collections::hash_map::Entry::Vacant;

use crate::account::Account;
use crate::accounts_manager::AccountsManager;
use crate::errors::ServerError;

pub struct MemoryAccountsManager {
    accounts: HashMap<usize, Arc<RwLock<Account>>>,
}

impl AccountsManager for MemoryAccountsManager {
    fn new() -> Self {
        MemoryAccountsManager {
            accounts: HashMap::new(),
        }
    }

    fn add_points(
        &mut self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if let Vacant(e) = self.accounts.entry(account_id) {
            if let Some(new_account) = Account::new(account_id, points) {
                e.insert(Arc::new(RwLock::new(new_account)));
            }
        } else {
            if let Some(account) = self.accounts.get(&account_id) {
                if let Ok(mut account_guard) = account.write() {
                    account_guard.add_points(points, operation_time)?
                }
            }
        }
        Ok(())
    }

    fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError> {
        if self.accounts.contains_key(&account_id) {
            if let Some(account) = self.accounts.get(&account_id) {
                if let Ok(mut account_guard) = account.write() {
                    if account_guard.points() >= points {
                        account_guard.reserve()?
                    }
                }
            }
        } else {
            return Err(ServerError::AccountNotFound);
        }
        Ok(())
    }
    fn cancel_requested_points(&self, account_id: usize) -> Result<(), ServerError> {
        if self.accounts.contains_key(&account_id) {
            if let Some(account) = self.accounts.get(&account_id) {
                if let Ok(mut account_guard) = account.write() {
                    account_guard.cancel_reservation();
                }
            }
        } else {
            return Err(ServerError::AccountNotFound);
        }
        Ok(())
    }

    fn substract_points(
        &self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if self.accounts.contains_key(&account_id) {
            if let Some(account) = self.accounts.get(&account_id) {
                if let Ok(mut account_guard) = account.write() {
                    account_guard.substract_points(points, operation_time)?
                }
            }
        } else {
            return Err(ServerError::AccountNotFound);
        }
        Ok(())
    }
}
