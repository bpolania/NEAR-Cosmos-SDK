use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub data: Vec<u8>,
    pub state_changes: Vec<StateChange>,
    pub events: Vec<Event>,
    pub gas_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    Set { key: Vec<u8>, value: Vec<u8> },
    Remove { key: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub typ: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug)]
pub enum ExecutionError {
    CompilationError(String),
    InstantiationError(String),
    RuntimeError(String),
    InvalidInput(String),
    StateError(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::CompilationError(e) => write!(f, "Compilation error: {}", e),
            ExecutionError::InstantiationError(e) => write!(f, "Instantiation error: {}", e),
            ExecutionError::RuntimeError(e) => write!(f, "Runtime error: {}", e),
            ExecutionError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
            ExecutionError::StateError(e) => write!(f, "State error: {}", e),
        }
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmWasmEnv {
    pub block: BlockInfo,
    pub contract: ContractInfo,
    pub transaction: Option<TransactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub time: u64,
    pub chain_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address: String,
    pub creator: Option<String>,
    pub admin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInstance {
    pub contract_address: String,
    pub code_hash: Vec<u8>,
    pub state: ContractState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractState {
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}