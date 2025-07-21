use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Channel state enumeration following ICS-04 specification
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum State {
    /// Uninitialized channel
    Uninitialized,
    /// Channel initialized (ChanOpenInit called)
    Init,
    /// Channel in try open state (ChanOpenTry called)
    TryOpen,
    /// Channel established and open
    Open,
    /// Channel closed
    Closed,
}

/// Channel ordering enumeration
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Order {
    /// No ordering - packets can arrive in any order
    Unordered,
    /// Ordered - packets must arrive in order
    Ordered,
}

/// Channel end data structure following ICS-04
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChannelEnd {
    /// Current state of the channel
    pub state: State,
    /// Ordering of packets in this channel
    pub ordering: Order,
    /// Counterparty channel information
    pub counterparty: Counterparty,
    /// Connection ID that this channel uses
    pub connection_hops: Vec<String>,
    /// Channel version for application-specific data
    pub version: String,
}

impl ChannelEnd {
    /// Create a new channel end
    pub fn new(
        state: State,
        ordering: Order,
        counterparty: Counterparty,
        connection_hops: Vec<String>,
        version: String,
    ) -> Self {
        Self {
            state,
            ordering,
            counterparty,
            connection_hops,
            version,
        }
    }

    /// Check if channel is open
    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }

    /// Get the counterparty port ID
    pub fn counterparty_port_id(&self) -> &str {
        &self.counterparty.port_id
    }

    /// Get the counterparty channel ID if available
    pub fn counterparty_channel_id(&self) -> Option<&str> {
        self.counterparty.channel_id.as_deref()
    }
}

/// Counterparty channel information
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Counterparty {
    /// Port ID on the counterparty chain
    pub port_id: String,
    /// Channel ID on the counterparty chain (if established)
    pub channel_id: Option<String>,
}

impl Counterparty {
    /// Create a new counterparty
    pub fn new(port_id: String, channel_id: Option<String>) -> Self {
        Self {
            port_id,
            channel_id,
        }
    }
}

/// IBC packet for cross-chain communication
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Packet {
    /// Sequence number of the packet
    pub sequence: u64,
    /// Source port identifier
    pub source_port: String,
    /// Source channel identifier
    pub source_channel: String,
    /// Destination port identifier
    pub destination_port: String,
    /// Destination channel identifier
    pub destination_channel: String,
    /// Packet data payload
    pub data: Vec<u8>,
    /// Timeout height for the packet
    pub timeout_height: Height,
    /// Timeout timestamp for the packet
    pub timeout_timestamp: u64,
}

impl Packet {
    /// Create a new packet
    pub fn new(
        sequence: u64,
        source_port: String,
        source_channel: String,
        destination_port: String,
        destination_channel: String,
        data: Vec<u8>,
        timeout_height: Height,
        timeout_timestamp: u64,
    ) -> Self {
        Self {
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data,
            timeout_height,
            timeout_timestamp,
        }
    }

    /// Check if packet has timed out based on height
    pub fn is_timed_out_on_height(&self, current_height: &Height) -> bool {
        if self.timeout_height.revision_number == 0 && self.timeout_height.revision_height == 0 {
            return false; // No height timeout
        }
        current_height >= &self.timeout_height
    }

    /// Check if packet has timed out based on timestamp
    pub fn is_timed_out_on_timestamp(&self, current_timestamp: u64) -> bool {
        if self.timeout_timestamp == 0 {
            return false; // No timestamp timeout
        }
        current_timestamp >= self.timeout_timestamp
    }
}

/// Height represents a monotonically increasing height
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Height {
    /// Revision number (for chain upgrades)
    pub revision_number: u64,
    /// Revision height (block height within revision)
    pub revision_height: u64,
}

impl Height {
    /// Create a new height
    pub fn new(revision_number: u64, revision_height: u64) -> Self {
        Self {
            revision_number,
            revision_height,
        }
    }

    /// Check if height is zero
    pub fn is_zero(&self) -> bool {
        self.revision_number == 0 && self.revision_height == 0
    }
}

/// Packet acknowledgment
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Acknowledgement {
    /// Acknowledgment data - can be success or error
    pub data: Vec<u8>,
}

