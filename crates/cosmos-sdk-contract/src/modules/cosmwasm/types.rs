use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::U128;

/// Re-export of core CosmWasm types that contracts expect
/// These types maintain compatibility with cosmwasm-std

/// Represents a blockchain address (compatible with both NEAR and Cosmos addresses)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Addr(String);

impl Addr {
    /// Create an address without validation (for internal use)
    pub fn unchecked<S: Into<String>>(input: S) -> Self {
        Addr(input.into())
    }

    /// Get the address as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to String
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for Addr {
    fn from(s: String) -> Self {
        Addr(s)
    }
}

impl From<&str> for Addr {
    fn from(s: &str) -> Self {
        Addr(s.to_string())
    }
}

/// Represents a coin with denomination and amount
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Coin {
    pub denom: String,
    pub amount: Uint128,
}

/// A wrapper around u128 for CosmWasm compatibility
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint128(pub u128);

impl Uint128 {
    pub fn new(value: u128) -> Self {
        Uint128(value)
    }

    pub fn zero() -> Self {
        Uint128(0)
    }

    pub fn u128(&self) -> u128 {
        self.0
    }
}

impl From<u128> for Uint128 {
    fn from(value: u128) -> Self {
        Uint128(value)
    }
}

impl From<U128> for Uint128 {
    fn from(value: U128) -> Self {
        Uint128(value.0)
    }
}

impl From<Uint128> for U128 {
    fn from(value: Uint128) -> Self {
        U128(value.0)
    }
}

/// Binary data wrapper for CosmWasm compatibility
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Binary(#[serde(with = "base64_serde")] Vec<u8>);

impl Binary {
    pub fn from_base64(encoded: &str) -> Result<Self, base64::DecodeError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let decoded = STANDARD.decode(encoded)?;
        Ok(Binary(decoded))
    }

    pub fn to_base64(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(&self.0)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u8>> for Binary {
    fn from(vec: Vec<u8>) -> Self {
        Binary(vec)
    }
}

impl From<&[u8]> for Binary {
    fn from(slice: &[u8]) -> Self {
        Binary(slice.to_vec())
    }
}

/// Custom base64 serialization module
mod base64_serde {
    use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
    use base64::{engine::general_purpose::STANDARD, Engine};

    pub fn serialize<S: Serializer>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
        STANDARD.encode(data).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(&encoded).map_err(|e| near_sdk::serde::de::Error::custom(e.to_string()))
    }
}

/// Timestamp in nanoseconds
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub nanos: u64,
}

impl Timestamp {
    pub fn from_nanos(nanos: u64) -> Self {
        Timestamp { nanos }
    }

    pub fn from_seconds(seconds: u64) -> Self {
        Timestamp {
            nanos: seconds * 1_000_000_000,
        }
    }

    pub fn seconds(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    pub fn nanos(&self) -> u64 {
        self.nanos
    }
}

/// Block information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockInfo {
    pub height: u64,
    pub time: Timestamp,
    pub chain_id: String,
}

/// Transaction information (optional in CosmWasm)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionInfo {
    pub index: Option<u32>,
}

/// Contract information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractInfo {
    pub address: Addr,
}

/// Attribute for events
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

/// Event emitted by contract
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub ty: String,
    pub attributes: Vec<Attribute>,
}

impl Event {
    pub fn new(ty: impl Into<String>) -> Self {
        Event {
            ty: ty.into(),
            attributes: vec![],
        }
    }

    pub fn add_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(Attribute {
            key: key.into(),
            value: value.into(),
        });
        self
    }
}

/// Sub-message for cross-contract calls
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubMsg<T = Empty> {
    pub id: u64,
    pub msg: CosmosMsg<T>,
    pub gas_limit: Option<u64>,
    pub reply_on: ReplyOn,
}

/// Reply behavior for sub-messages
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum ReplyOn {
    Always,
    Error,
    Success,
    Never,
}

/// CosmWasm message types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CosmosMsg<T = Empty> {
    Bank(BankMsg),
    Custom(T),
    Wasm(WasmMsg),
}

/// Bank module messages
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BankMsg {
    Send {
        to_address: String,
        amount: Vec<Coin>,
    },
    Burn {
        amount: Vec<Coin>,
    },
}

/// Wasm module messages
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WasmMsg {
    Execute {
        contract_addr: String,
        msg: Binary,
        funds: Vec<Coin>,
    },
    Instantiate {
        admin: Option<String>,
        code_id: u64,
        msg: Binary,
        funds: Vec<Coin>,
        label: String,
    },
    Migrate {
        contract_addr: String,
        new_code_id: u64,
        msg: Binary,
    },
}

/// Empty type for generic parameters
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Empty {}

/// Order for range queries
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Order {
    Ascending,
    Descending,
}

/// Record type for range query results
pub type Record = (Vec<u8>, Vec<u8>);

