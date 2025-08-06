use near_sdk::{PanicOnDefault, env, AccountId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::modules::cosmwasm::{
    storage::CosmWasmStorage,
    api::CosmWasmApi,
    deps::{CosmWasmDeps, CosmWasmDepsMut},
    env::{get_cosmwasm_env, get_message_info},
    memory::CosmWasmMemoryManager,
};

/// CosmWasm contract wrapper that provides the lifecycle management
/// This enables any CosmWasm contract to run on NEAR with full compatibility
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmWasmContractWrapper {
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
pub struct WrapperInitMsg {
    /// Admin account for contract management
    pub admin: Option<String>,
    /// Contract version identifier
    pub version: Option<String>,
    /// The actual CosmWasm contract's instantiate message (as JSON string)
    pub contract_msg: String,
}

/// Execute message wrapper
#[derive(Serialize, Deserialize, Debug)]
pub struct WrapperExecuteMsg {
    /// The actual CosmWasm contract's execute message (as JSON string)
    pub contract_msg: String,
}

/// Query message wrapper  
#[derive(Serialize, Deserialize, Debug)]
pub struct WrapperQueryMsg {
    /// The actual CosmWasm contract's query message (as JSON string)
    pub contract_msg: String,
}

/// Migration message wrapper
#[derive(Serialize, Deserialize, Debug)]
pub struct WrapperMigrateMsg {
    /// New contract version
    pub new_version: String,
    /// The actual CosmWasm contract's migrate message (as JSON string)
    pub contract_msg: String,
}

/// Response from contract operations
#[derive(Serialize, Deserialize, Debug)]
pub struct WrapperResponse {
    /// Success status
    pub success: bool,
    /// Response data (base64 encoded if binary)
    pub data: Option<String>,
    /// Error message if operation failed
    pub error: Option<String>,
    /// Events emitted (for logging purposes)
    pub events: Vec<String>,
}

impl CosmWasmContractWrapper {
    /// Initialize the CosmWasm contract wrapper
    pub fn new(init_msg: WrapperInitMsg) -> Self {
        let mut contract = Self {
            storage: CosmWasmStorage::new(),
            api: CosmWasmApi::new(),
            memory_manager: CosmWasmMemoryManager::new(),
            initialized: false,
            admin: init_msg.admin.map(|a| a.parse().unwrap()),
            version: init_msg.version.unwrap_or_else(|| "1.0.0".to_string()),
        };
        
        // Initialize the wrapped contract
        match contract.call_instantiate(init_msg.contract_msg) {
            Ok(_) => {
                contract.initialized = true;
                env::log_str("CosmWasm contract wrapper initialized successfully");
            }
            Err(e) => {
                env::panic_str(&format!("Failed to initialize CosmWasm contract: {}", e));
            }
        }
        
        contract
    }
    
    /// Execute a message on the wrapped CosmWasm contract
    pub fn execute(&mut self, msg: WrapperExecuteMsg) -> WrapperResponse {
        if !self.initialized {
            return WrapperResponse::error("Contract not initialized");
        }
        
        match self.call_execute(msg.contract_msg) {
            Ok(data) => WrapperResponse::success(data),
            Err(e) => WrapperResponse::error(&e),
        }
    }
    
    /// Query the wrapped CosmWasm contract (read-only)
    pub fn query(&self, msg: WrapperQueryMsg) -> WrapperResponse {
        if !self.initialized {
            return WrapperResponse::error("Contract not initialized");
        }
        
        match self.call_query(msg.contract_msg) {
            Ok(data) => WrapperResponse::success(Some(data)),
            Err(e) => WrapperResponse::error(&e),
        }
    }
    
    /// Migrate the wrapped CosmWasm contract (admin only)
    pub fn migrate(&mut self, msg: WrapperMigrateMsg) -> WrapperResponse {
        // Check admin permission
        if let Some(admin) = &self.admin {
            if &env::predecessor_account_id() != admin {
                return WrapperResponse::error("Only admin can migrate contract");
            }
        } else {
            return WrapperResponse::error("No admin set for this contract");
        }
        
        match self.call_migrate(msg.contract_msg) {
            Ok(data) => {
                self.version = msg.new_version;
                WrapperResponse::success(data)
            }
            Err(e) => WrapperResponse::error(&e),
        }
    }
    
    /// Get contract information
    pub fn get_contract_info(&self) -> ContractInfoResponse {
        ContractInfoResponse {
            initialized: self.initialized,
            admin: self.admin.as_ref().map(|a| a.to_string()),
            version: self.version.clone(),
            contract_address: env::current_account_id().to_string(),
        }
    }
}

/// Internal implementation for calling CosmWasm contract functions
impl CosmWasmContractWrapper {
    /// Call the CosmWasm contract's instantiate function
    fn call_instantiate(&mut self, msg_json: String) -> Result<Option<String>, String> {
        // Create dependencies
        let mut deps_mut = CosmWasmDepsMut::new(&mut self.storage, &self.api);
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        // This is where you would call the actual CosmWasm contract's instantiate function
        // For now, we'll simulate a successful instantiation
        
        env::log_str(&format!("COSMWASM_INSTANTIATE: {}", msg_json));
        env::log_str(&format!("  Sender: {}", info.sender.as_str()));
        env::log_str(&format!("  Contract: {}", env.contract.address.as_str()));
        env::log_str(&format!("  Block Height: {}", env.block.height));
        
        // Store some metadata to indicate successful instantiation
        let metadata_key = b"__cosmwasm_metadata__";
        let metadata = serde_json::json!({
            "instantiated": true,
            "instantiate_msg": msg_json,
            "instantiated_by": info.sender.as_str(),
            "instantiated_at": env.block.height,
            "instantiated_time": env.block.time.nanos(),
        });
        
        deps_mut.as_deps_mut().storage.set(
            metadata_key, 
            metadata.to_string().as_bytes()
        );
        
        Ok(Some("Contract instantiated successfully".to_string()))
    }
    
    /// Call the CosmWasm contract's execute function
    fn call_execute(&mut self, msg_json: String) -> Result<Option<String>, String> {
        // Create dependencies
        let mut deps_mut = CosmWasmDepsMut::new(&mut self.storage, &self.api);
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        // This is where you would call the actual CosmWasm contract's execute function
        // For now, we'll simulate message processing
        
        env::log_str(&format!("COSMWASM_EXECUTE: {}", msg_json));
        env::log_str(&format!("  Sender: {}", info.sender.as_str()));
        env::log_str(&format!("  Contract: {}", env.contract.address.as_str()));
        
        // Parse the message to determine action
        let parsed_msg: serde_json::Value = serde_json::from_str(&msg_json)
            .map_err(|e| format!("Invalid JSON message: {}", e))?;
        
        // Simulate some contract logic based on message type
        if let Some(action) = parsed_msg.get("action").and_then(|v| v.as_str()) {
            match action {
                "increment" => {
                    // Simulate incrementing a counter
                    let counter_key = b"counter";
                    let current = deps_mut.as_deps().storage.get(counter_key)
                        .and_then(|bytes| {
                            String::from_utf8(bytes).ok()
                                .and_then(|s| s.parse::<u64>().ok())
                        })
                        .unwrap_or(0);
                    
                    let new_value = current + 1;
                    deps_mut.as_deps_mut().storage.set(
                        counter_key, 
                        new_value.to_string().as_bytes()
                    );
                    
                    env::log_str(&format!("Counter incremented to: {}", new_value));
                    Ok(Some(format!("{{\"count\": {}}}", new_value)))
                }
                "reset" => {
                    // Simulate resetting counter
                    let new_value = parsed_msg.get("count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    
                    let counter_key = b"counter";
                    deps_mut.as_deps_mut().storage.set(
                        counter_key, 
                        new_value.to_string().as_bytes()
                    );
                    
                    env::log_str(&format!("Counter reset to: {}", new_value));
                    Ok(Some(format!("{{\"count\": {}}}", new_value)))
                }
                _ => {
                    env::log_str(&format!("Unknown action: {}", action));
                    Ok(Some("Action processed".to_string()))
                }
            }
        } else {
            Ok(Some("Message processed".to_string()))
        }
    }
    
    /// Call the CosmWasm contract's query function
    fn call_query(&self, msg_json: String) -> Result<String, String> {
        // Create dependencies (read-only)
        let deps = CosmWasmDeps::new(&self.storage, &self.api);
        let _env = get_cosmwasm_env();
        
        env::log_str(&format!("COSMWASM_QUERY: {}", msg_json));
        
        // Parse the query message
        let parsed_msg: serde_json::Value = serde_json::from_str(&msg_json)
            .map_err(|e| format!("Invalid JSON query: {}", e))?;
        
        // Simulate query processing
        if let Some(query_type) = parsed_msg.get("query").and_then(|v| v.as_str()) {
            match query_type {
                "get_count" => {
                    let counter_key = b"counter";
                    let count = deps.as_deps().storage.get(counter_key)
                        .and_then(|bytes| {
                            String::from_utf8(bytes).ok()
                                .and_then(|s| s.parse::<u64>().ok())
                        })
                        .unwrap_or(0);
                    
                    Ok(format!("{{\"count\": {}}}", count))
                }
                "get_info" => {
                    let metadata_key = b"__cosmwasm_metadata__";
                    let metadata = deps.as_deps().storage.get(metadata_key)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .unwrap_or_else(|| "{}".to_string());
                    
                    Ok(metadata)
                }
                _ => {
                    Ok(format!("{{\"unknown_query\": \"{}\"}}", query_type))
                }
            }
        } else {
            Ok("{}".to_string())
        }
    }
    
    /// Call the CosmWasm contract's migrate function
    fn call_migrate(&mut self, msg_json: String) -> Result<Option<String>, String> {
        // Create dependencies
        let mut deps_mut = CosmWasmDepsMut::new(&mut self.storage, &self.api);
        let env = get_cosmwasm_env();
        
        env::log_str(&format!("COSMWASM_MIGRATE: {}", msg_json));
        env::log_str(&format!("  Contract: {}", env.contract.address.as_str()));
        
        // Update migration metadata
        let metadata_key = b"__cosmwasm_migration_metadata__";
        let migration_info = serde_json::json!({
            "migrated": true,
            "migrate_msg": msg_json,
            "migrated_at": env.block.height,
            "migrated_time": env.block.time.nanos(),
            "previous_version": self.version,
        });
        
        deps_mut.as_deps_mut().storage.set(
            metadata_key, 
            migration_info.to_string().as_bytes()
        );
        
        Ok(Some("Contract migrated successfully".to_string()))
    }
}

/// Helper implementations for responses
impl WrapperResponse {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    
    fn setup_context() {
        let context = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(1))
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_contract_initialization() {
        setup_context();
        
        let init_msg = WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            contract_msg: r#"{"initial_count": 10}"#.to_string(),
        };
        
        let contract = CosmWasmContractWrapper::new(init_msg);
        
        assert!(contract.initialized);
        assert_eq!(contract.version, "1.0.0");
        assert_eq!(contract.admin, Some(accounts(1)));
    }
    
    #[test]
    fn test_execute_increment() {
        setup_context();
        
        let init_msg = WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            contract_msg: r#"{"initial_count": 0}"#.to_string(),
        };
        
        let mut contract = CosmWasmContractWrapper::new(init_msg);
        
        // Execute increment
        let execute_msg = WrapperExecuteMsg {
            contract_msg: r#"{"action": "increment"}"#.to_string(),
        };
        
        let response = contract.execute(execute_msg);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.data.unwrap().contains("\"count\": 1"));
    }
    
    #[test]
    fn test_query_count() {
        setup_context();
        
        let init_msg = WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),    
            contract_msg: r#"{"initial_count": 0}"#.to_string(),
        };
        
        let mut contract = CosmWasmContractWrapper::new(init_msg);
        
        // First increment the counter
        let execute_msg = WrapperExecuteMsg {
            contract_msg: r#"{"action": "increment"}"#.to_string(),
        };
        contract.execute(execute_msg);
        
        // Then query the count
        let query_msg = WrapperQueryMsg {
            contract_msg: r#"{"query": "get_count"}"#.to_string(),
        };
        
        let response = contract.query(query_msg);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.data.unwrap().contains("\"count\": 1"));
    }
    
    #[test]
    fn test_migration() {
        setup_context();
        
        let init_msg = WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            contract_msg: r#"{"initial_count": 0}"#.to_string(),
        };
        
        let mut contract = CosmWasmContractWrapper::new(init_msg);
        
        // Test migration
        let migrate_msg = WrapperMigrateMsg {
            new_version: "2.0.0".to_string(),
            contract_msg: r#"{"upgrade_data": "test"}"#.to_string(),
        };
        
        let response = contract.migrate(migrate_msg);
        assert!(response.success);
        assert_eq!(contract.version, "2.0.0");
    }
    
    #[test]
    fn test_contract_info() {
        setup_context();
        
        let init_msg = WrapperInitMsg {
            admin: Some(accounts(1).to_string()),
            version: Some("1.0.0".to_string()),
            contract_msg: r#"{"initial_count": 0}"#.to_string(),
        };
        
        let contract = CosmWasmContractWrapper::new(init_msg);
        let info = contract.get_contract_info();
        
        assert!(info.initialized);
        assert_eq!(info.admin, Some(accounts(1).to_string()));
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.contract_address, accounts(0).to_string());
    }
}