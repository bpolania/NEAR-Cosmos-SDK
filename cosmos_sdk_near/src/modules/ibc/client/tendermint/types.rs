use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// IBC Height - identifies a specific block height on a chain
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Height {
    /// The revision number of the chain (typically 0 for Tendermint chains)
    pub revision_number: u64,
    /// The block height within the revision
    pub revision_height: u64,
}

/// Fraction representing a trust level (e.g., 1/3 for Tendermint)
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}

/// IBC Client State for Tendermint light client
/// 
/// Contains chain-specific information and light client parameters
/// required for verifying updates and performing state verification.
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct ClientState {
    /// Chain ID of the Tendermint chain being tracked
    pub chain_id: String,
    
    /// Trust level threshold for header verification (typically 1/3)
    pub trust_level: Fraction,
    
    /// Duration in seconds for which consensus states are considered trusted
    pub trust_period: u64,
    
    /// Duration in seconds of the unbonding period on the counterparty chain
    pub unbonding_period: u64,
    
    /// Maximum allowed clock drift between chains in seconds
    pub max_clock_drift: u64,
    
    /// The latest height that has been verified
    pub latest_height: Height,
    
    /// Proof specifications for Merkle proof verification
    pub proof_specs: Vec<ProofSpec>,
    
    /// Path to upgrade keys for client upgrades
    pub upgrade_path: Vec<String>,
    
    /// Whether to allow updates after the trust period has expired
    pub allow_update_after_expiry: bool,
    
    /// Whether to allow updates after client misbehaviour detection
    pub allow_update_after_misbehaviour: bool,
}

/// IBC Consensus State for Tendermint
/// 
/// Represents the consensus state at a specific height, containing
/// the necessary information to verify proofs against that height.
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct ConsensusState {
    /// Block timestamp from the header
    pub timestamp: u64,
    
    /// Application hash (Merkle root of application state)
    pub root: Vec<u8>,
    
    /// Hash of the next validator set
    pub next_validators_hash: Vec<u8>,
}

/// Tendermint block header information
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Header {
    /// The signed header containing the block header and commit
    pub signed_header: SignedHeader,
    
    /// The validator set that signed this header
    pub validator_set: ValidatorSet,
    
    /// The next validator set (for validator set changes)
    pub trusted_height: Height,
    
    /// Trusted validator set (used for verification)
    pub trusted_validators: ValidatorSet,
}

/// Signed header containing block header and commit information
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct SignedHeader {
    /// The block header
    pub header: BlockHeader,
    
    /// The commit containing validator signatures
    pub commit: Commit,
}

/// Tendermint block header
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct BlockHeader {
    /// Version information
    pub version: Consensus,
    
    /// Chain ID
    pub chain_id: String,
    
    /// Block height
    pub height: u64,
    
    /// Block timestamp (Unix timestamp in seconds)
    pub time: u64,
    
    /// Hash of the previous block
    pub last_block_id: BlockId,
    
    /// Hash of the last commit
    pub last_commit_hash: Vec<u8>,
    
    /// Hash of the data (transactions)
    pub data_hash: Vec<u8>,
    
    /// Hash of the validator set
    pub validators_hash: Vec<u8>,
    
    /// Hash of the next validator set
    pub next_validators_hash: Vec<u8>,
    
    /// Hash of the consensus parameters
    pub consensus_hash: Vec<u8>,
    
    /// Application hash (Merkle root of application state)
    pub app_hash: Vec<u8>,
    
    /// Hash of the last results
    pub last_results_hash: Vec<u8>,
    
    /// Hash of the evidence
    pub evidence_hash: Vec<u8>,
    
    /// Address of the proposer
    pub proposer_address: Vec<u8>,
}

/// Consensus version information
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Consensus {
    pub block: u64,
    pub app: u64,
}

/// Block ID containing hash and part set header
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct BlockId {
    pub hash: Vec<u8>,
    pub part_set_header: PartSetHeader,
}

/// Part set header for block parts
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct PartSetHeader {
    pub total: u32,
    pub hash: Vec<u8>,
}

/// Commit containing validator signatures
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Commit {
    /// Block height
    pub height: u64,
    
    /// Round number
    pub round: i32,
    
    /// Block ID being committed
    pub block_id: BlockId,
    
    /// Validator signatures
    pub signatures: Vec<CommitSig>,
}

