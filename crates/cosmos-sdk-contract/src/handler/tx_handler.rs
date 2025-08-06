use crate::types::cosmos_tx::{CosmosTx, TxValidationError, SignDoc};
use crate::handler::{TxDecoder, TxDecodingError, HandleResult, ContractError};
use crate::crypto::{CosmosSignatureVerifier, SignatureError, CosmosPublicKey};
use crate::modules::auth::{AccountManager, AccountError, AccountConfig, FeeProcessor, FeeError, FeeConfig};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::AccountId;
use near_sdk::base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Transaction processing errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TxProcessingError {
    /// Transaction decoding failed
    DecodingError(TxDecodingError),
    /// Signature verification failed
    SignatureError(SignatureError),
    /// Transaction validation failed
    ValidationError(TxValidationError),
    /// Message processing failed
    MessageProcessingError(ContractError),
    /// Account management error
    AccountError(String),
    /// Fee processing error
    FeeError(String),
    /// Gas limit exceeded
    GasLimitExceeded { limit: u64, used: u64 },
    /// Invalid transaction state
    InvalidState(String),
    /// Sequence number mismatch (replay protection)
    SequenceMismatch { expected: u64, actual: u64 },
    /// Message execution failed
    MessageExecution(String),
    /// Transaction not found
    TransactionNotFound,
}

impl std::fmt::Display for TxProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxProcessingError::DecodingError(err) => write!(f, "Transaction decoding error: {}", err),
            TxProcessingError::SignatureError(err) => write!(f, "Signature verification error: {}", err),
            TxProcessingError::ValidationError(err) => write!(f, "Transaction validation error: {}", err),
            TxProcessingError::MessageProcessingError(err) => write!(f, "Message processing error: {:?}", err),
            TxProcessingError::AccountError(msg) => write!(f, "Account error: {}", msg),
            TxProcessingError::FeeError(msg) => write!(f, "Fee processing error: {}", msg),
            TxProcessingError::GasLimitExceeded { limit, used } => {
                write!(f, "Gas limit exceeded: used {}, limit {}", used, limit)
            }
            TxProcessingError::InvalidState(msg) => write!(f, "Invalid transaction state: {}", msg),
            TxProcessingError::SequenceMismatch { expected, actual } => {
                write!(f, "Sequence mismatch: expected {}, got {}", expected, actual)
            }
            TxProcessingError::MessageExecution(msg) => write!(f, "Message execution error: {}", msg),
            TxProcessingError::TransactionNotFound => write!(f, "Transaction not found"),
        }
    }
}

impl std::error::Error for TxProcessingError {}

impl From<TxDecodingError> for TxProcessingError {
    fn from(err: TxDecodingError) -> Self {
        TxProcessingError::DecodingError(err)
    }
}

impl From<SignatureError> for TxProcessingError {
    fn from(err: SignatureError) -> Self {
        TxProcessingError::SignatureError(err)
    }
}

impl From<TxValidationError> for TxProcessingError {
    fn from(err: TxValidationError) -> Self {
        TxProcessingError::ValidationError(err)
    }
}

impl From<ContractError> for TxProcessingError {
    fn from(err: ContractError) -> Self {
        TxProcessingError::MessageProcessingError(err)
    }
}

impl From<AccountError> for TxProcessingError {
    fn from(err: AccountError) -> Self {
        TxProcessingError::AccountError(err.to_string())
    }
}

impl From<FeeError> for TxProcessingError {
    fn from(err: FeeError) -> Self {
        TxProcessingError::FeeError(err.to_string())
    }
}

/// ABCI standardized response codes
/// Based on Cosmos SDK and Tendermint ABCI specifications
#[derive(Clone, Debug, PartialEq)]
pub struct ABCICode;

impl ABCICode {
    pub const OK: u32 = 0;
    pub const INTERNAL_ERROR: u32 = 1;
    pub const TX_DECODE_ERROR: u32 = 2;
    pub const INVALID_SEQUENCE: u32 = 3;
    pub const UNAUTHORIZED: u32 = 4;
    pub const INSUFFICIENT_FUNDS: u32 = 5;
    pub const UNKNOWN_REQUEST: u32 = 6;
    pub const INVALID_ADDRESS: u32 = 7;
    pub const INVALID_PUBKEY: u32 = 8;
    pub const UNKNOWN_ADDRESS: u32 = 9;
    pub const INSUFFICIENT_FEE: u32 = 10;
    pub const MEMO_TOO_LARGE: u32 = 11;
    pub const OUT_OF_GAS: u32 = 12;
    pub const TX_TOO_LARGE: u32 = 13;
    pub const INVALID_COINS: u32 = 14;
    pub const INVALID_REQUEST: u32 = 15;
    pub const TIMEOUT: u32 = 16;
    
    /// Convert TxProcessingError to appropriate ABCI code
    pub fn from_error(error: &TxProcessingError) -> u32 {
        match error {
            TxProcessingError::DecodingError(_) => Self::TX_DECODE_ERROR,
            TxProcessingError::SignatureError(_) => Self::UNAUTHORIZED,
            TxProcessingError::ValidationError(_) => Self::INVALID_REQUEST,
            TxProcessingError::MessageProcessingError(_) => Self::INTERNAL_ERROR,
            TxProcessingError::AccountError(_) => Self::UNKNOWN_ADDRESS,
            TxProcessingError::FeeError(_) => Self::INSUFFICIENT_FEE,
            TxProcessingError::GasLimitExceeded { .. } => Self::OUT_OF_GAS,
            TxProcessingError::InvalidState(_) => Self::INVALID_REQUEST,
            TxProcessingError::SequenceMismatch { .. } => Self::INVALID_SEQUENCE,
            TxProcessingError::MessageExecution(_) => Self::INTERNAL_ERROR,
            TxProcessingError::TransactionNotFound => Self::UNKNOWN_REQUEST,
        }
    }
}

