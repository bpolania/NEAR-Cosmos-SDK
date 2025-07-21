use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::env;

pub mod types;

pub use types::{ConnectionEnd, Counterparty, Version, State};

/// IBC Connection Module
/// 
/// This module implements the ICS-03 Connection specification for establishing
/// authenticated connections between two chains via IBC.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ConnectionModule {
    /// Mapping from connection_id to ConnectionEnd
    connections: LookupMap<String, ConnectionEnd>,
    
    /// Counter for generating unique connection IDs
    next_connection_sequence: u64,
}

impl ConnectionModule {
    /// Initialize the IBC Connection module
    pub fn new() -> Self {
        Self {
            connections: LookupMap::new(b"n"),
            next_connection_sequence: 0,
        }
    }

    /// Initiate a connection handshake (ConnOpenInit)
    /// 
    /// This function creates a new connection in INIT state and assigns it a unique ID.
    /// 
    /// # Arguments
    /// * `client_id` - The client ID for this chain
    /// * `counterparty` - Information about the counterparty chain
    /// * `version` - The connection version to use
    /// * `delay_period` - The delay period for this connection
    /// 
    /// # Returns
    /// * The generated connection ID
    pub fn conn_open_init(
        &mut self,
        client_id: String,
        counterparty: Counterparty,
        version: Option<Version>,
        delay_period: u64,
    ) -> String {
        // Generate unique connection ID
        let connection_id = format!("connection-{}", self.next_connection_sequence);
        self.next_connection_sequence += 1;

        // Use default version if none provided
        let conn_version = version.unwrap_or_else(|| Version::default());

        // Create connection end in INIT state
        let connection_end = ConnectionEnd {
            state: State::Init,
            client_id,
            counterparty,
            versions: vec![conn_version],
            delay_period,
        };

        // Store the connection
        self.connections.insert(&connection_id, &connection_end);

        env::log_str(&format!(
            "Connection: Initiated connection {} in INIT state",
            connection_id
        ));

        connection_id
    }

    /// Respond to a connection handshake (ConnOpenTry)
    /// 
    /// This function creates a new connection in TRYOPEN state in response to
    /// a ConnOpenInit from the counterparty.
    /// 
    /// # Arguments
    /// * `previous_connection_id` - The connection ID (if retrying)
    /// * `counterparty` - Information about the counterparty chain
    /// * `delay_period` - The delay period for this connection
    /// * `client_id` - The client ID for this chain
    /// * `client_state_proof` - Proof of the client state on counterparty
    /// * `consensus_state_proof` - Proof of the consensus state on counterparty
    /// * `connection_proof` - Proof of the connection on counterparty
    /// * `proof_height` - The height at which proofs were generated
    /// * `version` - The connection version
    /// 
    /// # Returns
    /// * The connection ID
    pub fn conn_open_try(
        &mut self,
        previous_connection_id: Option<String>,
        counterparty: Counterparty,
        delay_period: u64,
        client_id: String,
        _client_state_proof: Vec<u8>,
        _consensus_state_proof: Vec<u8>,
        _connection_proof: Vec<u8>,
        _proof_height: u64,
        version: Version,
    ) -> Result<String, String> {
        // TODO: Verify proofs using the light client module
        // For now, we'll implement basic validation

        let connection_id = if let Some(conn_id) = previous_connection_id {
            // Reuse existing connection ID
            conn_id
        } else {
            // Generate new connection ID
            let conn_id = format!("connection-{}", self.next_connection_sequence);
            self.next_connection_sequence += 1;
            conn_id
        };

        // Create connection end in TRYOPEN state
        let connection_end = ConnectionEnd {
            state: State::TryOpen,
            client_id,
            counterparty,
            versions: vec![version],
            delay_period,
        };

        // Store the connection
        self.connections.insert(&connection_id, &connection_end);

        env::log_str(&format!(
            "Connection: Created connection {} in TRYOPEN state",
            connection_id
        ));

        Ok(connection_id)
    }

