use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

// Common types used across all modules
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Any {
    pub type_url: String,
    pub value: Vec<u8>,
}

// Height for IBC operations
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Height {
    pub revision_number: u64,
    pub revision_height: u64,
}

// ============================================================================
// BANK MODULE MESSAGES
// ============================================================================

/// MsgSend represents a message to send coins from one account to another.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgSend {
    pub from_address: String,
    pub to_address: String,
    pub amount: Vec<Coin>,
}

/// Input for MsgMultiSend
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Input {
    pub address: String,
    pub coins: Vec<Coin>,
}

/// Output for MsgMultiSend
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Output {
    pub address: String,
    pub coins: Vec<Coin>,
}

/// MsgMultiSend represents an arbitrary multi-in, multi-out send message.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgMultiSend {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

/// MsgBurn represents a message to burn coins from an account.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgBurn {
    pub from_address: String,
    pub amount: Vec<Coin>,
}

// ============================================================================
// STAKING MODULE MESSAGES
// ============================================================================

/// MsgDelegate defines a SDK message for performing a delegation of coins from a delegator to a validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgDelegate {
    pub delegator_address: String,
    pub validator_address: String,
    pub amount: Coin,
}

/// MsgUndelegate defines a SDK message for performing an undelegation from a delegate and a validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgUndelegate {
    pub delegator_address: String,
    pub validator_address: String,
    pub amount: Coin,
}

/// MsgBeginRedelegate defines a SDK message for performing a redelegation of coins from a delegator and source validator to a destination validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgBeginRedelegate {
    pub delegator_address: String,
    pub validator_src_address: String,
    pub validator_dst_address: String,
    pub amount: Coin,
}

/// Commission defines commission parameters for a given validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Commission {
    pub rate: String,
    pub max_rate: String,
    pub max_change_rate: String,
}

/// Description defines a validator description.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Description {
    pub moniker: String,
    pub identity: String,
    pub website: String,
    pub security_contact: String,
    pub details: String,
}

/// MsgCreateValidator defines a SDK message for creating a new validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgCreateValidator {
    pub description: Description,
    pub commission: Commission,
    pub min_self_delegation: String,
    pub delegator_address: String,
    pub validator_address: String,
    pub pubkey: Any,
    pub value: Coin,
}

/// MsgEditValidator defines a SDK message for editing an existing validator.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgEditValidator {
    pub description: Description,
    pub validator_address: String,
    pub commission_rate: Option<String>,
    pub min_self_delegation: Option<String>,
}

// ============================================================================
// GOVERNANCE MODULE MESSAGES
// ============================================================================

/// VoteOption enumerates the valid vote options for a given governance proposal.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum VoteOption {
    /// VOTE_OPTION_UNSPECIFIED defines a no-op vote option.
    Unspecified,
    /// VOTE_OPTION_YES defines a yes vote option.
    Yes,
    /// VOTE_OPTION_ABSTAIN defines an abstain vote option.
    Abstain,
    /// VOTE_OPTION_NO defines a no vote option.
    No,
    /// VOTE_OPTION_NO_WITH_VETO defines a no with veto vote option.
    NoWithVeto,
}

/// WeightedVoteOption defines a unit of vote for vote split.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WeightedVoteOption {
    pub option: VoteOption,
    pub weight: String,
}

/// MsgSubmitProposal defines an sdk.Msg type that supports submitting arbitrary proposal Content.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgSubmitProposal {
    pub content: Any,
    pub initial_deposit: Vec<Coin>,
    pub proposer: String,
}

/// MsgVote defines a message to cast a vote.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgVote {
    pub proposal_id: u64,
    pub voter: String,
    pub option: VoteOption,
}

/// MsgVoteWeighted defines a message to cast a vote with weights.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgVoteWeighted {
    pub proposal_id: u64,
    pub voter: String,
    pub options: Vec<WeightedVoteOption>,
}

/// MsgDeposit defines a message to submit a deposit to an existing proposal.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgDeposit {
    pub proposal_id: u64,
    pub depositor: String,
    pub amount: Vec<Coin>,
}

// ============================================================================
// IBC MODULE MESSAGES
// ============================================================================

