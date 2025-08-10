/// CosmWasm Module Types
/// 
/// Following Cosmos SDK x/wasm module architecture for contract deployment and management

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// CodeID uniquely identifies stored WASM code
pub type CodeID = u64;

/// ContractAddress is the unique address of an instantiated contract
pub type ContractAddress = String;

/// CodeInfo stores metadata about uploaded WASM code
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct CodeInfo {
    pub code_id: CodeID,
    pub creator: String,
    pub code_hash: Vec<u8>,
    pub source: String,
    pub builder: String,
    pub instantiate_permission: AccessType,
}

/// AccessType defines who can instantiate a contract from the code
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AccessType {
    Nobody,
    OnlyAddress(String),
    Everybody,
    AnyOfAddresses(Vec<String>),
}

/// ContractInfo stores metadata about an instantiated contract
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct ContractInfo {
    pub address: String,
    pub code_id: CodeID,
    pub creator: String,
    pub admin: Option<String>,
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
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

impl Coin {
    pub fn new(denom: &str, amount: &str) -> Self {
        Self {
            denom: denom.to_string(),
            amount: amount.to_string(),
        }
    }
}