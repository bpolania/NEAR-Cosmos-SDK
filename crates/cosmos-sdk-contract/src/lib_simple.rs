// Simplified Modular Cosmos SDK Contract for Local Testing
// This version focuses on basic functionality without complex dependencies

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::collections::UnorderedMap;
use serde_json::json;
use std::collections::HashMap;

pub type Balance = u128;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    /// Registered modules (module_type -> contract_id)
    modules: UnorderedMap<String, String>,
    /// Owner of the contract
    owner: AccountId,
    /// Chain ID
    chain_id: String,
    /// Mock balances for testing
    balances: UnorderedMap<AccountId, Balance>,
    /// Mock IBC clients
    ibc_clients: UnorderedMap<String, String>,
    /// Mock WASM codes
    wasm_codes: UnorderedMap<u64, String>,
    /// Next WASM code ID
    next_code_id: u64,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        Self {
            modules: UnorderedMap::new(b"modules".to_vec()),
            owner: env::current_account_id(),
            chain_id: "near-localnet".to_string(),
            balances: UnorderedMap::new(b"balances".to_vec()),
            ibc_clients: UnorderedMap::new(b"ibc_clients".to_vec()),
            wasm_codes: UnorderedMap::new(b"wasm_codes".to_vec()),
            next_code_id: 1,
        }
    }

    // =============================================================================
    // Module Registry Functions (Router functionality)
    // =============================================================================

    /// Register a module
    pub fn register_module(&mut self, module_type: String, contract_id: String, version: String) -> bool {
        // Only owner can register modules
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can register modules");
        
        self.modules.insert(&module_type, &contract_id);
        env::log_str(&format!("Registered module: {} -> {} (v{})", module_type, contract_id, version));
        true
    }

    /// Get all registered modules
    pub fn get_modules(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for (module_type, contract_id) in self.modules.iter() {
            result.insert(module_type, contract_id);
        }
        result
    }

    /// Health check
    pub fn health_check(&self) -> HashMap<String, bool> {
        let mut health = HashMap::new();
        health.insert("router".to_string(), true);
        health.insert("overall".to_string(), true);
        
        // Check registered modules
        for (module_type, _) in self.modules.iter() {
            health.insert(module_type, true); // Mock as healthy
        }
        
        health
    }

    /// Get metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        json!({
            "name": "Modular Cosmos SDK Router",
            "version": "1.0.0-local",
            "chain_id": self.chain_id,
            "owner": self.owner,
            "description": "Simplified modular architecture for local testing",
            "modules": self.get_modules()
        })
    }

    // =============================================================================
    // Basic Bank Functions
    // =============================================================================

    /// Transfer tokens (mock implementation)
    pub fn transfer(&mut self, receiver: AccountId, amount: Balance) -> String {
        let sender = env::predecessor_account_id();
        
        // Get current balances
        let sender_balance = self.balances.get(&sender).unwrap_or(1_000_000); // Default 1M
        let receiver_balance = self.balances.get(&receiver).unwrap_or(0);
        
        assert!(sender_balance >= amount, "Insufficient balance");
        
        // Update balances
        self.balances.insert(&sender, &(sender_balance - amount));
        self.balances.insert(&receiver, &(receiver_balance + amount));
        
        env::log_str(&format!("Transfer: {} -> {} amount: {}", sender, receiver, amount));
        format!("Transferred {} from {} to {}", amount, sender, receiver)
    }

    /// Get balance
    pub fn get_balance(&self, account: AccountId) -> Balance {
        self.balances.get(&account).unwrap_or(1_000_000) // Default 1M for testing
    }

    /// Mint tokens (only owner)
    pub fn mint(&mut self, receiver: AccountId, amount: Balance) -> String {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can mint");
        
        let current_balance = self.balances.get(&receiver).unwrap_or(0);
        self.balances.insert(&receiver, &(current_balance + amount));
        
        format!("Minted {} to {}", amount, receiver)
    }

    // =============================================================================
    // Basic IBC Functions (Mock)
    // =============================================================================

    /// Create IBC client (mock)
    pub fn ibc_create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
    ) -> String {
        let client_id = format!("07-tendermint-{}", self.ibc_clients.len());
        let client_info = format!("chain:{}, trust:{}, unbonding:{}, drift:{}", 
                                 chain_id, trust_period, unbonding_period, max_clock_drift);
        
        self.ibc_clients.insert(&client_id, &client_info);
        
        env::log_str(&format!("Created IBC client: {}", client_id));
        client_id
    }

    /// Get all IBC clients (mock)
    pub fn get_all_clients(&self) -> Vec<String> {
        self.ibc_clients.keys().collect()
    }

    /// Update IBC client (mock)
    pub fn ibc_update_client(&mut self, client_id: String, _header_data: Vec<u8>) -> bool {
        if self.ibc_clients.get(&client_id).is_some() {
            env::log_str(&format!("Updated IBC client: {}", client_id));
            true
        } else {
            false
        }
    }

    // =============================================================================
    // Basic WASM Functions (Mock)
    // =============================================================================

    /// Store WASM code (mock)
    pub fn wasm_store_code(
        &mut self,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
    ) -> u64 {
        let code_id = self.next_code_id;
        self.next_code_id += 1;
        
        let code_info = json!({
            "size": wasm_byte_code.len(),
            "source": source.unwrap_or("unknown".to_string()),
            "builder": builder.unwrap_or("unknown".to_string()),
            "stored_by": env::predecessor_account_id()
        });
        
        self.wasm_codes.insert(&code_id, &code_info.to_string());
        
        env::log_str(&format!("Stored WASM code: {} bytes -> ID {}", wasm_byte_code.len(), code_id));
        code_id
    }

    /// List WASM codes (mock)
    pub fn wasm_list_codes(&self, _start_after: Option<u64>, limit: Option<u32>) -> Vec<serde_json::Value> {
        let limit = limit.unwrap_or(10).min(50) as usize;
        let mut codes = Vec::new();
        
        for (code_id, code_info) in self.wasm_codes.iter() {
            if codes.len() >= limit {
                break;
            }
            
            let code_data: serde_json::Value = serde_json::from_str(&code_info).unwrap_or(json!({}));
            codes.push(json!({
                "code_id": code_id,
                "creator": code_data["stored_by"],
                "source": code_data["source"],
                "builder": code_data["builder"],
                "size": code_data["size"]
            }));
        }
        
        codes
    }

    /// Get code info (mock)
    pub fn wasm_code_info(&self, code_id: u64) -> Option<serde_json::Value> {
        if let Some(code_info) = self.wasm_codes.get(&code_id) {
            let code_data: serde_json::Value = serde_json::from_str(&code_info).unwrap_or(json!({}));
            Some(json!({
                "code_id": code_id,
                "creator": code_data["stored_by"],
                "source": code_data["source"],
                "builder": code_data["builder"],
                "size": code_data["size"]
            }))
        } else {
            None
        }
    }

    // =============================================================================
    // Administrative Functions
    // =============================================================================

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can transfer ownership");
        
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        
        env::log_str(&format!("Ownership transferred: {} -> {}", old_owner, new_owner));
    }

    /// Get owner
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    /// Test function to verify deployment
    pub fn test_function(&self) -> String {
        format!("Modular Cosmos SDK is working! Chain: {}, Modules: {}", 
                self.chain_id, self.modules.len())
    }

    /// Get contract statistics
    pub fn get_stats(&self) -> serde_json::Value {
        json!({
            "modules_registered": self.modules.len(),
            "ibc_clients": self.ibc_clients.len(),
            "wasm_codes_stored": self.wasm_codes.len(),
            "next_code_id": self.next_code_id,
            "chain_id": self.chain_id,
            "owner": self.owner
        })
    }
}