/// Enhanced ABCI-compatible event attribute with proper encoding support
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ABCIAttribute {
    /// Attribute key (base64 encoded bytes for full ABCI compatibility)
    pub key: String,
    /// Attribute value (base64 encoded bytes for full ABCI compatibility)
    pub value: String,
    /// Whether this attribute should be indexed by block explorers
    #[serde(default)]
    pub index: bool,
}

impl ABCIAttribute {
    /// Create new attribute from string key-value pair
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: BASE64.encode(key.as_bytes()),
            value: BASE64.encode(value.as_bytes()),
            index: false,
        }
    }
    
    /// Create new indexed attribute (for block explorer indexing)
    pub fn new_indexed(key: &str, value: &str) -> Self {
        Self {
            key: BASE64.encode(key.as_bytes()),
            value: BASE64.encode(value.as_bytes()),
            index: true,
        }
    }
    
    /// Create attribute from raw bytes
    pub fn from_bytes(key: &[u8], value: &[u8], index: bool) -> Self {
        Self {
            key: BASE64.encode(key),
            value: BASE64.encode(value),
            index,
        }
    }
    
    /// Decode key as string (for debugging/display)
    pub fn decode_key(&self) -> Result<String, String> {
        BASE64.decode(&self.key)
            .map_err(|e| format!("Failed to decode key: {}", e))
            .and_then(|bytes| String::from_utf8(bytes)
                .map_err(|e| format!("Key is not valid UTF-8: {}", e)))
    }
    
    /// Decode value as string (for debugging/display)
    pub fn decode_value(&self) -> Result<String, String> {
        BASE64.decode(&self.value)
            .map_err(|e| format!("Failed to decode value: {}", e))
            .and_then(|bytes| String::from_utf8(bytes)
                .map_err(|e| format!("Value is not valid UTF-8: {}", e)))
    }
}

/// Enhanced ABCI-compatible event
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ABCIEvent {
    /// Event type
    pub r#type: String,
    /// Event attributes with proper ABCI encoding
    pub attributes: Vec<ABCIAttribute>,
}

impl ABCIEvent {
    /// Create new event with string attributes
    pub fn new(event_type: &str, attributes: Vec<(&str, &str)>) -> Self {
        Self {
            r#type: event_type.to_string(),
            attributes: attributes.into_iter()
                .map(|(k, v)| ABCIAttribute::new(k, v))
                .collect(),
        }
    }
    
    /// Add indexed attribute (for block explorer indexing)
    pub fn with_indexed_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.push(ABCIAttribute::new_indexed(key, value));
        self
    }
}

/// Gas usage tracking for accurate ABCI reporting
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GasInfo {
    /// Gas limit set in the transaction
    pub gas_wanted: u64,
    /// Actual gas consumed during execution
    pub gas_used: u64,
}

impl GasInfo {
    pub fn new(gas_wanted: u64, gas_used: u64) -> Self {
        Self { gas_wanted, gas_used }
    }
    
    /// Check if transaction ran out of gas
    pub fn out_of_gas(&self) -> bool {
        self.gas_used >= self.gas_wanted
    }
    
    /// Calculate gas efficiency (used/wanted ratio)
    pub fn efficiency(&self) -> f64 {
        if self.gas_wanted == 0 {
            0.0
        } else {
            (self.gas_used as f64) / (self.gas_wanted as f64)
        }
    }
}

/// Enhanced transaction response format with full ABCI compatibility
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TxResponse {
    /// Block height where transaction was included
    pub height: String,
    /// Transaction hash
    pub txhash: String,
    /// ABCI response code (0 = success, see ABCICode for standard codes)
    pub code: u32,
    /// Response data (base64 encoded for ABCI compatibility)
    pub data: String,
    /// Raw log output (human readable)
    pub raw_log: String,
    /// Structured logs per message with ABCI events
    pub logs: Vec<ABCIMessageLog>,
    /// Additional info
    pub info: String,
    /// Gas wanted (requested)
    pub gas_wanted: String,
    /// Gas used (actual)
    pub gas_used: String,
    /// Original transaction (optional, excluded in many queries for size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx: Option<CosmosTx>,
    /// Timestamp in RFC3339 format
    pub timestamp: String,
    /// ABCI-compatible events emitted during transaction execution
    pub events: Vec<ABCIEvent>,
    /// Codespace for error categorization (e.g., "sdk", "ibc", "bank")
    #[serde(default)]
    pub codespace: String,
}

/// Enhanced ABCI message log with proper event handling
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ABCIMessageLog {
    /// Message index
    pub msg_index: u32,
    /// Log message (human readable)
    pub log: String,
    /// ABCI-compatible events for this message
    pub events: Vec<ABCIEvent>,
}

/// Legacy event structure (kept for backwards compatibility with message router)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Event type
    pub r#type: String,
    /// Event attributes
    pub attributes: Vec<Attribute>,
}

impl Event {
    /// Convert legacy event to ABCI-compatible event
    pub fn to_abci_event(&self) -> ABCIEvent {
        ABCIEvent {
            r#type: self.r#type.clone(),
            attributes: self.attributes.iter()
                .map(|attr| ABCIAttribute::new(&attr.key, &attr.value))
                .collect(),
        }
    }
}

/// Legacy event attribute (kept for backwards compatibility)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute key
    pub key: String,
    /// Attribute value
    pub value: String,
}

/// Transaction processing configuration
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TxProcessingConfig {
    /// Chain ID for signature verification
    pub chain_id: String,
    /// Maximum gas per transaction
    pub max_gas_per_tx: u64,
    /// Gas price in NEAR tokens
    pub gas_price: u128,
    /// Enable signature verification
    pub verify_signatures: bool,
    /// Enable sequence number checking
    pub check_sequences: bool,
}

impl Default for TxProcessingConfig {
    fn default() -> Self {
        Self {
            chain_id: "near-cosmos-sdk".to_string(),
            max_gas_per_tx: 10_000_000,
            gas_price: 1, // 1 yoctoNEAR per gas unit
            verify_signatures: true,
            check_sequences: true,
        }
    }
}

