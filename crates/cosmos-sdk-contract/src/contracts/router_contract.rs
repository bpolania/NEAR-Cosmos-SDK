/// Main Router Contract for Modular Cosmos SDK Architecture
/// 
/// This contract coordinates between specialized module contracts to provide
/// a unified Cosmos SDK interface while keeping individual modules small enough
/// for NEAR runtime instantiation limits.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise, PromiseResult};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

/// Configuration for module contracts
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ModuleConfig {
    pub contract_id: String,
    pub enabled: bool,
    pub version: String,
}

/// Registry of all module contracts in the system
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ModuleRegistry {
    pub ibc_client: Option<ModuleConfig>,
    pub ibc_connection: Option<ModuleConfig>,
    pub ibc_channel: Option<ModuleConfig>,
    pub ibc_transfer: Option<ModuleConfig>,
    pub wasm: Option<ModuleConfig>,
    pub bank: Option<ModuleConfig>,
    pub staking: Option<ModuleConfig>,
    pub governance: Option<ModuleConfig>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self {
            ibc_client: None,
            ibc_connection: None,
            ibc_channel: None,
            ibc_transfer: None,
            wasm: None,
            bank: None,
            staking: None,
            governance: None,
        }
    }
}

/// Router contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct RouterContract {
    /// Owner of the router (can update module registry)
    pub owner: AccountId,
    /// Registry of all module contracts
    pub modules: ModuleRegistry,
    /// Block height for chain operations
    pub block_height: u64,
    /// Chain configuration
    pub chain_id: String,
}

/// Response from cross-module operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct CrossModuleResponse {
    pub success: bool,
    pub data: Option<String>,
    pub events: Vec<String>,
    pub gas_used: u64,
}

/// Request for cross-module operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct CrossModuleRequest {
    pub operation: String,
    pub target_modules: Vec<String>,
    pub data: String,
}

#[near_bindgen]
impl RouterContract {
    #[init]
    pub fn new(owner: AccountId, chain_id: String) -> Self {
        Self {
            owner,
            modules: ModuleRegistry::default(),
            block_height: 0,
            chain_id,
        }
    }

    /// Register a module contract
    pub fn register_module(
        &mut self,
        module_type: String,
        contract_id: String,
        version: String,
    ) -> bool {
        self.assert_owner();
        
        let config = ModuleConfig {
            contract_id,
            enabled: true,
            version,
        };

        let contract_id = config.contract_id.clone();
        match module_type.as_str() {
            "ibc_client" => self.modules.ibc_client = Some(config),
            "ibc_connection" => self.modules.ibc_connection = Some(config.clone()),
            "ibc_channel" => self.modules.ibc_channel = Some(config.clone()),
            "ibc_transfer" => self.modules.ibc_transfer = Some(config.clone()),
            "wasm" => self.modules.wasm = Some(config.clone()),
            "bank" => self.modules.bank = Some(config.clone()),
            "staking" => self.modules.staking = Some(config.clone()),
            "governance" => self.modules.governance = Some(config.clone()),
            _ => return false,
        }
        
        env::log_str(&format!("Registered module: {} -> {}", module_type, contract_id));
        true
    }

