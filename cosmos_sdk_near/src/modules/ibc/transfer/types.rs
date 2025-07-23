use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use crate::Balance;
use schemars::JsonSchema;
use sha2::{Digest, Sha256};

/// Error types for ICS-20 fungible token transfers
#[derive(Debug, Clone, PartialEq)]
pub enum TransferError {
    /// Insufficient funds to complete transfer
    InsufficientFunds,
    /// Insufficient escrowed tokens
    InsufficientEscrow,
    /// Insufficient voucher supply
    InsufficientVoucherSupply,
    /// Invalid sender address format
    InvalidSender,
    /// Invalid receiver address format
    InvalidReceiver,
    /// Invalid denomination format
    InvalidDenomination,
    /// Channel not found or not open
    ChannelNotOpen,
    /// Packet timeout error
    PacketTimeout,
    /// Invalid amount (zero or negative)
    InvalidAmount,
    /// Token not found
    TokenNotFound,
    /// Denomination trace not found
    DenomTraceNotFound,
    /// Invalid trace path format
    InvalidTracePath,
}

/// Fungible Token Packet Data as defined by ICS-20
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FungibleTokenPacketData {
    /// Token denomination on the sending chain
    pub denom: String,
    /// Amount to transfer (as string to maintain precision)
    pub amount: String,
    /// Sender address on the sending chain
    pub sender: String,
    /// Receiver address on the destination chain
    pub receiver: String,
    /// Optional memo field for additional data
    pub memo: String,
}

impl FungibleTokenPacketData {
    /// Create a new fungible token packet data
    pub fn new(
        denom: String,
        amount: String,
        sender: String,
        receiver: String,
        memo: Option<String>,
    ) -> Self {
        Self {
            denom,
            amount,
            sender,
            receiver,
            memo: memo.unwrap_or_default(),
        }
    }

    /// Convert to JSON bytes for packet transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>, TransferError> {
        serde_json::to_vec(self).map_err(|_| TransferError::InvalidDenomination)
    }

    /// Parse from JSON bytes received in packet
    pub fn from_bytes(data: &[u8]) -> Result<Self, TransferError> {
        serde_json::from_slice(data).map_err(|_| TransferError::InvalidDenomination)
    }

    /// Validate the packet data fields
    pub fn validate(&self) -> Result<(), TransferError> {
        if self.denom.is_empty() {
            return Err(TransferError::InvalidDenomination);
        }
        
        if self.amount.is_empty() || self.amount == "0" {
            return Err(TransferError::InvalidAmount);
        }
        
        if self.sender.is_empty() {
            return Err(TransferError::InvalidSender);
        }
        
        if self.receiver.is_empty() {
            return Err(TransferError::InvalidReceiver);
        }

        // Parse amount to validate it's a valid number
        self.amount.parse::<u128>().map_err(|_| TransferError::InvalidAmount)?;
        
        Ok(())
    }

    /// Get amount as Balance type
    pub fn amount_as_balance(&self) -> Result<Balance, TransferError> {
        self.amount.parse::<Balance>().map_err(|_| TransferError::InvalidAmount)
    }
}

/// Denomination Trace for tracking token path across chains
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DenomTrace {
    /// The trace path showing the chain of port/channel pairs
    /// Format: "port1/channel1/port2/channel2/.../base_denom"
    pub path: String,
    /// The base denomination (original token name)
    pub base_denom: String,
}

impl DenomTrace {
    /// Create a new denomination trace
    pub fn new(path: String, base_denom: String) -> Self {
        Self { path, base_denom }
    }

    /// Create from a full trace path
    pub fn from_path(full_path: &str) -> Result<Self, TransferError> {
        if full_path.is_empty() {
            return Err(TransferError::InvalidTracePath);
        }

        // Split the path by '/' and take the last part as base denomination
        let parts: Vec<&str> = full_path.split('/').collect();
        if parts.is_empty() {
            return Err(TransferError::InvalidTracePath);
        }

        let base_denom = parts.last().unwrap().to_string();
        
        if parts.len() == 1 {
            // Native token, no path
            Ok(Self::new(String::new(), base_denom))
        } else {
            // Remove base denom from path
            let path_parts = &parts[..parts.len() - 1];
            let path = path_parts.join("/");
            Ok(Self::new(path, base_denom))
        }
    }

    /// Get the full trace path (path + base_denom)
    pub fn get_full_path(&self) -> String {
        if self.path.is_empty() {
            self.base_denom.clone()
        } else {
            format!("{}/{}", self.path, self.base_denom)
        }
    }

    /// Generate hash of the denomination trace for IBC denomination
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.get_full_path().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if this is a native token (no path)
    pub fn is_native(&self) -> bool {
        self.path.is_empty()
    }

    /// Add a hop to the trace path (when sending to another chain)
    pub fn add_hop(&self, port_id: &str, channel_id: &str) -> Self {
        let new_path = if self.path.is_empty() {
            format!("{}/{}", port_id, channel_id)
        } else {
            format!("{}/{}/{}", port_id, channel_id, self.path)
        };
        
        Self::new(new_path, self.base_denom.clone())
    }