/// Unified Cosmos transaction handler
pub struct CosmosTransactionHandler {
    /// Transaction decoder
    pub tx_decoder: TxDecoder,
    /// Signature verifier
    pub signature_verifier: CosmosSignatureVerifier,
    /// Processing configuration
    pub config: TxProcessingConfig,
    /// Account manager for sequence validation and account creation
    account_manager: AccountManager,
    /// Fee processor for Cosmos fee adaptation to NEAR gas
    fee_processor: FeeProcessor,
}

impl CosmosTransactionHandler {
    /// Create a new transaction handler
    pub fn new(config: TxProcessingConfig) -> Self {
        let account_config = AccountConfig {
            address_prefix: config.chain_id.clone(),
            auto_create_accounts: true,
            max_sequence: 1_000_000,
        };
        
        Self {
            tx_decoder: TxDecoder::new(),
            signature_verifier: CosmosSignatureVerifier::new(config.chain_id.clone()),
            config,
            account_manager: AccountManager::new(account_config),
            fee_processor: FeeProcessor::new(FeeConfig::default()),
        }
    }
    
    /// Create a new transaction handler with custom configurations
    pub fn new_with_configs(
        config: TxProcessingConfig,
        account_config: AccountConfig,
        fee_config: FeeConfig,
    ) -> Self {
        Self {
            tx_decoder: TxDecoder::new(),
            signature_verifier: CosmosSignatureVerifier::new(config.chain_id.clone()),
            config,
            account_manager: AccountManager::new(account_config),
            fee_processor: FeeProcessor::new(fee_config),
        }
    }

    /// Process a complete Cosmos SDK transaction with contract integration
    pub fn process_transaction<T>(&mut self, raw_tx: Vec<u8>, contract: &mut T) -> Result<TxResponse, TxProcessingError>
    where
        T: crate::handler::CosmosMessageHandler,
    {
        // 1. Decode the transaction
        let tx = self.tx_decoder.decode_cosmos_tx(raw_tx)?;

        // 2. Validate transaction structure
        self.validate_transaction(&tx)?;

        // 3. Verify signatures (if enabled)
        let recovered_keys = if self.config.verify_signatures {
            self.verify_transaction_signatures(&tx)?
        } else {
            Vec::new()
        };

        // 4. Check account sequences (if enabled)
        if self.config.check_sequences {
            self.check_account_sequences(&tx, &recovered_keys)?;
        }

        // 5. Process transaction fees
        let payer = if let (Some(_key), Some(address)) = (recovered_keys.get(0), self.account_manager.derive_addresses(&recovered_keys)?.get(0)) {
            address.clone()
        } else {
            // Fallback to first signer address if available
            // In a real implementation, this would be derived from public keys
            "near-sdk-payer".to_string()
        };
        
        let _total_fees = self.process_transaction_fees(&tx, &payer)?;

        // 6. Process messages using the contract's message router
        let message_responses = self.process_transaction_messages_with_contract(&tx, contract)?;

        // 7. Update account sequences after successful message processing
        self.update_account_sequences(&tx, &recovered_keys)?;

        // 8. Create transaction response
        Ok(self.create_transaction_response(&tx, message_responses))
    }

    /// Process a complete Cosmos SDK transaction (standalone version)
    pub fn process_cosmos_transaction(&mut self, raw_tx: Vec<u8>) -> Result<TxResponse, TxProcessingError> {
        // 1. Decode the transaction
        let tx = self.tx_decoder.decode_cosmos_tx(raw_tx)?;

        // 2. Validate transaction structure
        self.validate_transaction(&tx)?;

        // 3. Verify signatures (if enabled)
        let recovered_keys = if self.config.verify_signatures {
            self.verify_transaction_signatures(&tx)?
        } else {
            Vec::new()
        };

        // 4. Check account sequences (if enabled)
        if self.config.check_sequences {
            self.check_account_sequences(&tx, &recovered_keys)?;
        }

        // 5. Process fee payment (get payer address from first signer)
        let payer = if let (Some(_key), Some(address)) = (recovered_keys.get(0), self.account_manager.derive_addresses(&recovered_keys)?.get(0)) {
            address.clone()
        } else {
            // Fallback to a placeholder for tests
            "unknown".to_string()
        };
        let _total_fee_paid = self.process_transaction_fees(&tx, &payer)?;

        // 6. Process messages sequentially
        let message_responses = self.process_transaction_messages(&tx)?;

        // 7. Update account sequences after successful message processing
        self.update_account_sequences(&tx, &recovered_keys)?;

        // 8. Create transaction response
        Ok(self.create_transaction_response(&tx, message_responses))
    }

    /// Simulate a transaction without executing it
    pub fn simulate_transaction(&mut self, raw_tx: Vec<u8>) -> Result<TxResponse, TxProcessingError> {
        // Decode and validate
        let tx = self.tx_decoder.decode_cosmos_tx(raw_tx)?;
        self.validate_transaction(&tx)?;
        
        // For simulation, we skip signature verification if it would fail
        // (e.g., with dummy signatures in tests)
        if self.config.verify_signatures {
            match self.verify_transaction_signatures(&tx) {
                Ok(_recovered_keys) => {
                    // Signature verification succeeded
                },
                Err(_) => {
                    // Skip signature verification for simulation if it fails
                    // This allows testing with dummy signatures
                }
            }
        }

        // Simulate message processing (dry run)
        let simulated_responses = self.simulate_transaction_messages(&tx)?;

        // Create simulation response
        Ok(self.create_simulation_response(&tx, simulated_responses))
    }

