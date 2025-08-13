/// VM Executor for CosmWasm Contracts
/// 
/// This module provides the actual execution of CosmWasm contracts
/// using NEAR's runtime with our compatibility layer.

use near_sdk::{env, AccountId, Promise};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::host_functions;
use crate::message_translator::{MessageTranslator, Response as CosmResponse};

/// Result type for VM operations
pub type VmResult<T> = Result<T, VmError>;

/// Errors that can occur during VM execution
#[derive(Debug, Serialize, Deserialize)]
pub enum VmError {
    WasmExecutionFailed(String),
    InvalidMessage(String),
    StorageError(String),
    InstantiationFailed(String),
    QueryFailed(String),
    MigrationFailed(String),
    OutOfGas,
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::WasmExecutionFailed(msg) => write!(f, "WASM execution failed: {}", msg),
            VmError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            VmError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            VmError::InstantiationFailed(msg) => write!(f, "Instantiation failed: {}", msg),
            VmError::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
            VmError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            VmError::OutOfGas => write!(f, "Out of gas"),
        }
    }
}

/// VM Executor that handles CosmWasm contract execution
pub struct VmExecutor {
    /// Contract address being executed
    pub contract_addr: String,
    /// Storage prefix for this contract
    pub storage_prefix: Vec<u8>,
}

impl VmExecutor {
    /// Create a new VM executor for a contract
    pub fn new(contract_addr: String) -> Self {
        let storage_prefix = format!("contract:{}", contract_addr).into_bytes();
        Self {
            contract_addr,
            storage_prefix,
        }
    }
    
    /// Execute the instantiate entry point of a CosmWasm contract
    pub fn instantiate(
        &mut self,
        code: &[u8],
        sender: &AccountId,
        msg: String,
        label: String,
        admin: Option<String>,
    ) -> VmResult<CosmResponse> {
        env::log_str(&format!("Instantiating contract with label: {}", label));
        
        // Prepare the arguments in CosmWasm format
        let args = MessageTranslator::prepare_instantiate_args(
            &msg,
            sender,
            &self.contract_addr
        ).map_err(|e| VmError::InvalidMessage(e))?;
        
        // In a real implementation, we would:
        // 1. Load the WASM module using NEAR's Wasmer
        // 2. Create instance with our host functions as imports
        // 3. Call the instantiate export
        // 4. Handle the response
        
        // For now, we simulate the execution
        let response = self.simulate_wasm_execution("instantiate", &args)?;
        
        // Store contract metadata
        let metadata_key = format!("{}:metadata", self.contract_addr);
        let code_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(code);
            hex::encode(hasher.finalize())
        };
        let metadata = json!({
            "label": label,
            "admin": admin,
            "creator": sender.to_string(),
            "code_hash": code_hash,
        });
        env::storage_write(metadata_key.as_bytes(), metadata.to_string().as_bytes());
        