    /// Remove a hop from the trace path (when returning to previous chain)
    pub fn remove_hop(&self) -> Result<Self, TransferError> {
        if self.path.is_empty() {
            return Err(TransferError::InvalidTracePath);
        }

        let parts: Vec<&str> = self.path.split('/').collect();
        if parts.len() < 2 {
            // Remove the only hop, becomes native
            return Ok(Self::new(String::new(), self.base_denom.clone()));
        }

        // Remove first two elements (port/channel)
        let new_path = parts[2..].join("/");
        Ok(Self::new(new_path, self.base_denom.clone()))
    }
}

/// Token escrow information for tracking locked tokens
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TokenEscrow {
    /// Port ID where tokens are escrowed
    pub port_id: String,
    /// Channel ID where tokens are escrowed
    pub channel_id: String,
    /// Token denomination
    pub denom: String,
    /// Escrowed amount
    pub amount: Balance,
    /// Account that escrowed the tokens (as string for JSON schema compatibility)
    pub escrow_account: String,
}


/// Acknowledgement for ICS-20 fungible token transfers
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FungibleTokenPacketAcknowledgement {
    /// Transfer was successful
    Success(Vec<u8>),
    /// Transfer failed with error message
    Error(String),
}

impl FungibleTokenPacketAcknowledgement {
    /// Create a success acknowledgement
    pub fn success() -> Self {
        Self::Success(b"AQ==".to_vec()) // Base64 encoded "success"
    }


    /// Convert to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Success(data) => data.clone(),
            Self::Error(msg) => msg.as_bytes().to_vec(),
        }
    }

}

/// Transfer packet timeout information
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TransferTimeout {
    /// Timeout height (block height)
    pub height: Option<u64>,
    /// Timeout timestamp (nanoseconds since Unix epoch)
    pub timestamp: Option<u64>,
}


/// Transfer request parameters
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TransferRequest {
    /// Source port (usually "transfer")
    pub source_port: String,
    /// Source channel ID
    pub source_channel: String,
    /// Token to transfer
    pub token: String,
    /// Amount to transfer
    pub amount: Balance,
    /// Receiver address on destination chain
    pub receiver: String,
    /// Transfer timeout
    pub timeout: TransferTimeout,
    /// Optional memo
    pub memo: Option<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fungible_token_packet_data() {
        let packet_data = FungibleTokenPacketData::new(
            "unear".to_string(),
            "1000000".to_string(),
            "alice.near".to_string(),
            "cosmos1abc".to_string(),
            Some("test transfer".to_string()),
        );

        assert_eq!(packet_data.denom, "unear");
        assert_eq!(packet_data.amount_as_balance().unwrap(), 1000000);
        assert!(packet_data.validate().is_ok());
    }

    #[test]
    fn test_denom_trace() {
        // Test native token
        let native_trace = DenomTrace::new(String::new(), "unear".to_string());
        assert!(native_trace.is_native());
        assert_eq!(native_trace.get_full_path(), "unear");

        // Test token with path
        let trace = DenomTrace::from_path("transfer/channel-0/uatom").unwrap();
        assert!(!trace.is_native());
        assert_eq!(trace.base_denom, "uatom");
        assert_eq!(trace.path, "transfer/channel-0");

        // Test adding hop
        let new_trace = trace.add_hop("transfer", "channel-1");
        assert_eq!(new_trace.get_full_path(), "transfer/channel-1/transfer/channel-0/uatom");
    }

    #[test]
    fn test_denom_trace_hash() {
        let trace = DenomTrace::new("transfer/channel-0".to_string(), "uatom".to_string());
        let hash = trace.hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_acknowledgement() {
        let success_ack = FungibleTokenPacketAcknowledgement::success();
        let error_ack = FungibleTokenPacketAcknowledgement::Error("insufficient funds".to_string());

        // Test serialization
        let success_bytes = success_ack.to_bytes();
        let error_bytes = error_ack.to_bytes();

        assert!(!success_bytes.is_empty());
        assert!(!error_bytes.is_empty());
        
        // Test enum variants
        assert!(matches!(success_ack, FungibleTokenPacketAcknowledgement::Success(_)));
        assert!(matches!(error_ack, FungibleTokenPacketAcknowledgement::Error(_)));
    }


    #[test]
    fn test_transfer_request_structure() {
        // Test creating a transfer request structure manually
        let request = TransferRequest {
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            token: "unear".to_string(),
            amount: 1000000,
            receiver: "cosmos1abc".to_string(),
            timeout: TransferTimeout {
                height: Some(1000),
                timestamp: None,
            },
            memo: None,
        };

        assert_eq!(request.source_port, "transfer");
        assert_eq!(request.token, "unear");
        assert_eq!(request.amount, 1000000);
    }
}