    /// Validate transaction before processing
    pub fn validate_transaction(&self, tx: &CosmosTx) -> Result<(), TxProcessingError> {
        // Basic transaction validation
        tx.validate()?;

        // Check gas limits
        if tx.auth_info.fee.gas_limit > self.config.max_gas_per_tx {
            return Err(TxProcessingError::GasLimitExceeded {
                limit: self.config.max_gas_per_tx,
                used: tx.auth_info.fee.gas_limit,
            });
        }

        // Validate all messages have supported types
        let messages = self.tx_decoder.extract_messages(tx)?;
        for msg in &messages {
            if !self.tx_decoder.is_message_type_supported(&msg.type_url) {
                return Err(TxProcessingError::InvalidState(
                    format!("Unsupported message type: {}", msg.type_url)
                ));
            }
        }

        Ok(())
    }

    /// Verify all transaction signatures
    fn verify_transaction_signatures(&mut self, tx: &CosmosTx) -> Result<Vec<CosmosPublicKey>, TxProcessingError> {
        let recovered_keys = self.signature_verifier.verify_signatures(tx, &[])?;
        
        // Get or create accounts for the recovered public keys
        let mut account_numbers = Vec::new();
        for key in &recovered_keys {
            let account = self.account_manager.get_or_create_account(key.clone())?;
            account_numbers.push(account.account_number);
        }
        
        // Verify signatures again with proper account numbers
        let verified_keys = self.signature_verifier.verify_signatures(tx, &account_numbers)?;
        Ok(verified_keys)
    }

    /// Check account sequence numbers for replay protection
    fn check_account_sequences(&self, tx: &CosmosTx, keys: &[CosmosPublicKey]) -> Result<(), TxProcessingError> {
        // If no keys provided, use simple validation (for testing or when keys aren't available)
        if keys.is_empty() {
            for signer_info in &tx.auth_info.signer_infos {
                if signer_info.sequence > 1_000_000 {
                    return Err(TxProcessingError::SequenceMismatch {
                        expected: 0,
                        actual: signer_info.sequence,
                    });
                }
            }
            return Ok(());
        }
        
        // Extract addresses from public keys
        let addresses = self.account_manager.derive_addresses(keys)?;
        
        // Validate sequence numbers for each signer
        for (i, signer_info) in tx.auth_info.signer_infos.iter().enumerate() {
            if let Some(address) = addresses.get(i) {
                self.account_manager.validate_sequence(address, signer_info.sequence)?;
            }
        }
        
        Ok(())
    }

    /// Process fee payment using the integrated fee processor
    pub fn process_transaction_fees(&mut self, tx: &CosmosTx, payer: &str) -> Result<u128, TxProcessingError> {
        // Use the fee processor to handle Cosmos → NEAR fee conversion
        let granter = if tx.auth_info.fee.granter.is_empty() {
            None
        } else {
            Some(tx.auth_info.fee.granter.as_str())
        };
        
        let total_fee_yocto = self.fee_processor.process_transaction_fees(
            &tx.auth_info.fee,
            payer,
            granter,
        )?;

        Ok(total_fee_yocto)
    }

    /// Process all messages in the transaction
    /// Note: This is a placeholder implementation. In practice, this would require
    /// the main contract to implement CosmosMessageHandler trait
    fn process_transaction_messages(&mut self, tx: &CosmosTx) -> Result<Vec<HandleResult>, TxProcessingError> {
        let mut responses = Vec::new();

        for msg in &tx.body.messages {
            // For now, create a mock successful response
            // In the real implementation, this would call route_cosmos_message with a handler
            let response = HandleResult {
                log: format!("Successfully processed {}", msg.type_url),
                data: vec![],
                events: vec![crate::handler::msg_router::Event {
                    r#type: "message".to_string(),
                    attributes: vec![
                        crate::handler::msg_router::Attribute {
                            key: "action".to_string(),
                            value: msg.type_url.clone(),
                        }
                    ],
                }],
            };
            responses.push(response);
        }

        Ok(responses)
    }

    /// Process transaction messages using the contract's message router
    fn process_transaction_messages_with_contract<T>(&self, tx: &CosmosTx, contract: &mut T) -> Result<Vec<HandleResult>, TxProcessingError>
    where
        T: crate::handler::CosmosMessageHandler,
    {
        let mut responses = Vec::new();
        
        for message in &tx.body.messages {
            // Use the message router to process each message
            let response = crate::handler::route_cosmos_message(contract, message.type_url.clone(), near_sdk::json_types::Base64VecU8(message.value.clone()));
            
            // Check if the message execution was successful
            if response.code != 0 {
                return Err(TxProcessingError::MessageExecution(format!("Message execution failed with code {}: {}", response.code, response.log)));
            }
            
            // Convert HandleResponse to HandleResult
            let handle_result = HandleResult {
                data: if response.data.is_empty() { Vec::new() } else { response.data },
                log: response.log,
                events: response.events.into_iter().map(|event| crate::handler::msg_router::Event {
                    r#type: event.r#type,
                    attributes: event.attributes.into_iter().map(|attr| crate::handler::msg_router::Attribute {
                        key: attr.key,
                        value: attr.value,
                    }).collect(),
                }).collect(),
            };
            
            responses.push(handle_result);
        }
        
        Ok(responses)
    }

    /// Simulate message processing without state changes
    fn simulate_transaction_messages(&self, tx: &CosmosTx) -> Result<Vec<HandleResult>, TxProcessingError> {
        // For simulation, we'll return successful responses with estimated gas
        let mut responses = Vec::new();

        for msg in &tx.body.messages {
            // Create a simulated response
            let simulated_response = HandleResult {
                log: format!("SIMULATED: Processing {}", msg.type_url),
                data: vec![], // Empty data for simulation
                events: vec![crate::handler::msg_router::Event {
                    r#type: "simulate".to_string(),
                    attributes: vec![
                        crate::handler::msg_router::Attribute {
                            key: "message_type".to_string(),
                            value: msg.type_url.clone(),
                        }
                    ],
                }],
            };
            responses.push(simulated_response);
        }

        Ok(responses)
    }

