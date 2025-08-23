/// Execution Queue for CosmWasm Relayer
/// 
/// This module handles queuing execution requests for the relayer to process

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ExecutionRequest {
    pub request_id: String,
    pub contract_address: String,
    pub code_id: u64,
    pub entry_point: String,
    pub msg: Vec<u8>,
    pub sender: String,
    pub funds: Vec<CosmWasmCoin>,
    pub block_height: u64,
    pub timestamp: u64,
    pub status: ExecutionStatus,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct CosmWasmCoin {
    pub denom: String,
    pub amount: String,  // Changed from U128 to String for JsonSchema compatibility
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecutionStatus {
    Pending,
    Processing,
    Executed,
    Failed(String),
}

impl ExecutionRequest {
    /// Create a new execution request
    pub fn new(
        contract_address: String,
        code_id: u64,
        entry_point: String,
        msg: Vec<u8>,
        sender: String,
        block_height: u64,
        timestamp: u64,
    ) -> Self {
        let request_id = format!("exec_{}_{}", block_height, timestamp);
        
        Self {
            request_id,
            contract_address,
            code_id,
            entry_point,
            msg,
            sender,
            funds: Vec::new(),
            block_height,
            timestamp,
            status: ExecutionStatus::Pending,
        }
    }
}