/// Individual validator signature in a commit
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct CommitSig {
    /// Type of signature (commit, nil, absent)
    pub block_id_flag: u8,
    
    /// Validator address
    pub validator_address: Vec<u8>,
    
    /// Timestamp of the signature
    pub timestamp: u64,
    
    /// The signature bytes
    pub signature: Option<Vec<u8>>,
}

/// Validator set information
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorSet {
    /// List of validators
    pub validators: Vec<Validator>,
    
    /// Proposer for this validator set
    pub proposer: Option<Validator>,
    
    /// Total voting power
    pub total_voting_power: i64,
}

/// Individual validator information
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct Validator {
    /// Validator address (derived from public key)
    pub address: Vec<u8>,
    
    /// Public key for signature verification
    pub pub_key: PublicKey,
    
    /// Voting power of this validator
    pub voting_power: i64,
    
    /// Proposer priority
    pub proposer_priority: i64,
}

/// Public key types supported by Tendermint
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub enum PublicKey {
    /// Ed25519 public key (32 bytes)
    Ed25519(Vec<u8>),
    
    /// Secp256k1 public key (33 bytes compressed)
    Secp256k1(Vec<u8>),
}

/// Proof specification for Merkle proof verification
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct ProofSpec {
    /// Specification for leaf nodes
    pub leaf_spec: LeafOp,
    
    /// Specification for inner nodes
    pub inner_spec: InnerSpec,
    
    /// Maximum depth of the tree (0 for no limit)
    pub max_depth: i32,
    
    /// Minimum depth of the tree
    pub min_depth: i32,
}

/// Leaf operation specification for Merkle proofs
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct LeafOp {
    /// Hash function for the leaf
    pub hash: HashOp,
    
    /// Hash function for the key before hashing
    pub prehash_key: HashOp,
    
    /// Hash function for the value before hashing
    pub prehash_value: HashOp,
    
    /// Length operation for variable-length encoding
    pub length: LengthOp,
    
    /// Prefix bytes for the leaf
    pub prefix: Vec<u8>,
}

/// Inner node operation specification for Merkle proofs
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub struct InnerSpec {
    /// Order of child hashes (0 = left, 1 = right)
    pub child_order: Vec<i32>,
    
    /// Size of child hash in bytes
    pub child_size: i32,
    
    /// Minimum prefix length
    pub min_prefix_length: i32,
    
    /// Maximum prefix length
    pub max_prefix_length: i32,
    
    /// Empty child representation
    pub empty_child: Vec<u8>,
    
    /// Hash function for inner nodes
    pub hash: HashOp,
}

/// Hash operation types for Merkle proofs
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub enum HashOp {
    NoHash,
    Sha256,
    Sha512,
    Keccak,
    Ripemd160,
    Bitcoin,
}

/// Length operation types for variable-length encoding
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema)]
pub enum LengthOp {
    NoPrefix,
    VarProto,
    VarRlp,
    Fixed32Big,
    Fixed32Little,
    Fixed64Big,
    Fixed64Little,
    Require32Bytes,
    Require64Bytes,
}

// === Implementation helpers ===

impl Height {
    /// Create a new height
    pub fn new(revision_number: u64, revision_height: u64) -> Self {
        Self {
            revision_number,
            revision_height,
        }
    }
    
    /// Check if this height is greater than another
    pub fn is_greater_than(&self, other: &Height) -> bool {
        if self.revision_number != other.revision_number {
            self.revision_number > other.revision_number
        } else {
            self.revision_height > other.revision_height
        }
    }
}

impl Fraction {
    /// Create a new fraction and validate it
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        if denominator == 0 {
            return Err("Denominator cannot be zero".to_string());
        }
        if numerator > denominator {
            return Err("Numerator cannot be greater than denominator".to_string());
        }
        Ok(Self { numerator, denominator })
    }
    
    /// Get the fraction as a decimal value
    pub fn as_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl PublicKey {
    /// Get the raw bytes of the public key
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PublicKey::Ed25519(bytes) => bytes,
            PublicKey::Secp256k1(bytes) => bytes,
        }
    }
    
    /// Get the type identifier for this public key
    pub fn type_url(&self) -> &'static str {
        match self {
            PublicKey::Ed25519(_) => "/cosmos.crypto.ed25519.PubKey",
            PublicKey::Secp256k1(_) => "/cosmos.crypto.secp256k1.PubKey",
        }
    }
}