/// MsgTransfer defines a msg to transfer fungible tokens (i.e Coins) between ICS20 enabled chains.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgTransfer {
    /// the port on which the packet will be sent
    pub source_port: String,
    /// the channel by which the packet will be sent
    pub source_channel: String,
    /// the tokens to be transferred
    pub token: Coin,
    /// the sender address
    pub sender: String,
    /// the recipient address on the destination chain
    pub receiver: String,
    /// timeout height relative to the current block height.
    /// The timeout is disabled when set to 0.
    pub timeout_height: Height,
    /// timeout timestamp in absolute nanoseconds since unix epoch.
    /// The timeout is disabled when set to 0.
    pub timeout_timestamp: u64,
}

/// MsgChannelOpenInit defines a msg sent by a Relayer to Chain A to initialize a channel opening handshake with Chain B.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgChannelOpenInit {
    pub port_id: String,
    pub channel: Any, // Channel type
    pub signer: String,
}

/// MsgChannelOpenTry defines a msg sent by a Relayer to Chain B to accept a channel opening handshake initiated by Chain A.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgChannelOpenTry {
    pub port_id: String,
    pub desired_channel_id: String,
    pub counterparty_version: String,
    pub channel: Any, // Channel type
    pub counterparty_version_proof: Vec<u8>,
    pub proof_height: Height,
    pub signer: String,
}

/// MsgRecvPacket defines a msg that receives a packet
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgRecvPacket {
    pub packet: Any, // Packet type
    pub proof_commitment: Vec<u8>,
    pub proof_height: Height,
    pub signer: String,
}

/// MsgAcknowledgement defines a msg that acknowledges a packet
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgAcknowledgement {
    pub packet: Any, // Packet type
    pub acknowledgement: Vec<u8>,
    pub proof_acked: Vec<u8>,
    pub proof_height: Height,
    pub signer: String,
}

/// MsgTimeout defines a msg that receives a timeout packet
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MsgTimeout {
    pub packet: Any, // Packet type
    pub proof_unreceived: Vec<u8>,
    pub proof_height: Height,
    pub next_sequence_recv: u64,
    pub signer: String,
}

// ============================================================================
// MESSAGE TYPE CONSTANTS
// ============================================================================

/// Type URLs for Cosmos SDK messages - these match the standard Cosmos SDK protobuf type URLs
pub mod type_urls {
    // Bank module
    pub const MSG_SEND: &str = "/cosmos.bank.v1beta1.MsgSend";
    pub const MSG_MULTI_SEND: &str = "/cosmos.bank.v1beta1.MsgMultiSend";
    pub const MSG_BURN: &str = "/cosmos.bank.v1beta1.MsgBurn";

    // Staking module
    pub const MSG_DELEGATE: &str = "/cosmos.staking.v1beta1.MsgDelegate";
    pub const MSG_UNDELEGATE: &str = "/cosmos.staking.v1beta1.MsgUndelegate";
    pub const MSG_BEGIN_REDELEGATE: &str = "/cosmos.staking.v1beta1.MsgBeginRedelegate";
    pub const MSG_CREATE_VALIDATOR: &str = "/cosmos.staking.v1beta1.MsgCreateValidator";
    pub const MSG_EDIT_VALIDATOR: &str = "/cosmos.staking.v1beta1.MsgEditValidator";

    // Governance module
    pub const MSG_SUBMIT_PROPOSAL: &str = "/cosmos.gov.v1beta1.MsgSubmitProposal";
    pub const MSG_VOTE: &str = "/cosmos.gov.v1beta1.MsgVote";
    pub const MSG_VOTE_WEIGHTED: &str = "/cosmos.gov.v1beta1.MsgVoteWeighted";
    pub const MSG_DEPOSIT: &str = "/cosmos.gov.v1beta1.MsgDeposit";

    // IBC module
    pub const MSG_TRANSFER: &str = "/ibc.applications.transfer.v1.MsgTransfer";
    pub const MSG_CHANNEL_OPEN_INIT: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";
    pub const MSG_CHANNEL_OPEN_TRY: &str = "/ibc.core.channel.v1.MsgChannelOpenTry";
    pub const MSG_RECV_PACKET: &str = "/ibc.core.channel.v1.MsgRecvPacket";
    pub const MSG_ACKNOWLEDGEMENT: &str = "/ibc.core.channel.v1.MsgAcknowledgement";
    pub const MSG_TIMEOUT: &str = "/ibc.core.channel.v1.MsgTimeout";
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

impl Coin {
    pub fn new(denom: impl Into<String>, amount: impl Into<String>) -> Self {
        Self {
            denom: denom.into(),
            amount: amount.into(),
        }
    }
}

impl Height {
    pub fn new(revision_number: u64, revision_height: u64) -> Self {
        Self {
            revision_number,
            revision_height,
        }
    }

