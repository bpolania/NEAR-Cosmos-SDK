use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmer::{Module, Store};

use super::executor::WasmerExecutor;
use super::state::StateManager;
use super::types::{
    ContractInstance, ContractState, CosmWasmEnv, Event, ExecutionError, ExecutionResult,
    StateChange,
};

pub struct WasmerExecutionService {
    /// Wasmer store for compilation
    store: Store,
    
    /// Cache of compiled modules (code_hash -> Module)
    module_cache: Arc<RwLock<HashMap<Vec<u8>, Module>>>,
    
    /// Active contract instances
    instances: Arc<RwLock<HashMap<String, ContractInstance>>>,
    
    /// State management
    state_manager: Arc<StateManager>,
}

impl WasmerExecutionService {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            store: Store::default(),
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            state_manager,
        }
    }

    /// Execute a CosmWasm contract function
    pub async fn execute_contract(
        &self,
        contract_address: &str,
        wasm_code: &[u8],
        entry_point: &str,
        msg: &[u8],
        env: CosmWasmEnv,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Calculate code hash
        let code_hash = self.calculate_hash(wasm_code);
        
        // Get or compile module
        let module = self.get_or_compile_module(&code_hash, wasm_code).await?;
        
        // Create executor with proper environment
        let mut executor = WasmerExecutor::new(contract_address.as_bytes().to_vec());
        executor.set_env(env.clone());
        
        // Execute the contract
        let result_data = executor.execute_wasm(wasm_code, entry_point, msg)?;
        
        // Collect state changes
        let state_changes = self.process_state_changes(
            contract_address,
            executor.get_state_changes(),
        );
        
        // Collect events
        let events = self.process_events(executor.get_events());
        
        // Calculate gas used (simplified for now)
        let gas_used = self.calculate_gas_used(&state_changes, &events);
        
        Ok(ExecutionResult {
            data: Some(result_data),
            state_changes,
            events,
            gas_used,
        })
    }

    /// Get or compile a WASM module
    async fn get_or_compile_module(
        &self,
        code_hash: &[u8],
        wasm_code: &[u8],
    ) -> Result<Module, ExecutionError> {
        // Check cache first
        {
            let cache = self.module_cache.read().await;
            if let Some(module) = cache.get(code_hash) {
                return Ok(module.clone());
            }
        }
        
        // Compile the module
        let module = Module::new(&self.store, wasm_code)
            .map_err(|e| ExecutionError::CompilationError(format!("Failed to compile: {}", e)))?;
        
        // Cache it
        {
            let mut cache = self.module_cache.write().await;
            cache.insert(code_hash.to_vec(), module.clone());
        }
        
        Ok(module)
    }

    /// Calculate hash of WASM code
    fn calculate_hash(&self, wasm_code: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(wasm_code);
        hasher.finalize().to_vec()
    }

    /// Process raw state changes into structured format
    fn process_state_changes(
        &self,
        contract_address: &str,
        raw_changes: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    ) -> Vec<StateChange> {
        raw_changes
            .into_iter()
            .map(|(key, value)| {
                // Prefix keys with contract address
                let prefixed_key = format!("{}:{}", contract_address, hex::encode(&key));
                
                match value {
                    Some(val) => StateChange::Set {
                        key: prefixed_key.into_bytes(),
                        value: val,
                    },
                    None => StateChange::Remove {
                        key: prefixed_key.into_bytes(),
                    },
                }
            })
            .collect()
    }

    /// Process raw events into structured format
    fn process_events(&self, raw_events: Vec<(String, Vec<(String, String)>)>) -> Vec<Event> {
        raw_events
            .into_iter()
            .map(|(typ, attrs)| Event {
                typ,
                attributes: attrs.into_iter().collect(),
            })
            .collect()
    }

    /// Calculate gas used (simplified version)
    pub fn calculate_gas_used(&self, state_changes: &[StateChange], events: &[Event]) -> u64 {
        let mut gas = 1000; // Base execution cost
        
        // Add cost for state changes
        for change in state_changes {
            match change {
                StateChange::Set { key, value } => {
                    gas += 100 + key.len() as u64 + value.len() as u64;
                }
                StateChange::Remove { key } => {
                    gas += 50 + key.len() as u64;
                }
            }
        }
        
        // Add cost for events
        for event in events {
            gas += 20 + event.attributes.len() as u64 * 5;
        }
        
        gas
    }

    /// Query contract state (read-only)
    pub async fn query_contract(
        &self,
        contract_address: &str,
        wasm_code: &[u8],
        query_msg: &[u8],
        env: CosmWasmEnv,
    ) -> Result<Vec<u8>, ExecutionError> {
        // For queries, we use a fresh executor without state tracking
        let mut executor = WasmerExecutor::new(contract_address.as_bytes().to_vec());
        executor.set_env(env);
        
        // Execute the query (typically "query" entry point in CosmWasm)
        executor.execute_wasm(wasm_code, "query", query_msg)
    }

    /// Instantiate a new contract
    pub async fn instantiate_contract(
        &self,
        code_id: u64,
        wasm_code: &[u8],
        init_msg: &[u8],
        env: CosmWasmEnv,
    ) -> Result<(String, ExecutionResult), ExecutionError> {
        // Generate contract address
        let contract_address = self.generate_contract_address(code_id, &env);
        
        // Execute instantiation
        let result = self.execute_contract(
            &contract_address,
            wasm_code,
            "instantiate",
            init_msg,
            env,
        ).await?;
        
        // Store contract instance
        {
            let mut instances = self.instances.write().await;
            instances.insert(
                contract_address.clone(),
                ContractInstance {
                    contract_address: contract_address.clone(),
                    code_hash: self.calculate_hash(wasm_code),
                    state: ContractState::default(),
                },
            );
        }
        
        Ok((contract_address, result))
    }

    /// Generate a deterministic contract address
    pub fn generate_contract_address(&self, code_id: u64, env: &CosmWasmEnv) -> String {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(code_id.to_le_bytes());
        hasher.update(env.block.height.to_le_bytes());
        hasher.update(env.block.time.to_le_bytes());
        if let Some(creator) = &env.contract.creator {
            hasher.update(creator.as_bytes());
        }
        
        let hash = hasher.finalize();
        format!("proxima1{}", hex::encode(&hash[0..20]))
    }
}