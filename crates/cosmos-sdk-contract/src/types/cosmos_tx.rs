use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Standard Cosmos SDK transaction structure
/// This mirrors the CosmosTx format used across the Cosmos ecosystem
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct CosmosTx {
    /// Transaction body containing the actual messages and metadata
    pub body: TxBody,
    /// Authentication information including signatures and fee
    pub auth_info: AuthInfo,
    /// Raw signatures in the same order as signer_infos in auth_info
    pub signatures: Vec<Vec<u8>>,
}

/// Transaction body containing messages and metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct TxBody {
    /// List of messages to be executed in this transaction
    pub messages: Vec<Any>,
    /// Human-readable memo for the transaction
    pub memo: String,
    /// Block height at which this transaction times out (0 = no timeout)
    pub timeout_height: u64,
    /// Extension options for future protocol upgrades
    pub extension_options: Vec<Any>,
    /// Non-critical extension options
    pub non_critical_extension_options: Vec<Any>,
}

/// Authentication information for the transaction
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct AuthInfo {
    /// Information about each signer
    pub signer_infos: Vec<SignerInfo>,
    /// Fee information for this transaction
    pub fee: Fee,
    /// Optional tip for prioritization
    pub tip: Option<Tip>,
}

/// Information about a transaction signer
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct SignerInfo {
    /// Public key of the signer (optional for known accounts)
    pub public_key: Option<Any>,
    /// Signing mode information
    pub mode_info: ModeInfo,
    /// Account sequence number for replay protection
    pub sequence: u64,
}

/// Signing mode information
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct ModeInfo {
    /// The signing mode used
    pub mode: SignMode,
    /// Multi-signature information (if applicable)
    pub multi: Option<MultiSignatureInfo>,
}

/// Supported signing modes
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub enum SignMode {
    /// Standard single signature mode
    Direct,
    /// Textual representation signing (for hardware wallets)
    Textual,
    /// Legacy Amino signing mode (deprecated)
    LegacyAminoJson,
}

/// Multi-signature information
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct MultiSignatureInfo {
    /// Bitmap indicating which signatures are present
    pub bitarray: CompactBitArray,
    /// Mode information for each signature
    pub mode_infos: Vec<ModeInfo>,
}

/// Compact bit array for multi-signature bitmaps
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct CompactBitArray {
    /// Number of extra bits in the last byte
    pub extra_bits_stored: u32,
    /// Raw bit data
    pub elems: Vec<u8>,
}

/// Fee information for the transaction
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct Fee {
    /// Amount of tokens to pay as fees
    pub amount: Vec<Coin>,
    /// Gas limit for this transaction
    pub gas_limit: u64,
    /// Account that will pay the fees (empty = first signer pays)
    pub payer: String,
    /// Account that granted the fee payment (for fee grants)
    pub granter: String,
}

/// Optional tip for transaction prioritization
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct Tip {
    /// Amount of tokens to tip
    pub amount: Vec<Coin>,
    /// Account that will receive the tip
    pub tipper: String,
}

/// Generic protobuf Any type for encoding arbitrary message types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct Any {
    /// Type URL identifying the message type
    pub type_url: String,
    /// Serialized message data
    pub value: Vec<u8>,
}

/// Coin represents a token amount with denomination
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct Coin {
    /// Token denomination (e.g., "uatom", "near")
    pub denom: String,
    /// Token amount as string to avoid precision issues
    pub amount: String,
}

/// Document used for transaction signing
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct SignDoc {
    /// Serialized transaction body
    pub body_bytes: Vec<u8>,
    /// Serialized auth info
    pub auth_info_bytes: Vec<u8>,
    /// Chain identifier
    pub chain_id: String,
    /// Account number
    pub account_number: u64,
}

/// Hardware wallet compatible signing document
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct HwSignDoc {
    /// Chain identifier
    pub chain_id: String,
    /// Account number
    pub account_number: u64,
    /// Sequence number
    pub sequence: u64,
    /// Fee information
    pub fee: Fee,
    /// List of messages (simplified for hardware display)
    pub msgs: Vec<HwMessage>,
    /// Memo
    pub memo: String,
}

/// Simplified message format for hardware wallet display
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct HwMessage {
    /// Message type
    pub msg_type: String,
    /// Simplified message content for display (as JSON string)
    pub value: String,
}

impl CosmosTx {
    /// Create a new Cosmos transaction
    pub fn new(body: TxBody, auth_info: AuthInfo, signatures: Vec<Vec<u8>>) -> Self {
        Self {
            body,
            auth_info,
            signatures,
        }
    }

