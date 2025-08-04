use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::Base64VecU8;

use crate::types::cosmos_messages::*;

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Standard response from a message handler
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HandleResponse {
    /// Result code (0 = success, non-zero = error)
    pub code: u32,
    /// Response data (optional)
    pub data: Vec<u8>,
    /// Log message
    pub log: String,
    /// Events emitted by the handler
    pub events: Vec<Event>,
}

/// Cosmos SDK compatible event
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Event {
    /// Event type (e.g., "transfer", "delegate")
    pub r#type: String,
    /// Event attributes (key-value pairs)
    pub attributes: Vec<Attribute>,
}

/// Event attribute (key-value pair)
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

/// Internal result type for message handling
#[derive(Debug)]
pub struct HandleResult {
    pub data: Vec<u8>,
    pub log: String,
    pub events: Vec<Event>,
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Error types for message handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractError {
    /// Unknown message type
    UnknownMessageType(String),
    /// Invalid message format
    InvalidMessageFormat(String),
    /// Message decoding error
    DecodeError(String),
    /// Insufficient funds
    InsufficientFunds,
    /// Unauthorized operation
    Unauthorized,
    /// Invalid address format
    InvalidAddress,
    /// Custom error with message
    Custom(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::UnknownMessageType(msg_type) => write!(f, "Unknown message type: {}", msg_type),
            ContractError::InvalidMessageFormat(msg) => write!(f, "Invalid message format: {}", msg),
            ContractError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
            ContractError::InsufficientFunds => write!(f, "Insufficient funds"),
            ContractError::Unauthorized => write!(f, "Unauthorized"),
            ContractError::InvalidAddress => write!(f, "Invalid address"),
            ContractError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ContractError {}

/// Result type for message operations
pub type MessageResult<T> = Result<T, ContractError>;

// ============================================================================
// DECODE FUNCTIONS
// ============================================================================

/// Decode protobuf-compatible message data
/// Initially supports JSON format, can be extended to support actual protobuf
pub fn decode_protobuf_compatible<T>(data: Vec<u8>) -> MessageResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    // For now, we support JSON encoding which is protobuf-compatible
    // In the future, this can be extended to support actual protobuf binary format
    serde_json::from_slice(&data)
        .map_err(|e| ContractError::DecodeError(format!("JSON decode error: {}", e)))
}

/// Encode response data to bytes
pub fn encode_response<T>(response: &T) -> MessageResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(response)
        .map_err(|e| ContractError::DecodeError(format!("JSON encode error: {}", e)))
}

// ============================================================================
// MESSAGE ROUTER TRAIT
// ============================================================================

/// Trait for handling Cosmos SDK messages
/// This will be implemented by the main contract
pub trait CosmosMessageHandler {
    // Bank module handlers
    fn handle_msg_send(&mut self, msg: MsgSend) -> MessageResult<HandleResult>;
    fn handle_msg_multi_send(&mut self, msg: MsgMultiSend) -> MessageResult<HandleResult>;
    fn handle_msg_burn(&mut self, msg: MsgBurn) -> MessageResult<HandleResult>;

    // Staking module handlers
    fn handle_msg_delegate(&mut self, msg: MsgDelegate) -> MessageResult<HandleResult>;
    fn handle_msg_undelegate(&mut self, msg: MsgUndelegate) -> MessageResult<HandleResult>;
    fn handle_msg_begin_redelegate(&mut self, msg: MsgBeginRedelegate) -> MessageResult<HandleResult>;
    fn handle_msg_create_validator(&mut self, msg: MsgCreateValidator) -> MessageResult<HandleResult>;
    fn handle_msg_edit_validator(&mut self, msg: MsgEditValidator) -> MessageResult<HandleResult>;

    // Governance module handlers
    fn handle_msg_submit_proposal(&mut self, msg: MsgSubmitProposal) -> MessageResult<HandleResult>;
    fn handle_msg_vote(&mut self, msg: MsgVote) -> MessageResult<HandleResult>;
    fn handle_msg_vote_weighted(&mut self, msg: MsgVoteWeighted) -> MessageResult<HandleResult>;
    fn handle_msg_deposit(&mut self, msg: MsgDeposit) -> MessageResult<HandleResult>;