/// Standard error types that match CosmWasm
#[derive(Debug, Clone, PartialEq)]
pub enum StdError {
    GenericErr { msg: String },
    InvalidBase64 { msg: String },
    InvalidDataSize { expected: usize, actual: usize },
    InvalidUtf8 { msg: String },
    NotFound { kind: String },
    ParseErr { target_type: String, msg: String },
    SerializeErr { source_type: String, msg: String },
}

impl StdError {
    pub fn generic_err<S: Into<String>>(msg: S) -> Self {
        StdError::GenericErr { msg: msg.into() }
    }

    pub fn invalid_base64<S: Into<String>>(msg: S) -> Self {
        StdError::InvalidBase64 { msg: msg.into() }
    }

    pub fn invalid_utf8<S: Into<String>>(msg: S) -> Self {
        StdError::InvalidUtf8 { msg: msg.into() }
    }

    pub fn not_found<S: Into<String>>(kind: S) -> Self {
        StdError::NotFound { kind: kind.into() }
    }

    pub fn parse_err<S: Into<String>, T: Into<String>>(target: S, msg: T) -> Self {
        StdError::ParseErr {
            target_type: target.into(),
            msg: msg.into(),
        }
    }

    pub fn serialize_err<S: Into<String>, T: Into<String>>(source: S, msg: T) -> Self {
        StdError::SerializeErr {
            source_type: source.into(),
            msg: msg.into(),
        }
    }
}

impl std::fmt::Display for StdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StdError::GenericErr { msg } => write!(f, "Generic error: {}", msg),
            StdError::InvalidBase64 { msg } => write!(f, "Invalid base64: {}", msg),
            StdError::InvalidDataSize { expected, actual } => {
                write!(f, "Invalid data size: expected {}, got {}", expected, actual)
            }
            StdError::InvalidUtf8 { msg } => write!(f, "Invalid UTF-8: {}", msg),
            StdError::NotFound { kind } => write!(f, "Not found: {}", kind),
            StdError::ParseErr { target_type, msg } => {
                write!(f, "Parse error for type {}: {}", target_type, msg)
            }
            StdError::SerializeErr { source_type, msg } => {
                write!(f, "Serialize error for type {}: {}", source_type, msg)
            }
        }
    }
}

impl std::error::Error for StdError {}

/// Standard result type
pub type StdResult<T> = Result<T, StdError>;

/// Response type for contract execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response<T = Empty> {
    pub messages: Vec<SubMsg<T>>,
    pub attributes: Vec<Attribute>,
    pub events: Vec<Event>,
    pub data: Option<Binary>,
}

impl<T> Default for Response<T> {
    fn default() -> Self {
        Response {
            messages: vec![],
            attributes: vec![],
            events: vec![],
            data: None,
        }
    }
}

impl<T> Response<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(Attribute {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    pub fn add_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    pub fn add_message(mut self, msg: SubMsg<T>) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn set_data(mut self, data: impl Into<Binary>) -> Self {
        self.data = Some(data.into());
        self
    }
}

/// Environment information passed to contracts
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Env {
    pub block: BlockInfo,
    pub transaction: Option<TransactionInfo>,
    pub contract: ContractInfo,
}

/// Message info containing sender and funds
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageInfo {
    pub sender: Addr,
    pub funds: Vec<Coin>,
}

/// Storage trait that CosmWasm contracts expect
pub trait Storage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn remove(&mut self, key: &[u8]);
}

/// API trait for address and crypto operations
pub trait Api {
    fn addr_validate(&self, human: &str) -> StdResult<Addr>;
    fn addr_canonicalize(&self, human: &str) -> StdResult<Vec<u8>>;
    fn addr_humanize(&self, canonical: &[u8]) -> StdResult<Addr>;
    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> StdResult<bool>;
    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_id: u8,
    ) -> StdResult<Vec<u8>>;
    fn ed25519_verify(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> StdResult<bool>;
}

/// Querier wrapper for external queries
#[derive(Clone)]
pub struct QuerierWrapper<'a> {
    querier: &'a dyn Querier,
}

impl<'a> QuerierWrapper<'a> {
    pub fn new(querier: &'a dyn Querier) -> Self {
        QuerierWrapper { querier }
    }

    pub fn query_balance(&self, address: impl Into<String>, denom: impl Into<String>) -> StdResult<Coin> {
        self.querier.query_balance(address.into(), denom.into())
    }
}

/// Querier trait for external state queries
pub trait Querier {
    fn query_balance(&self, address: String, denom: String) -> StdResult<Coin>;
}

/// Deps - immutable dependencies for query handlers
pub struct Deps<'a> {
    pub storage: &'a dyn Storage,
    pub api: &'a dyn Api,
    pub querier: QuerierWrapper<'a>,
}

/// DepsMut - mutable dependencies for execute handlers
pub struct DepsMut<'a> {
    pub storage: &'a mut dyn Storage,
    pub api: &'a dyn Api,
    pub querier: QuerierWrapper<'a>,
}