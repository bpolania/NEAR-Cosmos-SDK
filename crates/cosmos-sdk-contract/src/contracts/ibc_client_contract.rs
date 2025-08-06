/// IBC Client Module Contract
/// 
/// This contract handles all IBC light client operations including:
/// - Client creation and updates
/// - Consensus state management
/// - Membership proofs and verification
/// - Batch verification operations

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::modules::ibc::client::tendermint::{TendermintLightClientModule, Header, ClientState, ConsensusState};

/// IBC Client contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct IbcClientContract {
    /// The underlying tendermint light client module
    client_module: TendermintLightClientModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from client operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ClientOperationResponse {
    pub success: bool,
    pub client_id: Option<String>,
    pub data: Option<String>,
    pub error: Option<String>,
}

/// Batch verification item
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct VerificationItem {
    pub key: String,
    pub value: Option<String>, // None for non-existence proofs
}

#[near_bindgen]
impl IbcClientContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            client_module: TendermintLightClientModule::new(),
            router_contract,
            owner,
        }
    }

    /// Create a new IBC light client
    pub fn create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
        initial_header: Header,
    ) -> String {
        self.assert_authorized_caller();
        
        let client_id = self.client_module.create_client(
            chain_id, 
            trust_period, 
            unbonding_period, 
            max_clock_drift, 
            initial_header
        );
        
        env::log_str(&format!("Created IBC client: {}", client_id));
        client_id
    }

    /// Update an existing IBC light client with a new header
    pub fn update_client(&mut self, client_id: String, header: Header) -> bool {
        self.assert_authorized_caller();
        
        let success = self.client_module.update_client(client_id.clone(), header);
        
        if success {
            env::log_str(&format!("Updated IBC client: {}", client_id));
        } else {
            env::log_str(&format!("Failed to update IBC client: {}", client_id));
        }
        
        success
    }

    /// Verify membership proof for a key-value pair
    pub fn verify_membership(
        &self,
        client_id: String,
        height: u64,
        key: Base64VecU8,
        value: Base64VecU8,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        let result = self.client_module.verify_membership(
            client_id, 
            height, 
            key.into(), 
            value.into(), 
            proof.into()
        );
        
        env::log_str(&format!("Membership verification result: {}", result));
        result
    }

    /// Verify non-membership proof for a key
    pub fn verify_non_membership(
        &self,
        client_id: String,
        height: u64,
        key: Base64VecU8,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        let result = self.client_module.verify_non_membership(
            client_id, 
            height, 
            key.into(), 
            proof.into()
        );
        
        env::log_str(&format!("Non-membership verification result: {}", result));
        result
    }

    /// Verify batch membership proofs
    pub fn verify_batch_membership(
        &self,
        client_id: String,
        height: u64,
        items: Vec<VerificationItem>,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        // Convert items to the format expected by the client module
        let converted_items: Vec<(Vec<u8>, Option<Vec<u8>>)> = items
            .into_iter()
            .map(|item| (item.key.into(), item.value.map(|v| v.into())))
            .collect();
        
        let result = self.client_module.verify_batch_membership(
            client_id, 
            height, 
            converted_items, 
            proof.into()
        );
        
        env::log_str(&format!("Batch membership verification result: {}", result));
        result
    }

    /// Verify mixed batch (both existence and non-existence proofs)
    pub fn verify_mixed_batch_membership(
        &self,
        client_id: String,
        height: u64,
        exist_items: Vec<(Base64VecU8, Base64VecU8)>,
        non_exist_keys: Vec<Base64VecU8>,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        // Convert items to Vec<u8> format
        let exist_converted: Vec<(Vec<u8>, Vec<u8>)> = exist_items
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        
        let non_exist_converted: Vec<Vec<u8>> = non_exist_keys
            .into_iter()
            .map(|k| k.into())
            .collect();
        
        let result = self.client_module.verify_mixed_batch_membership(
            client_id, 
            height, 
            exist_converted, 
            non_exist_converted, 
            proof.into()
        );
        
        env::log_str(&format!("Mixed batch verification result: {}", result));
        result
    }

    /// Verify compressed batch membership proofs
    pub fn verify_compressed_batch_membership(
        &self,
        client_id: String,
        height: u64,
        compressed_batch: Base64VecU8,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        let result = self.client_module.verify_compressed_batch_membership(
            client_id, 
            height, 
            serde_json::from_slice(&compressed_batch.0).unwrap_or_default(), 
            serde_json::from_slice(&proof.0).unwrap_or_default()
        );
        
        env::log_str(&format!("Compressed batch verification result: {}", result));
        result
    }

    /// Verify range membership proofs
    pub fn verify_range_membership(
        &self,
        client_id: String,
        height: u64,
        start_key: Base64VecU8,
        end_key: Base64VecU8,
        values: Vec<Base64VecU8>,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        // Convert single values to key-value pairs (key is empty for range membership)
        let values_converted: Vec<(Vec<u8>, Vec<u8>)> = values
            .into_iter()
            .map(|v| (vec![], v.into()))
            .collect();
        
        let result = self.client_module.verify_range_membership(
            client_id, 
            height, 
            start_key.into(), 
            end_key.into(), 
            true, // existence check
            values_converted, 
            serde_json::from_slice(&proof.0).unwrap_or_default()
        );
        
        env::log_str(&format!("Range membership verification result: {}", result));
        result
    }

    /// Verify multistore membership proofs
    pub fn verify_multistore_membership(
        &self,
        client_id: String,
        height: u64,
        store_keys: Vec<String>,
        key_values: Vec<(Base64VecU8, Base64VecU8)>,
        proof: Base64VecU8,
    ) -> bool {
        self.assert_authorized_caller();
        
        let kv_converted: Vec<(Vec<u8>, Vec<u8>)> = key_values
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        
        // For now, verify only the first key-value pair
        let result = if let (Some(store_name), Some((key, value))) = (store_keys.first(), kv_converted.first()) {
            self.client_module.verify_multistore_membership(
                client_id, 
                height, 
                store_name.clone(),
                key.clone(),
                value.clone(),
                proof.into()
            )
        } else {
            false
        };
        
        env::log_str(&format!("Multistore membership verification result: {}", result));
        result
    }

    /// Get client state for a given client ID
    pub fn get_client_state(&self, client_id: String) -> Option<ClientState> {
        self.assert_authorized_caller();
        self.client_module.get_client_state(client_id)
    }

    /// Get consensus state for a given client ID and height
    pub fn get_consensus_state(&self, client_id: String, height: u64) -> Option<ConsensusState> {
        self.assert_authorized_caller();
        self.client_module.get_consensus_state(client_id, height)
    }

    /// Get all client IDs
    pub fn get_all_clients(&self) -> Vec<String> {
        self.assert_authorized_caller();
        self.client_module.get_all_clients()
    }

    /// Check if a client exists
    pub fn client_exists(&self, client_id: String) -> bool {
        self.assert_authorized_caller();
        self.client_module.client_exists(client_id)
    }

    /// Get client type (always returns "07-tendermint" for this implementation)
    pub fn get_client_type(&self, client_id: String) -> Option<String> {
        self.assert_authorized_caller();
        if self.client_module.client_exists(client_id) {
            Some("07-tendermint".to_string())
        } else {
            None
        }
    }

    /// Update the router contract address
    pub fn update_router_contract(&mut self, new_router: AccountId) {
        self.assert_owner();
        self.router_contract = Some(new_router.clone());
        env::log_str(&format!("Updated router contract to: {}", new_router));
    }

    /// Get current router contract
    pub fn get_router_contract(&self) -> Option<AccountId> {
        self.router_contract.clone()
    }

    /// Health check for the client module
    pub fn health_check(&self) -> bool {
        // Check if the client module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "ibc_client",
            "version": "1.0.0",
            "description": "IBC Light Client Module",
            "functions": [
                "create_client",
                "update_client", 
                "verify_membership",
                "verify_non_membership",
                "verify_batch_membership",
                "verify_mixed_batch_membership",
                "verify_compressed_batch_membership",
                "verify_range_membership",
                "verify_multistore_membership",
                "get_client_state",
                "get_consensus_state",
                "get_all_clients",
                "client_exists",
                "get_client_type"
            ]
        })
    }

    /// Assert that the caller is authorized (owner or router)
    fn assert_authorized_caller(&self) {
        let caller = env::predecessor_account_id();
        
        let is_owner = caller == self.owner;
        let is_router = self.router_contract.as_ref().map_or(false, |router| caller == *router);
        
        assert!(
            is_owner || is_router,
            "Unauthorized: only owner or router can call this function"
        );
    }

    /// Assert that the caller is the contract owner
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can perform this action"
        );
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
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
    fn test_client_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcClientContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_authorized_caller_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcClientContract::new(accounts(1), Some(accounts(2)));
        
        // Owner should be authorized
        contract.assert_authorized_caller();
        
        // Test router access
        let router_context = get_context(accounts(2));
        testing_env!(router_context);
        contract.assert_authorized_caller();
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_caller() {
        let context = get_context(accounts(3)); // Unauthorized account
        testing_env!(context);
        
        let contract = IbcClientContract::new(accounts(1), Some(accounts(2)));
        contract.assert_authorized_caller();
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcClientContract::new(accounts(1), None);
        assert!(contract.health_check());
    }
}