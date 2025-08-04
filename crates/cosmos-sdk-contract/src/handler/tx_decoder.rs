use crate::types::cosmos_tx::{CosmosTx, TxBody, AuthInfo, Any, TxValidationError};
use near_sdk::serde::{Deserialize, Serialize};

/// Transaction decoding errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TxDecodingError {
    /// Invalid transaction format
    InvalidFormat(String),
    /// JSON deserialization error
    JsonError(String),
    /// Missing required field
    MissingField(String),
    /// Invalid transaction structure
    InvalidStructure(String),
    /// Validation error
    ValidationError(TxValidationError),
}

impl std::fmt::Display for TxDecodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxDecodingError::InvalidFormat(msg) => write!(f, "Invalid transaction format: {}", msg),
            TxDecodingError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            TxDecodingError::MissingField(field) => write!(f, "Missing required field: {}", field),
            TxDecodingError::InvalidStructure(msg) => write!(f, "Invalid transaction structure: {}", msg),
            TxDecodingError::ValidationError(err) => write!(f, "Validation error: {}", err),
        }
    }
}

impl std::error::Error for TxDecodingError {}

impl From<TxValidationError> for TxDecodingError {
    fn from(err: TxValidationError) -> Self {
        TxDecodingError::ValidationError(err)
    }
}

/// Configuration for the transaction decoder
#[derive(Clone, Debug)]
pub struct TxDecoderConfig {
    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
    /// Maximum number of messages per transaction
    pub max_messages: usize,
    /// Maximum gas limit per transaction
    pub max_gas_limit: u64,
    /// Supported message types (type URLs)
    pub supported_message_types: Vec<String>,
}

impl Default for TxDecoderConfig {
    fn default() -> Self {
        Self {
            max_tx_size: 1024 * 1024, // 1MB
            max_messages: 100,
            max_gas_limit: 10_000_000, // 10M gas
            supported_message_types: vec![
                // Bank module
                "/cosmos.bank.v1beta1.MsgSend".to_string(),
                "/cosmos.bank.v1beta1.MsgMultiSend".to_string(),
                "/cosmos.bank.v1beta1.MsgBurn".to_string(),
                
                // Staking module
                "/cosmos.staking.v1beta1.MsgDelegate".to_string(),
                "/cosmos.staking.v1beta1.MsgUndelegate".to_string(),
                "/cosmos.staking.v1beta1.MsgBeginRedelegate".to_string(),
                "/cosmos.staking.v1beta1.MsgCreateValidator".to_string(),
                "/cosmos.staking.v1beta1.MsgEditValidator".to_string(),
                
                // Governance module
                "/cosmos.gov.v1beta1.MsgSubmitProposal".to_string(),
                "/cosmos.gov.v1beta1.MsgVote".to_string(),
                "/cosmos.gov.v1beta1.MsgVoteWeighted".to_string(),
                "/cosmos.gov.v1beta1.MsgDeposit".to_string(),
                
                // IBC module
                "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                "/ibc.core.channel.v1.MsgChannelOpenInit".to_string(),
                "/ibc.core.channel.v1.MsgChannelOpenTry".to_string(),
                "/ibc.core.channel.v1.MsgRecvPacket".to_string(),
                "/ibc.core.channel.v1.MsgAcknowledgement".to_string(),
                "/ibc.core.channel.v1.MsgTimeout".to_string(),
            ],
        }
    }
}

/// Transaction decoder for processing raw Cosmos SDK transactions
pub struct TxDecoder {
    config: TxDecoderConfig,
}

impl TxDecoder {
    /// Create a new transaction decoder with default configuration
    pub fn new() -> Self {
        Self {
            config: TxDecoderConfig::default(),
        }
    }

    /// Create a new transaction decoder with custom configuration
    pub fn with_config(config: TxDecoderConfig) -> Self {
        Self { config }
    }