    /// Validate the transaction structure
    pub fn validate(&self) -> Result<(), TxValidationError> {
        // Validate that we have signatures for all signers
        if self.signatures.len() != self.auth_info.signer_infos.len() {
            return Err(TxValidationError::SignatureMismatch {
                expected: self.auth_info.signer_infos.len(),
                actual: self.signatures.len(),
            });
        }

        // Validate that all messages have valid type URLs
        for msg in &self.body.messages {
            if msg.type_url.is_empty() {
                return Err(TxValidationError::EmptyTypeUrl);
            }
        }

        // Validate fee structure
        self.auth_info.fee.validate()?;

        Ok(())
    }

    /// Get the transaction hash (for indexing and queries)
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        
        // Serialize the transaction for hashing
        let tx_bytes = serde_json::to_vec(self).unwrap_or_default();
        let hash = Sha256::digest(&tx_bytes);
        hex::encode(hash).to_uppercase()
    }

    /// Get the signing document for verification
    pub fn get_sign_doc(&self, chain_id: &str, account_number: u64) -> Result<SignDoc, TxValidationError> {
        let body_bytes = serde_json::to_vec(&self.body)
            .map_err(|e| TxValidationError::SerializationError(e.to_string()))?;
        
        let auth_info_bytes = serde_json::to_vec(&self.auth_info)
            .map_err(|e| TxValidationError::SerializationError(e.to_string()))?;

        Ok(SignDoc {
            body_bytes,
            auth_info_bytes,
            chain_id: chain_id.to_string(),
            account_number,
        })
    }
}

impl TxBody {
    /// Create a new transaction body
    pub fn new(messages: Vec<Any>) -> Self {
        Self {
            messages,
            memo: String::new(),
            timeout_height: 0,
            extension_options: Vec::new(),
            non_critical_extension_options: Vec::new(),
        }
    }

    /// Add a message to the transaction body
    pub fn add_message(&mut self, message: Any) {
        self.messages.push(message);
    }

    /// Set the memo for the transaction
    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = memo;
        self
    }

    /// Set the timeout height
    pub fn with_timeout_height(mut self, height: u64) -> Self {
        self.timeout_height = height;
        self
    }
}

impl AuthInfo {
    /// Create new auth info
    pub fn new(signer_infos: Vec<SignerInfo>, fee: Fee) -> Self {
        Self {
            signer_infos,
            fee,
            tip: None,
        }
    }

    /// Add a tip to the transaction
    pub fn with_tip(mut self, tip: Tip) -> Self {
        self.tip = Some(tip);
        self
    }
}

impl SignerInfo {
    /// Create new signer info with direct signing mode
    pub fn direct(public_key: Option<Any>, sequence: u64) -> Self {
        Self {
            public_key,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence,
        }
    }

    /// Create new signer info for multi-signature
    pub fn multi_sig(sequence: u64, multi_info: MultiSignatureInfo) -> Self {
        Self {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: Some(multi_info),
            },
            sequence,
        }
    }
}

impl Fee {
    /// Create a new fee
    pub fn new(amount: Vec<Coin>, gas_limit: u64) -> Self {
        Self {
            amount,
            gas_limit,
            payer: String::new(),
            granter: String::new(),
        }
    }

    /// Set the fee payer
    pub fn with_payer(mut self, payer: String) -> Self {
        self.payer = payer;
        self
    }

    /// Set the fee granter
    pub fn with_granter(mut self, granter: String) -> Self {
        self.granter = granter;
        self
    }

    /// Validate the fee structure
    pub fn validate(&self) -> Result<(), TxValidationError> {
        if self.gas_limit == 0 {
            return Err(TxValidationError::InvalidGasLimit);
        }

        for coin in &self.amount {
            coin.validate()?;
        }

        Ok(())
    }
}

impl Coin {
    /// Create a new coin
    pub fn new(denom: &str, amount: &str) -> Self {
        Self {
            denom: denom.to_string(),
            amount: amount.to_string(),
        }
    }

    /// Validate the coin
    pub fn validate(&self) -> Result<(), TxValidationError> {
        if self.denom.is_empty() {
            return Err(TxValidationError::EmptyDenomination);
        }

        // Validate amount is a valid number
        self.amount.parse::<u128>()
            .map_err(|_| TxValidationError::InvalidAmount(self.amount.clone()))?;

        Ok(())
    }
}

