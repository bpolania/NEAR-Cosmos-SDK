/// CosmWasm Module Implementation
/// 
/// This module provides the core functionality for deploying and managing
/// CosmWasm smart contracts following the Cosmos SDK x/wasm module pattern.

use near_sdk::{AccountId, env};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, Vector};
use super::types::*;

/// The main CosmWasm module state
#[derive(BorshDeserialize, BorshSerialize)]
pub struct WasmModule {
    /// Stored WASM code by CodeID
    codes: UnorderedMap<CodeID, Vec<u8>>,
    /// Code metadata by CodeID
    code_infos: UnorderedMap<CodeID, CodeInfo>,
    /// Contract instances by address
    contracts: UnorderedMap<ContractAddress, ContractInfo>,
    /// Contract addresses by CodeID for efficient querying
    contracts_by_code: UnorderedMap<CodeID, Vector<ContractAddress>>,
    /// Next available CodeID
    next_code_id: CodeID,
    /// Contract state storage (address -> key -> value)
    contract_states: UnorderedMap<String, UnorderedMap<Vec<u8>, Vec<u8>>>,
}

impl WasmModule {
    pub fn new() -> Self {
        Self {
            codes: UnorderedMap::new(b"wasm_codes".to_vec()),
            code_infos: UnorderedMap::new(b"wasm_code_infos".to_vec()),
            contracts: UnorderedMap::new(b"wasm_contracts".to_vec()),
            contracts_by_code: UnorderedMap::new(b"wasm_contracts_by_code".to_vec()),
            next_code_id: 1,
            contract_states: UnorderedMap::new(b"wasm_contract_states".to_vec()),
        }
    }