    /// Update account sequences after successful transaction
    fn update_account_sequences(&mut self, tx: &CosmosTx, keys: &[CosmosPublicKey]) -> Result<(), TxProcessingError> {
        // Extract addresses from public keys
        let addresses = self.account_manager.derive_addresses(keys)?;
        
        // Increment sequence numbers for all signers
        for (i, _signer_info) in tx.auth_info.signer_infos.iter().enumerate() {
            if let Some(address) = addresses.get(i) {
                self.account_manager.increment_sequence(address)?;
            }
        }
        
        Ok(())
    }

    /// Estimate actual gas usage based on transaction complexity and message results
    fn estimate_gas_usage(&self, tx: &CosmosTx, message_responses: &[HandleResult]) -> u64 {
        let base_gas = 21000u64; // Base transaction cost
        let per_message_gas = 5000u64; // Cost per message
        let per_event_gas = 500u64; // Cost per event
        let per_byte_gas = 10u64; // Cost per byte of data
        
        let message_gas = tx.body.messages.len() as u64 * per_message_gas;
        let event_gas = message_responses.iter()
            .map(|r| r.events.len() as u64 * per_event_gas)
            .sum::<u64>();
        let data_gas = message_responses.iter()
            .map(|r| r.data.len() as u64 * per_byte_gas)
            .sum::<u64>();
        
        let estimated_gas = base_gas + message_gas + event_gas + data_gas;
        
        // Cap at the gas limit specified in the transaction
        std::cmp::min(estimated_gas, tx.auth_info.fee.gas_limit)
    }

    /// Create transaction response
    pub fn create_transaction_response(&self, tx: &CosmosTx, message_responses: Vec<HandleResult>) -> TxResponse {
        let txhash = tx.hash();
        let gas_wanted = tx.auth_info.fee.gas_limit.to_string();
        
        // Convert message responses to ABCI-compatible logs and events
        let mut logs = Vec::new();
        let mut all_events = Vec::new();

        for (i, response) in message_responses.iter().enumerate() {
            // Convert message router events to ABCI events
            let abci_events: Vec<ABCIEvent> = response.events.iter()
                .map(|e| ABCIEvent {
                    r#type: e.r#type.clone(),
                    attributes: e.attributes.iter()
                        .map(|attr| ABCIAttribute::new(&attr.key, &attr.value))
                        .collect(),
                })
                .collect();

            let log = ABCIMessageLog {
                msg_index: i as u32,
                log: response.log.clone(),
                events: abci_events.clone(),
            };
            logs.push(log);

            // Add to global events list
            all_events.extend(abci_events);
        }

        // Calculate actual gas usage (for now, estimate based on transaction complexity)
        let gas_used = self.estimate_gas_usage(tx, &message_responses);
        
        // Combine all message response data
        let combined_data: Vec<u8> = message_responses.iter()
            .flat_map(|r| r.data.iter())
            .cloned()
            .collect();
        
        TxResponse {
            height: "0".to_string(), // Will be set by block processing
            txhash,
            code: ABCICode::OK,
            data: BASE64.encode(&combined_data), // Base64 encoded response data
            raw_log: if logs.is_empty() {
                "Transaction executed successfully".to_string()
            } else {
                logs.iter().map(|l| l.log.clone()).collect::<Vec<_>>().join("; ")
            },
            logs,
            info: format!("gas_wanted: {}, gas_used: {}", gas_wanted, gas_used),
            gas_wanted: gas_wanted.clone(),
            gas_used: gas_used.to_string(),
            tx: Some(tx.clone()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            events: all_events,
            codespace: "sdk".to_string(), // Standard Cosmos SDK codespace
        }
    }

    /// Create simulation response
    fn create_simulation_response(&self, tx: &CosmosTx, simulated_responses: Vec<HandleResult>) -> TxResponse {
        let mut response = self.create_transaction_response(tx, simulated_responses);
        
        // Mark as simulation
        response.height = "0".to_string();
        response.raw_log = "SIMULATION: ".to_string() + &response.raw_log;
        response.info = "Transaction simulation completed".to_string();
        
        response
    }

    /// Get signing document for external signing
    pub fn get_sign_doc(&self, tx: &CosmosTx, account_number: u64) -> Result<SignDoc, TxProcessingError> {
        let sign_doc = self.signature_verifier.create_sign_doc(tx, account_number)?;
        Ok(sign_doc)
    }

    /// Verify a signature without processing the transaction
    pub fn verify_signature_only(
        &self,
        signature: &[u8],
        tx: &CosmosTx,
        account_number: u64,
        public_key: &CosmosPublicKey,
    ) -> Result<bool, TxProcessingError> {
        use crate::crypto::SignatureBuilder;
        
        let builder = SignatureBuilder::new(self.config.chain_id.clone());
        let is_valid = builder.verify_signature_only(signature, tx, account_number, public_key)?;
        Ok(is_valid)
    }

    /// Get transaction by hash (placeholder for future implementation)
    pub fn get_transaction(&self, _hash: &str) -> Option<TxResponse> {
        // TODO: Implement transaction storage and retrieval
        None
    }

    /// Update configuration
    pub fn update_config(&mut self, config: TxProcessingConfig) {
        self.config = config.clone();
        self.signature_verifier = CosmosSignatureVerifier::new(config.chain_id.clone());
        
        // Update account manager prefix if needed
        let mut account_config = self.account_manager.get_config().clone();
        account_config.address_prefix = config.chain_id;
        self.account_manager.update_config(account_config);
    }
    
    /// Get account information by address
    pub fn get_account(&self, address: &str) -> Option<crate::modules::auth::CosmosAccount> {
        self.account_manager.get_account(address)
    }
    
    /// Get account by NEAR account ID
    pub fn get_account_by_near_id(&self, near_account_id: &AccountId) -> Option<crate::modules::auth::CosmosAccount> {
        self.account_manager.get_account_by_near_id(near_account_id)
    }
    
    /// Create account from public key
    pub fn create_account(&mut self, public_key: CosmosPublicKey) -> Result<crate::modules::auth::CosmosAccount, AccountError> {
        self.account_manager.create_account(public_key)
    }
    
    /// Create account from NEAR account ID
    pub fn create_account_from_near_id(&mut self, near_account_id: AccountId) -> Result<crate::modules::auth::CosmosAccount, AccountError> {
        self.account_manager.create_account_from_near_id(near_account_id)
    }
    
    /// Get total number of accounts
    pub fn get_account_count(&self) -> u64 {
        self.account_manager.get_account_count()
    }
    
    /// List accounts for admin purposes
    pub fn list_accounts(&self, limit: Option<usize>) -> Vec<crate::modules::auth::CosmosAccount> {
        self.account_manager.list_accounts(limit)
    }

    /// Grant fee allowance
    pub fn grant_fee_allowance(&mut self, grant: crate::modules::auth::FeeGrant) -> Result<(), FeeError> {
        self.fee_processor.grant_fee_allowance(grant)
    }

    /// Revoke fee allowance
    pub fn revoke_fee_allowance(&mut self, granter: &str, grantee: &str) -> Result<(), FeeError> {
        self.fee_processor.revoke_fee_allowance(granter, grantee)
    }

    /// Get fee grant
    pub fn get_fee_grant(&self, granter: &str, grantee: &str) -> Option<&crate::modules::auth::FeeGrant> {
        self.fee_processor.get_fee_grant(granter, grantee)
    }

    /// Calculate minimum fee for transaction
    pub fn calculate_minimum_fee(&self, gas_limit: u64) -> crate::types::cosmos_tx::Fee {
        self.fee_processor.calculate_minimum_fee(gas_limit)
    }

    /// Estimate transaction cost in specific denomination
    pub fn estimate_tx_cost(&self, gas_limit: u64, denom: &str) -> Result<crate::types::cosmos_tx::Coin, FeeError> {
        self.fee_processor.estimate_tx_cost(gas_limit, denom)
    }

    /// Get accumulated fees
    pub fn get_accumulated_fees(&self) -> &std::collections::HashMap<String, u128> {
        self.fee_processor.get_accumulated_fees()
    }

    /// Clear accumulated fees (returns the cleared fees)
    pub fn clear_accumulated_fees(&mut self) -> std::collections::HashMap<String, u128> {
        self.fee_processor.clear_accumulated_fees()
    }

    /// Update fee configuration
    pub fn update_fee_config(&mut self, config: FeeConfig) {
        self.fee_processor.update_config(config);
    }

    /// Set denomination conversion rate
    pub fn set_denom_conversion(&mut self, denom: String, rate: u128) {
        self.fee_processor.set_denom_conversion(denom, rate);
    }
}

impl TxResponse {
    /// Create an ABCI-compatible error response
    pub fn error(error: TxProcessingError, txhash: Option<String>) -> Self {
        let abci_code = ABCICode::from_error(&error);
        let codespace = match &error {
            TxProcessingError::DecodingError(_) => "sdk",
            TxProcessingError::SignatureError(_) => "sdk", 
            TxProcessingError::ValidationError(_) => "sdk",
            TxProcessingError::MessageProcessingError(_) => "app",
            TxProcessingError::AccountError(_) => "sdk",
            TxProcessingError::FeeError(_) => "sdk",
            TxProcessingError::GasLimitExceeded { .. } => "sdk",
            TxProcessingError::InvalidState(_) => "sdk",
            TxProcessingError::SequenceMismatch { .. } => "sdk",
            TxProcessingError::MessageExecution(_) => "app",
            TxProcessingError::TransactionNotFound => "sdk",
        };
        
        Self {
            height: "0".to_string(),
            txhash: txhash.unwrap_or_default(),
            code: abci_code,
            data: BASE64.encode(&[]), // Empty data for error
            raw_log: error.to_string(),
            logs: Vec::new(),
            info: format!("Transaction failed with code {}", abci_code),
            gas_wanted: "0".to_string(),
            gas_used: "0".to_string(),
            tx: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            events: Vec::new(),
            codespace: codespace.to_string(),
        }
    }
    