        Ok(response)
    }
    
    /// Execute a contract method
    pub fn execute(
        &mut self,
        code: &[u8],
        sender: &AccountId,
        msg: String,
    ) -> VmResult<CosmResponse> {
        env::log_str(&format!("Executing contract: {}", self.contract_addr));
        
        // Prepare the arguments in CosmWasm format
        let args = MessageTranslator::prepare_execute_args(
            &msg,
            sender,
            &self.contract_addr
        ).map_err(|e| VmError::InvalidMessage(e))?;
        
        // Check gas before execution
        if !host_functions::gas::check_gas(10_000_000_000_000) {
            return Err(VmError::OutOfGas);
        }
        
        // Execute the contract
        let response = self.simulate_wasm_execution("execute", &args)?;
        
        // Process submessages if any
        if !response.messages.is_empty() {
            self.process_submessages(&response)?;
        }
        
        Ok(response)
    }
    
    /// Query contract state
    pub fn query(
        &self,
        code: &[u8],
        msg: String,
    ) -> VmResult<Vec<u8>> {
        env::log_str(&format!("Querying contract: {}", self.contract_addr));
        
        // Prepare the arguments in CosmWasm format
        let args = MessageTranslator::prepare_query_args(
            &msg,
            &self.contract_addr
        ).map_err(|e| VmError::InvalidMessage(e))?;
        
        // Query is read-only, no state changes
        let response = self.simulate_wasm_query("query", &args)?;
        
        Ok(response)
    }
    
    /// Migrate contract to new code
    pub fn migrate(
        &mut self,
        new_code: &[u8],
        sender: &AccountId,
        msg: String,
    ) -> VmResult<CosmResponse> {
        env::log_str(&format!("Migrating contract: {}", self.contract_addr));
        
        // Check if sender is admin
        let metadata_key = format!("{}:metadata", self.contract_addr);
        if let Some(metadata_bytes) = env::storage_read(metadata_key.as_bytes()) {
            let metadata: Value = serde_json::from_slice(&metadata_bytes)
                .map_err(|e| VmError::StorageError(e.to_string()))?;
            
            if let Some(admin) = metadata.get("admin").and_then(|a| a.as_str()) {
                if admin != sender.as_str() {
                    return Err(VmError::MigrationFailed("Unauthorized".to_string()));
                }
            }
        }
        
        // Prepare migration arguments
        let env = MessageTranslator::create_env(&self.contract_addr);
        let args = json!({
            "env": env,
            "msg": serde_json::from_str::<Value>(&msg)
                .map_err(|e| VmError::InvalidMessage(e.to_string()))?,
        }).to_string();
        
        // Execute migration
        let response = self.simulate_wasm_execution("migrate", &args)?;
        
        // Update code hash in metadata
        let new_code_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(new_code);
            hasher.finalize()
        };
        if let Some(mut metadata_bytes) = env::storage_read(metadata_key.as_bytes()) {
            if let Ok(mut metadata) = serde_json::from_slice::<Value>(&metadata_bytes) {
                metadata["code_hash"] = json!(hex::encode(&new_code_hash));
                env::storage_write(metadata_key.as_bytes(), metadata.to_string().as_bytes());
            }
        }
        
        Ok(response)
    }
    
    /// Simulate WASM execution (placeholder for actual WASM execution)
    /// In production, this would use NEAR's Wasmer to execute the actual WASM
    fn simulate_wasm_execution(&mut self, entry_point: &str, args: &str) -> VmResult<CosmResponse> {
        // This is where we would integrate with NEAR's Wasmer runtime
        // For now, we return a simulated response based on the entry point
        
        match entry_point {
            "instantiate" => {
                // Simulate successful instantiation
                Ok(CosmResponse {
                    messages: vec![],
                    attributes: vec![
                        crate::message_translator::Attribute {
                            key: "action".to_string(),
                            value: "instantiate".to_string(),
                        },
                        crate::message_translator::Attribute {
                            key: "contract_addr".to_string(),
                            value: self.contract_addr.clone(),
                        },
                    ],
                    events: vec![],
                    data: Some(base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        b"instantiation successful"
                    )),
                })
            },
            "execute" => {
                // Parse the args to determine the action
                if let Ok(args_json) = serde_json::from_str::<Value>(args) {
                    if let Some(msg) = args_json.get("msg") {
                        // Simulate different execute actions
                        if msg.get("transfer").is_some() {
                            Ok(CosmResponse {
                                messages: vec![],
                                attributes: vec![
                                    crate::message_translator::Attribute {
                                        key: "action".to_string(),
                                        value: "transfer".to_string(),
                                    },
                                ],
                                events: vec![
                                    crate::message_translator::Event {
                                        r#type: "transfer".to_string(),
                                        attributes: vec![
                                            crate::message_translator::Attribute {
                                                key: "action".to_string(),
                                                value: "transfer".to_string(),
                                            },
                                        ],
                                    },
                                ],
                                data: None,
                            })
                        } else if msg.get("mint").is_some() {
                            Ok(CosmResponse {
                                messages: vec![],
                                attributes: vec![
                                    crate::message_translator::Attribute {
                                        key: "action".to_string(),
                                        value: "mint".to_string(),
                                    },
                                ],
                                events: vec![
                                    crate::message_translator::Event {
                                        r#type: "mint".to_string(),
                                        attributes: vec![
                                            crate::message_translator::Attribute {
                                                key: "action".to_string(),
                                                value: "mint".to_string(),
                                            },
                                        ],
                                    },
                                ],
                                data: None,
                            })
                        } else {
                            Ok(CosmResponse::default())
                        }
                    } else {
                        Ok(CosmResponse::default())
                    }
                } else {
                    Ok(CosmResponse::default())
                }
            },
            "migrate" => {
                Ok(CosmResponse {
                    messages: vec![],
                    attributes: vec![
                        crate::message_translator::Attribute {
                            key: "action".to_string(),
                            value: "migrate".to_string(),
                        },
                    ],
                    events: vec![],
                    data: None,
                })
            },
            _ => Err(VmError::WasmExecutionFailed(format!("Unknown entry point: {}", entry_point))),
        }
    }
    
    /// Simulate WASM query (placeholder for actual WASM query)
    fn simulate_wasm_query(&self, entry_point: &str, args: &str) -> VmResult<Vec<u8>> {
        // This would execute a read-only query on the WASM contract
        // For now, return a simulated response
        
        let response = json!({
            "query_result": {
                "balance": "1000000",
                "contract": self.contract_addr,
            }
        });
        
        Ok(response.to_string().into_bytes())
    }
    
    /// Process submessages from contract response
    fn process_submessages(&self, response: &CosmResponse) -> VmResult<()> {
        let near_calls = MessageTranslator::process_submessages(response);
        
        for call_json in near_calls {
            if let Ok(call) = serde_json::from_str::<Value>(&call_json) {
                // In production, we would create actual Promises here
                env::log_str(&format!("Would execute submessage: {}", call));
            }
        }
        
        Ok(())
    }
}