    /// Decode a raw transaction from bytes
    /// Initially uses JSON format, can be extended to support protobuf
    pub fn decode_cosmos_tx(&self, raw_tx: Vec<u8>) -> Result<CosmosTx, TxDecodingError> {
        // Check transaction size limits
        if raw_tx.len() > self.config.max_tx_size {
            return Err(TxDecodingError::InvalidFormat(
                format!("Transaction size {} exceeds maximum {}", raw_tx.len(), self.config.max_tx_size)
            ));
        }

        // Decode from JSON (protobuf-compatible format)
        let tx: CosmosTx = self.decode_from_json(&raw_tx)?;

        // Validate the decoded transaction
        self.validate_tx_structure(&tx)?;

        Ok(tx)
    }

    /// Decode transaction from JSON format
    fn decode_from_json(&self, data: &[u8]) -> Result<CosmosTx, TxDecodingError> {
        serde_json::from_slice(data)
            .map_err(|e| TxDecodingError::JsonError(e.to_string()))
    }

    /// Validate the transaction structure according to Cosmos SDK rules
    pub fn validate_tx_structure(&self, tx: &CosmosTx) -> Result<(), TxDecodingError> {
        // Basic transaction validation
        tx.validate()?;

        // Check message count limits
        if tx.body.messages.len() > self.config.max_messages {
            return Err(TxDecodingError::InvalidStructure(
                format!("Transaction has {} messages, maximum allowed is {}", 
                    tx.body.messages.len(), self.config.max_messages)
            ));
        }

        // Check gas limits
        if tx.auth_info.fee.gas_limit > self.config.max_gas_limit {
            return Err(TxDecodingError::InvalidStructure(
                format!("Gas limit {} exceeds maximum {}", 
                    tx.auth_info.fee.gas_limit, self.config.max_gas_limit)
            ));
        }

        // Validate all message types are supported
        for msg in &tx.body.messages {
            if !self.is_message_type_supported(&msg.type_url) {
                return Err(TxDecodingError::InvalidStructure(
                    format!("Unsupported message type: {}", msg.type_url)
                ));
            }
        }

        // Validate signatures match signers
        if tx.signatures.len() != tx.auth_info.signer_infos.len() {
            return Err(TxDecodingError::InvalidStructure(
                format!("Signature count mismatch: {} signatures for {} signers",
                    tx.signatures.len(), tx.auth_info.signer_infos.len())
            ));
        }

        // Additional validation for timeout height
        if tx.body.timeout_height != 0 {
            // In a real implementation, we would check against current block height
            // For now, just ensure it's reasonable
            if tx.body.timeout_height > 999_999_999 {
                return Err(TxDecodingError::InvalidStructure(
                    "Timeout height too far in the future".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Extract messages from the transaction body
    pub fn extract_messages(&self, tx: &CosmosTx) -> Result<Vec<DecodedMessage>, TxDecodingError> {
        let mut decoded_messages = Vec::new();

        for (index, msg) in tx.body.messages.iter().enumerate() {
            let decoded = self.decode_message(msg, index)?;
            decoded_messages.push(decoded);
        }

        Ok(decoded_messages)
    }

    /// Decode a single message from Any type
    fn decode_message(&self, msg: &Any, index: usize) -> Result<DecodedMessage, TxDecodingError> {
        // Validate message type is supported
        if !self.is_message_type_supported(&msg.type_url) {
            return Err(TxDecodingError::InvalidStructure(
                format!("Unsupported message type at index {}: {}", index, msg.type_url)
            ));
        }

        // For now, we keep the message as-is and let the message router handle decoding
        // In the future, we could add specific decoding logic for each message type
        Ok(DecodedMessage {
            index,
            type_url: msg.type_url.clone(),
            raw_data: msg.value.clone(),
            decoded_data: None, // Will be decoded by message router
        })
    }

    /// Check if a message type is supported
    pub fn is_message_type_supported(&self, type_url: &str) -> bool {
        self.config.supported_message_types.contains(&type_url.to_string())
    }

    /// Add support for a new message type
    pub fn add_supported_message_type(&mut self, type_url: String) {
        if !self.config.supported_message_types.contains(&type_url) {
            self.config.supported_message_types.push(type_url);
        }
    }

    /// Encode a transaction back to bytes (for testing and storage)
    pub fn encode_cosmos_tx(&self, tx: &CosmosTx) -> Result<Vec<u8>, TxDecodingError> {
        serde_json::to_vec(tx)
            .map_err(|e| TxDecodingError::JsonError(e.to_string()))
    }

    /// Create a transaction from individual components with validation
    pub fn create_transaction(
        &self,
        body: TxBody,
        auth_info: AuthInfo,
        signatures: Vec<Vec<u8>>
    ) -> Result<CosmosTx, TxDecodingError> {
        let tx = CosmosTx::new(body, auth_info, signatures);
        self.validate_tx_structure(&tx)?;
        Ok(tx)
    }
}

/// Decoded message with metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecodedMessage {
    /// Message index in the transaction
    pub index: usize,
    /// Message type URL
    pub type_url: String,
    /// Raw message data
    pub raw_data: Vec<u8>,
    /// Decoded message data (if available)
    pub decoded_data: Option<serde_json::Value>,
}

impl DecodedMessage {
    /// Get the message data as a specific type
    pub fn decode_as<T>(&self) -> Result<T, TxDecodingError>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(&self.raw_data)
            .map_err(|e| TxDecodingError::JsonError(e.to_string()))
    }
}

/// Builder for creating transactions programmatically
pub struct TxBuilder {
    decoder: TxDecoder,
    messages: Vec<Any>,
    memo: String,
    timeout_height: u64,
    gas_limit: u64,
    fee_amount: Vec<crate::types::cosmos_tx::Coin>,
    signatures: Vec<Vec<u8>>,
}

impl TxBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            decoder: TxDecoder::new(),
            messages: Vec::new(),
            memo: String::new(),
            timeout_height: 0,
            gas_limit: 200_000,
            fee_amount: Vec::new(),
            signatures: Vec::new(),
        }
    }

