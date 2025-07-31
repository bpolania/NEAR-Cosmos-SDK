# Cosmos SDK Compatibility Plan

## Overview

This document outlines the plan to transform the NEAR-Cosmos-SDK from a Cosmos-inspired smart contract into a system that closely mimics a traditional Cosmos chain. While this isn't a traditional Cosmos chain yet, the implementation of these phases will bring us remarkably close to achieving full Cosmos SDK compatibility within NEAR's smart contract architecture.

**Current Status**: ~40% Traditional Cosmos Chain Compatibility
**Target Status**: ~75% Traditional Cosmos Chain Compatibility

## Three-Phase Implementation Roadmap

### Phase 1: Cosmos SDK Message & Query Compatibility (2-3 weeks)
Transform the contract to accept and process standard Cosmos SDK message types and queries, enabling compatibility with Cosmos tooling.

**Key Deliverables:**
- Standard Cosmos SDK message types (`/cosmos.bank.v1beta1.MsgSend`, etc.)
- ABCI query interface (`/cosmos.bank.v1beta1.Query/Balance`, etc.)
- Message router with type URL support
- Backward compatibility layer

### Phase 2: Transaction Processing Compatibility (3-4 weeks)
Implement standard Cosmos SDK transaction format processing, enabling wallets and CLI tools to interact seamlessly.

**Key Deliverables:**
- Cosmos SDK transaction format (`CosmosTx` with body, auth_info, signatures)
- Signature verification for Cosmos transactions
- Transaction result formatting
- Fee handling adaptation

### Phase 3: Advanced Cosmos SDK Features (2-3 weeks)
Implement advanced patterns that make the contract behave like a full Cosmos SDK chain.

**Key Deliverables:**
- Module Manager pattern
- Keeper pattern implementation
- Standard event emission
- Block processing hooks enhancement

## What This Achieves

**Compatibility Gained:**
- ✅ CosmJS library compatibility
- ✅ Cosmos CLI tools integration
- ✅ Block explorer API support
- ✅ Wallet integration (Keplr, etc.)
- ✅ IBC tooling compatibility

**NEAR Benefits Retained:**
- ✅ NEAR's performance and low costs
- ✅ NEAR's account model
- ✅ NEAR's developer ecosystem
- ✅ No need for separate consensus

**Limitations (Due to NEAR Architecture):**
- ❌ Tendermint consensus (uses NEAR consensus)
- ❌ Native Cosmos SDK Go modules
- ❌ CosmWasm (different runtime)
- ❌ Cosmos Hub validator participation

---

# Phase 1: Cosmos SDK Message & Query Compatibility - Detailed Plan

## Overview
Transform the NEAR-Cosmos-SDK contract to accept and process standard Cosmos SDK message types and queries, enabling compatibility with Cosmos tooling while maintaining the existing NEAR architecture.

## Timeline: 2-3 weeks

## Week 1: Message Type Implementation

### 1.1 Standard Message Types (3 days)

Create standard Cosmos SDK message structures for all modules:

**File**: `crates/cosmos-sdk-contract/src/types/cosmos_messages.rs`

**Bank Module Messages:**
- `MsgSend`: Transfer tokens between accounts
- `MsgMultiSend`: Multiple transfers in one transaction
- `MsgBurn`: Burn tokens (if supported)

**Staking Module Messages:**
- `MsgDelegate`: Delegate tokens to validator
- `MsgUndelegate`: Undelegate tokens
- `MsgBeginRedelegate`: Redelegate to another validator
- `MsgCreateValidator`: Create new validator
- `MsgEditValidator`: Edit validator details

**Governance Module Messages:**
- `MsgSubmitProposal`: Submit governance proposal
- `MsgVote`: Vote on proposal
- `MsgDeposit`: Add deposit to proposal
- `MsgVoteWeighted`: Weighted voting

**IBC Module Messages:**
- `MsgTransfer`: ICS-20 token transfer
- `MsgChannelOpenInit`: Initialize channel
- `MsgChannelOpenTry`: Channel handshake
- `MsgRecvPacket`: Receive IBC packet
- `MsgAcknowledgement`: Acknowledge packet

### 1.2 Message Router Implementation (2 days)

**File**: `crates/cosmos-sdk-contract/src/handler/msg_router.rs`

Create a unified message handler that:
- Accepts Cosmos SDK type URLs (e.g., `/cosmos.bank.v1beta1.MsgSend`)
- Decodes message data from protobuf-compatible format
- Routes to appropriate module handler
- Returns standard response format

**Key Functions:**
```rust
pub fn handle_cosmos_msg(&mut self, msg_type: String, msg_data: Base64VecU8) -> HandleResponse
fn decode_protobuf_compatible<T>(data: Vec<u8>) -> Result<T, DecodeError>
fn route_to_handler(&mut self, type_url: &str, msg: Any) -> Result<HandleResult, ContractError>
```

### 1.3 Message Handlers Refactoring (2 days)

Adapt existing module handlers to use standard message types:

**Bank Module Handler Updates:**
- Refactor `transfer` to use `MsgSend`
- Add multi-send support
- Emit standard Cosmos events

