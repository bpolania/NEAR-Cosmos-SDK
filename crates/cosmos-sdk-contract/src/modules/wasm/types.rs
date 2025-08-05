/// CosmWasm Module Types
/// 
/// Following Cosmos SDK x/wasm module architecture for contract deployment and management

use near_sdk::AccountId;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// CodeID uniquely identifies stored WASM code
pub type CodeID = u64;

/// ContractAddress is the unique address of an instantiated contract
pub type ContractAddress = AccountId;

/// CodeInfo stores metadata about uploaded WASM code
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct CodeInfo {
    pub code_id: CodeID,
    pub creator: AccountId,
    pub code_hash: Vec<u8>,
    pub source: String,
    pub builder: String,
    pub instantiate_permission: AccessType,
}

/// AccessType defines who can instantiate a contract from the code
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AccessType {
    Nobody,
    OnlyAddress(AccountId),
    Everybody,
    AnyOfAddresses(Vec<AccountId>),
}

/// ContractInfo stores metadata about an instantiated contract
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct ContractInfo {
    pub address: ContractAddress,
    pub code_id: CodeID,
    pub creator: AccountId,
    pub admin: Option<AccountId>,
    pub label: String,
    pub created: u64, // block height
    pub ibc_port_id: Option<String>,
    pub extension: Option<String>,
}

/// WasmMsg represents actions that can be taken on the wasm module
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum WasmMsg {
    /// StoreCode uploads WASM bytecode to the chain
    StoreCode {
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    },
    /// Instantiate creates a new contract instance from stored code
    Instantiate {
        code_id: CodeID,
        msg: Vec<u8>, // JSON encoded init message
        funds: Vec<Coin>,
        label: String,
        admin: Option<String>,
    },
    /// Execute calls a function on a contract
    Execute {
        contract_addr: String,
        msg: Vec<u8>, // JSON encoded execute message
        funds: Vec<Coin>,
    },
    /// Migrate updates a contract to use new code
    Migrate {
        contract_addr: String,
        new_code_id: CodeID,
        msg: Vec<u8>, // JSON encoded migrate message
    },
    /// UpdateAdmin changes the admin of a contract
    UpdateAdmin {
        contract_addr: String,
        admin: String,
    },
    /// ClearAdmin removes the admin of a contract
    ClearAdmin {
        contract_addr: String,
    },
}

/// AccessConfig defines instantiation permissions
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AccessConfig {
    Nobody {},
    OnlyAddress { address: String },
    Everybody {},
    AnyOfAddresses { addresses: Vec<String> },
}

/// Coin represents a token amount
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

/// Response from contract instantiation
#[derive(Serialize, Deserialize, Debug)]
pub struct InstantiateResponse {
    pub address: String,
    pub data: Option<Vec<u8>>,
}

/// Response from contract execution
#[derive(Serialize, Deserialize, Debug)]
pub struct ExecuteResponse {
    pub data: Option<Vec<u8>>,
}

/// Query messages for the wasm module
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum WasmQuery {
    /// Get contract info
    ContractInfo { address: String },
    /// Get code info
    CodeInfo { code_id: CodeID },
    /// List all codes
    ListCodes {
        start_after: Option<CodeID>,
        limit: Option<u32>,
    },
    /// List all contracts by code ID
    ListContractsByCode {
        code_id: CodeID,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get raw contract state
    RawContractState {
        address: String,
        key: Vec<u8>,
    },
    /// Query a contract
    Smart {
        address: String,
        msg: Vec<u8>, // JSON encoded query
    },
}