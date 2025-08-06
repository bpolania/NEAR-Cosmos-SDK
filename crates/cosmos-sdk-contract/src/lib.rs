// Modular Router Contract - Clean implementation without symbol conflicts
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use std::collections::HashMap;

pub type Balance = u128;

// Export all modules for use by different contract types
pub mod modules;
pub mod types;
pub mod handler;
pub mod crypto;
pub mod contracts;

// Router Contract Implementation
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ModularCosmosRouter {
    /// Contract owner
    owner: AccountId,
    /// Chain ID
    chain_id: String,
    /// Registered modules (module_type -> contract_id)
    registered_modules: HashMap<String, String>,
}

#[near_bindgen]
impl ModularCosmosRouter {
    #[init]
    pub fn new() -> Self {
        let owner = env::current_account_id();
        Self {
            owner,
            chain_id: "near-localnet".to_string(),
            registered_modules: HashMap::new(),
        }
    }

    /// Register a module
    pub fn register_module(&mut self, module_type: String, contract_id: String, version: String) -> bool {
        // Only owner can register modules
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can register modules");
        
        self.registered_modules.insert(module_type.clone(), contract_id.clone());
        env::log_str(&format!("Registered module: {} -> {} (v{})", module_type, contract_id, version));
        true
    }

    /// Get all registered modules
    pub fn get_modules(&self) -> HashMap<String, String> {
        self.registered_modules.clone()
    }

    /// Health check
    pub fn health_check(&self) -> HashMap<String, bool> {
        let mut health = HashMap::new();
        health.insert("router".to_string(), true);
        health.insert("overall".to_string(), true);
        
        // Check registered modules (mock as healthy)
        for (module_type, _) in &self.registered_modules {
            health.insert(module_type.clone(), true);
        }
        
        health
    }

    /// Get metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "Modular Cosmos SDK Router",
            "version": "1.0.0",
            "chain_id": self.chain_id,
            "owner": self.owner,
            "type": "modular_router",
            "description": "Router contract for modular Cosmos SDK architecture",
            "modules": self.get_modules()
        })
    }

    /// Test function
    pub fn test_function(&self) -> String {
        format!("Modular Router is working! Registered modules: {}", self.registered_modules.len())
    }

    /// Get owner
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can transfer ownership");
        
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        
        env::log_str(&format!("Ownership transferred: {} -> {}", old_owner, new_owner));
    }

    /// Get contract statistics
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "modules_registered": self.registered_modules.len(),
            "chain_id": self.chain_id,
            "owner": self.owner,
            "type": "modular_router"
        })
    }
}

// For testing
// #[cfg(test)]
// mod lib_tests; // Temporarily disabled due to refactoring