/// Entry point adapters that integrate with the main contract
pub mod entry_points {
    use super::*;
    
    /// Adapter for instantiate entry point
    pub fn instantiate_adapter(
        code: Vec<u8>,
        sender: AccountId,
        msg: String,
        label: String,
        admin: Option<String>,
        contract_addr: String,
    ) -> Result<String, String> {
        let mut executor = VmExecutor::new(contract_addr);
        
        match executor.instantiate(&code, &sender, msg, label, admin) {
            Ok(response) => Ok(MessageTranslator::response_to_near(&response)),
            Err(e) => Err(e.to_string()),
        }
    }
    
    /// Adapter for execute entry point
    pub fn execute_adapter(
        code: Vec<u8>,
        sender: AccountId,
        msg: String,
        contract_addr: String,
    ) -> Result<String, String> {
        let mut executor = VmExecutor::new(contract_addr);
        
        match executor.execute(&code, &sender, msg) {
            Ok(response) => Ok(MessageTranslator::response_to_near(&response)),
            Err(e) => Err(e.to_string()),
        }
    }
    
    /// Adapter for query entry point
    pub fn query_adapter(
        code: Vec<u8>,
        msg: String,
        contract_addr: String,
    ) -> Result<String, String> {
        let executor = VmExecutor::new(contract_addr);
        
        match executor.query(&code, msg) {
            Ok(result) => Ok(String::from_utf8_lossy(&result).to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    
    /// Adapter for migrate entry point
    pub fn migrate_adapter(
        new_code: Vec<u8>,
        sender: AccountId,
        msg: String,
        contract_addr: String,
    ) -> Result<String, String> {
        let mut executor = VmExecutor::new(contract_addr);
        
        match executor.migrate(&new_code, &sender, msg) {
            Ok(response) => Ok(MessageTranslator::response_to_near(&response)),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::accounts;
    
    #[test]
    fn test_vm_executor_creation() {
        let executor = VmExecutor::new("test.contract".to_string());
        assert_eq!(executor.contract_addr, "test.contract");
        assert_eq!(executor.storage_prefix, b"contract:test.contract");
    }
    
    #[test]
    fn test_instantiate_simulation() {
        let mut executor = VmExecutor::new("test.contract".to_string());
        let code = b"fake wasm code";
        let sender = accounts(0);
        let msg = r#"{"name": "Test Token", "symbol": "TEST"}"#.to_string();
        
        let result = executor.instantiate(
            code,
            &sender,
            msg,
            "test-token".to_string(),
            Some(sender.to_string()),
        );
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.attributes.is_empty());
    }
    
    #[test]
    fn test_execute_simulation() {
        let mut executor = VmExecutor::new("test.contract".to_string());
        let code = b"fake wasm code";
        let sender = accounts(0);
        let msg = r#"{"transfer": {"recipient": "alice", "amount": "1000"}}"#.to_string();
        
        let result = executor.execute(code, &sender, msg);
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.attributes.iter().any(|a| a.value == "transfer"));
    }
}