    /// Store WASM code on chain and return CodeID
    pub fn store_code(
        &mut self,
        sender: &AccountId,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> Result<CodeID, String> {
        // Basic validation
        if wasm_byte_code.len() > 3_000_000 { // 3MB limit
            return Err("Code size exceeds maximum allowed".to_string());
        }

        // TODO: Add WASM validation here
        // For now, we assume the code is valid

        let code_id = self.next_code_id;
        self.next_code_id += 1;

        // Store the code
        self.codes.insert(&code_id, &wasm_byte_code);

        // Store code metadata
        let code_info = CodeInfo {
            code_id,
            creator: sender.clone(),
            code_hash: env::sha256(&wasm_byte_code),
            source: source.unwrap_or_default(),
            builder: builder.unwrap_or_default(),
            instantiate_permission: self.convert_access_config(instantiate_permission),
        };

        self.code_infos.insert(&code_id, &code_info);

        env::log_str(&format!("WASM: Stored code with ID {}", code_id));
        Ok(code_id)
    }

    /// Instantiate a contract from stored code
    pub fn instantiate_contract(
        &mut self,
        sender: &AccountId,
        code_id: CodeID,
        init_msg: Vec<u8>,
        funds: Vec<Coin>,
        label: String,
        admin: Option<AccountId>,
    ) -> Result<InstantiateResponse, String> {
        // Check if code exists
        let code_info = self.code_infos.get(&code_id)
            .ok_or_else(|| format!("Code ID {} not found", code_id))?;

        // Check instantiate permissions
        if !self.can_instantiate(&code_info.instantiate_permission, sender) {
            return Err("Unauthorized to instantiate this code".to_string());
        }

        // Generate contract address (using a simple scheme for now)
        let contract_address: ContractAddress = format!("contract.{}.{}", code_id, self.get_next_instance_id(code_id))
            .parse()
            .map_err(|_| "Failed to generate contract address")?;

        // Create contract info
        let contract_info = ContractInfo {
            address: contract_address.clone(),
            code_id,
            creator: sender.clone(),
            admin: admin.clone(),
            label,
            created: env::block_height(),
            ibc_port_id: None,
            extension: None,
        };

        // Store contract info
        self.contracts.insert(&contract_address, &contract_info);

        // Add to contracts_by_code index
        let mut contracts_for_code = self.contracts_by_code.get(&code_id)
            .unwrap_or_else(|| Vector::new(format!("contracts_by_code_{}", code_id).into_bytes()));
        contracts_for_code.push(&contract_address);
        self.contracts_by_code.insert(&code_id, &contracts_for_code);

        // Initialize contract state storage
        let state_key = contract_address.to_string();
        let contract_state = UnorderedMap::new(format!("state_{}", state_key).into_bytes());
        self.contract_states.insert(&state_key, &contract_state);

        // TODO: Actually instantiate the contract with the CosmWasm wrapper
        // For now, we'll simulate successful instantiation

        env::log_str(&format!("WASM: Instantiated contract {} from code {}", 
            contract_address, code_id));

        Ok(InstantiateResponse {
            address: contract_address.to_string(),
            data: None,
        })
    }

    /// Execute a message on a contract
    pub fn execute_contract(
        &mut self,
        sender: &AccountId,
        contract_addr: &ContractAddress,
        msg: Vec<u8>,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResponse, String> {
        // Check if contract exists
        let _contract_info = self.contracts.get(contract_addr)
            .ok_or_else(|| format!("Contract {} not found", contract_addr))?;

        // TODO: Load and execute the actual contract
        // For now, we'll simulate successful execution

        env::log_str(&format!("WASM: Executed message on contract {}", contract_addr));

        Ok(ExecuteResponse {
            data: None,
        })
    }

    /// Query a contract
    pub fn query_contract(
        &self,
        contract_addr: &ContractAddress,
        msg: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        // Check if contract exists
        let _contract_info = self.contracts.get(contract_addr)
            .ok_or_else(|| format!("Contract {} not found", contract_addr))?;

        // TODO: Load and query the actual contract
        // For now, we'll return empty response

        Ok(vec![])
    }

    /// Get contract info
    pub fn get_contract_info(&self, address: &ContractAddress) -> Option<ContractInfo> {
        self.contracts.get(address)
    }

    /// Get code info
    pub fn get_code_info(&self, code_id: CodeID) -> Option<CodeInfo> {
        self.code_infos.get(&code_id)
    }

    /// List all stored codes
    pub fn list_codes(&self, start_after: Option<CodeID>, limit: Option<u32>) -> Vec<CodeInfo> {
        let limit = limit.unwrap_or(30).min(100) as usize;
        let start = start_after.unwrap_or(0);
        
        let mut codes = Vec::new();
        let mut count = 0;
        
        for code_id in (start + 1)..self.next_code_id {
            if count >= limit {
                break;
            }
            if let Some(code_info) = self.code_infos.get(&code_id) {
                codes.push(code_info);
                count += 1;
            }
        }
        
        codes
    }

    /// List contracts by code ID
    pub fn list_contracts_by_code(
        &self, 
        code_id: CodeID, 
        start_after: Option<String>, 
        limit: Option<u32>
    ) -> Vec<ContractInfo> {
        let limit = limit.unwrap_or(30).min(100) as usize;
        let mut contracts = Vec::new();
        
        if let Some(addresses) = self.contracts_by_code.get(&code_id) {
            let mut count = 0;
            let mut found_start = start_after.is_none();
            
            for address in addresses.iter() {
                if count >= limit {
                    break;
                }
                
                if !found_start {
                    if Some(address.to_string()) == start_after {
                        found_start = true;
                    }
                    continue;
                }
                
                if let Some(contract_info) = self.contracts.get(&address) {
                    contracts.push(contract_info);
                    count += 1;
                }
            }
        }
        
        contracts
    }

    // Helper methods

    fn convert_access_config(&self, config: Option<AccessConfig>) -> AccessType {
        match config {
            None | Some(AccessConfig::Everybody {}) => AccessType::Everybody,
            Some(AccessConfig::Nobody {}) => AccessType::Nobody,
            Some(AccessConfig::OnlyAddress { address }) => {
                AccessType::OnlyAddress(address.parse().unwrap_or_else(|_| env::current_account_id()))
            }
            Some(AccessConfig::AnyOfAddresses { addresses }) => {
                let parsed_addresses: Vec<AccountId> = addresses
                    .into_iter()
                    .filter_map(|addr| addr.parse().ok())
                    .collect();
                AccessType::AnyOfAddresses(parsed_addresses)
            }
        }
    }

    fn can_instantiate(&self, permission: &AccessType, sender: &AccountId) -> bool {
        match permission {
            AccessType::Nobody => false,
            AccessType::Everybody => true,
            AccessType::OnlyAddress(addr) => addr == sender,
            AccessType::AnyOfAddresses(addrs) => addrs.contains(sender),
        }
    }

    fn get_next_instance_id(&self, code_id: CodeID) -> u64 {
        self.contracts_by_code.get(&code_id)
            .map(|contracts| contracts.len())
            .unwrap_or(0) as u64 + 1
    }
}