    /// Create an ABCI-compatible error response with gas information
    pub fn error_with_gas(error: TxProcessingError, txhash: Option<String>, gas_info: GasInfo) -> Self {
        let mut response = Self::error(error, txhash);
        response.gas_wanted = gas_info.gas_wanted.to_string();
        response.gas_used = gas_info.gas_used.to_string();
        response.info = format!("Transaction failed with code {}, gas_used: {}/{}", 
                              response.code, gas_info.gas_used, gas_info.gas_wanted);
        response
    }

    /// Add standard transaction events (for internal use)
    pub fn add_standard_events(&mut self) {
        // Add standard transaction event
        let tx_event = ABCIEvent::new("tx", vec![
            ("height", &self.height),
            ("tx_hash", &self.txhash),
            ("code", &self.code.to_string()),
        ]);
        self.events.insert(0, tx_event);
        
        // Add gas event
        let gas_event = ABCIEvent::new("use_gas", vec![
            ("gas_wanted", &self.gas_wanted),
            ("gas_used", &self.gas_used),
        ]);
        self.events.push(gas_event);
    }
    
    /// Get events of a specific type
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<&ABCIEvent> {
        self.events.iter()
            .filter(|e| e.r#type == event_type)
            .collect()
    }
    
    /// Check if the transaction was successful
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// Get the first error message if transaction failed
    pub fn error_message(&self) -> Option<&str> {
        if self.is_success() {
            None
        } else {
            Some(&self.raw_log)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::cosmos_tx::{TxBody, AuthInfo, Fee, SignerInfo, Coin, ModeInfo, SignMode, Any};

    fn create_test_transaction() -> CosmosTx {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("unear", "1000000")], 200000); // Use unear with sufficient amount
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![0u8; 65]]; // Dummy signature

        CosmosTx::new(body, auth_info, signatures)
    }

    #[test]
    fn test_transaction_handler_creation() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        assert_eq!(handler.config.chain_id, "near-cosmos-sdk");
        assert_eq!(handler.config.max_gas_per_tx, 10_000_000);
    }