    /// Acknowledge a connection handshake (ConnOpenAck)
    /// 
    /// This function moves a connection from INIT to OPEN state upon receiving
    /// acknowledgment from the counterparty.
    /// 
    /// # Arguments
    /// * `connection_id` - The connection ID to acknowledge
    /// * `counterparty_connection_id` - The connection ID on the counterparty
    /// * `version` - The agreed connection version
    /// * `client_state_proof` - Proof of client state on counterparty
    /// * `connection_proof` - Proof of connection on counterparty
    /// * `consensus_state_proof` - Proof of consensus state on counterparty
    /// * `proof_height` - The height at which proofs were generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn conn_open_ack(
        &mut self,
        connection_id: String,
        counterparty_connection_id: String,
        version: Version,
        _client_state_proof: Vec<u8>,
        _connection_proof: Vec<u8>,
        _consensus_state_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the existing connection
        let mut connection = self.connections.get(&connection_id)
            .ok_or("Connection not found")?;

        // Verify connection is in INIT state
        if connection.state != State::Init {
            return Err("Connection not in INIT state".to_string());
        }

        // TODO: Verify proofs using the light client module
        // For now, we'll implement basic validation

        // Update connection to OPEN state
        connection.state = State::Open;
        connection.counterparty.connection_id = Some(counterparty_connection_id.clone());
        connection.versions = vec![version];

        // Store updated connection
        self.connections.insert(&connection_id, &connection);

        env::log_str(&format!(
            "Connection: Acknowledged connection {} - now OPEN with counterparty {}",
            connection_id, counterparty_connection_id
        ));

        Ok(())
    }

    /// Confirm a connection handshake (ConnOpenConfirm)
    /// 
    /// This function moves a connection from TRYOPEN to OPEN state upon
    /// confirmation from the counterparty.
    /// 
    /// # Arguments
    /// * `connection_id` - The connection ID to confirm
    /// * `connection_proof` - Proof of connection on counterparty
    /// * `proof_height` - The height at which proof was generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn conn_open_confirm(
        &mut self,
        connection_id: String,
        _connection_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the existing connection
        let mut connection = self.connections.get(&connection_id)
            .ok_or("Connection not found")?;

        // Verify connection is in TRYOPEN state
        if connection.state != State::TryOpen {
            return Err("Connection not in TRYOPEN state".to_string());
        }

        // TODO: Verify proof using the light client module
        // For now, we'll implement basic validation

        // Update connection to OPEN state
        connection.state = State::Open;

        // Store updated connection
        self.connections.insert(&connection_id, &connection);

        env::log_str(&format!(
            "Connection: Confirmed connection {} - now OPEN",
            connection_id
        ));

        Ok(())
    }

    /// Get a connection by ID
    /// 
    /// # Arguments
    /// * `connection_id` - The connection ID to query
    /// 
    /// # Returns
    /// * The ConnectionEnd if it exists
    pub fn get_connection(&self, connection_id: String) -> Option<ConnectionEnd> {
        self.connections.get(&connection_id)
    }

    /// Get all connection IDs
    /// 
    /// # Returns
    /// * Vector of all connection IDs
    pub fn get_connection_ids(&self) -> Vec<String> {
        // LookupMap doesn't have a keys() method, so we'll need to track connection IDs separately
        // For now, return an empty vector - this would need a separate storage for connection IDs
        vec![]
    }

    /// Check if a connection exists and is in OPEN state
    /// 
    /// # Arguments
    /// * `connection_id` - The connection ID to check
    /// 
    /// # Returns
    /// * True if connection exists and is open
    pub fn is_connection_open(&self, connection_id: &str) -> bool {
        self.connections.get(&connection_id.to_string())
            .map(|conn| conn.state == State::Open)
            .unwrap_or(false)
    }
}