    // IBC module handlers
    fn handle_msg_transfer(&mut self, msg: MsgTransfer) -> MessageResult<HandleResult>;
    fn handle_msg_channel_open_init(&mut self, msg: MsgChannelOpenInit) -> MessageResult<HandleResult>;
    fn handle_msg_channel_open_try(&mut self, msg: MsgChannelOpenTry) -> MessageResult<HandleResult>;
    fn handle_msg_recv_packet(&mut self, msg: MsgRecvPacket) -> MessageResult<HandleResult>;
    fn handle_msg_acknowledgement(&mut self, msg: MsgAcknowledgement) -> MessageResult<HandleResult>;
    fn handle_msg_timeout(&mut self, msg: MsgTimeout) -> MessageResult<HandleResult>;
}

// ============================================================================
// MAIN ROUTER FUNCTION
// ============================================================================

/// Route a Cosmos SDK message to the appropriate handler
pub fn route_cosmos_message<T>(
    handler: &mut T,
    msg_type: String,
    msg_data: Base64VecU8,
) -> HandleResponse
where
    T: CosmosMessageHandler,
{
    // Validate message type
    if !is_valid_type_url(&msg_type) {
        return HandleResponse {
            code: 1,
            data: vec![],
            log: format!("Invalid message type: {}", msg_type),
            events: vec![],
        };
    }

    let msg_bytes = msg_data.0;

    // Route message based on type URL
    let result = match msg_type.as_str() {
        // Bank module messages
        type_urls::MSG_SEND => {
            decode_protobuf_compatible::<MsgSend>(msg_bytes)
                .and_then(|msg| handler.handle_msg_send(msg))
        }
        type_urls::MSG_MULTI_SEND => {
            decode_protobuf_compatible::<MsgMultiSend>(msg_bytes)
                .and_then(|msg| handler.handle_msg_multi_send(msg))
        }
        type_urls::MSG_BURN => {
            decode_protobuf_compatible::<MsgBurn>(msg_bytes)
                .and_then(|msg| handler.handle_msg_burn(msg))
        }

        // Staking module messages
        type_urls::MSG_DELEGATE => {
            decode_protobuf_compatible::<MsgDelegate>(msg_bytes)
                .and_then(|msg| handler.handle_msg_delegate(msg))
        }
        type_urls::MSG_UNDELEGATE => {
            decode_protobuf_compatible::<MsgUndelegate>(msg_bytes)
                .and_then(|msg| handler.handle_msg_undelegate(msg))
        }
        type_urls::MSG_BEGIN_REDELEGATE => {
            decode_protobuf_compatible::<MsgBeginRedelegate>(msg_bytes)
                .and_then(|msg| handler.handle_msg_begin_redelegate(msg))
        }
        type_urls::MSG_CREATE_VALIDATOR => {
            decode_protobuf_compatible::<MsgCreateValidator>(msg_bytes)
                .and_then(|msg| handler.handle_msg_create_validator(msg))
        }
        type_urls::MSG_EDIT_VALIDATOR => {
            decode_protobuf_compatible::<MsgEditValidator>(msg_bytes)
                .and_then(|msg| handler.handle_msg_edit_validator(msg))
        }

        // Governance module messages
        type_urls::MSG_SUBMIT_PROPOSAL => {
            decode_protobuf_compatible::<MsgSubmitProposal>(msg_bytes)
                .and_then(|msg| handler.handle_msg_submit_proposal(msg))
        }
        type_urls::MSG_VOTE => {
            decode_protobuf_compatible::<MsgVote>(msg_bytes)
                .and_then(|msg| handler.handle_msg_vote(msg))
        }
        type_urls::MSG_VOTE_WEIGHTED => {
            decode_protobuf_compatible::<MsgVoteWeighted>(msg_bytes)
                .and_then(|msg| handler.handle_msg_vote_weighted(msg))
        }
        type_urls::MSG_DEPOSIT => {
            decode_protobuf_compatible::<MsgDeposit>(msg_bytes)
                .and_then(|msg| handler.handle_msg_deposit(msg))
        }

        // IBC module messages
        type_urls::MSG_TRANSFER => {
            decode_protobuf_compatible::<MsgTransfer>(msg_bytes)
                .and_then(|msg| handler.handle_msg_transfer(msg))
        }
        type_urls::MSG_CHANNEL_OPEN_INIT => {
            decode_protobuf_compatible::<MsgChannelOpenInit>(msg_bytes)
                .and_then(|msg| handler.handle_msg_channel_open_init(msg))
        }
        type_urls::MSG_CHANNEL_OPEN_TRY => {
            decode_protobuf_compatible::<MsgChannelOpenTry>(msg_bytes)
                .and_then(|msg| handler.handle_msg_channel_open_try(msg))
        }
        type_urls::MSG_RECV_PACKET => {
            decode_protobuf_compatible::<MsgRecvPacket>(msg_bytes)
                .and_then(|msg| handler.handle_msg_recv_packet(msg))
        }
        type_urls::MSG_ACKNOWLEDGEMENT => {
            decode_protobuf_compatible::<MsgAcknowledgement>(msg_bytes)
                .and_then(|msg| handler.handle_msg_acknowledgement(msg))
        }
        type_urls::MSG_TIMEOUT => {
            decode_protobuf_compatible::<MsgTimeout>(msg_bytes)
                .and_then(|msg| handler.handle_msg_timeout(msg))
        }

        _ => Err(ContractError::UnknownMessageType(msg_type.clone())),
    };

    // Convert result to response
    match result {
        Ok(handle_result) => HandleResponse {
            code: 0,
            data: handle_result.data,
            log: handle_result.log,
            events: handle_result.events,
        },
        Err(error) => HandleResponse {
            code: 1,
            data: vec![],
            log: error.to_string(),
            events: vec![],
        },
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a standard Cosmos SDK event
pub fn create_event(event_type: &str, attributes: Vec<(&str, &str)>) -> Event {
    Event {
        r#type: event_type.to_string(),
        attributes: attributes
            .into_iter()
            .map(|(key, value)| Attribute {
                key: key.to_string(),
                value: value.to_string(),
            })
            .collect(),
    }
}

/// Create a successful handle result
pub fn success_result(log: &str, events: Vec<Event>) -> HandleResult {
    HandleResult {
        data: vec![],
        log: log.to_string(),
        events,
    }
}

/// Create a successful handle result with data
pub fn success_result_with_data(data: Vec<u8>, log: &str, events: Vec<Event>) -> HandleResult {
    HandleResult {
        data,
        log: log.to_string(),
        events,
    }
}

/// Validate Cosmos address format
pub fn validate_cosmos_address(address: &str) -> MessageResult<()> {
    if address.is_empty() {
        return Err(ContractError::InvalidAddress);
    }

    // Accept Cosmos bech32 addresses, NEAR addresses, and account IDs
    if address.starts_with("cosmos1") 
        || address.starts_with("cosmosvaloper1")
        || address.starts_with("near:")
        || address.ends_with(".near")
        || address.ends_with(".testnet") {
        Ok(())
    } else {
        // For now, accept any non-empty string as a valid address
        // More strict validation can be added later
        Ok(())
    }
}

/// Format coins for display in logs
pub fn log_coins(coins: &[Coin]) -> String {
    if coins.is_empty() {
        return "[]".to_string();
    }
    
    format!(
        "[{}]",
        coins
            .iter()
            .map(|coin| format!("{}{}", coin.amount, coin.denom))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock handler for testing
    struct MockHandler {
        call_count: u32,
    }

    impl MockHandler {
        fn new() -> Self {
            Self { call_count: 0 }
        }
    }

    impl CosmosMessageHandler for MockHandler {
        fn handle_msg_send(&mut self, msg: MsgSend) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result(
                &format!("transfer {} from {} to {}", 
                    log_coins(&msg.amount), msg.from_address, msg.to_address),
                vec![create_event("transfer", vec![
                    ("sender", &msg.from_address),
                    ("recipient", &msg.to_address),
                    ("amount", &format_coins(&msg.amount)),
                ])],
            ))
        }

        fn handle_msg_multi_send(&mut self, _msg: MsgMultiSend) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("multi-send executed", vec![]))
        }

        fn handle_msg_burn(&mut self, msg: MsgBurn) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result(
                &format!("burned {} from {}", log_coins(&msg.amount), msg.from_address),
                vec![create_event("burn", vec![
                    ("burner", &msg.from_address),
                    ("amount", &format_coins(&msg.amount)),
                ])],
            ))
        }

        fn handle_msg_delegate(&mut self, msg: MsgDelegate) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result(
                &format!("delegated {} from {} to {}", 
                    format!("{}{}", msg.amount.amount, msg.amount.denom),
                    msg.delegator_address, 
                    msg.validator_address),
                vec![create_event("delegate", vec![
                    ("delegator", &msg.delegator_address),
                    ("validator", &msg.validator_address),
                    ("amount", &format!("{}{}", msg.amount.amount, msg.amount.denom)),
                ])],
            ))
        }

        // Implement remaining handlers with simple mock responses
        fn handle_msg_undelegate(&mut self, _msg: MsgUndelegate) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("undelegate executed", vec![]))
        }

        fn handle_msg_begin_redelegate(&mut self, _msg: MsgBeginRedelegate) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("redelegate executed", vec![]))
        }

        fn handle_msg_create_validator(&mut self, _msg: MsgCreateValidator) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("validator created", vec![]))
        }

        fn handle_msg_edit_validator(&mut self, _msg: MsgEditValidator) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("validator edited", vec![]))
        }

        fn handle_msg_submit_proposal(&mut self, _msg: MsgSubmitProposal) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("proposal submitted", vec![]))
        }

        fn handle_msg_vote(&mut self, msg: MsgVote) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result(
                &format!("vote cast by {} on proposal {}", msg.voter, msg.proposal_id),
                vec![create_event("proposal_vote", vec![
                    ("proposal_id", &msg.proposal_id.to_string()),
                    ("voter", &msg.voter),
                    ("option", &format!("{:?}", msg.option)),
                ])],
            ))
        }

        fn handle_msg_vote_weighted(&mut self, _msg: MsgVoteWeighted) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("weighted vote cast", vec![]))
        }

        fn handle_msg_deposit(&mut self, _msg: MsgDeposit) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("deposit made", vec![]))
        }

        fn handle_msg_transfer(&mut self, msg: MsgTransfer) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result(
                &format!("IBC transfer {} from {} to {}", 
                    format!("{}{}", msg.token.amount, msg.token.denom),
                    msg.sender, 
                    msg.receiver),
                vec![create_event("ibc_transfer", vec![
                    ("sender", &msg.sender),
                    ("receiver", &msg.receiver),
                    ("amount", &format!("{}{}", msg.token.amount, msg.token.denom)),
                    ("source_channel", &msg.source_channel),
                ])],
            ))
        }

        fn handle_msg_channel_open_init(&mut self, _msg: MsgChannelOpenInit) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("channel open init", vec![]))
        }

        fn handle_msg_channel_open_try(&mut self, _msg: MsgChannelOpenTry) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("channel open try", vec![]))
        }

        fn handle_msg_recv_packet(&mut self, _msg: MsgRecvPacket) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("packet received", vec![]))
        }

        fn handle_msg_acknowledgement(&mut self, _msg: MsgAcknowledgement) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("packet acknowledged", vec![]))
        }

        fn handle_msg_timeout(&mut self, _msg: MsgTimeout) -> MessageResult<HandleResult> {
            self.call_count += 1;
            Ok(success_result("packet timeout", vec![]))
        }
    }

    #[test]
    fn test_decode_protobuf_compatible() {
        let coin = Coin::new("uatom", "1000000");
        let json_bytes = serde_json::to_vec(&coin).unwrap();
        
        let decoded: Coin = decode_protobuf_compatible(json_bytes).unwrap();
        assert_eq!(coin, decoded);
    }

    #[test]
    fn test_decode_invalid_json() {
        let invalid_json = b"invalid json".to_vec();
        let result: MessageResult<Coin> = decode_protobuf_compatible(invalid_json);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ContractError::DecodeError(_) => (),
            _ => panic!("Expected DecodeError"),
        }
    }

    #[test]
    fn test_route_msg_send() {
        let mut handler = MockHandler::new();
        
        let msg = MsgSend {
            from_address: "cosmos1sender".to_string(),
            to_address: "cosmos1receiver".to_string(),
            amount: vec![Coin::new("uatom", "1000000")],
        };
        
        let msg_bytes = serde_json::to_vec(&msg).unwrap();
        let response = route_cosmos_message(
            &mut handler,
            type_urls::MSG_SEND.to_string(),
            Base64VecU8(msg_bytes),
        );
        
        assert_eq!(response.code, 0);
        assert!(response.log.contains("transfer"));
        assert!(response.log.contains("cosmos1sender"));
        assert!(response.log.contains("cosmos1receiver"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "transfer");
        assert_eq!(handler.call_count, 1);
    }

    #[test]
    fn test_route_invalid_message_type() {
        let mut handler = MockHandler::new();
        
        let response = route_cosmos_message(
            &mut handler,
            "/invalid.message.Type".to_string(),
            Base64VecU8(vec![1, 2, 3]),
        );
        
        assert_eq!(response.code, 1);
        assert!(response.log.contains("Invalid message type"));
        assert_eq!(handler.call_count, 0);
    }

    #[test]
    fn test_route_msg_delegate() {
        let mut handler = MockHandler::new();
        
        let msg = MsgDelegate {
            delegator_address: "cosmos1delegator".to_string(),
            validator_address: "cosmosvaloper1validator".to_string(),
            amount: Coin::new("uatom", "1000000"),
        };
        
        let msg_bytes = serde_json::to_vec(&msg).unwrap();
        let response = route_cosmos_message(
            &mut handler,
            type_urls::MSG_DELEGATE.to_string(),
            Base64VecU8(msg_bytes),
        );
        
        assert_eq!(response.code, 0);
        assert!(response.log.contains("delegated"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "delegate");
        assert_eq!(handler.call_count, 1);
    }

    #[test]
    fn test_route_msg_vote() {
        let mut handler = MockHandler::new();
        
        let msg = MsgVote {
            proposal_id: 42,
            voter: "cosmos1voter".to_string(),
            option: VoteOption::Yes,
        };
        
        let msg_bytes = serde_json::to_vec(&msg).unwrap();
        let response = route_cosmos_message(
            &mut handler,
            type_urls::MSG_VOTE.to_string(),
            Base64VecU8(msg_bytes),
        );
        
        assert_eq!(response.code, 0);
        assert!(response.log.contains("vote cast"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "proposal_vote");
        assert_eq!(handler.call_count, 1);
    }

    #[test]
    fn test_validate_cosmos_address() {
        // Valid addresses
        assert!(validate_cosmos_address("cosmos1sender").is_ok());
        assert!(validate_cosmos_address("cosmosvaloper1validator").is_ok());
        assert!(validate_cosmos_address("near:user.near").is_ok());
        assert!(validate_cosmos_address("user.near").is_ok());
        assert!(validate_cosmos_address("user.testnet").is_ok());
        assert!(validate_cosmos_address("some_other_format").is_ok()); // Permissive for now
        
        // Invalid addresses
        assert!(validate_cosmos_address("").is_err());
    }

    #[test]
    fn test_create_event() {
        let event = create_event("transfer", vec![
            ("sender", "cosmos1sender"),
            ("recipient", "cosmos1receiver"),
            ("amount", "1000000uatom"),
        ]);
        
        assert_eq!(event.r#type, "transfer");
        assert_eq!(event.attributes.len(), 3);
        assert_eq!(event.attributes[0].key, "sender");
        assert_eq!(event.attributes[0].value, "cosmos1sender");
    }

    #[test]
    fn test_log_coins() {
        let coins = vec![
            Coin::new("uatom", "1000000"),
            Coin::new("uosmo", "500000"),
        ];
        
        let formatted = log_coins(&coins);
        assert_eq!(formatted, "[1000000uatom, 500000uosmo]");
        
        let empty_coins: Vec<Coin> = vec![];
        assert_eq!(log_coins(&empty_coins), "[]");
    }

    #[test]
    fn test_all_message_types() {
        let mut handler = MockHandler::new();
        
        // Test each message type URL is handled correctly
        let message_types = vec![
            type_urls::MSG_SEND,
            type_urls::MSG_MULTI_SEND,
            type_urls::MSG_BURN,
            type_urls::MSG_DELEGATE,
            type_urls::MSG_UNDELEGATE,
            type_urls::MSG_BEGIN_REDELEGATE,
            type_urls::MSG_CREATE_VALIDATOR,
            type_urls::MSG_EDIT_VALIDATOR,
            type_urls::MSG_SUBMIT_PROPOSAL,
            type_urls::MSG_VOTE,
            type_urls::MSG_VOTE_WEIGHTED,
            type_urls::MSG_DEPOSIT,
            type_urls::MSG_TRANSFER,
            type_urls::MSG_CHANNEL_OPEN_INIT,
            type_urls::MSG_CHANNEL_OPEN_TRY,
            type_urls::MSG_RECV_PACKET,
            type_urls::MSG_ACKNOWLEDGEMENT,
            type_urls::MSG_TIMEOUT,
        ];
        
        // For simplicity, we'll test that each type URL is recognized
        // (actual message decoding would fail with empty bytes, but that's expected)
        for msg_type in message_types {
            let response = route_cosmos_message(
                &mut handler,
                msg_type.to_string(),
                Base64VecU8(vec![]), // Empty bytes will cause decode error
            );
            
            // Should get decode error, not unknown message type error
            assert_eq!(response.code, 1);
            assert!(response.log.contains("decode error") || response.log.contains("JSON decode error"));
        }
    }
}