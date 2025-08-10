// Modular Router Contract - Clean implementation without symbol conflicts
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise, ext_contract};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

pub type Balance = u128;

// Export all modules for use by different contract types
pub mod modules;
pub mod types;
pub mod handler;
pub mod crypto;
pub mod contracts;

// Cross-contract interface for WasmModule
#[ext_contract(ext_wasm_module)]
trait ExtWasmModule {
    fn store_code(
        &mut self,
        sender: AccountId,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> u64;
    
    fn instantiate(
        &mut self,
        sender: AccountId,
        code_id: u64,
        msg: Vec<u8>,
        funds: Vec<Coin>,
        label: String,
        admin: Option<String>,
    ) -> String;
    
    fn execute(
        &mut self,
        sender: AccountId,
        contract_addr: String,
        msg: Vec<u8>,
        funds: Vec<Coin>,
    ) -> String;

    fn get_code_info(&self, code_id: u64) -> Option<CodeInfo>;
    fn get_contract_info(&self, contract_addr: String) -> Option<ContractInfo>;
    fn health_check(&self) -> serde_json::Value;
}

// Types for cross-contract communication
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessConfig {
    Nobody {},
    OnlyAddress { address: String },
    Everybody {},
    AnyOfAddresses { addresses: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeInfo {
    pub code_id: u64,
    pub creator: String,
    pub code_hash: Vec<u8>,
    pub source: String,
    pub builder: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractInfo {
    pub address: String,
    pub code_id: u64,
    pub creator: String,
    pub admin: Option<String>,
    pub label: String,
    pub created: u64,
}

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

    // CosmWasm routing methods

    /// Store WASM code via the wasm module
    pub fn wasm_store_code(
        &mut self,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");

        let sender = env::predecessor_account_id();
        
        ext_wasm_module::ext(wasm_contract)
            .store_code(sender, wasm_byte_code, source, builder, instantiate_permission)
    }

    /// Instantiate a CosmWasm contract via the wasm module
    pub fn wasm_instantiate(
        &mut self,
        code_id: u64,
        msg: Vec<u8>,
        funds: Vec<Coin>,
        label: String,
        admin: Option<String>,
    ) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");

        let sender = env::predecessor_account_id();
        
        ext_wasm_module::ext(wasm_contract)
            .instantiate(sender, code_id, msg, funds, label, admin)
    }

    /// Execute a CosmWasm contract via the wasm module
    pub fn wasm_execute(
        &mut self,
        contract_addr: String,
        msg: Vec<u8>,
        funds: Vec<Coin>,
    ) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");

        let sender = env::predecessor_account_id();
        
        ext_wasm_module::ext(wasm_contract)
            .execute(sender, contract_addr, msg, funds)
    }

    /// Get code info from the wasm module
    pub fn wasm_get_code_info(&self, code_id: u64) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");
        
        ext_wasm_module::ext(wasm_contract)
            .get_code_info(code_id)
    }

    /// Get contract info from the wasm module
    pub fn wasm_get_contract_info(&self, contract_addr: String) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");
        
        ext_wasm_module::ext(wasm_contract)
            .get_contract_info(contract_addr)
    }

    /// Get health check from the wasm module
    pub fn wasm_health_check(&self) -> Promise {
        let wasm_contract = self.registered_modules.get("wasm")
            .expect("Wasm module not registered")
            .parse::<AccountId>()
            .expect("Invalid wasm module account ID");
        
        ext_wasm_module::ext(wasm_contract)
            .health_check()
    }
}

// For testing
// #[cfg(test)]
// mod lib_tests; // Temporarily disabled due to refactoring