    /// Add a message to the transaction
    pub fn add_message(mut self, type_url: String, data: Vec<u8>) -> Self {
        self.messages.push(Any::new(&type_url, data));
        self
    }

    /// Set the memo
    pub fn memo(mut self, memo: String) -> Self {
        self.memo = memo;
        self
    }

    /// Set the timeout height
    pub fn timeout_height(mut self, height: u64) -> Self {
        self.timeout_height = height;
        self
    }

    /// Set the gas limit
    pub fn gas_limit(mut self, limit: u64) -> Self {
        self.gas_limit = limit;
        self
    }

    /// Add fee amount
    pub fn fee_amount(mut self, denom: String, amount: String) -> Self {
        self.fee_amount.push(crate::types::cosmos_tx::Coin::new(&denom, &amount));
        self
    }

    /// Add a signature
    pub fn signature(mut self, sig: Vec<u8>) -> Self {
        self.signatures.push(sig);
        self
    }

    /// Build the transaction
    pub fn build(self, signer_infos: Vec<crate::types::cosmos_tx::SignerInfo>) -> Result<CosmosTx, TxDecodingError> {
        use crate::types::cosmos_tx::{TxBody, AuthInfo, Fee};

        let body = TxBody {
            messages: self.messages,
            memo: self.memo,
            timeout_height: self.timeout_height,
            extension_options: Vec::new(),
            non_critical_extension_options: Vec::new(),
        };

        let fee = Fee::new(self.fee_amount, self.gas_limit);
        let auth_info = AuthInfo::new(signer_infos, fee);

        self.decoder.create_transaction(body, auth_info, self.signatures)
    }
}

