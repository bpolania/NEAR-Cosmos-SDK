// Modular-only contract for local deployment
// This is a simplified version that includes only the router functionality
// for easy local testing of the modular architecture

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

// Re-export the modular contracts
pub use crate::contracts::router_contract::RouterContract;

// For local testing, we'll use just the router contract
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ModularCosmosContract {
    router: RouterContract,
}

#[near_bindgen] 
impl ModularCosmosContract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            router: RouterContract::new(owner),
        }
    }

    // Delegate all calls to the router
    pub fn register_module(&mut self, module_type: String, contract_id: String, version: String) -> bool {
        self.router.register_module(module_type, contract_id, version)
    }

    pub fn get_modules(&self) -> std::collections::HashMap<String, String> {
        self.router.get_modules()
    }

    pub fn health_check(&self) -> std::collections::HashMap<String, bool> {
        self.router.health_check()
    }

    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "Modular Cosmos SDK",
            "version": "1.0.0",
            "type": "modular_deployment",
            "description": "Router-based modular architecture for local testing"
        })
    }
}