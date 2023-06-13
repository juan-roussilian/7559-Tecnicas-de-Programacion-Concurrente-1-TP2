use crate::account::Account;
use crate::accounts_manager::AccountsManager;
use crate::errors::ServerError;
use crate::server_messages::UpdatedAccount;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;

#[derive(Debug)]
/// Implementacion en memoria de manejador de cuentas
pub struct MemoryAccountsManager {
    accounts: HashMap<usize, Account>,
}

impl MemoryAccountsManager {
    pub fn new() -> Self {
        MemoryAccountsManager {
            accounts: HashMap::new(),
        }
    }
}

impl AccountsManager for MemoryAccountsManager {
    /// Metodo que toma el lock de una cuenta e invoca su metodo de sumar puntos
    fn add_points(
        &mut self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if let Vacant(e) = self.accounts.entry(account_id) {
            if let Some(new_account) = Account::new(account_id, points) {
                e.insert(new_account);
            }
        } else if let Some(account) = self.accounts.get_mut(&account_id) {
            account.add_points(points, operation_time)?;
        }
        Ok(())
    }
    /// Metodo que toma el lock de una cuenta e invoca su metodo de restar puntos
    fn substract_points(
        &mut self,
        account_id: usize,
        points: usize,
        operation_time: Option<u128>,
    ) -> Result<(), ServerError> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.substract_points(points, operation_time)?;
            return Ok(());
        }

        Err(ServerError::AccountNotFound)
    }
    /// Metodo que toma el lock de una cuenta e invoca su metodo de actualizar puntos
    fn update(&mut self, account_id: usize, points: usize, operation_time: u128) {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.update(points, operation_time);
            return;
        }
        self.accounts.insert(
            account_id,
            Account::new_from_update(account_id, points, operation_time),
        );
    }
    /// Metodo que toma el lock de una cuenta y la reserva para que nadie pueda operar sobre ella
    fn request_points(&mut self, account_id: usize, points: usize) -> Result<(), ServerError> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            if account.points() >= points {
                return account.reserve();
            }
            return Err(ServerError::NotEnoughPointsInAccount);
        }

        Err(ServerError::AccountNotFound)
    }
    /// Metodo que toma el lock de una cuenta e invalida la reserva que realizo sobre esta.
    fn cancel_requested_points(&mut self, account_id: usize) -> Result<(), ServerError> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.cancel_reservation();
            return Ok(());
        }

        Err(ServerError::AccountNotFound)
    }
    /// Metodo que devuelve el timestamp de la cuenta que fue actualizada por ultima vez entre todas las existentes
    fn get_most_recent_update(&self) -> u128 {
        let mut latest_update: u128 = 0;
        for account in self.accounts.values() {
            let account_last_update = account.last_updated_on();
            if latest_update < account_last_update {
                latest_update = account_last_update;
            }
        }
        latest_update
    }
    /// Metodo que devuelve las cuentas que fueron actualizadas luego de cierto timestamp
    fn get_accounts_updated_after(&self, timestamp: u128) -> Vec<UpdatedAccount> {
        let mut updated_accounts = vec![];
        for (id, account) in self.accounts.iter() {
            let last_updated_on = account.last_updated_on();
            if timestamp < last_updated_on {
                updated_accounts.push(UpdatedAccount {
                    id: *id,
                    amount: account.points(),
                    last_updated_on,
                });
            }
        }
        updated_accounts
    }

    /// Metodo que elimina las reservas realizadas sobre todas las cuentas
    fn clear_reservations(&mut self) {
        for account in self.accounts.values_mut() {
            account.cancel_reservation();
        }
    }
}

impl Default for MemoryAccountsManager {
    fn default() -> Self {
        Self::new()
    }
}
