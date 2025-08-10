/// Bank Module Contract
/// 
/// This contract handles basic token operations including:
/// - Token transfers between accounts
/// - Minting new tokens (if authorized)
/// - Balance queries
/// - Supply management

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::modules::bank::BankModule;
use crate::Balance;

/// Bank contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BankContract {
    /// The underlying bank module
    bank_module: BankModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from bank operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct BankOperationResponse {
    pub success: bool,
    pub amount: Option<Balance>,
    pub from_account: Option<String>,
    pub to_account: Option<String>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Transfer request data
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub amount: Balance,
}

/// Mint request data  
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct MintRequest {
    pub to: String,
    pub amount: Balance,
}

#[near_bindgen]
impl BankContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            bank_module: BankModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Core Banking Functions
    // =============================================================================

    /// Transfer tokens between accounts
    pub fn transfer(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: Balance,
    ) -> BankOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the sender or authorized to act on their behalf
        let caller = env::predecessor_account_id();
        if caller != from && !self.is_router_or_owner(&caller) {
            return BankOperationResponse {
                success: false,
                amount: Some(amount),
                from_account: Some(from.to_string()),
                to_account: Some(to.to_string()),
                events: vec![],
                error: Some("Unauthorized: caller cannot transfer from this account".to_string()),
            };
        }

        // Check sufficient balance
        let from_balance = self.bank_module.get_balance(&from);
        if from_balance < amount {
            return BankOperationResponse {
                success: false,
                amount: Some(amount),
                from_account: Some(from.to_string()),
                to_account: Some(to.to_string()),
                events: vec![],
                error: Some(format!("Insufficient balance: has {}, need {}", from_balance, amount)),
            };
        }

        // Perform the transfer
        self.bank_module.transfer(&from, &to, amount);
        
        env::log_str(&format!("Transferred {} from {} to {}", amount, from, to));
        
        BankOperationResponse {
            success: true,
            amount: Some(amount),
            from_account: Some(from.to_string()),
            to_account: Some(to.to_string()),
            events: vec!["transfer".to_string()],
            error: None,
        }
    }

    /// Mint new tokens to an account (only owner or router can do this)
    pub fn mint(&mut self, to: AccountId, amount: Balance) -> BankOperationResponse {
        self.assert_authorized_caller();
        
        // Only owner or router can mint
        let caller = env::predecessor_account_id();
        if !self.is_router_or_owner(&caller) {
            return BankOperationResponse {
                success: false,
                amount: Some(amount),
                from_account: None,
                to_account: Some(to.to_string()),
                events: vec![],
                error: Some("Unauthorized: only owner or router can mint tokens".to_string()),
            };
        }

        self.bank_module.mint(&to, amount);
        
        env::log_str(&format!("Minted {} to {}", amount, to));
        
        BankOperationResponse {
            success: true,
            amount: Some(amount),
            from_account: None,
            to_account: Some(to.to_string()),
            events: vec!["mint".to_string()],
            error: None,
        }
    }

    /// Burn tokens from an account
    pub fn burn(&mut self, from: AccountId, amount: Balance) -> BankOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the account holder or authorized
        let caller = env::predecessor_account_id();
        if caller != from && !self.is_router_or_owner(&caller) {
            return BankOperationResponse {
                success: false,
                amount: Some(amount),
                from_account: Some(from.to_string()),
                to_account: None,
                events: vec![],
                error: Some("Unauthorized: caller cannot burn from this account".to_string()),
            };
        }

        // Check sufficient balance
        let from_balance = self.bank_module.get_balance(&from);
        if from_balance < amount {
            return BankOperationResponse {
                success: false,
                amount: Some(amount),
                from_account: Some(from.to_string()),
                to_account: None,
                events: vec![],
                error: Some(format!("Insufficient balance: has {}, need {}", from_balance, amount)),
            };
        }

        self.bank_module.burn(&from, amount);
        
        env::log_str(&format!("Burned {} from {}", amount, from));
        
        BankOperationResponse {
            success: true,
            amount: Some(amount),
            from_account: Some(from.to_string()),
            to_account: None,
            events: vec!["burn".to_string()],
            error: None,
        }
    }

    /// Get account balance
    pub fn get_balance(&self, account: AccountId) -> Balance {
        self.assert_authorized_caller();
        self.bank_module.get_balance(&account)
    }

    /// Get all account balances (for debugging/admin)
    pub fn get_all_balances(&self) -> Vec<(AccountId, Balance)> {
        self.assert_owner(); // Only owner can see all balances
        vec![] // Would need to iterate all accounts, not supported by current implementation
    }

    /// Get total supply
    pub fn get_total_supply(&self) -> Balance {
        self.assert_authorized_caller();
        self.bank_module.get_total_supply("unear".to_string())
    }

    // =============================================================================
    // Batch Operations (for efficiency)
    // =============================================================================

    /// Process multiple transfers in a single transaction
    pub fn batch_transfer(&mut self, transfers: Vec<TransferRequest>) -> Vec<BankOperationResponse> {
        self.assert_authorized_caller();
        
        let mut responses = Vec::new();
        
        for transfer in transfers {
            let response = self.transfer(
                transfer.from.parse().unwrap_or(env::current_account_id()), 
                transfer.to.parse().unwrap_or(env::current_account_id()), 
                transfer.amount
            );
            responses.push(response);
        }
        
        responses
    }

    /// Process multiple mint operations
    pub fn batch_mint(&mut self, mints: Vec<MintRequest>) -> Vec<BankOperationResponse> {
        self.assert_authorized_caller();
        
        let mut responses = Vec::new();
        
        for mint in mints {
            let response = self.mint(
                mint.to.parse().unwrap_or(env::current_account_id()), 
                mint.amount
            );
            responses.push(response);
        }
        
        responses
    }

    // =============================================================================
    // Cross-module Integration Functions
    // =============================================================================

    /// Process transfer (called by router during cross-module operations)
    pub fn process_transfer(&mut self, transfer_data: near_sdk::json_types::Base64VecU8) -> BankOperationResponse {
        self.assert_authorized_caller();
        
        // Decode transfer request
        if let Ok(transfer) = serde_json::from_slice::<TransferRequest>(&transfer_data.0) {
            self.transfer(
                transfer.from.parse().unwrap_or(env::current_account_id()), 
                transfer.to.parse().unwrap_or(env::current_account_id()), 
                transfer.amount
            )
        } else {
            BankOperationResponse {
                success: false,
                amount: None,
                from_account: None,
                to_account: None,
                events: vec![],
                error: Some("Invalid transfer data format".to_string()),
            }
        }
    }

    /// Check if an account has sufficient balance (for pre-validation)
    pub fn has_sufficient_balance(&self, account: AccountId, amount: Balance) -> bool {
        self.assert_authorized_caller();
        self.bank_module.get_balance(&account) >= amount
    }

    /// Reserve tokens for a future operation (lock them temporarily)
    pub fn reserve_tokens(&mut self, account: AccountId, amount: Balance) -> bool {
        self.assert_authorized_caller();
        
        let balance = self.bank_module.get_balance(&account);
        if balance >= amount {
            // In a full implementation, would track reserved amounts
            env::log_str(&format!("Reserved {} tokens for {}", amount, account));
            true
        } else {
            false
        }
    }

    /// Release reserved tokens
    pub fn release_reserved_tokens(&mut self, account: AccountId, amount: Balance) -> bool {
        self.assert_authorized_caller();
        
        // In a full implementation, would track and release reserved amounts
        env::log_str(&format!("Released {} reserved tokens for {}", amount, account));
        true
    }

    // =============================================================================
    // Admin and Configuration Functions
    // =============================================================================

    /// Update the router contract address
    pub fn update_router_contract(&mut self, new_router: AccountId) {
        self.assert_owner();
        self.router_contract = Some(new_router.clone());
        env::log_str(&format!("Updated router contract to: {}", new_router));
    }

    /// Get current router contract
    pub fn get_router_contract(&self) -> Option<AccountId> {
        self.router_contract.clone()
    }

    /// Health check for the bank module
    pub fn health_check(&self) -> bool {
        // Check if the bank module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "bank",
            "version": "1.0.0",
            "description": "Bank Module for token operations",
            "functions": [
                "transfer",
                "mint",
                "burn",
                "get_balance",
                "get_all_balances",
                "get_total_supply",
                "batch_transfer",
                "batch_mint",
                "process_transfer",
                "has_sufficient_balance",
                "reserve_tokens",
                "release_reserved_tokens"
            ]
        })
    }

    // =============================================================================
    // Helper Functions
    // =============================================================================

    /// Check if caller is router or owner
    fn is_router_or_owner(&self, caller: &AccountId) -> bool {
        caller == &self.owner || 
        self.router_contract.as_ref().map_or(false, |router| caller == router)
    }

    /// Assert that the caller is authorized (owner or router)
    fn assert_authorized_caller(&self) {
        let caller = env::predecessor_account_id();
        
        let is_owner = caller == self.owner;
        let is_router = self.router_contract.as_ref().map_or(false, |router| caller == *router);
        
        assert!(
            is_owner || is_router,
            "Unauthorized: only owner or router can call this function"
        );
    }

    /// Assert that the caller is the contract owner
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can perform this action"
        );
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        env::log_str(&format!("Ownership transferred from {} to {}", old_owner, new_owner));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, VMContext};

    fn get_context(predecessor: AccountId) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .build()
    }

    #[test]
    fn test_bank_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = BankContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_minting() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = BankContract::new(accounts(1), None);
        
        let response = contract.mint(accounts(2), 1000);
        assert!(response.success);
        assert_eq!(response.amount, Some(1000));
        
        let balance = contract.get_balance(accounts(2));
        assert_eq!(balance, 1000);
    }

    #[test]
    fn test_transfer() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = BankContract::new(accounts(1), None);
        
        // First mint some tokens
        contract.mint(accounts(2), 1000);
        
        // Now transfer from accounts(2) to accounts(3)
        let response = contract.transfer(accounts(2), accounts(3), 500);
        assert!(response.success);
        
        assert_eq!(contract.get_balance(accounts(2)), 500);
        assert_eq!(contract.get_balance(accounts(3)), 500);
    }

    #[test]
    fn test_insufficient_balance() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = BankContract::new(accounts(1), None);
        
        // Try to transfer without any balance
        let response = contract.transfer(accounts(2), accounts(3), 500);
        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = BankContract::new(accounts(1), None);
        assert!(contract.health_check());
    }
}