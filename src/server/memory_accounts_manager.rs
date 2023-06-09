use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use crate::server_messages::UpdatedAccount;
use crate::account::Account;
use crate::accounts_manager::AccountsManager;
use crate::errors::ServerError;

pub struct MemoryAccountsManager {
    accounts: HashMap<usize, Arc<RwLock<Account>>>,
}

impl MemoryAccountsManager {
    pub fn new() -> Self {
        MemoryAccountsManager {
            accounts: HashMap::new(),
        }
    }
}

impl AccountsManager for MemoryAccountsManager {
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
        if let Some(account) = self.accounts.get(&account_id) {
            let mut account_guard = account.write()?;
            if account_guard.points() >= points {
                return account_guard.reserve();
            }
            return Err(ServerError::NotEnoughPointsInAccount);
        }

        Err(ServerError::AccountNotFound)
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
    fn update(
        &self,
        account_id: usize,
        points: usize,
        operation_time: u128,
    ) -> Result<(), ServerError> {
        if self.accounts.contains_key(&account_id) {
            if let Some(account) = self.accounts.get(&account_id) {
                if let Ok(mut account_guard) = account.write() {
                    account_guard.update(points, operation_time)?
                }
            }
        } else {
            return Err(ServerError::AccountNotFound);
        }
        Ok(())
    }
    fn get_most_recent_update(&self) -> u128 {
        let mut latest_update: u128 = 0;
        for (_, account) in self.accounts.iter() {
            if let Ok(guard) = account.read() {
                let account_last_update = guard.last_updated_on();
                if latest_update < account_last_update {
                    latest_update = account_last_update;
                }
            }
        }
        latest_update
    }

    fn get_accounts_updated_after(&self, timestamp: u128) -> Vec<UpdatedAccount> {
        let mut updated_accounts = vec![];
        for (id, account) in self.accounts.iter() {
            if let Ok(guard) = account.read() {
                let last_updated_on = guard.last_updated_on();
                if timestamp < last_updated_on {
                    updated_accounts.push(UpdatedAccount{id:*id, amount:guard.points(),last_updated_on});
                }
            }
        }
        updated_accounts
    }
}