    pub fn zero() -> Self {
        Self {
            revision_number: 0,
            revision_height: 0,
        }
    }
}

/// Utility function to format coins for display
pub fn format_coins(coins: &[Coin]) -> String {
    coins
        .iter()
        .map(|coin| format!("{}{}", coin.amount, coin.denom))
        .collect::<Vec<_>>()
        .join(",")
}

/// Utility function to validate a Cosmos SDK message type URL
pub fn is_valid_type_url(type_url: &str) -> bool {
    match type_url {
        type_urls::MSG_SEND
        | type_urls::MSG_MULTI_SEND
        | type_urls::MSG_BURN
        | type_urls::MSG_DELEGATE
        | type_urls::MSG_UNDELEGATE
        | type_urls::MSG_BEGIN_REDELEGATE
        | type_urls::MSG_CREATE_VALIDATOR
        | type_urls::MSG_EDIT_VALIDATOR
        | type_urls::MSG_SUBMIT_PROPOSAL
        | type_urls::MSG_VOTE
        | type_urls::MSG_VOTE_WEIGHTED
        | type_urls::MSG_DEPOSIT
        | type_urls::MSG_TRANSFER
        | type_urls::MSG_CHANNEL_OPEN_INIT
        | type_urls::MSG_CHANNEL_OPEN_TRY
        | type_urls::MSG_RECV_PACKET
        | type_urls::MSG_ACKNOWLEDGEMENT
        | type_urls::MSG_TIMEOUT => true,
        _ => false,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::borsh::{from_slice, to_vec};
    use serde_json;

    // ========================================================================
    // SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_coin_serialization() {
        let coin = Coin::new("uatom", "1000000");
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&coin).unwrap();
        let deserialized_coin: Coin = from_slice(&borsh_bytes).unwrap();
        assert_eq!(coin, deserialized_coin);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&coin).unwrap();
        let json_coin: Coin = serde_json::from_str(&json_str).unwrap();
        assert_eq!(coin, json_coin);
    }

    #[test]
    fn test_height_serialization() {
        let height = Height::new(1, 12345);
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&height).unwrap();
        let deserialized_height: Height = from_slice(&borsh_bytes).unwrap();
        assert_eq!(height, deserialized_height);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&height).unwrap();
        let json_height: Height = serde_json::from_str(&json_str).unwrap();
        assert_eq!(height, json_height);
    }

    #[test]
    fn test_msg_send_serialization() {
        let msg = MsgSend {
            from_address: "cosmos1sender".to_string(),
            to_address: "cosmos1receiver".to_string(),
            amount: vec![
                Coin::new("uatom", "1000000"),
                Coin::new("stake", "500000"),
            ],
        };
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgSend = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgSend = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    #[test]
    fn test_msg_multi_send_serialization() {
        let msg = MsgMultiSend {
            inputs: vec![
                Input {
                    address: "cosmos1input1".to_string(),
                    coins: vec![Coin::new("uatom", "1000000")],
                }
            ],
            outputs: vec![
                Output {
                    address: "cosmos1output1".to_string(),
                    coins: vec![Coin::new("uatom", "500000")],
                },
                Output {
                    address: "cosmos1output2".to_string(),
                    coins: vec![Coin::new("uatom", "500000")],
                },
            ],
        };
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgMultiSend = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgMultiSend = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    #[test]
    fn test_msg_delegate_serialization() {
        let msg = MsgDelegate {
            delegator_address: "cosmos1delegator".to_string(),
            validator_address: "cosmosvaloper1validator".to_string(),
            amount: Coin::new("uatom", "1000000"),
        };
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgDelegate = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgDelegate = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    #[test]
    fn test_vote_option_serialization() {
        let options = vec![
            VoteOption::Unspecified,
            VoteOption::Yes,
            VoteOption::Abstain,
            VoteOption::No,
            VoteOption::NoWithVeto,
        ];
        
        for option in options {
            // Test Borsh serialization
            let borsh_bytes = to_vec(&option).unwrap();
            let deserialized_option: VoteOption = from_slice(&borsh_bytes).unwrap();
            assert_eq!(option, deserialized_option);
            
            // Test JSON serialization
            let json_str = serde_json::to_string(&option).unwrap();
            let json_option: VoteOption = serde_json::from_str(&json_str).unwrap();
            assert_eq!(option, json_option);
        }
    }

    #[test]
    fn test_msg_vote_serialization() {
        let msg = MsgVote {
            proposal_id: 42,
            voter: "cosmos1voter".to_string(),
            option: VoteOption::Yes,
        };
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgVote = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgVote = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    #[test]
    fn test_msg_transfer_serialization() {
        let msg = MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            token: Coin::new("uatom", "1000000"),
            sender: "cosmos1sender".to_string(),
            receiver: "near1receiver".to_string(),
            timeout_height: Height::new(1, 12345),
            timeout_timestamp: 1640995200000000000,
        };
        
        // Test Borsh serialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgTransfer = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgTransfer = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    // ========================================================================
    // HELPER FUNCTION TESTS
    // ========================================================================

    #[test]
    fn test_coin_new() {
        let coin = Coin::new("uatom", "1000000");
        assert_eq!(coin.denom, "uatom");
        assert_eq!(coin.amount, "1000000");
        
        // Test with different types that implement Into<String>
        let coin2 = Coin::new(String::from("stake"), 500000.to_string());
        assert_eq!(coin2.denom, "stake");
        assert_eq!(coin2.amount, "500000");
    }

    #[test]
    fn test_height_new() {
        let height = Height::new(1, 12345);
        assert_eq!(height.revision_number, 1);
        assert_eq!(height.revision_height, 12345);
    }

    #[test]
    fn test_height_zero() {
        let height = Height::zero();
        assert_eq!(height.revision_number, 0);
        assert_eq!(height.revision_height, 0);
    }

    #[test]
    fn test_format_coins() {
        let coins = vec![
            Coin::new("uatom", "1000000"),
            Coin::new("stake", "500000"),
        ];
        
        let formatted = format_coins(&coins);
        assert_eq!(formatted, "1000000uatom,500000stake");
        
        // Test empty coins
        let empty_coins: Vec<Coin> = vec![];
        let empty_formatted = format_coins(&empty_coins);
        assert_eq!(empty_formatted, "");
        
        // Test single coin
        let single_coin = vec![Coin::new("uatom", "1000000")];
        let single_formatted = format_coins(&single_coin);
        assert_eq!(single_formatted, "1000000uatom");
    }

    #[test]
    fn test_is_valid_type_url() {
        // Test valid type URLs
        assert!(is_valid_type_url(type_urls::MSG_SEND));
        assert!(is_valid_type_url(type_urls::MSG_MULTI_SEND));
        assert!(is_valid_type_url(type_urls::MSG_BURN));
        assert!(is_valid_type_url(type_urls::MSG_DELEGATE));
        assert!(is_valid_type_url(type_urls::MSG_UNDELEGATE));
        assert!(is_valid_type_url(type_urls::MSG_BEGIN_REDELEGATE));
        assert!(is_valid_type_url(type_urls::MSG_CREATE_VALIDATOR));
        assert!(is_valid_type_url(type_urls::MSG_EDIT_VALIDATOR));
        assert!(is_valid_type_url(type_urls::MSG_SUBMIT_PROPOSAL));
        assert!(is_valid_type_url(type_urls::MSG_VOTE));
        assert!(is_valid_type_url(type_urls::MSG_VOTE_WEIGHTED));
        assert!(is_valid_type_url(type_urls::MSG_DEPOSIT));
        assert!(is_valid_type_url(type_urls::MSG_TRANSFER));
        assert!(is_valid_type_url(type_urls::MSG_CHANNEL_OPEN_INIT));
        assert!(is_valid_type_url(type_urls::MSG_CHANNEL_OPEN_TRY));
        assert!(is_valid_type_url(type_urls::MSG_RECV_PACKET));
        assert!(is_valid_type_url(type_urls::MSG_ACKNOWLEDGEMENT));
        assert!(is_valid_type_url(type_urls::MSG_TIMEOUT));
        
        // Test invalid type URLs
        assert!(!is_valid_type_url("/invalid.type.Url"));
        assert!(!is_valid_type_url(""));
        assert!(!is_valid_type_url("/cosmos.bank.v1beta1.InvalidMsg"));
        assert!(!is_valid_type_url("cosmos.bank.v1beta1.MsgSend")); // Missing leading slash
    }

    // ========================================================================
    // TYPE URL CONSTANTS TESTS
    // ========================================================================

    #[test]
    fn test_type_url_constants() {
        // Verify type URL format follows Cosmos SDK convention
        assert!(type_urls::MSG_SEND.starts_with("/cosmos."));
        assert!(type_urls::MSG_DELEGATE.starts_with("/cosmos."));
        assert!(type_urls::MSG_VOTE.starts_with("/cosmos."));
        assert!(type_urls::MSG_TRANSFER.starts_with("/ibc."));
        
        // Verify specific values
        assert_eq!(type_urls::MSG_SEND, "/cosmos.bank.v1beta1.MsgSend");
        assert_eq!(type_urls::MSG_DELEGATE, "/cosmos.staking.v1beta1.MsgDelegate");
        assert_eq!(type_urls::MSG_VOTE, "/cosmos.gov.v1beta1.MsgVote");
        assert_eq!(type_urls::MSG_TRANSFER, "/ibc.applications.transfer.v1.MsgTransfer");
    }

    // ========================================================================
    // COMPLEX MESSAGE VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_msg_create_validator_complete() {
        let description = Description {
            moniker: "My Validator".to_string(),
            identity: "ABC123".to_string(),
            website: "https://myvalidator.com".to_string(),
            security_contact: "security@myvalidator.com".to_string(),
            details: "A reliable validator".to_string(),
        };
        
        let commission = Commission {
            rate: "0.10".to_string(),
            max_rate: "0.20".to_string(),
            max_change_rate: "0.01".to_string(),
        };
        
        let pubkey = Any {
            type_url: "/cosmos.crypto.ed25519.PubKey".to_string(),
            value: vec![1, 2, 3, 4, 5],
        };
        
        let msg = MsgCreateValidator {
            description,
            commission,
            min_self_delegation: "1000000".to_string(),
            delegator_address: "cosmos1delegator".to_string(),
            validator_address: "cosmosvaloper1validator".to_string(),
            pubkey,
            value: Coin::new("uatom", "10000000"),
        };
        
        // Test complete serialization/deserialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgCreateValidator = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgCreateValidator = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    #[test]
    fn test_msg_vote_weighted_complete() {
        let weighted_options = vec![
            WeightedVoteOption {
                option: VoteOption::Yes,
                weight: "0.6".to_string(),
            },
            WeightedVoteOption {
                option: VoteOption::No,
                weight: "0.3".to_string(),
            },
            WeightedVoteOption {
                option: VoteOption::Abstain,
                weight: "0.1".to_string(),
            },
        ];

        let msg = MsgVoteWeighted {
            proposal_id: 42,
            voter: "cosmos1voter".to_string(),
            options: weighted_options,
        };
        
        // Test complete serialization/deserialization
        let borsh_bytes = to_vec(&msg).unwrap();
        let deserialized_msg: MsgVoteWeighted = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg, deserialized_msg);
        
        let json_str = serde_json::to_string(&msg).unwrap();
        let json_msg: MsgVoteWeighted = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, json_msg);
    }

    // ========================================================================
    // EDGE CASE TESTS
    // ========================================================================

    #[test]
    fn test_empty_message_fields() {
        // Test MsgSend with empty amounts
        let msg_send_empty = MsgSend {
            from_address: "cosmos1sender".to_string(),
            to_address: "cosmos1receiver".to_string(),
            amount: vec![],
        };
        
        let borsh_bytes = to_vec(&msg_send_empty).unwrap();
        let deserialized: MsgSend = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg_send_empty, deserialized);
        assert!(deserialized.amount.is_empty());
    }

    #[test]
    fn test_zero_amounts() {
        let coin_zero = Coin::new("uatom", "0");
        assert_eq!(coin_zero.amount, "0");
        
        let msg_with_zero = MsgSend {
            from_address: "cosmos1sender".to_string(),
            to_address: "cosmos1receiver".to_string(),
            amount: vec![coin_zero],
        };
        
        let borsh_bytes = to_vec(&msg_with_zero).unwrap();
        let deserialized: MsgSend = from_slice(&borsh_bytes).unwrap();
        assert_eq!(msg_with_zero, deserialized);
        assert_eq!(deserialized.amount[0].amount, "0");
    }

    #[test]
    fn test_large_numbers() {
        let large_amount = "999999999999999999999999999999"; // Very large number as string
        let coin_large = Coin::new("uatom", large_amount);
        
        let borsh_bytes = to_vec(&coin_large).unwrap();
        let deserialized: Coin = from_slice(&borsh_bytes).unwrap();
        assert_eq!(coin_large, deserialized);
        assert_eq!(deserialized.amount, large_amount);
    }
}