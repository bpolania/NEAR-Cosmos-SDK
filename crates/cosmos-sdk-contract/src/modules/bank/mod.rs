use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, AccountId};
use crate::Balance;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct BankModule {
    balances: UnorderedMap<AccountId, Balance>,
}

impl BankModule {
    pub fn new() -> Self {
        Self {
            balances: UnorderedMap::new(b"b".to_vec()),
        }
    }

    pub fn transfer(&mut self, sender: &AccountId, receiver: &AccountId, amount: Balance) {
        let sender_balance = self.get_balance(sender);
        assert!(sender_balance >= amount, "Insufficient balance");

        // Update sender balance
        if sender_balance == amount {
            self.balances.remove(sender);
        } else {
            self.balances.insert(sender, &(sender_balance - amount));
        }

        // Update receiver balance
        let receiver_balance = self.get_balance(receiver);
        self.balances.insert(receiver, &(receiver_balance + amount));

        env::log_str(&format!("Bank: Transferred {} from {} to {}", amount, sender, receiver));
    }

    pub fn mint(&mut self, receiver: &AccountId, amount: Balance) {
        let current_balance = self.get_balance(receiver);
        self.balances.insert(receiver, &(current_balance + amount));
        
        env::log_str(&format!("Bank: Minted {} to {}", amount, receiver));
    }

    pub fn get_balance(&self, account: &AccountId) -> Balance {
        self.balances.get(account).unwrap_or(0)
    }

    pub fn has_balance(&self, account: &AccountId, amount: Balance) -> bool {
        self.get_balance(account) >= amount
    }

    pub fn burn(&mut self, account: &AccountId, amount: Balance) {
        let current_balance = self.get_balance(account);
        assert!(current_balance >= amount, "Insufficient balance to burn");
        
        if current_balance == amount {
            self.balances.remove(account);
        } else {
            self.balances.insert(account, &(current_balance - amount));
        }
        
        env::log_str(&format!("Bank: Burned {} from {}", amount, account));
    }

    pub fn get_all_balances(&self, account: AccountId) -> Vec<(String, Balance)> {
        // Return the single balance entry for the account
        let balance = self.get_balance(&account);
        if balance > 0 {
            vec![("unear".to_string(), balance)]
        } else {
            Vec::new()
        }
    }

    pub fn get_total_supply(&self, _denom: String) -> Balance {
        // For now, return 0 - in a full implementation, we'd track total supply
        0
    }
}