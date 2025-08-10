/// IBC Connection Module Contract
/// 
/// This contract handles all IBC connection operations including:
/// - Connection handshake (open, try, ack, confirm)
/// - Connection state management
/// - Connection queries and validation
/// - Client association management

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::modules::ibc::connection::{ConnectionModule, ConnectionEnd, Version, State as ConnectionState, Counterparty, MerklePrefix};

/// IBC Connection contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct IbcConnectionContract {
    /// The underlying connection module
    connection_module: ConnectionModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from connection operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ConnectionOperationResponse {
    pub success: bool,
    pub connection_id: Option<String>,
    pub client_id: Option<String>,
    pub data: Option<String>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Connection handshake data
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ConnectionHandshakeData {
    pub client_id: String,
    pub counterparty_client_id: String,
    pub counterparty_connection_id: Option<String>,
    pub counterparty_prefix: String,
    pub version: Version,
    pub delay_period: u64,
}

#[near_bindgen]
impl IbcConnectionContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            connection_module: ConnectionModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Connection Handshake Functions
    // =============================================================================

    /// Open a new IBC connection (first step of handshake)
    pub fn connection_open_init(
        &mut self,
        handshake_data: ConnectionHandshakeData,
    ) -> ConnectionOperationResponse {
        self.assert_authorized_caller();
        
        let counterparty = Counterparty {
            client_id: handshake_data.counterparty_client_id,
            connection_id: None,
            prefix: MerklePrefix { key_prefix: handshake_data.counterparty_prefix.into_bytes() },
        };
        
        let connection_id = self.connection_module.connection_open_init(
            handshake_data.client_id.clone(),
            counterparty,
            Some(handshake_data.version),
            handshake_data.delay_period,
        );
        
        if !connection_id.is_empty() {
                env::log_str(&format!("Connection opened: {} for client {}", connection_id, handshake_data.client_id));
                ConnectionOperationResponse {
                    success: true,
                    connection_id: Some(connection_id),
                    client_id: Some(handshake_data.client_id),
                    data: None,
                    events: vec!["connection_open_init".to_string()],
                    error: None,
                }
        } else {
                env::log_str("Connection open failed: empty connection ID");
                ConnectionOperationResponse {
                    success: false,
                    connection_id: None,
                    client_id: Some(handshake_data.client_id),
                    data: None,
                    events: vec![],
                    error: Some("Operation failed".to_string()),
                }
        }
    }

    /// Try to open a connection (second step of handshake)
    pub fn connection_open_try(
        &mut self,
        handshake_data: ConnectionHandshakeData,
        proof_init: Base64VecU8,
        proof_client: Base64VecU8,
        proof_consensus: Base64VecU8,
        proof_height: u64,
    ) -> ConnectionOperationResponse {
        self.assert_authorized_caller();
        
        let counterparty = Counterparty {
            client_id: handshake_data.counterparty_client_id,
            connection_id: handshake_data.counterparty_connection_id,
            prefix: MerklePrefix { key_prefix: handshake_data.counterparty_prefix.into_bytes() },
        };
        
        match self.connection_module.connection_open_try(
            handshake_data.client_id.clone(),
            None, // previous_connection_id
            counterparty,
            handshake_data.delay_period,
            vec![handshake_data.version], // supported versions
            proof_init.into(),
            proof_client.into(),
            proof_consensus.into(),
            proof_height,
            proof_height, // consensus_height - should be passed separately
        ) {
            Ok(connection_id) => {
                env::log_str(&format!("Connection try successful: {} for client {}", connection_id, handshake_data.client_id));
                ConnectionOperationResponse {
                    success: true,
                    connection_id: Some(connection_id),
                    client_id: Some(handshake_data.client_id),
                    data: None,
                    events: vec!["connection_open_try".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Connection try failed: {:?}", e));
                ConnectionOperationResponse {
                    success: false,
                    connection_id: None,
                    client_id: Some(handshake_data.client_id),
                    data: None,
                    events: vec![],
                    error: Some("Operation failed".to_string()),
                }
            }
        }
    }

    /// Acknowledge connection opening (third step of handshake)
    pub fn connection_open_ack(
        &mut self,
        connection_id: String,
        counterparty_connection_id: String,
        version: Version,
        proof_try: Base64VecU8,
        proof_client: Base64VecU8,
        proof_consensus: Base64VecU8,
        proof_height: u64,
    ) -> ConnectionOperationResponse {
        self.assert_authorized_caller();
        
        match self.connection_module.connection_open_ack(
            connection_id.clone(),
            counterparty_connection_id,
            version,
            proof_try.into(),
            proof_client.into(),
            proof_consensus.into(),
            proof_height,
            proof_height, // consensus_height - should be passed separately
        ) {
            Ok(_) => {
                env::log_str(&format!("Connection ack successful: {}", connection_id));
                ConnectionOperationResponse {
                    success: true,
                    connection_id: Some(connection_id),
                    client_id: None,
                    data: None,
                    events: vec!["connection_open_ack".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Connection ack failed: {:?}", e));
                ConnectionOperationResponse {
                    success: false,
                    connection_id: Some(connection_id),
                    client_id: None,
                    data: None,
                    events: vec![],
                    error: Some("Operation failed".to_string()),
                }
            }
        }
    }

    /// Confirm connection opening (final step of handshake)
    pub fn connection_open_confirm(
        &mut self,
        connection_id: String,
        proof_ack: Base64VecU8,
        proof_height: u64,
    ) -> ConnectionOperationResponse {
        self.assert_authorized_caller();
        
        match self.connection_module.connection_open_confirm(
            connection_id.clone(),
            proof_ack.into(),
            proof_height,
        ) {
            Ok(_) => {
                env::log_str(&format!("Connection confirmed: {}", connection_id));
                ConnectionOperationResponse {
                    success: true,
                    connection_id: Some(connection_id),
                    client_id: None,
                    data: None,
                    events: vec!["connection_open_confirm".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Connection confirm failed: {:?}", e));
                ConnectionOperationResponse {
                    success: false,
                    connection_id: Some(connection_id),
                    client_id: None,
                    data: None,
                    events: vec![],
                    error: Some("Operation failed".to_string()),
                }
            }
        }
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get connection information
    pub fn get_connection(&self, connection_id: String) -> Option<ConnectionEnd> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> Vec<(String, ConnectionEnd)> {
        self.assert_authorized_caller();
        // Get connection IDs and fetch each connection
        self.connection_module.get_all_connections()
            .into_iter()
            .filter_map(|conn_id| {
                self.connection_module.get_connection(conn_id.clone())
                    .map(|conn_end| (conn_id, conn_end))
            })
            .collect()
    }

    /// Get connections for a specific client
    pub fn get_connections_for_client(&self, client_id: String) -> Vec<String> {
        self.assert_authorized_caller();
        self.connection_module.get_connections_for_client(client_id)
    }

    /// Check if a connection exists
    pub fn connection_exists(&self, connection_id: String) -> bool {
        self.assert_authorized_caller();
        self.connection_module.connection_exists(connection_id)
    }

    /// Get connection state
    pub fn get_connection_state(&self, connection_id: String) -> Option<ConnectionState> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
            .map(|conn| conn.state)
    }

    /// Get connection client ID
    pub fn get_connection_client(&self, connection_id: String) -> Option<String> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
            .map(|conn| conn.client_id)
    }

    /// Get connection counterparty
    pub fn get_connection_counterparty(&self, connection_id: String) -> Option<String> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
            .map(|conn| conn.counterparty.connection_id.unwrap_or_default())
    }

    // =============================================================================
    // Validation Functions (called by router for cross-module operations)
    // =============================================================================

    /// Validate connection for channel operations
    pub fn validate_connection_for_channel(&self, connection_id: String) -> bool {
        self.assert_authorized_caller();
        
        if let Some(connection) = self.connection_module.get_connection(connection_id) {
            matches!(connection.state, ConnectionState::Open)
        } else {
            false
        }
    }

    /// Validate connection exists and is in expected state
    pub fn validate_connection_state(&self, connection_id: String, expected_state: String) -> bool {
        self.assert_authorized_caller();
        
        if let Some(connection) = self.connection_module.get_connection(connection_id) {
            let expected = match expected_state.as_str() {
                "UNINITIALIZED" => ConnectionState::Uninitialized,
                "INIT" => ConnectionState::Init,
                "TRYOPEN" => ConnectionState::TryOpen,
                "OPEN" => ConnectionState::Open,
                _ => return false,
            };
            connection.state == expected
        } else {
            false
        }
    }

    /// Get connection version (for channel creation)
    pub fn get_connection_version(&self, connection_id: String) -> Option<Version> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
            .and_then(|conn| conn.versions.first().cloned())
    }

    /// Get connection delay period
    pub fn get_connection_delay_period(&self, connection_id: String) -> Option<u64> {
        self.assert_authorized_caller();
        self.connection_module.get_connection(connection_id)
            .map(|conn| conn.delay_period)
    }

    // =============================================================================
    // Admin and Configuration Functions
    // =============================================================================

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

    /// Health check for the connection module
    pub fn health_check(&self) -> bool {
        // Check if the connection module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "ibc_connection",
            "version": "1.0.0",
            "description": "IBC Connection Module",
            "functions": [
                "connection_open_init",
                "connection_open_try",
                "connection_open_ack", 
                "connection_open_confirm",
                "get_connection",
                "get_all_connections",
                "get_connections_for_client",
                "connection_exists",
                "get_connection_state",
                "get_connection_client",
                "get_connection_counterparty",
                "validate_connection_for_channel",
                "validate_connection_state",
                "get_connection_version",
                "get_connection_delay_period"
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
    fn test_connection_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcConnectionContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_authorized_caller_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcConnectionContract::new(accounts(1), Some(accounts(2)));
        
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
        
        let contract = IbcConnectionContract::new(accounts(1), Some(accounts(2)));
        contract.assert_authorized_caller();
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcConnectionContract::new(accounts(1), None);
        assert!(contract.health_check());
    }

    #[test]
    fn test_connection_state_validation() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcConnectionContract::new(accounts(1), None);
        
        // Test with non-existent connection
        let result = contract.validate_connection_state("connection-0".to_string(), "OPEN".to_string());
        assert!(!result);
    }
}