**Staking Module Handler Updates:**
- Update delegation logic for `MsgDelegate`
- Standardize validator operations
- Add redelegation support

**Governance Module Handler Updates:**
- Align proposal submission with `MsgSubmitProposal`
- Standardize voting with `MsgVote`
- Add weighted voting support

## Week 2: Query Interface Standardization

### 2.1 Query Types Implementation (2 days)

**File**: `crates/cosmos-sdk-contract/src/types/cosmos_queries.rs`

Implement standard query request/response types:

**Bank Queries:**
- `QueryBalanceRequest/Response`: Single denomination balance
- `QueryAllBalancesRequest/Response`: All balances for account
- `QueryTotalSupplyRequest/Response`: Total supply of tokens
- `QuerySupplyOfRequest/Response`: Supply of specific denom

**Staking Queries:**
- `QueryValidatorsRequest/Response`: List validators
- `QueryValidatorRequest/Response`: Single validator info
- `QueryDelegationRequest/Response`: Delegation details
- `QueryUnbondingDelegationRequest/Response`: Unbonding info

**Governance Queries:**
- `QueryProposalRequest/Response`: Single proposal
- `QueryProposalsRequest/Response`: List proposals
- `QueryVoteRequest/Response`: Vote details
- `QueryTallyResultRequest/Response`: Proposal tally

### 2.2 ABCI Query Router (2 days)

**File**: `crates/cosmos-sdk-contract/src/handler/query_router.rs`

Implement standard ABCI query interface:
- Accept standard Cosmos SDK query paths
- Decode query parameters
- Route to appropriate query handler
- Return ABCI-compatible responses

**Key Functions:**
```rust
pub fn abci_query(&self, path: String, data: Base64VecU8) -> QueryResponse
fn decode_query<T>(data: Vec<u8>) -> Result<T, QueryError>
fn encode_response<T>(response: &T) -> Result<Vec<u8>, QueryError>
```

### 2.3 Query Handler Implementation (1 day)

Implement query handlers using existing module data:
- Map existing storage to standard query responses
- Add pagination support where applicable
- Ensure efficient query execution

## Week 3: Integration & Testing

### 3.1 Compatibility Layer (2 days)

**File**: `crates/cosmos-sdk-contract/src/compat/mod.rs`

Create compatibility functions:
- Maintain backward compatibility with existing NEAR methods
- Add helper functions for migration
- Create type conversion utilities
- Document migration path for existing users

### 3.2 Testing Suite (3 days)

**File**: `crates/cosmos-sdk-contract/tests/cosmos_compatibility_tests.rs`

Comprehensive test coverage:
- Message handling tests for all modules
- Query interface tests
- Integration tests with encoded messages
- Performance benchmarks
- Backward compatibility tests

## Implementation Details

### Message Encoding/Decoding

We'll use a protobuf-compatible JSON encoding initially:
```rust
fn decode_protobuf_compatible<T: DeserializeOwned>(data: Vec<u8>) -> Result<T, DecodeError> {
    // Start with JSON, migrate to protobuf later
    serde_json::from_slice(&data).map_err(|e| DecodeError::InvalidFormat(e.to_string()))
}
```

### Event Emission

Adapt NEAR logs to Cosmos event format:
```rust
fn emit_cosmos_event(&self, event_type: &str, attributes: Vec<(String, String)>) {
    let event = CosmosEvent {
        r#type: event_type.to_string(),
        attributes: attributes.into_iter()
            .map(|(k, v)| Attribute { key: k, value: v })
            .collect(),
    };
    near_sdk::log!("COSMOS_EVENT: {}", serde_json::to_string(&event).unwrap());
}
```

### Address Handling

Support both NEAR and Cosmos address formats:
```rust
fn validate_cosmos_address(addr: &str) -> Result<(), ContractError> {
    if addr.starts_with("cosmos1") || addr.starts_with("near:") || addr.ends_with(".near") {
        Ok(())
    } else {
        Err(ContractError::InvalidAddress)
    }
}
```

## Testing Strategy

### Unit Tests
- Test each message type handler
- Test each query handler
- Test encoding/decoding functions

### Integration Tests
- Test with actual CosmJS library
- Test with Cosmos SDK CLI tools
- Test backward compatibility

### Performance Tests
- Benchmark message processing overhead
- Measure query response times
- Ensure < 5% performance impact

## Migration Guide

For existing users:
1. Existing methods continue to work unchanged
2. New Cosmos SDK methods available in parallel
3. Gradual migration path provided
4. Full documentation with examples

## Success Criteria

- [ ] All standard message types implemented
- [ ] All standard query types implemented
- [ ] CosmJS can successfully interact with contract
- [ ] Cosmos CLI tools can query the contract
- [ ] 100% backward compatibility maintained
- [ ] Performance overhead < 5%
- [ ] Comprehensive test coverage
- [ ] Documentation complete

## Next Steps

After Phase 1 completion:
- Phase 2: Add transaction processing compatibility
- Phase 3: Implement advanced Cosmos SDK patterns
- Integration with existing Cosmos tools
- Community feedback and iteration