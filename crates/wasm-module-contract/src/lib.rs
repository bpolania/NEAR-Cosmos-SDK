/// x/wasm Module Contract - Standalone
/// 
/// This contract handles all CosmWasm smart contract operations including:
/// - Code storage and management
/// - Contract instantiation and execution
/// - Contract queries and metadata
/// - Admin functions and access control

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::collections::{UnorderedMap, LookupMap};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use sha2::{Sha256, Digest};

// =============================================================================
// Types
// =============================================================================

pub type CodeID = u64;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessConfig {
    Nobody {},
    OnlyAddress { address: String },
    Everybody {},
    AnyOfAddresses { addresses: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, BorshSerialize, BorshDeserialize)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct CodeInfo {
    pub code_id: CodeID,
    pub creator: String,
    pub code_hash: Vec<u8>,
    pub source: String,
    pub builder: String,
    pub instantiate_permission: AccessConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct ContractInfo {
    pub address: String,
    pub code_id: CodeID,
    pub creator: String,
    pub admin: Option<String>,
    pub label: String,
    pub created: u64,
}

// =============================================================================
// Contract State
// =============================================================================

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct WasmModuleContract {
    /// Next code ID to assign
    next_code_id: CodeID,
    /// Next instance ID for contract addresses
    next_instance_id: u64,
    /// Stored code information (code_id -> CodeInfo)
    codes: UnorderedMap<CodeID, CodeInfo>,
    /// Contract instances (address -> ContractInfo)
    contracts: UnorderedMap<String, ContractInfo>,
    /// Contract state storage (contract_address -> state_key -> value)
    contract_state: LookupMap<String, Vec<u8>>,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
    /// Maximum code size in bytes (3MB default for NEAR)
    max_code_size: u64,
}

// =============================================================================
// Responses
// =============================================================================

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct StoreCodeResponse {
    pub code_id: CodeID,
    pub checksum: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct InstantiateResponse {
    pub address: String,
    pub data: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ExecuteResponse {
    pub data: Option<String>,
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

// =============================================================================
// Contract Implementation
// =============================================================================

#[near_bindgen]
impl WasmModuleContract {
    #[init]
    pub fn new(owner: Option<AccountId>, router_contract: Option<AccountId>) -> Self {
        Self {
            next_code_id: 1,
            next_instance_id: 1,
            codes: UnorderedMap::new(b"c"),
            contracts: UnorderedMap::new(b"i"),
            contract_state: LookupMap::new(b"s"),
            router_contract,
            owner: owner.unwrap_or_else(|| env::current_account_id()),
            max_code_size: 3 * 1024 * 1024, // 3MB
        }
    }

    // =============================================================================
    // Code Management
    // =============================================================================

    /// Store WASM code and return CodeID
    #[payable]
    pub fn store_code(
        &mut self,
        wasm_byte_code: Base64VecU8,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> StoreCodeResponse {
        self.assert_authorized();
        
        let wasm_bytes: Vec<u8> = wasm_byte_code.into();
        
        // Validate code size
        assert!(
            wasm_bytes.len() <= self.max_code_size as usize,
            "WASM code exceeds maximum size of {} bytes",
            self.max_code_size
        );
        
        // Basic WASM validation
        assert!(wasm_bytes.len() >= 4, "WASM code too small");
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6d], "Invalid WASM magic number");
        
        // Calculate code hash
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let code_hash = hasher.finalize().to_vec();
        let checksum = hex::encode(&code_hash);
        
        // Assign code ID
        let code_id = self.next_code_id;
        self.next_code_id += 1;
        
        // Store code info
        let code_info = CodeInfo {
            code_id,
            creator: env::predecessor_account_id().to_string(),
            code_hash: code_hash.clone(),
            source: source.unwrap_or_default(),
            builder: builder.unwrap_or_default(),
            instantiate_permission: instantiate_permission.unwrap_or(AccessConfig::Everybody {}),
        };
        
        self.codes.insert(&code_id, &code_info);
        
        // Store actual WASM code
        let code_key = format!("code:{}", code_id);
        self.contract_state.insert(&code_key, &wasm_bytes);
        
        env::log_str(&format!("Stored WASM code with ID: {} (checksum: {})", code_id, checksum));
        
        StoreCodeResponse {
            code_id,
            checksum,
        }
    }

    /// Instantiate a contract from stored code
    pub fn instantiate(
        &mut self,
        code_id: CodeID,
        msg: String,
        _funds: Option<Vec<Coin>>,
        label: String,
        admin: Option<String>,
    ) -> InstantiateResponse {
        // No assert_authorized here - permission check is sufficient
        
        // Verify code exists
        let code_info = self.codes.get(&code_id)
            .expect("Code ID does not exist");
        
        // Check instantiate permission
        self.check_instantiate_permission(&code_info.instantiate_permission);
        
        // Generate contract address
        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;
        let contract_addr = format!("contract{}.{}", instance_id, env::current_account_id());
        
        // Store contract info
        let contract_info = ContractInfo {
            address: contract_addr.clone(),
            code_id,
            creator: env::predecessor_account_id().to_string(),
            admin,
            label,
            created: env::block_height(),
        };
        
        self.contracts.insert(&contract_addr, &contract_info);
        
        // Initialize contract state (mock for now)
        let state_key = format!("{}:state", contract_addr);
        self.contract_state.insert(&state_key, &msg.as_bytes().to_vec());
        
        env::log_str(&format!("Instantiated contract at: {}", contract_addr));
        
        InstantiateResponse {
            address: contract_addr,
            data: Some(format!("Contract instantiated with message: {}", msg)),
        }
    }

    /// Execute a contract function
    pub fn execute(
        &mut self,
        contract_addr: String,
        msg: String,
        _funds: Option<Vec<Coin>>,
    ) -> ExecuteResponse {
        self.assert_authorized();
        
        // Verify contract exists
        let _contract_info = self.contracts.get(&contract_addr)
            .expect("Contract does not exist");
        
        // Mock execution - in a real implementation, this would run the WASM code
        env::log_str(&format!("Executing contract {} with message: {}", contract_addr, msg));
        
        // Create mock events
        let events = vec![
            Event {
                r#type: "wasm".to_string(),
                attributes: vec![
                    Attribute {
                        key: "_contract_address".to_string(),
                        value: contract_addr.clone(),
                    },
                    Attribute {
                        key: "action".to_string(),
                        value: "execute".to_string(),
                    },
                ],
            },
        ];
        
        ExecuteResponse {
            data: Some(format!("Executed: {}", msg)),
            events,
        }
    }

    /// Query contract state (read-only)
    pub fn query(
        &self,
        contract_addr: String,
        msg: String,
    ) -> String {
        // Verify contract exists
        let _contract_info = self.contracts.get(&contract_addr)
            .expect("Contract does not exist");
        
        // Mock query - in a real implementation, this would query the WASM contract
        format!("{{\"query_result\": \"mocked response for: {}\"}}", msg)
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get code info by ID
    pub fn get_code_info(&self, code_id: CodeID) -> Option<CodeInfo> {
        self.codes.get(&code_id)
    }

    /// Get contract info by address
    pub fn get_contract_info(&self, contract_addr: String) -> Option<ContractInfo> {
        self.contracts.get(&contract_addr)
    }

    /// List all codes with pagination
    pub fn list_codes(&self, limit: Option<u32>, start_after: Option<CodeID>) -> Vec<CodeInfo> {
        let limit = limit.unwrap_or(10).min(100) as usize;
        let start = start_after.unwrap_or(0);
        
        let mut codes = Vec::new();
        for code_id in (start + 1)..self.next_code_id {
            if let Some(code_info) = self.codes.get(&code_id) {
                codes.push(code_info);
                if codes.len() >= limit {
                    break;
                }
            }
        }
        codes
    }

    /// List all contracts with pagination
    pub fn list_contracts(
        &self,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> Vec<ContractInfo> {
        let limit = limit.unwrap_or(10).min(100) as usize;
        
        self.contracts.iter()
            .skip_while(|(addr, _)| {
                if let Some(ref start) = start_after {
                    addr <= start
                } else {
                    false
                }
            })
            .take(limit)
            .map(|(_, info)| info)
            .collect()
    }

    /// Get contract code ID
    pub fn get_contract_code_id(&self, contract_addr: String) -> Option<CodeID> {
        self.contracts.get(&contract_addr).map(|info| info.code_id)
    }

    /// Health check
    pub fn health_check(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "healthy",
            "module": "x/wasm",
            "codes_stored": self.next_code_id - 1,
            "contracts_instantiated": self.next_instance_id - 1,
            "max_code_size": self.max_code_size,
            "owner": self.owner,
            "router": self.router_contract,
        })
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "CosmWasm x/wasm Module",
            "version": "0.1.0",
            "description": "Handles CosmWasm contract deployment and execution on NEAR",
            "capabilities": [
                "store_code",
                "instantiate",
                "execute",
                "query",
                "migrate"
            ],
            "stats": {
                "codes_stored": self.next_code_id - 1,
                "contracts_deployed": self.next_instance_id - 1,
            }
        })
    }

    // =============================================================================
    // Admin Functions
    // =============================================================================

    /// Update max code size limit
    pub fn update_max_code_size(&mut self, new_size: u64) {
        self.assert_owner();
        assert!(new_size > 0 && new_size <= 10 * 1024 * 1024, "Invalid size");
        self.max_code_size = new_size;
        env::log_str(&format!("Updated max code size to: {} bytes", new_size));
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
        self.owner = new_owner.clone();
        env::log_str(&format!("Ownership transferred to: {}", new_owner));
    }

    /// Update router contract
    pub fn update_router(&mut self, new_router: AccountId) {
        self.assert_owner();
        self.router_contract = Some(new_router.clone());
        env::log_str(&format!("Router updated to: {}", new_router));
    }

    // =============================================================================
    // Helper Functions
    // =============================================================================

    fn assert_authorized(&self) {
        let caller = env::predecessor_account_id();
        assert!(
            caller == self.owner || 
            self.router_contract.as_ref() == Some(&caller) ||
            caller == env::current_account_id(),
            "Unauthorized: caller must be owner, router, or self"
        );
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can call this function"
        );
    }

    fn check_instantiate_permission(&self, permission: &AccessConfig) {
        let sender = env::predecessor_account_id();
        let sender_str = sender.to_string();
        
        match permission {
            AccessConfig::Nobody {} => {
                panic!("Nobody can instantiate this code");
            },
            AccessConfig::OnlyAddress { address } => {
                assert_eq!(&sender_str, address, "Only {} can instantiate", address);
            },
            AccessConfig::Everybody {} => {
                // Anyone can instantiate
            },
            AccessConfig::AnyOfAddresses { addresses } => {
                assert!(
                    addresses.contains(&sender_str),
                    "Address not in allowed list"
                );
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

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
    fn test_new() {
        let context = get_context(accounts(0));
        testing_env!(context);
        
        let contract = WasmModuleContract::new(Some(accounts(0)), None);
        assert_eq!(contract.next_code_id, 1);
        assert_eq!(contract.next_instance_id, 1);
        assert_eq!(contract.owner, accounts(0));
    }

    #[test]
    fn test_store_code() {
        let context = get_context(accounts(0));
        testing_env!(context);
        
        let mut contract = WasmModuleContract::new(Some(accounts(0)), None);
        
        let wasm_code = Base64VecU8::from(vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        let response = contract.store_code(
            wasm_code,
            Some("test source".to_string()),
            Some("test builder".to_string()),
            None,
        );
        
        assert_eq!(response.code_id, 1);
        assert!(!response.checksum.is_empty());
        assert_eq!(contract.next_code_id, 2);
    }

    #[test]
    fn test_instantiate() {
        let context = get_context(accounts(0));
        testing_env!(context);
        
        let mut contract = WasmModuleContract::new(Some(accounts(0)), None);
        
        // First store code
        let wasm_code = Base64VecU8::from(vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        let store_response = contract.store_code(wasm_code, None, None, None);
        
        // Then instantiate
        let instantiate_response = contract.instantiate(
            store_response.code_id,
            "{\"count\": 0}".to_string(),
            None,
            "test contract".to_string(),
            None,
        );
        
        assert!(instantiate_response.address.starts_with("contract1."));
        assert_eq!(contract.next_instance_id, 2);
    }
}