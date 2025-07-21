use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, near_bindgen, PanicOnDefault};

mod types;
mod crypto;
mod verification;

use types::{ClientState, ConsensusState, Header, Height};
use verification::{verify_header, validate_client_state, is_consensus_state_expired};
use crypto::verify_merkle_proof;

/// IBC Tendermint Light Client Contract for NEAR Protocol
/// 
/// This contract implements the 07-tendermint IBC light client specification
/// as a NEAR smart contract, enabling verification of Tendermint/CometBFT
/// consensus for cross-chain communication via IBC.
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TendermintLightClient {
    /// Mapping from client_id to ClientState
    client_states: LookupMap<String, ClientState>,
    
    /// Mapping from (client_id, height) to ConsensusState
    /// Key format: "{client_id}#{height}"
    consensus_states: LookupMap<String, ConsensusState>,
    
    /// Counter for generating unique client IDs
    next_client_sequence: u64,
}

#[near_bindgen]
impl TendermintLightClient {
    /// Initialize the IBC Tendermint Light Client contract
    #[init]
    pub fn new() -> Self {
        Self {
            client_states: LookupMap::new(b"c"),
            consensus_states: LookupMap::new(b"s"),
            next_client_sequence: 0,
        }
    }

    /// Create a new Tendermint light client instance
    /// 
    /// This function initializes a new light client with an initial trusted
    /// header and validator set. The client will be assigned a unique client ID.
    /// 
    /// # Arguments
    /// * `chain_id` - The chain ID of the Tendermint chain being tracked
    /// * `trust_period` - Duration in seconds for which consensus states are trusted
    /// * `unbonding_period` - Duration in seconds of the unbonding period
    /// * `max_clock_drift` - Maximum allowed clock drift in seconds
    /// * `initial_header` - The initial trusted block header
    /// 
    /// # Returns
    /// * The generated client ID for this light client instance
    pub fn create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
        initial_header: Header,
    ) -> String {
        // Generate unique client ID
        let client_id = format!("07-tendermint-{}", self.next_client_sequence);
        self.next_client_sequence += 1;

        // Validate initial header
        self.validate_initial_header(&initial_header);

        // Validate client state parameters
        if let Err(err) = validate_client_state(&ClientState {
            chain_id: chain_id.clone(),
            trust_level: types::Fraction { numerator: 1, denominator: 3 },
            trust_period,
            unbonding_period,
            max_clock_drift,
            latest_height: Height { revision_number: 0, revision_height: initial_header.signed_header.header.height },
            proof_specs: self.get_tendermint_proof_specs(),
            upgrade_path: vec!["upgrade".to_string(), "upgradedIBCState".to_string()],
            allow_update_after_expiry: false,
            allow_update_after_misbehaviour: false,
        }) {
            env::panic_str(&format!("Invalid client state: {}", err));
        }

        // Extract height and timestamp from header
        let height = Height {
            revision_number: 0, // Tendermint chains typically use revision 0
            revision_height: initial_header.signed_header.header.height,
        };
        
        let timestamp = initial_header.signed_header.header.time;
        let app_hash = initial_header.signed_header.header.app_hash.clone();
        let next_validators_hash = initial_header.signed_header.header.next_validators_hash.clone();

        // Create ClientState
        let client_state = ClientState {
            chain_id: chain_id.clone(),
            trust_level: types::Fraction { numerator: 1, denominator: 3 }, // 1/3 trust level
            trust_period,
            unbonding_period,
            max_clock_drift,
            latest_height: height.clone(),
            proof_specs: self.get_tendermint_proof_specs(),
            upgrade_path: vec!["upgrade".to_string(), "upgradedIBCState".to_string()],
            allow_update_after_expiry: false,
            allow_update_after_misbehaviour: false,
        };

        // Create initial ConsensusState
        let consensus_state = ConsensusState {
            timestamp,
            root: app_hash,
            next_validators_hash,
        };

        // Store ClientState and ConsensusState
        self.client_states.insert(&client_id, &client_state);
        
        let consensus_key = format!("{}#{}", client_id, height.revision_height);
        self.consensus_states.insert(&consensus_key, &consensus_state);

        env::log_str(&format!(
            "Created Tendermint light client {} for chain {} at height {}",
            client_id, chain_id, height.revision_height
        ));

        client_id
    }

    /// Update an existing light client with a new header
    /// 
    /// This function verifies a new header against the current trusted state
    /// and updates the light client if verification succeeds.
    /// 
    /// # Arguments
    /// * `client_id` - The ID of the client to update
    /// * `header` - The new header to verify and add
    /// 
    /// # Returns
    /// * Success or failure of the update operation
    pub fn update_client(&mut self, client_id: String, header: Header) -> bool {
        // Get current client state
        let mut client_state = match self.client_states.get(&client_id) {
            Some(state) => state,
            None => {
                env::log_str(&format!("Client {} not found", client_id));
                return false;
            }
        };

        // Get the trusted consensus state for verification
        let trusted_consensus_key = format!("{}#{}", client_id, client_state.latest_height.revision_height);
        let trusted_consensus_state = match self.consensus_states.get(&trusted_consensus_key) {
            Some(state) => state,
            None => {
                env::log_str("Trusted consensus state not found");
                return false;
            }
        };

        // Verify the new header against current trusted state
        if let Err(err) = verify_header(&client_state, &trusted_consensus_state, &header) {
            env::log_str(&format!("Header verification failed: {}", err));
            return false;
        }
        
        let new_height = Height {
            revision_number: 0,
            revision_height: header.signed_header.header.height,
        };

        // Create new consensus state
        let consensus_state = ConsensusState {
            timestamp: header.signed_header.header.time,
            root: header.signed_header.header.app_hash.clone(),
            next_validators_hash: header.signed_header.header.next_validators_hash.clone(),
        };

        // Update client state
        client_state.latest_height = new_height.clone();
        self.client_states.insert(&client_id, &client_state);

        // Store new consensus state
        let consensus_key = format!("{}#{}", client_id, new_height.revision_height);
        self.consensus_states.insert(&consensus_key, &consensus_state);

        env::log_str(&format!(
            "Updated client {} to height {}",
            client_id, new_height.revision_height
        ));

        true
    }

    /// Verify membership of a key-value pair in the IAVL tree
    /// 
    /// This function proves that a specific key exists in the counterparty
    /// chain's state at a given height.
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to verify against
    /// * `height` - The height at which to verify
    /// * `key` - The key to verify
    /// * `value` - The expected value
    /// * `proof` - The Merkle proof
    /// 
    /// # Returns
    /// * True if the key-value pair exists, false otherwise
    pub fn verify_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        value: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        // Get consensus state at the specified height
        let consensus_key = format!("{}#{}", client_id, height);
        let _consensus_state = match self.consensus_states.get(&consensus_key) {
            Some(state) => state,
            None => {
                env::log_str(&format!("Consensus state not found for height {}", height));
                return false;
            }
        };

        // Verify the Merkle proof against the consensus state root
        verify_merkle_proof(&_consensus_state.root, &key, Some(&value), &proof)
    }

    /// Verify non-membership of a key in the IAVL tree
    /// 
    /// This function proves that a specific key does not exist in the
    /// counterparty chain's state at a given height.
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to verify against  
    /// * `height` - The height at which to verify
    /// * `key` - The key to verify absence of
    /// * `proof` - The Merkle proof of non-existence
    /// 
    /// # Returns
    /// * True if the key does not exist, false otherwise
    pub fn verify_non_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        // Get consensus state at the specified height
        let consensus_key = format!("{}#{}", client_id, height);
        let _consensus_state = match self.consensus_states.get(&consensus_key) {
            Some(state) => state,
            None => {
                env::log_str(&format!("Consensus state not found for height {}", height));
                return false;
            }
        };

        // Verify the non-membership proof against the consensus state root
        verify_merkle_proof(&_consensus_state.root, &key, None, &proof)
    }

    /// Get the current state of a light client
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to query
    /// 
    /// # Returns
    /// * The ClientState if it exists
    pub fn get_client_state(&self, client_id: String) -> Option<ClientState> {
        self.client_states.get(&client_id)
    }

    /// Get a consensus state at a specific height
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to query
    /// * `height` - The height to query
    /// 
    /// # Returns
    /// * The ConsensusState if it exists
    pub fn get_consensus_state(&self, client_id: String, height: u64) -> Option<ConsensusState> {
        let consensus_key = format!("{}#{}", client_id, height);
        self.consensus_states.get(&consensus_key)
    }

    /// Get the latest height for a client
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to query
    /// 
    /// # Returns
    /// * The latest height if the client exists
    pub fn get_latest_height(&self, client_id: String) -> Option<Height> {
        self.client_states.get(&client_id).map(|state| state.latest_height)
    }

    /// Check if a consensus state has expired and clean up expired states
    /// 
    /// This function can be called periodically to clean up old consensus states
    /// that have exceeded their trust period.
    /// 
    /// # Arguments
    /// * `client_id` - The client ID to check
    /// * `height` - The height of the consensus state to check
    /// 
    /// # Returns
    /// * True if the consensus state was expired and removed, false otherwise
    pub fn prune_expired_consensus_state(&mut self, client_id: String, height: u64) -> bool {
        // Get client state to check trust period
        let client_state = match self.client_states.get(&client_id) {
            Some(state) => state,
            None => return false,
        };

        // Get consensus state to check
        let consensus_key = format!("{}#{}", client_id, height);
        let consensus_state = match self.consensus_states.get(&consensus_key) {
            Some(state) => state,
            None => return false,
        };

        // Check if expired
        if is_consensus_state_expired(&consensus_state, client_state.trust_period) {
            self.consensus_states.remove(&consensus_key);
            env::log_str(&format!("Pruned expired consensus state for client {} at height {}", client_id, height));
            true
        } else {
            false
        }
    }

    // === Private Helper Functions ===

    /// Validate the initial header used for client creation
    fn validate_initial_header(&self, header: &Header) {
        // Basic validation of the initial header
        if header.signed_header.header.height == 0 {
            env::panic_str("Header height cannot be zero");
        }

        if header.signed_header.header.chain_id.is_empty() {
            env::panic_str("Chain ID cannot be empty");
        }

        if header.validator_set.validators.is_empty() {
            env::panic_str("Validator set cannot be empty");
        }

        // TODO: Add more comprehensive validation:
        // - Verify commit signatures
        // - Check voting power thresholds
        // - Validate timestamp
    }

    /// Get Tendermint-specific proof specifications
    fn get_tendermint_proof_specs(&self) -> Vec<types::ProofSpec> {
        vec![
            types::ProofSpec {
                leaf_spec: types::LeafOp {
                    hash: types::HashOp::Sha256,
                    prehash_key: types::HashOp::NoHash,
                    prehash_value: types::HashOp::Sha256,
                    length: types::LengthOp::VarProto,
                    prefix: hex::decode("00").unwrap(), // IAVL leaf prefix
                },
                inner_spec: types::InnerSpec {
                    child_order: vec![0, 1],
                    child_size: 33, // 32 byte hash + 1 byte height
                    min_prefix_length: 4,
                    max_prefix_length: 12,
                    empty_child: vec![],
                    hash: types::HashOp::Sha256,
                },
                max_depth: 0, // No depth limit
                min_depth: 0,
            }
        ]
    }
}