impl Acknowledgement {
    /// Create a new acknowledgment
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create a success acknowledgment
    pub fn success(result: Vec<u8>) -> Self {
        Self { data: result }
    }

    /// Create an error acknowledgment
    pub fn error(error: String) -> Self {
        Self {
            data: error.into_bytes(),
        }
    }

    /// Check if acknowledgment represents success
    pub fn is_success(&self) -> bool {
        // Simple heuristic - in practice, this would depend on the application protocol
        !self.data.is_empty() && !self.data.starts_with(b"error")
    }
}

/// Channel identifier
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChannelId {
    /// The channel identifier string
    pub id: String,
}

impl ChannelId {
    /// Create a new channel ID
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for ChannelId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for ChannelId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

/// Port identifier
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PortId {
    /// The port identifier string
    pub id: String,
}

impl PortId {
    /// Create a new port ID
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for PortId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for PortId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

/// Packet commitment for proving packet transmission
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PacketCommitment {
    /// Hash of the packet data
    pub data: Vec<u8>,
}

impl PacketCommitment {
    /// Create a new packet commitment
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create commitment from packet
    pub fn from_packet(packet: &Packet) -> Self {
        use sha2::{Digest, Sha256};
        
        // Commit to the packet fields
        let mut hasher = Sha256::new();
        hasher.update(&packet.sequence.to_be_bytes());
        hasher.update(packet.source_port.as_bytes());
        hasher.update(packet.source_channel.as_bytes());
        hasher.update(packet.destination_port.as_bytes());
        hasher.update(packet.destination_channel.as_bytes());
        hasher.update(&packet.data);
        hasher.update(&packet.timeout_height.revision_number.to_be_bytes());
        hasher.update(&packet.timeout_height.revision_height.to_be_bytes());
        hasher.update(&packet.timeout_timestamp.to_be_bytes());
        
        Self {
            data: hasher.finalize().to_vec(),
        }
    }
}

/// Packet receipt for proving packet reception
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PacketReceipt {
    /// Sequence number of the received packet
    pub sequence: u64,
}

impl PacketReceipt {
    /// Create a new packet receipt
    pub fn new(sequence: u64) -> Self {
        Self { sequence }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_end_creation() {
        let counterparty = Counterparty::new("transfer".to_string(), None);
        let channel = ChannelEnd::new(
            State::Init,
            Order::Unordered,
            counterparty,
            vec!["connection-0".to_string()],
            "ics20-1".to_string(),
        );

        assert_eq!(channel.state, State::Init);
        assert_eq!(channel.ordering, Order::Unordered);
        assert!(!channel.is_open());
    }

    #[test]
    fn test_packet_timeout() {
        let packet = Packet::new(
            1,
            "transfer".to_string(),
            "channel-0".to_string(),
            "transfer".to_string(),
            "channel-1".to_string(),
            b"test data".to_vec(),
            Height::new(1, 100),
            1000,
        );

        assert!(packet.is_timed_out_on_height(&Height::new(1, 101)));
        assert!(!packet.is_timed_out_on_height(&Height::new(1, 99)));
        assert!(packet.is_timed_out_on_timestamp(1001));
        assert!(!packet.is_timed_out_on_timestamp(999));
    }

    #[test]
    fn test_acknowledgement() {
        let success_ack = Acknowledgement::success(b"success".to_vec());
        let error_ack = Acknowledgement::error("error occurred".to_string());

        assert!(success_ack.is_success());
        assert!(!error_ack.is_success());
    }

    #[test]
    fn test_packet_commitment() {
        let packet = Packet::new(
            1,
            "transfer".to_string(),
            "channel-0".to_string(),
            "transfer".to_string(),
            "channel-1".to_string(),
            b"test data".to_vec(),
            Height::new(1, 100),
            1000,
        );

        let commitment = PacketCommitment::from_packet(&packet);
        assert!(!commitment.data.is_empty());
        assert_eq!(commitment.data.len(), 32); // SHA256 hash length
    }

    #[test]
    fn test_height_ordering() {
        let height1 = Height::new(1, 100);
        let height2 = Height::new(1, 101);
        let height3 = Height::new(2, 50);

        assert!(height1 < height2);
        assert!(height2 < height3);
        assert!(!height1.is_zero());
        assert!(Height::new(0, 0).is_zero());
    }
}