    /// Get all registered modules (for discovery)
    pub fn get_modules(&self) -> HashMap<String, String> {
        let mut modules = HashMap::new();
        
        if let Some(ref config) = self.modules.ibc_client {
            modules.insert("ibc_client".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.ibc_connection {
            modules.insert("ibc_connection".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.ibc_channel {
            modules.insert("ibc_channel".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.ibc_transfer {
            modules.insert("ibc_transfer".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.wasm {
            modules.insert("wasm".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.bank {
            modules.insert("bank".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.staking {
            modules.insert("staking".to_string(), config.contract_id.clone());
        }
        if let Some(ref config) = self.modules.governance {
            modules.insert("governance".to_string(), config.contract_id.clone());
        }
        
        modules
    }

    /// Route a function call to the appropriate module
    pub fn route_call(
        &self,
        module_type: String,
        method_name: String,
        args: Base64VecU8,
    ) -> Promise {
        let module_config = self.get_module_config(&module_type)
            .expect(&format!("Module {} not registered", module_type));
        
        env::log_str(&format!("Routing {} to module {}", method_name, module_config.contract_id));
        
        // Make cross-contract call to the module
        Promise::new(module_config.contract_id.parse().unwrap())
            .function_call(
                method_name,
                args.into(),
                near_sdk::NearToken::from_yoctonear(0), // no attached deposit
                near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 2), // use half remaining gas
            )
    }

    /// Execute a cross-module operation that spans multiple contracts
    pub fn execute_cross_module_operation(&mut self, request: CrossModuleRequest) -> Promise {
        env::log_str(&format!("Executing cross-module operation: {}", request.operation));
        
        match request.operation.as_str() {
            "ibc_send_packet" => self.handle_ibc_send_packet(request),
            "ibc_recv_packet" => self.handle_ibc_recv_packet(request),
            "ibc_ack_packet" => self.handle_ibc_ack_packet(request),
            "wasm_with_bank_transfer" => self.handle_wasm_with_bank_transfer(request),
            _ => {
                env::panic_str(&format!("Unknown cross-module operation: {}", request.operation));
            }
        }
    }

    /// Handle IBC packet sending (involves Channel + Connection + Client modules)
    fn handle_ibc_send_packet(&self, request: CrossModuleRequest) -> Promise {
        // Step 1: Validate with Channel module
        let channel_config = self.get_module_config("ibc_channel")
            .expect("IBC Channel module not registered");
        
        Promise::new(channel_config.contract_id.parse().unwrap())
            .function_call(
                "validate_send_packet".to_string(),
                request.data.clone().into(),
                near_sdk::NearToken::from_yoctonear(0),
                near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 4),
            )
            // Step 2: Check connection state
            .then(
                Promise::new(env::current_account_id())
                    .function_call(
                        "continue_ibc_send_packet".to_string(),
                        request.data.into(),
                        near_sdk::NearToken::from_yoctonear(0),
                        near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 4),
                    )
            )
    }

    /// Handle IBC packet receipt (involves multiple modules)
    fn handle_ibc_recv_packet(&self, request: CrossModuleRequest) -> Promise {
        let channel_config = self.get_module_config("ibc_channel")
            .expect("IBC Channel module not registered");
        
        Promise::new(channel_config.contract_id.parse().unwrap())
            .function_call(
                "receive_packet".to_string(),
                request.data.into(),
                near_sdk::NearToken::from_yoctonear(0),
                near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 2),
            )
    }

    /// Handle IBC packet acknowledgment
    fn handle_ibc_ack_packet(&self, request: CrossModuleRequest) -> Promise {
        let channel_config = self.get_module_config("ibc_channel")
            .expect("IBC Channel module not registered");
        
        Promise::new(channel_config.contract_id.parse().unwrap())
            .function_call(
                "acknowledge_packet".to_string(),
                request.data.into(),
                near_sdk::NearToken::from_yoctonear(0),
                near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 2),
            )
    }

    /// Handle WASM execution with bank transfer
    fn handle_wasm_with_bank_transfer(&self, request: CrossModuleRequest) -> Promise {
        let bank_config = self.get_module_config("bank")
            .expect("Bank module not registered");
        
        // First execute bank transfer, then WASM
        Promise::new(bank_config.contract_id.parse().unwrap())
            .function_call(
                "process_transfer".to_string(),
                request.data.clone().into(),
                near_sdk::NearToken::from_yoctonear(0),
                near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 3),
            )
            .then(
                Promise::new(env::current_account_id())
                    .function_call(
                        "continue_wasm_execution".to_string(),
                        request.data.into(),
                        near_sdk::NearToken::from_yoctonear(0),
                        near_sdk::Gas::from_gas(env::prepaid_gas().as_gas() / 3),
                    )
            )
    }

    /// Continue IBC send packet after channel validation
    #[private]
    pub fn continue_ibc_send_packet(&mut self, packet_data: Base64VecU8) -> CrossModuleResponse {
        // Check if previous call succeeded
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                env::log_str("Channel validation successful, proceeding with packet send");
                // In a full implementation, would continue with connection/client validation
                CrossModuleResponse {
                    success: true,
                    data: Some(general_purpose::STANDARD.encode(&packet_data.0)),
                    events: vec!["packet_sent".to_string()],
                    gas_used: env::used_gas().as_gas(),
                }
            }
            PromiseResult::Failed => {
                env::log_str("Channel validation failed");
                CrossModuleResponse {
                    success: false,
                    data: None,
                    events: vec!["packet_send_failed".to_string()],
                    gas_used: env::used_gas().as_gas(),
                }
            }
        }
    }

    /// Continue WASM execution after bank transfer
    #[private]
    pub fn continue_wasm_execution(&self, execution_data: Base64VecU8) -> CrossModuleResponse {
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                env::log_str("Bank transfer successful, executing WASM");
                // Would call WASM module here
                CrossModuleResponse {
                    success: true,
                    data: Some(general_purpose::STANDARD.encode(&execution_data.0)),
                    events: vec!["wasm_executed".to_string()],
                    gas_used: env::used_gas().as_gas(),
                }
            }
            PromiseResult::Failed => {
                env::log_str("Bank transfer failed, skipping WASM execution");
                CrossModuleResponse {
                    success: false,
                    data: None,
                    events: vec!["wasm_execution_failed".to_string()],
                    gas_used: env::used_gas().as_gas(),
                }
            }
        }
    }

    /// Update block height (called by block processing)
    pub fn update_block_height(&mut self, new_height: u64) {
        self.assert_owner();
        self.block_height = new_height;
        env::log_str(&format!("Updated block height to {}", new_height));
    }

    /// Get current block height
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }

    /// Get module configuration
    fn get_module_config(&self, module_type: &str) -> Option<&ModuleConfig> {
        match module_type {
            "ibc_client" => self.modules.ibc_client.as_ref(),
            "ibc_connection" => self.modules.ibc_connection.as_ref(),
            "ibc_channel" => self.modules.ibc_channel.as_ref(),
            "ibc_transfer" => self.modules.ibc_transfer.as_ref(),
            "wasm" => self.modules.wasm.as_ref(),
            "bank" => self.modules.bank.as_ref(),
            "staking" => self.modules.staking.as_ref(),
            "governance" => self.modules.governance.as_ref(),
            _ => None,
        }
    }

    /// Assert that caller is the contract owner
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

    /// Health check for the router and all modules
    pub fn health_check(&self) -> HashMap<String, bool> {
        let mut health = HashMap::new();
        health.insert("router".to_string(), true);
        
        // In a full implementation, would ping each module
        let modules = self.get_modules();
        for (module_name, _contract_id) in modules {
            // For now, assume all registered modules are healthy
            health.insert(module_name, true);
        }
        
        health
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
    fn test_router_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let router = RouterContract::new(accounts(1), "test-chain".to_string());
        assert_eq!(router.owner, accounts(1));
        assert_eq!(router.chain_id, "test-chain");
        assert_eq!(router.block_height, 0);
    }

    #[test]
    fn test_module_registration() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut router = RouterContract::new(accounts(1), "test-chain".to_string());
        
        let success = router.register_module(
            "bank".to_string(),
            accounts(2).to_string(),
            "1.0.0".to_string(),
        );
        
        assert!(success);
        assert!(router.modules.bank.is_some());
        assert_eq!(router.modules.bank.as_ref().unwrap().contract_id, accounts(2).to_string());
    }

    #[test]
    fn test_get_modules() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut router = RouterContract::new(accounts(1), "test-chain".to_string());
        router.register_module("bank".to_string(), accounts(2).to_string(), "1.0.0".to_string());
        router.register_module("wasm".to_string(), accounts(3).to_string(), "1.0.0".to_string());
        
        let modules = router.get_modules();
        assert_eq!(modules.len(), 2);
        assert!(modules.contains_key("bank"));
        assert!(modules.contains_key("wasm"));
    }
}