impl Any {
    /// Create a new Any message
    pub fn new(type_url: &str, value: Vec<u8>) -> Self {
        Self {
            type_url: type_url.to_string(),
            value,
        }
    }
}

impl SignDoc {
    /// Create the canonical bytes for signing
    pub fn signing_bytes(&self) -> Vec<u8> {
        use base64::Engine;
        
        // Create the canonical JSON representation for signing
        let sign_doc = serde_json::json!({
            "body_bytes": base64::engine::general_purpose::STANDARD.encode(&self.body_bytes),
            "auth_info_bytes": base64::engine::general_purpose::STANDARD.encode(&self.auth_info_bytes),
            "chain_id": self.chain_id,
            "account_number": self.account_number.to_string()
        });

        serde_json::to_vec(&sign_doc).unwrap_or_default()
    }
}

/// Transaction validation errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TxValidationError {
    /// Mismatch between number of signatures and signers
    SignatureMismatch { expected: usize, actual: usize },
    /// Empty type URL in message
    EmptyTypeUrl,
    /// Invalid gas limit
    InvalidGasLimit,
    /// Empty denomination in coin
    EmptyDenomination,
    /// Invalid amount in coin
    InvalidAmount(String),
    /// Serialization error
    SerializationError(String),
}

impl std::fmt::Display for TxValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxValidationError::SignatureMismatch { expected, actual } => {
                write!(f, "Signature count mismatch: expected {}, got {}", expected, actual)
            }
            TxValidationError::EmptyTypeUrl => write!(f, "Message has empty type URL"),
            TxValidationError::InvalidGasLimit => write!(f, "Gas limit must be greater than 0"),
            TxValidationError::EmptyDenomination => write!(f, "Coin denomination cannot be empty"),
            TxValidationError::InvalidAmount(amount) => write!(f, "Invalid coin amount: {}", amount),
            TxValidationError::SerializationError(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl std::error::Error for TxValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmos_tx_creation() {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo::direct(None, 1);
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];

        let tx = CosmosTx::new(body, auth_info, signatures);
        
        assert_eq!(tx.body.messages.len(), 1);
        assert_eq!(tx.body.messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        assert_eq!(tx.auth_info.signer_infos.len(), 1);
        assert_eq!(tx.signatures.len(), 1);
    }

    #[test]
    fn test_tx_validation_success() {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo::direct(None, 1);
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];

        let tx = CosmosTx::new(body, auth_info, signatures);
        
        assert!(tx.validate().is_ok());
    }

    #[test]
    fn test_tx_validation_signature_mismatch() {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo::direct(None, 1);
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![]; // No signatures provided

        let tx = CosmosTx::new(body, auth_info, signatures);
        
        assert!(matches!(
            tx.validate(),
            Err(TxValidationError::SignatureMismatch { expected: 1, actual: 0 })
        ));
    }

    #[test]
    fn test_coin_validation() {
        let valid_coin = Coin::new("uatom", "1000");
        assert!(valid_coin.validate().is_ok());

        let empty_denom = Coin::new("", "1000");
        assert!(matches!(
            empty_denom.validate(),
            Err(TxValidationError::EmptyDenomination)
        ));

        let invalid_amount = Coin::new("uatom", "invalid");
        assert!(matches!(
            invalid_amount.validate(),
            Err(TxValidationError::InvalidAmount(_))
        ));
    }

    #[test]
    fn test_fee_validation() {
        let valid_fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        assert!(valid_fee.validate().is_ok());

        let zero_gas = Fee::new(vec![Coin::new("uatom", "1000")], 0);
        assert!(matches!(
            zero_gas.validate(),
            Err(TxValidationError::InvalidGasLimit)
        ));
    }

    #[test]
    fn test_tx_hash() {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo::direct(None, 1);
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];

        let tx = CosmosTx::new(body, auth_info, signatures);
        let hash = tx.hash();
        
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hash in hex
    }

    #[test]
    fn test_sign_doc_creation() {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo::direct(None, 1);
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![4, 5, 6]];

        let tx = CosmosTx::new(body, auth_info, signatures);
        let sign_doc = tx.get_sign_doc("cosmoshub-4", 42).unwrap();
        
        assert_eq!(sign_doc.chain_id, "cosmoshub-4");
        assert_eq!(sign_doc.account_number, 42);
        assert!(!sign_doc.body_bytes.is_empty());
        assert!(!sign_doc.auth_info_bytes.is_empty());

        let signing_bytes = sign_doc.signing_bytes();
        assert!(!signing_bytes.is_empty());
    }
}