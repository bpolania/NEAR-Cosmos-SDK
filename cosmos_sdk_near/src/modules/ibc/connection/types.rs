use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Connection state enumeration following ICS-03 specification
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum State {
    /// Uninitialized connection
    Uninitialized,
    /// Connection initialized (ConnOpenInit called)
    Init,
    /// Connection in try open state (ConnOpenTry called)
    TryOpen,
    /// Connection established and open
    Open,
}

/// Connection version information
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Version {
    /// Version identifier string
    pub identifier: String,
    /// Supported features for this version
    pub features: Vec<String>,
}

impl Version {
    /// Create a new version
    pub fn new(identifier: String, features: Vec<String>) -> Self {
        Self {
            identifier,
            features,
        }
    }
}

impl Default for Version {
    /// Default IBC connection version
    fn default() -> Self {
        Self {
            identifier: "1".to_string(),
            features: vec!["ORDER_ORDERED".to_string(), "ORDER_UNORDERED".to_string()],
        }
    }
}

/// Counterparty connection information
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Counterparty {
    /// Client ID on the counterparty chain
    pub client_id: String,
    /// Connection ID on the counterparty chain (if established)
    pub connection_id: Option<String>,
    /// Commitment prefix used by the counterparty chain
    pub prefix: MerklePrefix,
}

impl Counterparty {
    /// Create a new counterparty
    pub fn new(client_id: String, connection_id: Option<String>, prefix: MerklePrefix) -> Self {
        Self {
            client_id,
            connection_id,
            prefix,
        }
    }
}

/// Merkle prefix for commitment proofs
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MerklePrefix {
    /// The key prefix used for commitment proofs
    pub key_prefix: Vec<u8>,
}

impl MerklePrefix {
    /// Create a new merkle prefix
    pub fn new(key_prefix: Vec<u8>) -> Self {
        Self { key_prefix }
    }
}

impl Default for MerklePrefix {
    /// Default IBC commitment prefix
    fn default() -> Self {
        Self {
            key_prefix: b"ibc".to_vec(),
        }
    }
}

/// Connection end data structure following ICS-03
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectionEnd {
    /// Current state of the connection
    pub state: State,
    /// Client ID for this connection
    pub client_id: String,
    /// Counterparty connection information
    pub counterparty: Counterparty,
    /// Supported/negotiated versions
    pub versions: Vec<Version>,
    /// Delay period for this connection
    pub delay_period: u64,
}

impl ConnectionEnd {
    /// Create a new connection end
    pub fn new(
        state: State,
        client_id: String,
        counterparty: Counterparty,
        versions: Vec<Version>,
        delay_period: u64,
    ) -> Self {
        Self {
            state,
            client_id,
            counterparty,
            versions,
            delay_period,
        }
    }

    /// Check if connection is open
    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }

    /// Get the counterparty client ID
    pub fn counterparty_client_id(&self) -> &str {
        &self.counterparty.client_id
    }

    /// Get the counterparty connection ID if available
    pub fn counterparty_connection_id(&self) -> Option<&str> {
        self.counterparty.connection_id.as_deref()
    }
}

/// Connection handshake messages and proofs
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionProofs {
    /// Proof of the client state on the counterparty
    pub client_state_proof: Vec<u8>,
    /// Proof of the consensus state on the counterparty
    pub consensus_state_proof: Vec<u8>,
    /// Proof of the connection on the counterparty
    pub connection_proof: Vec<u8>,
    /// Height at which the proofs were generated
    pub proof_height: u64,
}

/// Connection identifiers used throughout IBC
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectionId {
    /// The connection identifier string
    pub id: String,
}

impl ConnectionId {
    /// Create a new connection ID
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for ConnectionId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for ConnectionId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_default() {
        let version = Version::default();
        assert_eq!(version.identifier, "1");
        assert!(version.features.contains(&"ORDER_ORDERED".to_string()));
        assert!(version.features.contains(&"ORDER_UNORDERED".to_string()));
    }

    #[test]
    fn test_merkle_prefix_default() {
        let prefix = MerklePrefix::default();
        assert_eq!(prefix.key_prefix, b"ibc".to_vec());
    }

    #[test]
    fn test_connection_end_is_open() {
        let counterparty = Counterparty::new(
            "client-1".to_string(),
            None,
            MerklePrefix::default(),
        );

        let mut connection = ConnectionEnd::new(
            State::Init,
            "client-0".to_string(),
            counterparty,
            vec![Version::default()],
            0,
        );

        assert!(!connection.is_open());

        connection.state = State::Open;
        assert!(connection.is_open());
    }

    #[test]
    fn test_connection_id_conversion() {
        let conn_id = ConnectionId::from("connection-0");
        assert_eq!(conn_id.id, "connection-0");

        let conn_id_string = ConnectionId::from("connection-1".to_string());
        assert_eq!(conn_id_string.id, "connection-1");
    }
}