impl Default for TxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::cosmos_tx::{TxBody, AuthInfo, Fee, SignerInfo, Coin, SignMode, ModeInfo};

    fn create_test_transaction() -> CosmosTx {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];

        CosmosTx::new(body, auth_info, signatures)
    }

    #[test]
    fn test_tx_decoder_creation() {
        let decoder = TxDecoder::new();
        assert!(!decoder.config.supported_message_types.is_empty());
        assert_eq!(decoder.config.max_tx_size, 1024 * 1024);
    }

    #[test]
    fn test_decode_valid_transaction() {
        let decoder = TxDecoder::new();
        let tx = create_test_transaction();
        
        // Encode to JSON
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        
        // Decode back
        let decoded_tx = decoder.decode_cosmos_tx(tx_bytes).unwrap();
        
        assert_eq!(decoded_tx.body.messages.len(), 1);
        assert_eq!(decoded_tx.body.messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        assert_eq!(decoded_tx.signatures.len(), 1);
    }

    #[test]
    fn test_decode_invalid_json() {
        let decoder = TxDecoder::new();
        let invalid_json = b"invalid json";
        
        let result = decoder.decode_cosmos_tx(invalid_json.to_vec());
        assert!(matches!(result, Err(TxDecodingError::JsonError(_))));
    }

    #[test]
    fn test_transaction_size_limit() {
        let mut config = TxDecoderConfig::default();
        config.max_tx_size = 10; // Very small limit
        
        let decoder = TxDecoder::with_config(config);
        let tx = create_test_transaction();
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        
        let result = decoder.decode_cosmos_tx(tx_bytes);
        assert!(matches!(result, Err(TxDecodingError::InvalidFormat(_))));
    }

    #[test]
    fn test_unsupported_message_type() {
        let decoder = TxDecoder::new();
        let msg = Any::new("/unsupported.message.Type", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];
        
        let tx = CosmosTx::new(body, auth_info, signatures);
        
        let result = decoder.validate_tx_structure(&tx);
        assert!(matches!(result, Err(TxDecodingError::InvalidStructure(_))));
    }

    #[test]
    fn test_extract_messages() {
        let decoder = TxDecoder::new();
        let tx = create_test_transaction();
        
        let messages = decoder.extract_messages(&tx).unwrap();
        
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].index, 0);
        assert_eq!(messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        assert_eq!(messages[0].raw_data, vec![1, 2, 3]);
    }

    #[test]
    fn test_message_type_support() {
        let mut decoder = TxDecoder::new();
        
        assert!(decoder.is_message_type_supported("/cosmos.bank.v1beta1.MsgSend"));
        assert!(!decoder.is_message_type_supported("/custom.module.MsgCustom"));
        
        decoder.add_supported_message_type("/custom.module.MsgCustom".to_string());
        assert!(decoder.is_message_type_supported("/custom.module.MsgCustom"));
    }

    #[test]
    fn test_gas_limit_validation() {
        let mut config = TxDecoderConfig::default();
        config.max_gas_limit = 100_000; // Lower than test transaction
        
        let decoder = TxDecoder::with_config(config);
        let tx = create_test_transaction();
        
        let result = decoder.validate_tx_structure(&tx);
        assert!(matches!(result, Err(TxDecodingError::InvalidStructure(_))));
    }

    #[test]
    fn test_tx_builder() {
        let tx = TxBuilder::new()
            .add_message("/cosmos.bank.v1beta1.MsgSend".to_string(), vec![1, 2, 3])
            .memo("test memo".to_string())
            .gas_limit(300_000)
            .fee_amount("uatom".to_string(), "2000".to_string())
            .signature(vec![7, 8, 9])
            .build(vec![SignerInfo {
                public_key: None,
                mode_info: ModeInfo {
                    mode: SignMode::Direct,
                    multi: None,
                },
                sequence: 1,
            }])
            .unwrap();
        
        assert_eq!(tx.body.messages.len(), 1);
        assert_eq!(tx.body.memo, "test memo");
        assert_eq!(tx.auth_info.fee.gas_limit, 300_000);
        assert_eq!(tx.auth_info.fee.amount.len(), 1);
        assert_eq!(tx.signatures.len(), 1);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let decoder = TxDecoder::new();
        let original_tx = create_test_transaction();
        
        // Encode
        let encoded = decoder.encode_cosmos_tx(&original_tx).unwrap();
        
        // Decode
        let decoded_tx = decoder.decode_cosmos_tx(encoded).unwrap();
        
        assert_eq!(original_tx, decoded_tx);
    }

    #[test]
    fn test_signature_mismatch_validation() {
        let decoder = TxDecoder::new();
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![]; // No signatures for one signer
        
        let tx = CosmosTx::new(body, auth_info, signatures);
        
        let result = decoder.validate_tx_structure(&tx);
        assert!(matches!(result, Err(TxDecodingError::ValidationError(_))));
    }
}