use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::types::{AccountId, BlockReference};
use near_primitives::views::QueryRequest;

use super::types::StateChange;

/// Manages state synchronization between the relayer and NEAR
pub struct StateManager {
    /// Local state cache for each contract
    cache: Arc<RwLock<HashMap<String, ContractStateCache>>>,
    
    /// NEAR RPC client for reading state
    near_client: Arc<JsonRpcClient>,
    
    /// Pending state changes to be submitted
    pending: Arc<RwLock<HashMap<String, Vec<StateChange>>>>,
    
    /// NEAR account that holds the contracts
    near_account_id: AccountId,
}

pub struct ContractStateCache {
    /// Last synced block height
    pub last_height: u64,
    
    /// Cached key-value pairs
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    
    /// Timestamp of last sync
    pub last_sync: std::time::SystemTime,
}

impl StateManager {
    pub fn new(near_client: Arc<JsonRpcClient>, near_account_id: AccountId) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            near_client,
            pending: Arc::new(RwLock::new(HashMap::new())),
            near_account_id,
        }
    }

    /// Get value from contract storage
    pub async fn get(&self, contract: &str, key: &[u8]) -> Option<Vec<u8>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(contract_cache) = cache.get(contract) {
                // Check if cache is still fresh (less than 5 seconds old)
                if let Ok(elapsed) = contract_cache.last_sync.elapsed() {
                    if elapsed.as_secs() < 5 {
                        if let Some(value) = contract_cache.storage.get(key) {
                            return Some(value.clone());
                        }
                    }
                }
            }
        }
        
        // Read from NEAR
        self.read_from_near(contract, key).await
    }

    /// Set value in contract storage (pending)
    pub async fn set(&self, contract: &str, key: Vec<u8>, value: Vec<u8>) {
        // Clone for cache update
        let key_clone = key.clone();
        let value_clone = value.clone();
        
        let mut pending = self.pending.write().await;
        pending
            .entry(contract.to_string())
            .or_default()
            .push(StateChange::Set { key, value });
        
        // Also update cache
        let mut cache = self.cache.write().await;
        let contract_cache = cache
            .entry(contract.to_string())
            .or_insert_with(|| ContractStateCache {
                last_height: 0,
                storage: HashMap::new(),
                last_sync: std::time::SystemTime::now(),
            });
        contract_cache.storage.insert(key_clone, value_clone);
    }

    /// Remove key from contract storage (pending)
    pub async fn remove(&self, contract: &str, key: Vec<u8>) {
        let mut pending = self.pending.write().await;
        pending
            .entry(contract.to_string())
            .or_default()
            .push(StateChange::Remove { key: key.clone() });
        
        // Also update cache
        let mut cache = self.cache.write().await;
        if let Some(contract_cache) = cache.get_mut(contract) {
            contract_cache.storage.remove(&key);
        }
    }

    /// Get all pending changes for a contract
    pub async fn get_pending_changes(&self, contract: &str) -> Vec<StateChange> {
        let mut pending = self.pending.write().await;
        pending.remove(contract).unwrap_or_default()
    }

    /// Read state directly from NEAR
    async fn read_from_near(&self, contract: &str, key: &[u8]) -> Option<Vec<u8>> {
        // Construct the storage key
        // In NEAR, storage keys are typically prefixed with the contract's data
        let storage_key = format!("{}:{}", contract, hex::encode(key));
        
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::ViewState {
                account_id: self.near_account_id.clone(),
                prefix: storage_key.into_bytes().into(),
                include_proof: false,
            },
        };
        
        match self.near_client.call(request).await {
            Ok(response) => {
                if let QueryResponseKind::ViewState(state_view) = response.kind {
                    // Return the first value found (simplified)
                    state_view.values.first().map(|sv| sv.value.clone().into())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Sync all state for a contract from NEAR
    pub async fn sync_contract_state(&self, contract: &str) -> Result<(), String> {
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::ViewState {
                account_id: self.near_account_id.clone(),
                prefix: contract.as_bytes().to_vec().into(),
                include_proof: false,
            },
        };
        
        let response = self.near_client.call(request).await
            .map_err(|e| format!("Failed to query NEAR state: {}", e))?;
        
        if let QueryResponseKind::ViewState(state_view) = response.kind {
            let mut cache = self.cache.write().await;
            let contract_cache = cache
                .entry(contract.to_string())
                .or_insert_with(|| ContractStateCache {
                    last_height: 0,
                    storage: HashMap::new(),
                    last_sync: std::time::SystemTime::now(),
                });
            
            // Update cache with all values
            for state_value in state_view.values {
                contract_cache.storage.insert(
                    state_value.key.clone().into(),
                    state_value.value.clone().into(),
                );
            }
            
            contract_cache.last_sync = std::time::SystemTime::now();
            contract_cache.last_height = response.block_height;
        }
        
        Ok(())
    }

    /// Clear cache for a contract
    pub async fn clear_cache(&self, contract: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(contract);
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, (usize, u64)> {
        let cache = self.cache.read().await;
        cache
            .iter()
            .map(|(contract, state)| {
                (
                    contract.clone(),
                    (state.storage.len(), state.last_height),
                )
            })
            .collect()
    }
}