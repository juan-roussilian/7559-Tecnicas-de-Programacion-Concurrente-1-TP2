use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

use crate::errors::ServerError;
use crate::accounts_manager::AccountsManager;
use crate::account::Account;

pub struct MemoryAccountsManager{
    accounts: HashMap<usize, Arc<RwLock<Account>>>
}

impl AccountsManager for MemoryAccountsManager{
    fn new(&self)-> Self{
        MemoryAccountsManager{accounts : HashMap::new()}
    }

    fn add_points(&mut self, account_id: usize, points: usize){
        if self.accounts.contains_key(&account_id){  
            if let Some(account) = self.accounts.get(&account_id){
                if let Ok(account_guard) = account.write(){

                }
            }

        }else{
            let new_account = Arc::new(RwLock::new(Account::new(account_id,points)));
            self.accounts.insert(account_id, new_account);
        }
    }


    fn request_points(&self, account_id: usize, points: usize)-> Result<(), ServerError>{Ok(())}
    fn cancel_requested_points(&self, account_id: usize)-> Result<(), ServerError>{Ok(())}
    fn substract_points(&self, account_id: usize, points: usize)-> Result<(), ServerError>{Ok(())}
    fn update_account(&self, account_id: usize , points:usize) -> Result<(), ServerError>{Ok(())}
}