/// Real CW20 Contract Wrapper
/// 
/// This wrapper demonstrates how a real CW20 contract using cosmwasm_std imports
/// can be integrated with our compatibility layer to run on NEAR.

use near_sdk::{near_bindgen, PanicOnDefault, env, AccountId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::modules::cosmwasm::{
    storage::CosmWasmStorage,
    api::CosmWasmApi,
    deps::{CosmWasmDeps, CosmWasmDepsMut},
    env::{get_cosmwasm_env, get_message_info},
    memory::CosmWasmMemoryManager,
};

use crate::contracts::cw20_base::{
    Cw20Contract, InstantiateMsg, ExecuteMsg, QueryMsg
};

/// Real CW20 Contract Wrapper that uses our actual CW20 implementation
#[derive(BorshDeserialize, BorshSerialize)]
pub struct RealCw20Wrapper {
    /// CosmWasm-compatible storage
    storage: CosmWasmStorage,
    
    /// Cryptographic API handler
    api: CosmWasmApi,
    
    /// Memory management for contract execution
    memory_manager: CosmWasmMemoryManager,
    
    /// Contract initialization status
    initialized: bool,
    
    /// Contract admin (for migration purposes)
    admin: Option<AccountId>,
    
    /// Contract version for migration tracking
    version: String,
}

/// Initialization message for the wrapper
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cw20WrapperInitMsg {
    /// Admin account for contract management
    pub admin: Option<String>,
    /// Contract version identifier
    pub version: Option<String>,
    /// The CW20 instantiate message
    pub cw20_instantiate_msg: InstantiateMsg,
}

/// Execute message wrapper
#[derive(Serialize, Deserialize, Debug)]
pub struct Cw20WrapperExecuteMsg {
    /// The CW20 execute message
    pub cw20_execute_msg: ExecuteMsg,
}

/// Query message wrapper  
#[derive(Serialize, Deserialize, Debug)]
pub struct Cw20WrapperQueryMsg {
    /// The CW20 query message
    pub cw20_query_msg: QueryMsg,
}

/// Response from contract operations
#[derive(Serialize, Deserialize, Debug)]
pub struct Cw20WrapperResponse {
    /// Success status
    pub success: bool,
    /// Response data (JSON string)
    pub data: Option<String>,
    /// Error message if operation failed
    pub error: Option<String>,
    /// Events emitted (for logging purposes)
    pub events: Vec<String>,
}

#[near_bindgen]
impl RealCw20Wrapper {
    /// Initialize the CW20 contract wrapper
    #[init]
    pub fn new(init_msg: Cw20WrapperInitMsg) -> Self {
        let mut contract = Self {
            storage: CosmWasmStorage::new(),
            api: CosmWasmApi::new(),
            memory_manager: CosmWasmMemoryManager::new(),
            initialized: false,
            admin: init_msg.admin.map(|a| a.parse().unwrap()),
            version: init_msg.version.unwrap_or_else(|| "1.0.0".to_string()),
        };
        
        // Initialize the CW20 contract
        match contract.call_instantiate(init_msg.cw20_instantiate_msg) {
            Ok(_) => {
                contract.initialized = true;
                env::log_str("Real CW20 contract wrapper initialized successfully");
            }
            Err(e) => {
                env::panic_str(&format!("Failed to initialize CW20 contract: {}", e));
            }
        }
        
        contract
    }
    
    /// Execute a message on the CW20 contract
    pub fn execute(&mut self, msg: Cw20WrapperExecuteMsg) -> Cw20WrapperResponse {
        if !self.initialized {
            return Cw20WrapperResponse::error("Contract not initialized");
        }
        
        match self.call_execute(msg.cw20_execute_msg) {
            Ok(data) => Cw20WrapperResponse::success(data),
            Err(e) => Cw20WrapperResponse::error(&e),
        }
    }
    
    /// Query the CW20 contract (read-only)
    pub fn query(&self, msg: Cw20WrapperQueryMsg) -> Cw20WrapperResponse {
        if !self.initialized {
            return Cw20WrapperResponse::error("Contract not initialized");
        }
        
        match self.call_query(msg.cw20_query_msg) {
            Ok(data) => Cw20WrapperResponse::success(Some(data)),
            Err(e) => Cw20WrapperResponse::error(&e),
        }
    }
    
    /// Get contract information
    pub fn get_contract_info(&self) -> ContractInfoResponse {
        ContractInfoResponse {
            initialized: self.initialized,
            admin: self.admin.as_ref().map(|a| a.to_string()),
            version: self.version.clone(),
            contract_address: env::current_account_id().to_string(),
            contract_type: "CW20".to_string(),
        }
    }
}

/// Internal implementation for calling the real CW20 contract
impl RealCw20Wrapper {
    /// Call the CW20 contract's instantiate function
    fn call_instantiate(&mut self, msg: InstantiateMsg) -> Result<Option<String>, String> {
        // Create dependencies
        let mut deps_mut = CosmWasmDepsMut::new(&mut self.storage, &self.api);
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        // Call the REAL CW20 contract instantiate function
        match Cw20Contract::instantiate(deps_mut.as_deps_mut(), env, info, msg) {
            Ok(response) => {
                env::log_str(&format!("CW20_INSTANTIATE_SUCCESS: {:?}", response.attributes));
                
                // Log all events and attributes
                for attr in &response.attributes {
                    env::log_str(&format!("  {}: {}", attr.key, attr.value));
                }
                
                Ok(Some(serde_json::to_string(&response).unwrap_or_else(|_| "Success".to_string())))
            }
            Err(e) => {
                env::log_str(&format!("CW20_INSTANTIATE_ERROR: {}", e));
                Err(format!("Instantiation failed: {}", e))
            }
        }
    }
    
    /// Call the CW20 contract's execute function
    fn call_execute(&mut self, msg: ExecuteMsg) -> Result<Option<String>, String> {
        // Create dependencies
        let mut deps_mut = CosmWasmDepsMut::new(&mut self.storage, &self.api);
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        env::log_str(&format!("CW20_EXECUTE: {:?}", msg));
        env::log_str(&format!("  Sender: {}", info.sender.as_str()));
        env::log_str(&format!("  Contract: {}", env.contract.address.as_str()));
        
        // Call the REAL CW20 contract execute function
        match Cw20Contract::execute(deps_mut.as_deps_mut(), env, info, msg) {
            Ok(response) => {
                env::log_str(&format!("CW20_EXECUTE_SUCCESS: {:?}", response.attributes));
                
                // Log all events and attributes
                for attr in &response.attributes {
                    env::log_str(&format!("  {}: {}", attr.key, attr.value));
                }
                
                Ok(Some(serde_json::to_string(&response).unwrap_or_else(|_| "Success".to_string())))
            }
            Err(e) => {
                env::log_str(&format!("CW20_EXECUTE_ERROR: {}", e));
                Err(format!("Execution failed: {}", e))
            }
        }
    }
    
    /// Call the CW20 contract's query function
    fn call_query(&self, msg: QueryMsg) -> Result<String, String> {
        // Create dependencies (read-only)
        let deps = CosmWasmDeps::new(&self.storage, &self.api);
        let env = get_cosmwasm_env();
        
        env::log_str(&format!("CW20_QUERY: {:?}", msg));
        
        // Call the REAL CW20 contract query function
        match Cw20Contract::query(deps.as_deps(), env, msg) {
            Ok(binary_result) => {
                // Convert Binary to String for our wrapper response
                let binary_bytes = binary_result.to_vec();
                let result_str = String::from_utf8(binary_bytes.clone())
                    .unwrap_or_else(|_| {
                        use base64::{engine::general_purpose::STANDARD, Engine};
                        STANDARD.encode(binary_bytes)
                    });
                
                env::log_str(&format!("CW20_QUERY_SUCCESS: {}", result_str));
                Ok(result_str)
            }
            Err(e) => {
                env::log_str(&format!("CW20_QUERY_ERROR: {}", e));
                Err(format!("Query failed: {}", e))
            }
        }
    }
}

/// Helper implementations for responses
impl Cw20WrapperResponse {
    pub fn success(data: Option<String>) -> Self {
        Self {
            success: true,
            data,
            error: None,
            events: vec![],
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
            events: vec![],
        }
    }
}

/// Contract information response
#[derive(Serialize, Deserialize, Debug)]
pub struct ContractInfoResponse {
    pub initialized: bool,
    pub admin: Option<String>,
    pub version: String,
    pub contract_address: String,
    pub contract_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use crate::contracts::cw20_base::{Cw20Coin, MinterResponse};
    use crate::modules::cosmwasm::types::Uint128;
    
    fn setup_context() {
        let context = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(1))
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_real_cw20_instantiation() {
        setup_context();
        
        // Create a real CW20 instantiate message
        let cw20_init = InstantiateMsg {
            name: "Real Test Token".to_string(),
            symbol: "REAL".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: accounts(1).to_string(),
                    amount: Uint128::new(1_000_000_000), // 1,000 REAL tokens
                },
                Cw20Coin {
                    address: accounts(2).to_string(),
                    amount: Uint128::new(500_000_000), // 500 REAL tokens
                },
            ],
            mint: Some(MinterResponse {
                minter: accounts(1).to_string(),
                cap: Some(Uint128::new(10_000_000_000)), // 10,000 REAL max supply
            }),
            marketing: None,
        };
        
        let wrapper_init = Cw20WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            cw20_instantiate_msg: cw20_init,
        };
        
        // Initialize the real CW20 contract
        let contract = RealCw20Wrapper::new(wrapper_init);
        
        // Verify initialization
        let info = contract.get_contract_info();
        assert!(info.initialized);
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.contract_type, "CW20");
        assert_eq!(info.admin, Some(accounts(1).to_string()));
    }
    
    #[test]
    fn test_real_cw20_transfer() {
        setup_context();
        
        // Initialize with initial balances
        let cw20_init = InstantiateMsg {
            name: "Transfer Test Token".to_string(),
            symbol: "TTT".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: accounts(1).to_string(),
                    amount: Uint128::new(1_000_000),
                },
            ],
            mint: None,
            marketing: None,
        };
        
        let wrapper_init = Cw20WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            cw20_instantiate_msg: cw20_init,
        };
        
        let mut contract = RealCw20Wrapper::new(wrapper_init);
        
        // Execute transfer using the real CW20 contract
        let transfer_msg = ExecuteMsg::Transfer {
            recipient: accounts(2).to_string(),
            amount: Uint128::new(100_000),
        };
        
        let execute_wrapper = Cw20WrapperExecuteMsg {
            cw20_execute_msg: transfer_msg,
        };
        
        let response = contract.execute(execute_wrapper);
        assert!(response.success);
        assert!(response.data.is_some());
        
        // Query balance to verify transfer
        let balance_query = QueryMsg::Balance {
            address: accounts(2).to_string(),
        };
        
        let query_wrapper = Cw20WrapperQueryMsg {
            cw20_query_msg: balance_query,
        };
        
        let query_response = contract.query(query_wrapper);
        assert!(query_response.success);
        assert!(query_response.data.is_some());
        
        // The response should contain the balance information
        let response_data = query_response.data.unwrap();
        assert!(response_data.contains("100000")); // Should contain the transferred amount
    }
    
    #[test]
    fn test_real_cw20_mint() {
        setup_context();
        
        // Initialize with minter
        let cw20_init = InstantiateMsg {
            name: "Mint Test Token".to_string(),
            symbol: "MINT".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: accounts(1).to_string(),
                cap: Some(Uint128::new(10_000_000)),
            }),
            marketing: None,
        };
        
        let wrapper_init = Cw20WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            cw20_instantiate_msg: cw20_init,
        };
        
        let mut contract = RealCw20Wrapper::new(wrapper_init);
        
        // Execute mint using the real CW20 contract
        let mint_msg = ExecuteMsg::Mint {
            recipient: accounts(2).to_string(),
            amount: Uint128::new(500_000),
        };
        
        let execute_wrapper = Cw20WrapperExecuteMsg {
            cw20_execute_msg: mint_msg,
        };
        
        let response = contract.execute(execute_wrapper);
        assert!(response.success);
        assert!(response.data.is_some());
    }
    
    #[test]
    fn test_real_cw20_token_info_query() {
        setup_context();
        
        // Initialize contract
        let cw20_init = InstantiateMsg {
            name: "Query Test Token".to_string(),
            symbol: "QTT".to_string(),
            decimals: 8,
            initial_balances: vec![],
            mint: None,
            marketing: None,
        };
        
        let wrapper_init = Cw20WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            cw20_instantiate_msg: cw20_init,
        };
        
        let contract = RealCw20Wrapper::new(wrapper_init);
        
        // Query token info using the real CW20 contract
        let token_info_query = QueryMsg::TokenInfo {};
        
        let query_wrapper = Cw20WrapperQueryMsg {
            cw20_query_msg: token_info_query,
        };
        
        let response = contract.query(query_wrapper);
        assert!(response.success);
        assert!(response.data.is_some());
        
        let response_data = response.data.unwrap();
        assert!(response_data.contains("Query Test Token"));
        assert!(response_data.contains("QTT"));
        assert!(response_data.contains("8")); // decimals
    }
}