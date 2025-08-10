/// State Synchronization Contract for Modular Architecture
/// 
/// This contract provides state synchronization mechanisms between modules,
/// ensuring consistency and coordination in cross-module operations.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise, PromiseResult};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::{UnorderedMap, Vector};
use schemars::JsonSchema;
use std::collections::HashMap;

/// State synchronization event
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SyncEvent {
    pub id: String,
    pub module: String,
    pub event_type: String,
    pub data: String,
    pub timestamp: u64,
    pub block_height: u64,
}

/// Cross-module transaction record
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CrossModuleTx {
    pub tx_id: String,
    pub initiator_module: String,
    pub target_modules: Vec<String>,
    pub status: TxStatus,
    pub operations: Vec<String>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum TxStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Rolled_back,
}

/// State checkpoint for module synchronization
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct StateCheckpoint {
    pub module: String,
    pub checkpoint_id: String,
    pub state_hash: String,
    pub block_height: u64,
    pub timestamp: u64,
}

/// State synchronization contract
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StateSyncContract {
    /// Router contract that coordinates modules
    router_contract: AccountId,
    /// Contract owner for admin operations
    owner: AccountId,
    /// Registered modules that participate in sync
    registered_modules: UnorderedMap<String, AccountId>,
    /// Event log for cross-module synchronization
    sync_events: Vector<SyncEvent>,
    /// Cross-module transaction tracking
    cross_module_txs: UnorderedMap<String, CrossModuleTx>,
    /// State checkpoints for each module
    state_checkpoints: UnorderedMap<String, StateCheckpoint>,
    /// Module dependencies (which modules depend on others)
    module_dependencies: UnorderedMap<String, Vec<String>>,
    /// Current block height for synchronization
    current_block_height: u64,
}