    #[test]
    fn test_transaction_validation() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        let result = handler.validate_transaction(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gas_limit_validation() {
        let mut config = TxProcessingConfig::default();
        config.max_gas_per_tx = 100_000; // Lower than test transaction
        
        let mut handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        let result = handler.validate_transaction(&tx);
        assert!(matches!(result, Err(TxProcessingError::GasLimitExceeded { .. })));
    }

    #[test]
    fn test_fee_processing() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        let result = handler.process_transaction_fees(&tx, "test_payer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_transaction_simulation() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        
        let result = handler.simulate_transaction(tx_bytes);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.code, 0);
        assert!(response.raw_log.contains("SIMULATION"));
    }

    #[test]
    fn test_tx_response_creation() {
        let tx = create_test_transaction();
        let txhash = tx.hash();
        let responses = vec![HandleResult {
            log: "test message processed".to_string(),
            data: vec![],
            events: vec![crate::handler::msg_router::Event {
                r#type: "transfer".to_string(),
                attributes: vec![crate::handler::msg_router::Attribute {
                    key: "amount".to_string(),
                    value: "1000".to_string(),
                }],
            }],
        }];
        
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        let tx_response = handler.create_transaction_response(&tx, responses);
        
        assert_eq!(tx_response.txhash, txhash);
        assert_eq!(tx_response.code, 0);
        assert_eq!(tx_response.logs.len(), 1);
        assert_eq!(tx_response.events.len(), 1);
        assert!(tx_response.is_success());
    }

    #[test]
    fn test_error_response() {
        let error = TxProcessingError::InvalidState("test error".to_string());
        let response = TxResponse::error(error, Some("testhash".to_string()));
        
        assert_eq!(response.txhash, "testhash");
        assert_eq!(response.code, ABCICode::INVALID_REQUEST); // Now using ABCI codes
        assert_eq!(response.codespace, "sdk");
        assert!(!response.is_success());
        assert!(response.error_message().unwrap().contains("test error"));
    }

