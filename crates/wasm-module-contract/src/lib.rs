use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::collections::{UnorderedMap, Vector};

pub mod types;
use types::*;

/// The main WasmModule contract for deploying and managing CosmWasm contracts
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct WasmModuleContract {
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
    /// Router contract that can call this module
    router_contract: AccountId,
}

#[near_bindgen]
impl WasmModuleContract {
    #[init]
    pub fn new(router_contract: AccountId) -> Self {
        Self {
            codes: UnorderedMap::new(b"wasm_codes".to_vec()),
            code_infos: UnorderedMap::new(b"wasm_code_infos".to_vec()),
            contracts: UnorderedMap::new(b"wasm_contracts".to_vec()),
            contracts_by_code: UnorderedMap::new(b"wasm_contracts_by_code".to_vec()),
            next_code_id: 1,
            contract_states: UnorderedMap::new(b"wasm_contract_states".to_vec()),
            router_contract,
        }
    }

    /// Store WASM code on chain and return CodeID
    pub fn store_code(
        &mut self,
        sender: AccountId,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<AccessConfig>,
    ) -> CodeID {
        // Only router can call this
        assert_eq!(env::predecessor_account_id(), self.router_contract, "Only router can call");

        // Basic validation
        assert!(wasm_byte_code.len() <= 3_000_000, "Code size exceeds 3MB limit");
        assert!(!wasm_byte_code.is_empty(), "Code cannot be empty");

        // Simple WASM validation - check magic bytes
        if wasm_byte_code.len() >= 4 {
            let magic = &wasm_byte_code[0..4];
            assert_eq!(magic, &[0x00, 0x61, 0x73, 0x6d], "Invalid WASM magic bytes");
        }

        let code_id = self.next_code_id;
        self.next_code_id += 1;

        // Store the code
        self.codes.insert(&code_id, &wasm_byte_code);

        // Create code info
        let code_info = CodeInfo {
            code_id,
            creator: sender.to_string(),
            code_hash: env::sha256(&wasm_byte_code),
            source: source.unwrap_or_default(),
            builder: builder.unwrap_or_default(),
            instantiate_permission: match instantiate_permission {
                Some(AccessConfig::Nobody {}) => AccessType::Nobody,
                Some(AccessConfig::OnlyAddress { address }) => AccessType::OnlyAddress(address),
                Some(AccessConfig::Everybody {}) => AccessType::Everybody,
                Some(AccessConfig::AnyOfAddresses { addresses }) => AccessType::AnyOfAddresses(addresses),
                None => AccessType::Everybody,
            },
        };

        self.code_infos.insert(&code_id, &code_info);

        env::log_str(&format!("Stored code with ID: {}", code_id));
        code_id
    }

    /// Instantiate a new contract from stored code
    pub fn instantiate(
        &mut self,
        sender: AccountId,
        code_id: CodeID,
        _msg: Vec<u8>,
        _funds: Vec<Coin>,
        label: String,
        admin: Option<String>,
    ) -> ContractAddress {
        // Only router can call this
        assert_eq!(env::predecessor_account_id(), self.router_contract, "Only router can call");

        // Check if code exists
        let code_info = self.code_infos.get(&code_id).expect("Code not found");

        // Check instantiation permissions
        match code_info.instantiate_permission {
            AccessType::Nobody => panic!("Code cannot be instantiated"),
            AccessType::OnlyAddress(ref addr) => {
                assert_eq!(sender.to_string(), *addr, "Sender not authorized to instantiate");
            }
            AccessType::AnyOfAddresses(ref addresses) => {
                assert!(addresses.contains(&sender.to_string()), "Sender not in allowed addresses");
            }
            AccessType::Everybody => {} // Allow anyone
        }

        // Generate contract address
        let contract_address = format!(
            "contract.{}.{}",
            code_id,
            env::block_height()
        );

        // Create contract info
        let contract_info = ContractInfo {
            address: contract_address.clone(),
            code_id,
            creator: sender.to_string(),
            admin,
            label,
            created: env::block_height(),
            ibc_port_id: None,
            extension: None,
        };

        self.contracts.insert(&contract_address, &contract_info);

        // Add to contracts_by_code index
        let mut contracts_for_code = self.contracts_by_code
            .get(&code_id)
            .unwrap_or_else(|| Vector::new(format!("contracts_{}", code_id).as_bytes()));
        
        contracts_for_code.push(&contract_address);
        self.contracts_by_code.insert(&code_id, &contracts_for_code);

        // Initialize contract state storage
        let contract_state = UnorderedMap::new(format!("state_{}", contract_address).as_bytes());
        self.contract_states.insert(&contract_address, &contract_state);

        env::log_str(&format!("Instantiated contract: {} from code ID: {}", contract_address, code_id));
        contract_address
    }

    /// Execute a contract
    pub fn execute(
        &mut self,
        sender: AccountId,
        contract_addr: String,
        msg: Vec<u8>,
        _funds: Vec<Coin>,
    ) -> String {
        // Only router can call this
        assert_eq!(env::predecessor_account_id(), self.router_contract, "Only router can call");

        // Check if contract exists
        let _contract_info = self.contracts.get(&contract_addr).expect("Contract not found");

        // For now, return a mock response
        // In a full implementation, this would:
        // 1. Load the WASM code
        // 2. Execute it with the provided message
        // 3. Handle state changes
        // 4. Return the execution result

        env::log_str(&format!("Executed contract: {} with message length: {}", contract_addr, msg.len()));
        format!("{{\"executed\": \"{}\", \"sender\": \"{}\"}}", contract_addr, sender)
    }

    // Query methods
    
    /// Get code by ID
    pub fn get_code(&self, code_id: CodeID) -> Option<Vec<u8>> {
        self.codes.get(&code_id)
    }

    /// Get code info by ID
    pub fn get_code_info(&self, code_id: CodeID) -> Option<CodeInfo> {
        self.code_infos.get(&code_id)
    }

    /// Get contract info by address
    pub fn get_contract_info(&self, contract_addr: String) -> Option<ContractInfo> {
        self.contracts.get(&contract_addr)
    }

    /// Get contracts by code ID
    pub fn get_contracts_by_code(&self, code_id: CodeID) -> Vec<ContractAddress> {
        self.contracts_by_code
            .get(&code_id)
            .map(|contracts| contracts.to_vec())
            .unwrap_or_default()
    }

    /// Get next code ID
    pub fn get_next_code_id(&self) -> CodeID {
        self.next_code_id
    }

    /// Health check
    pub fn health_check(&self) -> serde_json::Value {
        serde_json::json!({
            "module": "wasm",
            "status": "healthy",
            "codes_count": self.codes.len(),
            "contracts_count": self.contracts.len(),
            "next_code_id": self.next_code_id
        })
    }
}