#[near_bindgen]
impl StateSyncContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: AccountId) -> Self {
        Self {
            router_contract,
            owner,
            registered_modules: UnorderedMap::new(b"modules".to_vec()),
            sync_events: Vector::new(b"events".to_vec()),
            cross_module_txs: UnorderedMap::new(b"txs".to_vec()),
            state_checkpoints: UnorderedMap::new(b"checkpoints".to_vec()),
            module_dependencies: UnorderedMap::new(b"deps".to_vec()),
            current_block_height: env::block_height(),
        }
    }

    // =============================================================================
    // Module Registration and Management
    // =============================================================================

    /// Register a module for state synchronization
    pub fn register_module(&mut self, module_name: String, module_contract: AccountId) {
        self.assert_router_or_owner();
        
        self.registered_modules.insert(&module_name, &module_contract);
        
        env::log_str(&format!("Registered module for sync: {} -> {}", 
                             module_name, module_contract));
    }

    /// Set module dependencies (which modules this module depends on)
    pub fn set_module_dependencies(&mut self, module_name: String, dependencies: Vec<String>) {
        self.assert_router_or_owner();
        
        // Validate that all dependencies are registered
        for dep in &dependencies {
            assert!(
                self.registered_modules.get(dep).is_some(),
                "Dependency module {} not registered", dep
            );
        }
        
        self.module_dependencies.insert(&module_name, &dependencies);
        
        env::log_str(&format!("Set dependencies for {}: {:?}", 
                             module_name, dependencies));
    }

    /// Get registered modules
    pub fn get_registered_modules(&self) -> HashMap<String, String> {
        let mut modules = HashMap::new();
        
        for (module_name, module_contract) in self.registered_modules.iter() {
            modules.insert(module_name, module_contract.to_string());
        }
        
        modules
    }

    // =============================================================================
    // Event Synchronization
    // =============================================================================

    /// Record a synchronization event from a module
    pub fn record_sync_event(
        &mut self,
        event_id: String,
        event_type: String,
        data: String,
    ) {
        let caller = env::predecessor_account_id();
        
        // Find which module this caller represents
        let module_name = self.find_module_by_contract(&caller)
            .expect("Caller is not a registered module");
        
        let sync_event = SyncEvent {
            id: event_id,
            module: module_name,
            event_type,
            data,
            timestamp: env::block_timestamp(),
            block_height: env::block_height(),
        };
        
        self.sync_events.push(&sync_event);
        
        env::log_str(&format!("Recorded sync event: {} from {}", 
                             sync_event.id, sync_event.module));
        
        // Trigger synchronization to dependent modules
        self.trigger_dependent_sync(&sync_event);
    }

    /// Get sync events for a specific module
    pub fn get_sync_events(&self, module_name: String, limit: Option<u32>) -> Vec<SyncEvent> {
        let limit = limit.unwrap_or(50).min(100) as usize;
        let mut events = Vec::new();
        
        // Iterate through events in reverse order (newest first)
        let total_events = self.sync_events.len();
        let start = if total_events > limit as u64 { total_events - limit as u64 } else { 0 };
        
        for i in start..total_events {
            if let Some(event) = self.sync_events.get(i) {
                if event.module == module_name {
                    events.push(event);
                }
            }
        }
        
        events.reverse(); // Newest first
        events
    }

    // =============================================================================
    // Cross-Module Transaction Coordination
    // =============================================================================

    /// Begin a cross-module transaction
    pub fn begin_cross_module_tx(
        &mut self,
        tx_id: String,
        target_modules: Vec<String>,
        operations: Vec<String>,
    ) -> String {
        let caller = env::predecessor_account_id();
        let initiator_module = self.find_module_by_contract(&caller)
            .expect("Caller is not a registered module");
        
        // Validate that all target modules are registered
        for module in &target_modules {
            assert!(
                self.registered_modules.get(module).is_some(),
                "Target module {} not registered", module
            );
        }
        
        let cross_module_tx = CrossModuleTx {
            tx_id: tx_id.clone(),
            initiator_module,
            target_modules,
            status: TxStatus::Pending,
            operations,
            created_at: env::block_timestamp(),
            completed_at: None,
        };
        
        self.cross_module_txs.insert(&tx_id, &cross_module_tx);
        
        env::log_str(&format!("Started cross-module tx: {}", tx_id));
        
        tx_id
    }

    /// Update cross-module transaction status
    pub fn update_cross_module_tx_status(&mut self, tx_id: String, status: TxStatus) {
        let caller = env::predecessor_account_id();
        let module_name = self.find_module_by_contract(&caller)
            .expect("Caller is not a registered module");
        
        if let Some(mut tx) = self.cross_module_txs.get(&tx_id) {
            // Verify the caller is part of this transaction
            assert!(
                tx.initiator_module == module_name || tx.target_modules.contains(&module_name),
                "Module {} not part of transaction {}", module_name, tx_id
            );
            
            tx.status = status;
            
            if matches!(tx.status, TxStatus::Completed | TxStatus::Failed | TxStatus::Rolled_back) {
                tx.completed_at = Some(env::block_timestamp());
            }
            
            self.cross_module_txs.insert(&tx_id, &tx);
            
            env::log_str(&format!("Updated tx {} status to {:?}", tx_id, tx.status));
        } else {
            env::panic_str(&format!("Transaction {} not found", tx_id));
        }
    }

    /// Get cross-module transaction status
    pub fn get_cross_module_tx(&self, tx_id: String) -> Option<CrossModuleTx> {
        self.cross_module_txs.get(&tx_id)
    }

    // =============================================================================
    // State Checkpointing
    // =============================================================================

    /// Create a state checkpoint for a module
    pub fn create_checkpoint(&mut self, checkpoint_id: String, state_hash: String) {
        let caller = env::predecessor_account_id();
        let module_name = self.find_module_by_contract(&caller)
            .expect("Caller is not a registered module");
        
        let checkpoint = StateCheckpoint {
            module: module_name.clone(),
            checkpoint_id: checkpoint_id.clone(),
            state_hash,
            block_height: env::block_height(),
            timestamp: env::block_timestamp(),
        };
        
        let checkpoint_key = format!("{}:{}", module_name, checkpoint_id);
        self.state_checkpoints.insert(&checkpoint_key, &checkpoint);
        
        env::log_str(&format!("Created checkpoint {} for module {}", 
                             checkpoint_id, module_name));
    }

    /// Get state checkpoint
    pub fn get_checkpoint(&self, module_name: String, checkpoint_id: String) -> Option<StateCheckpoint> {
        let checkpoint_key = format!("{}:{}", module_name, checkpoint_id);
        self.state_checkpoints.get(&checkpoint_key)
    }

    /// Get latest checkpoint for a module
    pub fn get_latest_checkpoint(&self, module_name: String) -> Option<StateCheckpoint> {
        let mut latest_checkpoint = None;
        let mut latest_block = 0u64;
        
        // Iterate through all checkpoints to find the latest for this module
        for (key, checkpoint) in self.state_checkpoints.iter() {
            if checkpoint.module == module_name && checkpoint.block_height > latest_block {
                latest_block = checkpoint.block_height;
                latest_checkpoint = Some(checkpoint);
            }
        }
        
        latest_checkpoint
    }

    // =============================================================================
    // Synchronization Operations
    // =============================================================================

    /// Wait for dependencies to reach a specific state
    pub fn wait_for_dependencies(&self, module_name: String, target_block: u64) -> Promise {
        if let Some(dependencies) = self.module_dependencies.get(&module_name) {
            // Check each dependency's latest checkpoint
            let mut all_ready = true;
            
            for dep in dependencies {
                if let Some(checkpoint) = self.get_latest_checkpoint(dep) {
                    if checkpoint.block_height < target_block {
                        all_ready = false;
                        break;
                    }
                } else {
                    all_ready = false;
                    break;
                }
            }
            
            if all_ready {
                // All dependencies are ready
                Promise::new(env::current_account_id())
                    .function_call(
                        "dependencies_ready".to_string(),
                        format!("{{\"module\": \"{}\", \"block\": {}}}", module_name, target_block).into(),
                        near_sdk::NearToken::from_yoctonear(0),
                        near_sdk::Gas::from_gas(5_000_000_000_000), // 5 TGas
                    )
            } else {
                // Dependencies not ready, retry after a delay
                Promise::new(env::current_account_id())
                    .function_call(
                        "retry_dependency_check".to_string(),
                        format!("{{\"module\": \"{}\", \"block\": {}}}", module_name, target_block).into(),
                        near_sdk::NearToken::from_yoctonear(0),
                        near_sdk::Gas::from_gas(5_000_000_000_000), // 5 TGas
                    )
            }
        } else {
            // No dependencies, always ready
            Promise::new(env::current_account_id())
                .function_call(
                    "dependencies_ready".to_string(),
                    format!("{{\"module\": \"{}\", \"block\": {}}}", module_name, target_block).into(),
                    near_sdk::NearToken::from_yoctonear(0),
                    near_sdk::Gas::from_gas(5_000_000_000_000), // 5 TGas
                )
        }
    }

    #[private]
    pub fn dependencies_ready(&self, module: String, block: u64) -> bool {
        env::log_str(&format!("Dependencies ready for module {} at block {}", module, block));
        true
    }

    #[private]
    pub fn retry_dependency_check(&self, module: String, block: u64) -> Promise {
        env::log_str(&format!("Retrying dependency check for module {} at block {}", module, block));
        
        // Retry with a small delay (simulated by calling self)
        Promise::new(env::current_account_id())
            .function_call(
                "wait_for_dependencies".to_string(),
                format!("{{\"module_name\": \"{}\", \"target_block\": {}}}", module, block).into(),
                near_sdk::NearToken::from_yoctonear(0),
                near_sdk::Gas::from_gas(10_000_000_000_000), // 10 TGas
            )
    }

    // =============================================================================
    // Helper Methods
    // =============================================================================

    /// Find module name by contract address
    fn find_module_by_contract(&self, contract: &AccountId) -> Option<String> {
        for (module_name, module_contract) in self.registered_modules.iter() {
            if module_contract == *contract {
                return Some(module_name);
            }
        }
        None
    }

    /// Trigger synchronization to modules that depend on the given event
    fn trigger_dependent_sync(&self, event: &SyncEvent) {
        // Find modules that depend on the event's module
        for (dependent_module, dependencies) in self.module_dependencies.iter() {
            if dependencies.contains(&event.module) {
                if let Some(dependent_contract) = self.registered_modules.get(&dependent_module) {
                    env::log_str(&format!("Triggering sync to dependent module: {}", dependent_module));
                    
                    // In a full implementation, would make a cross-contract call here
                    // For now, just log the synchronization trigger
                }
            }
        }
    }

    /// Update block height for synchronization tracking
    pub fn update_block_height(&mut self) {
        self.assert_router_or_owner();
        self.current_block_height = env::block_height();
    }

    /// Get current synchronization status
    pub fn get_sync_status(&self) -> HashMap<String, serde_json::Value> {
        let mut status = HashMap::new();
        
        status.insert("current_block_height".to_string(), 
                     serde_json::Value::Number(self.current_block_height.into()));
        
        status.insert("total_events".to_string(), 
                     serde_json::Value::Number(self.sync_events.len().into()));
        
        status.insert("active_transactions".to_string(), 
                     serde_json::Value::Number(self.cross_module_txs.len().into()));
        
        let mut module_checkpoints = HashMap::new();
        for module in self.registered_modules.keys() {
            if let Some(checkpoint) = self.get_latest_checkpoint(module.clone()) {
                module_checkpoints.insert(module, checkpoint.block_height);
            }
        }
        status.insert("module_checkpoints".to_string(), 
                     serde_json::to_value(module_checkpoints).unwrap());
        
        status
    }

    // =============================================================================
    // Access Control
    // =============================================================================

    /// Assert that caller is router or owner
    fn assert_router_or_owner(&self) {
        let caller = env::predecessor_account_id();
        assert!(
            caller == self.router_contract || caller == self.owner,
            "Only router or owner can perform this action"
        );
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can transfer ownership"
        );
        
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        env::log_str(&format!("Ownership transferred from {} to {}", old_owner, new_owner));
    }
}

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
    fn test_state_sync_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = StateSyncContract::new(accounts(1), accounts(2));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, accounts(2));
    }

    #[test]
    fn test_module_registration() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = StateSyncContract::new(accounts(1), accounts(2));
        contract.register_module("ibc_client".to_string(), accounts(3));
        
        let modules = contract.get_registered_modules();
        assert_eq!(modules.get("ibc_client"), Some(&accounts(3).to_string()));
    }

    #[test]
    fn test_sync_event_recording() {
        let context = get_context(accounts(3)); // Module contract calling
        testing_env!(context);
        
        let mut contract = StateSyncContract::new(accounts(1), accounts(2));
        contract.register_module("ibc_client".to_string(), accounts(3));
        
        contract.record_sync_event(
            "event_1".to_string(),
            "client_update".to_string(),
            "{}".to_string(),
        );
        
        let events = contract.get_sync_events("ibc_client".to_string(), Some(10));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "event_1");
    }
}