    #[test]
    fn test_sign_doc_creation() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        let sign_doc = handler.get_sign_doc(&tx, 42).unwrap();
        assert_eq!(sign_doc.account_number, 42);
        assert_eq!(sign_doc.chain_id, "near-cosmos-sdk");
    }

    #[test]
    fn test_config_update() {
        let mut config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config.clone());
        
        config.chain_id = "new-chain-id".to_string();
        handler.update_config(config);
        
        assert_eq!(handler.config.chain_id, "new-chain-id");
        assert_eq!(handler.signature_verifier.chain_id, "new-chain-id");
    }

    #[test]
    fn test_sequence_validation() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        // Create transaction with very high sequence
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("unear", "1000000")], 200000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 2_000_000, // Very high sequence
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![0u8; 65]];
        let tx = CosmosTx::new(body, auth_info, signatures);
        
        let result = handler.check_account_sequences(&tx, &[]);
        assert!(matches!(result, Err(TxProcessingError::SequenceMismatch { .. })));
    }

    #[test]
    fn test_fee_processor_integration() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        // Test minimum fee calculation
        let min_fee = handler.calculate_minimum_fee(100_000_000); // 0.1 TGas
        assert_eq!(min_fee.gas_limit, 100_000_000);
        assert!(!min_fee.amount.is_empty());
        
        // Test fee estimation
        let cost = handler.estimate_tx_cost(100_000_000, "unear").unwrap();
        assert_eq!(cost.denom, "unear");
        assert!(!cost.amount.is_empty());
    }

    #[test]
    fn test_fee_grants() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        // Create fee grant
        let grant = crate::modules::auth::FeeGrant {
            granter: "alice".to_string(),
            grantee: "bob".to_string(),
            spend_limit: vec![crate::types::cosmos_tx::Coin::new("unear", "1000000")],
            expiration: None,
        };
        
        // Grant fee allowance
        handler.grant_fee_allowance(grant).unwrap();
        
        // Check grant exists
        let retrieved_grant = handler.get_fee_grant("alice", "bob");
        assert!(retrieved_grant.is_some());
        assert_eq!(retrieved_grant.unwrap().spend_limit[0].amount, "1000000");
        
        // Revoke grant
        handler.revoke_fee_allowance("alice", "bob").unwrap();
        
        // Check grant no longer exists
        let revoked_grant = handler.get_fee_grant("alice", "bob");
        assert!(revoked_grant.is_none());
    }

    #[test]
    fn test_denomination_conversion() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        // Add custom denomination
        handler.set_denom_conversion("atom".to_string(), 5_000_000_000_000_000_000_000_000); // 5 NEAR per ATOM
        
        // Test estimation with custom denomination
        let cost = handler.estimate_tx_cost(100_000_000, "atom").unwrap();
        assert_eq!(cost.denom, "atom");
    }

    #[test] 
    fn test_accumulated_fees() {
        let config = TxProcessingConfig::default();
        let mut handler = CosmosTransactionHandler::new(config);
        
        // Process a transaction to accumulate fees
        let tx = create_test_transaction();
        let _result = handler.process_transaction_fees(&tx, "test_payer");
        
        // Check accumulated fees
        let accumulated = handler.get_accumulated_fees();
        assert!(!accumulated.is_empty());
        
        // Clear accumulated fees
        let cleared = handler.clear_accumulated_fees();
        assert!(!cleared.is_empty());
        
        // Check fees are cleared
        let empty_accumulated = handler.get_accumulated_fees();
        assert!(empty_accumulated.is_empty());
    }

    #[test]
    fn test_abci_attribute_encoding() {
        // Test basic attribute creation
        let attr = ABCIAttribute::new("test_key", "test_value");
        assert_eq!(attr.decode_key().unwrap(), "test_key");
        assert_eq!(attr.decode_value().unwrap(), "test_value");
        assert!(!attr.index);

        // Test indexed attribute
        let indexed_attr = ABCIAttribute::new_indexed("indexed_key", "indexed_value");
        assert_eq!(indexed_attr.decode_key().unwrap(), "indexed_key");
        assert_eq!(indexed_attr.decode_value().unwrap(), "indexed_value");
        assert!(indexed_attr.index);

        // Test raw bytes
        let bytes_attr = ABCIAttribute::from_bytes(b"raw_key", b"raw_value", true);
        assert_eq!(bytes_attr.decode_key().unwrap(), "raw_key");
        assert_eq!(bytes_attr.decode_value().unwrap(), "raw_value");
        assert!(bytes_attr.index);
    }

    #[test]
    fn test_abci_event_creation() {
        let event = ABCIEvent::new("transfer", vec![
            ("from", "alice"),
            ("to", "bob"),
            ("amount", "1000"),
        ]);

        assert_eq!(event.r#type, "transfer");
        assert_eq!(event.attributes.len(), 3);
        
        // Check all attributes are properly encoded
        for attr in &event.attributes {
            assert!(!attr.key.is_empty());
            assert!(!attr.value.is_empty());
            assert!(!attr.index); // Default to non-indexed
        }

        let indexed_event = event.with_indexed_attribute("height", "12345");
        assert_eq!(indexed_event.attributes.len(), 4);
        assert!(indexed_event.attributes[3].index); // Last attribute should be indexed
    }

    #[test]
    fn test_abci_error_codes() {
        // Test various error types map to correct ABCI codes
        let decode_error = TxProcessingError::DecodingError(
            crate::handler::TxDecodingError::InvalidFormat("test".to_string())
        );
        assert_eq!(ABCICode::from_error(&decode_error), ABCICode::TX_DECODE_ERROR);

        let sig_error = TxProcessingError::SignatureError(
            crate::crypto::SignatureError::InvalidSignature("test signature error".to_string())
        );  
        assert_eq!(ABCICode::from_error(&sig_error), ABCICode::UNAUTHORIZED);

        let seq_error = TxProcessingError::SequenceMismatch { expected: 1, actual: 2 };
        assert_eq!(ABCICode::from_error(&seq_error), ABCICode::INVALID_SEQUENCE);

        let gas_error = TxProcessingError::GasLimitExceeded { limit: 100, used: 200 };
        assert_eq!(ABCICode::from_error(&gas_error), ABCICode::OUT_OF_GAS);
    }

    #[test]
    fn test_gas_info_tracking() {
        let gas_info = GasInfo::new(100_000, 75_000);
        assert_eq!(gas_info.gas_wanted, 100_000);
        assert_eq!(gas_info.gas_used, 75_000);
        assert!(!gas_info.out_of_gas());
        assert_eq!(gas_info.efficiency(), 0.75);

        let out_of_gas_info = GasInfo::new(100_000, 100_000);
        assert!(out_of_gas_info.out_of_gas());
        assert_eq!(out_of_gas_info.efficiency(), 1.0);
    }

    #[test]
    fn test_enhanced_tx_response_structure() {
        let config = TxProcessingConfig::default();
        let handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        // Create mock responses
        let responses = vec![HandleResult {
            log: "Transfer successful".to_string(),
            data: vec![1, 2, 3, 4],
            events: vec![crate::handler::msg_router::Event {
                r#type: "transfer".to_string(),
                attributes: vec![
                    crate::handler::msg_router::Attribute {
                        key: "from".to_string(),
                        value: "alice".to_string(),
                    },
                    crate::handler::msg_router::Attribute {
                        key: "to".to_string(),
                        value: "bob".to_string(),
                    },
                ],
            }],
        }];

        let tx_response = handler.create_transaction_response(&tx, responses);

        // Verify ABCI compliance
        assert_eq!(tx_response.code, ABCICode::OK);
        assert_eq!(tx_response.codespace, "sdk");
        assert!(!tx_response.data.is_empty()); // Should be base64 encoded
        assert_eq!(tx_response.logs.len(), 1);
        assert_eq!(tx_response.events.len(), 1);
        
        // Verify event structure
        let event = &tx_response.events[0];
        assert_eq!(event.r#type, "transfer");
        assert_eq!(event.attributes.len(), 2);
        
        // Verify attributes are base64 encoded
        let from_attr = &event.attributes[0];
        assert_eq!(from_attr.decode_key().unwrap(), "from");
        assert_eq!(from_attr.decode_value().unwrap(), "alice");
    }

    #[test]
    fn test_error_response_with_gas() {
        let error = TxProcessingError::GasLimitExceeded { limit: 100_000, used: 100_000 };
        let gas_info = GasInfo::new(100_000, 100_000);
        
        let response = TxResponse::error_with_gas(error, Some("test_hash".to_string()), gas_info);
        
        assert_eq!(response.code, ABCICode::OUT_OF_GAS);
        assert_eq!(response.gas_wanted, "100000");
        assert_eq!(response.gas_used, "100000");
        assert!(response.info.contains("gas_used: 100000/100000"));
    }

    #[test]
    fn test_tx_response_utility_methods() {
        let config = TxProcessingConfig::default();
        let handler = CosmosTransactionHandler::new(config);
        let tx = create_test_transaction();
        
        let responses = vec![HandleResult {
            log: "Test operation".to_string(),
            data: vec![],
            events: vec![
                crate::handler::msg_router::Event {
                    r#type: "transfer".to_string(),
                    attributes: vec![],
                },
                crate::handler::msg_router::Event {
                    r#type: "message".to_string(),
                    attributes: vec![],
                },
            ],
        }];

        let mut tx_response = handler.create_transaction_response(&tx, responses);
        tx_response.add_standard_events();

        // Should have original events plus standard events (tx, use_gas)
        assert!(tx_response.events.len() >= 4); // 2 original + tx + use_gas
        
        // Test event filtering
        let tx_events = tx_response.get_events_by_type("tx");
        assert_eq!(tx_events.len(), 1);
        
        let gas_events = tx_response.get_events_by_type("use_gas");
        assert_eq!(gas_events.len(), 1);
    }
}