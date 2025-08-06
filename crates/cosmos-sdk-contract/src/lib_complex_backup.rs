// Modular Cosmos SDK Contract - Local Deployment Version
// This version focuses on the modular router-based architecture

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

pub type Balance = u128;

pub mod modules;
pub mod types;
pub mod handler;
pub mod crypto;
pub mod contracts;

// Use only the router contract for local deployment
use contracts::router_contract::RouterContract;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    router: RouterContract,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        let owner = env::current_account_id();
        Self {
            router: RouterContract::new(owner, "near-localnet".to_string()),
        }
    }

    // Router functions
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
            "name": "Modular Cosmos SDK Router",
            "version": "1.0.0",
            "type": "modular_deployment",
            "description": "Router-based modular architecture for local testing"
        })
    }

    // Basic functionality for testing
    pub fn test_function(&self) -> String {
        "Modular Cosmos SDK is working!".to_string()
    }

    // Bank-like functions for basic testing
    pub fn transfer(&mut self, receiver: AccountId, amount: Balance) -> String {
        let sender = env::predecessor_account_id();
        format!("Transfer simulation: {} -> {} amount: {}", sender, receiver, amount)
    }

    pub fn get_balance(&self, account: AccountId) -> Balance {
        // Return a mock balance for testing
        1000000 // 1M units
    }

    // Basic IBC simulation functions for testing
    pub fn ibc_create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
    ) -> String {
        format!("Created IBC client for chain: {} (trust: {}s, unbonding: {}s, drift: {}s)", 
                chain_id, trust_period, unbonding_period, max_clock_drift)
    }

    pub fn get_all_clients(&self) -> Vec<String> {
        vec!["client-0".to_string(), "client-1".to_string()]
    }

    // WASM simulation functions for testing
    pub fn wasm_store_code(
        &mut self,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
    ) -> u64 {
        let code_size = wasm_byte_code.len();
        env::log_str(&format!("Stored WASM code: {} bytes, source: {:?}, builder: {:?}", 
                             code_size, source, builder));
        1 // Return mock code ID
    }

    pub fn wasm_list_codes(&self, start_after: Option<u64>, limit: Option<u32>) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "code_id": 1,
                "creator": env::current_account_id(),
                "code_hash": "abc123",
                "source": "test",
                "builder": "test"
            })
        ]
    }
}