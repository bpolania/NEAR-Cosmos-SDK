/// x/wasm Module Contract
/// 
/// This contract handles all WASM smart contract operations including:
/// - Code storage and management
/// - Contract instantiation and execution
/// - Contract queries and metadata
/// - Admin functions and access control

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use base64::{Engine as _, engine::general_purpose};

use crate::modules::wasm::{
    WasmModule, CodeID,
    AccessConfig, CodeInfo, ContractInfo, Coin
};

/// x/wasm contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct WasmModuleContract {
    /// The underlying wasm module
    wasm_module: WasmModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from wasm operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct WasmOperationResponse {
    pub success: bool,
    pub data: Option<String>,
    pub code_id: Option<CodeID>,
    pub contract_address: Option<String>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Contract instantiation request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct InstantiateRequest {
    pub code_id: CodeID,
    pub msg: String,
    pub funds: Vec<Coin>,
    pub label: String,
    pub admin: Option<String>,
}

/// Contract execution request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ExecuteRequest {
    pub contract_addr: String,
    pub msg: String,
    pub funds: Vec<Coin>,
}

#[near_bindgen]
impl WasmModuleContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            wasm_module: WasmModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Code Management Functions
    // =============================================================================

    /// Store WASM code and return CodeID
    pub fn store_code(
        &mut self,
        wasm_byte_code: Base64VecU8,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> WasmOperationResponse {
        self.assert_authorized_caller();
        
        let sender = env::predecessor_account_id();
        
        match self.wasm_module.store_code(
            &sender,
            wasm_byte_code.into(),
            source,
            builder,
            instantiate_permission
        ) {
            Ok(code_id) => {
                env::log_str(&format!("Stored WASM code with ID: {}", code_id));
                WasmOperationResponse {
                    success: true,
                    data: None,
                    code_id: Some(code_id),
                    contract_address: None,
                    events: vec!["code_stored".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Code storage failed: {}", e));
                WasmOperationResponse {
                    success: false,
                    data: None,
                    code_id: None,
                    contract_address: None,
                    events: vec![],
                    error: Some(e),
                }
            }
        }
    }

    /// Instantiate a contract from stored code
    pub fn instantiate(&mut self, request: InstantiateRequest) -> WasmOperationResponse {
        self.assert_authorized_caller();
        
        let sender = env::predecessor_account_id();
        
        match self.wasm_module.instantiate_contract(
            &sender,
            request.code_id,
            request.msg.into(),
            request.funds,
            request.label,
            request.admin.as_ref().and_then(|a| a.parse().ok()),
        ) {
            Ok(response) => {
                env::log_str(&format!("Instantiated contract at: {}", response.address));
                WasmOperationResponse {
                    success: true,
                    data: response.data.map(|d| general_purpose::STANDARD.encode(d)),
                    code_id: Some(request.code_id),
                    contract_address: Some(response.address),
                    events: response.events,
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Contract instantiation failed: {}", e));
                WasmOperationResponse {
                    success: false,
                    data: None,
                    code_id: Some(request.code_id),
                    contract_address: None,
                    events: vec![],
                    error: Some(e),
                }
            }
        }
    }

    /// Execute a message on a contract
    pub fn execute(&mut self, request: ExecuteRequest) -> WasmOperationResponse {
        self.assert_authorized_caller();
        
        let sender = env::predecessor_account_id();
        
        match self.wasm_module.execute_contract(
            &sender,
            &request.contract_addr,
            request.msg.into(),
            request.funds,
        ) {
            Ok(response) => {
                env::log_str(&format!("Executed contract: {}", request.contract_addr));
                WasmOperationResponse {
                    success: true,
                    data: response.data.map(|d| general_purpose::STANDARD.encode(d)),
                    code_id: None,
                    contract_address: Some(request.contract_addr),
                    events: response.events,
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Contract execution failed: {}", e));
                WasmOperationResponse {
                    success: false,
                    data: None,
                    code_id: None,
                    contract_address: Some(request.contract_addr),
                    events: vec![],
                    error: Some(e),
                }
            }
        }
    }

    /// Query a contract (view function)
    pub fn query(&self, contract_addr: String, msg: Base64VecU8) -> Base64VecU8 {
        self.assert_authorized_caller();
        
        match self.wasm_module.query_contract(&contract_addr, msg.into()) {
            Ok(result) => result.into(),
            Err(e) => {
                env::log_str(&format!("Contract query failed: {}", e));
                vec![].into()
            }
        }
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get code info
    pub fn get_code_info(&self, code_id: CodeID) -> Option<CodeInfo> {
        self.assert_authorized_caller();
        self.wasm_module.get_code_info(code_id)
    }

    /// List stored codes with pagination
    pub fn list_codes(&self, start_after: Option<CodeID>, limit: Option<u32>) -> Vec<CodeInfo> {
        self.assert_authorized_caller();
        self.wasm_module.list_codes(start_after, limit)
    }

    /// List contracts by code ID
    pub fn list_contracts_by_code(
        &self,
        code_id: CodeID,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<ContractInfo> {
        self.assert_authorized_caller();
        self.wasm_module.list_contracts_by_code(code_id, start_after, limit)
    }

    /// Get contract info
    pub fn get_contract_info(&self, address: String) -> Option<ContractInfo> {
        self.assert_authorized_caller();
        self.wasm_module.get_contract_info(&address)
    }

    /// Get all contracts
    pub fn get_all_contracts(&self) -> Vec<ContractInfo> {
        self.assert_authorized_caller();
        self.wasm_module.get_all_contracts()
    }

    /// Check if contract exists
    pub fn contract_exists(&self, address: String) -> bool {
        self.assert_authorized_caller();
        self.wasm_module.get_contract_info(&address).is_some()
    }

    // =============================================================================
    // Cross-module Integration Functions
    // =============================================================================

    /// Process contract execution with bank transfer (called by router)
    pub fn execute_with_funds(&mut self, execution_data: Base64VecU8) -> WasmOperationResponse {
        self.assert_authorized_caller();
        
        // Decode execution request
        if let Ok(request) = serde_json::from_slice::<ExecuteRequest>(&execution_data.0) {
            self.execute(request)
        } else {
            WasmOperationResponse {
                success: false,
                data: None,
                code_id: None,
                contract_address: None,
                events: vec![],
                error: Some("Invalid execution data format".to_string()),
            }
        }
    }

    /// Validate contract execution (for pre-execution checks)
    pub fn validate_execution(&self, contract_addr: String, msg: Base64VecU8) -> bool {
        self.assert_authorized_caller();
        
        // Check if contract exists
        if !self.contract_exists(contract_addr) {
            return false;
        }
        
        // In a full implementation, would validate the message format
        !msg.0.is_empty()
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

    /// Health check for the wasm module
    pub fn health_check(&self) -> bool {
        // Check if the wasm module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "wasm",
            "version": "1.0.0",
            "description": "x/wasm Module for smart contract management",
            "functions": [
                "store_code",
                "instantiate",
                "execute",
                "query",
                "get_code_info",
                "list_codes",
                "list_contracts_by_code",
                "get_contract_info",
                "get_all_contracts",
                "contract_exists",
                "execute_with_funds",
                "validate_execution"
            ],
            "total_codes": self.wasm_module.list_codes(None, None).len()
        })
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
    fn test_wasm_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = WasmModuleContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_authorized_caller_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = WasmModuleContract::new(accounts(1), Some(accounts(2)));
        
        // Owner should be authorized
        contract.assert_authorized_caller();
        
        // Test router access
        let router_context = get_context(accounts(2));
        testing_env!(router_context);
        contract.assert_authorized_caller();
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_caller() {
        let context = get_context(accounts(3)); // Unauthorized account
        testing_env!(context);
        
        let contract = WasmModuleContract::new(accounts(1), Some(accounts(2)));
        contract.assert_authorized_caller();
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = WasmModuleContract::new(accounts(1), None);
        assert!(contract.health_check());
    }

    #[test]
    fn test_validation() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = WasmModuleContract::new(accounts(1), None);
        
        // Test with non-existent contract
        let result = contract.validate_execution("non-existent".to_string(), vec![1, 2, 3].into());
        assert!(!result);
    }
}