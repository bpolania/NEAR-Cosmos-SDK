# Phase 2 Development Plan: Transaction Processing Compatibility

## Overview

Transform Proxima to accept and process standard Cosmos SDK transaction formats, enabling seamless integration with Cosmos wallets, CLI tools, and ecosystem tooling while maintaining NEAR's performance benefits.

**Timeline**: 3-4 weeks  
**Current Status**: Phase 1 Complete (Message Router with Type URL Support)  
**Target**: ~85% Traditional Cosmos Chain Compatibility

## Phase 2 Goals

### Primary Objectives
- Accept standard Cosmos SDK transaction format (`CosmosTx`)
- Implement signature verification for Cosmos transactions  
- Enable wallet integration (Keplr, Leap, etc.)
- Support Cosmos CLI tools (`gaiad`, `osmosisd`, etc.)
- Maintain full backward compatibility with existing NEAR methods

### Compatibility Targets
- ✅ CosmJS library full compatibility
- ✅ Cosmos SDK CLI tools integration  
- ✅ Hardware wallet support (Ledger)
- ✅ Multi-signature transaction support
- ✅ Transaction fee handling adaptation

---

## Week 1: Core Transaction Structure Implementation

### 1.1 Cosmos Transaction Types (3 days)

**File**: `crates/cosmos-sdk-contract/src/types/cosmos_tx.rs`

Implement standard Cosmos SDK transaction structures:

```rust
// Core transaction structure
pub struct CosmosTx {
    pub body: TxBody,
    pub auth_info: AuthInfo,
    pub signatures: Vec<Vec<u8>>,
}

pub struct TxBody {
    pub messages: Vec<Any>,
    pub memo: String,
    pub timeout_height: u64,
    pub extension_options: Vec<Any>,
    pub non_critical_extension_options: Vec<Any>,
}

pub struct AuthInfo {
    pub signer_infos: Vec<SignerInfo>,
    pub fee: Fee,
    pub tip: Option<Tip>,
}

pub struct SignerInfo {
    pub public_key: Option<Any>,
    pub mode_info: ModeInfo,
    pub sequence: u64,
}

pub struct Fee {
    pub amount: Vec<Coin>,
    pub gas_limit: u64,
    pub payer: String,
    pub granter: String,
}
```

**Implementation Tasks:**
- Define all transaction-related structures
- Implement serialization/deserialization for each type
- Add validation methods for each structure
- Create builder patterns for easy construction

### 1.2 Transaction Decoding Pipeline (2 days)

**File**: `crates/cosmos-sdk-contract/src/handler/tx_decoder.rs`

Create transaction decoding infrastructure:

```rust
pub struct TxDecoder {
    // Configuration and validation rules
}

impl TxDecoder {
    pub fn decode_cosmos_tx(&self, raw_tx: Vec<u8>) -> Result<CosmosTx, TxError> {
        // Decode protobuf-compatible transaction
        // Validate structure integrity
        // Extract and validate messages
    }
    
    pub fn validate_tx_structure(&self, tx: &CosmosTx) -> Result<(), TxError> {
        // Validate transaction format
        // Check message compatibility
        // Verify fee structure
    }
    
    pub fn extract_messages(&self, tx: &CosmosTx) -> Result<Vec<CosmosMessage>, TxError> {
        // Extract messages from transaction body
        // Convert to internal message format
    }
}
```

**Implementation Tasks:**
- Implement protobuf-compatible decoding (start with JSON, migrate to protobuf)
- Add comprehensive validation for transaction structure
- Create error handling for malformed transactions
- Implement message extraction and conversion

### 1.3 Signature Verification System (2 days)

**File**: `crates/cosmos-sdk-contract/src/crypto/cosmos_signatures.rs`

Implement Cosmos SDK signature verification:

```rust
pub struct CosmosSignatureVerifier {
    // Signature verification configuration
}

impl CosmosSignatureVerifier {
    pub fn verify_signatures(&self, tx: &CosmosTx, sign_doc: &SignDoc) -> Result<(), SignatureError> {
        // Verify each signature in the transaction
        // Support secp256k1 signatures (Cosmos standard)
        // Handle multi-signature scenarios
    }
    
    pub fn create_sign_doc(&self, tx: &CosmosTx, chain_id: &str, account_number: u64) -> SignDoc {
        // Create canonical signing document
        // Match Cosmos SDK signing format exactly
    }
    
    pub fn recover_public_keys(&self, tx: &CosmosTx, sign_doc: &SignDoc) -> Result<Vec<PublicKey>, SignatureError> {
        // Recover public keys from signatures
        // Validate against signer_infos
    }
}
```

**Dependencies to Add:**
```toml
[dependencies]
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }
sha2 = "0.10"
ripemd = "0.1"
```

**Implementation Tasks:**
- Implement secp256k1 signature verification
- Create canonical signing document generation
- Add public key recovery and validation
- Support for compressed/uncompressed public keys

---

## Week 2: Transaction Processing Integration

### 2.1 Transaction Handler Integration (3 days)

**File**: `crates/cosmos-sdk-contract/src/handler/tx_handler.rs`

Create unified transaction processing pipeline:

```rust
pub struct CosmosTransactionHandler {
    pub msg_router: CosmosMessageRouter,
    pub tx_decoder: TxDecoder,
    pub signature_verifier: CosmosSignatureVerifier,
}

impl CosmosTransactionHandler {
    pub fn process_cosmos_transaction(&mut self, raw_tx: Vec<u8>) -> Result<TxResponse, ContractError> {
        // 1. Decode transaction
        let tx = self.tx_decoder.decode_cosmos_tx(raw_tx)?;
        
        // 2. Verify signatures
        let sign_doc = self.create_sign_doc(&tx);
        self.signature_verifier.verify_signatures(&tx, &sign_doc)?;
        
        // 3. Process messages sequentially
        let mut responses = Vec::new();
        for msg in tx.body.messages {
            let response = self.msg_router.handle_cosmos_msg(
                msg.type_url,
                msg.value
            )?;
            responses.push(response);
        }
        
        // 4. Handle fees and gas
        self.process_transaction_fees(&tx.auth_info.fee)?;
        
        // 5. Return standardized response
        Ok(TxResponse::from_message_responses(responses))
    }
}
```

**Implementation Tasks:**
- Integrate with existing message router from Phase 1
- Add transaction-level validation and processing
- Implement proper error handling and rollback
- Create standardized transaction response format

### 2.2 Account Management System (2 days)

**File**: `crates/cosmos-sdk-contract/src/modules/auth/accounts.rs`

Implement Cosmos SDK account management:

```rust
pub struct CosmosAccount {
    pub address: String,           // Cosmos bech32 address
    pub account_number: u64,       // Sequential account number
    pub sequence: u64,             // Transaction sequence number
    pub public_key: Option<PublicKey>,
}

pub struct AccountManager {
    // Account storage and management
}

impl AccountManager {
    pub fn get_account(&self, address: &str) -> Option<CosmosAccount> {
        // Retrieve account by Cosmos address
        // Support both Cosmos and NEAR address formats
    }
    
    pub fn create_account(&mut self, address: &str, public_key: PublicKey) -> Result<CosmosAccount, AuthError> {
        // Create new Cosmos-compatible account
        // Assign account number and initialize sequence
    }
    
    pub fn increment_sequence(&mut self, address: &str) -> Result<u64, AuthError> {
        // Increment and return new sequence number
        // Used for replay protection
    }
    
    pub fn validate_sequence(&self, address: &str, expected_sequence: u64) -> Result<(), AuthError> {
        // Validate transaction sequence number
        // Prevent replay attacks
    }
}
```

**Implementation Tasks:**
- Design account storage schema compatible with both NEAR and Cosmos
- Implement account number assignment system
- Add sequence number management for replay protection
- Create address conversion utilities (NEAR ↔ Cosmos)

### 2.3 Fee Processing Adaptation (2 days)

**File**: `crates/cosmos-sdk-contract/src/modules/auth/fees.rs`

Adapt Cosmos fee model to NEAR:

```rust
pub struct FeeProcessor {
    // Fee processing configuration
}

impl FeeProcessor {
    pub fn process_transaction_fees(&mut self, fee: &Fee, payer: &str) -> Result<(), FeeError> {
        // Convert Cosmos fees to NEAR gas costs
        // Handle fee payment from specified account
        // Support fee grants and delegation
    }
    
    pub fn calculate_minimum_fee(&self, gas_limit: u64) -> Fee {
        // Calculate minimum required fee based on gas limit
        // Use NEAR gas prices as basis
    }
    
    pub fn validate_fee_payment(&self, fee: &Fee, payer: &str) -> Result<(), FeeError> {
        // Validate fee payer has sufficient balance
        // Check fee amount meets minimum requirements
    }
}
```

**Fee Mapping Strategy:**
- Map Cosmos gas units to NEAR gas units (1:1000 ratio)
- Use NEAR's native token for fee payment initially
- Support multi-token fees in future iterations

---

## Week 3: Advanced Transaction Features

### 3.1 Multi-Signature Support (3 days)

**File**: `crates/cosmos-sdk-contract/src/crypto/multisig.rs`

Implement multi-signature transaction support:

```rust
pub struct MultiSigInfo {
    pub threshold: u32,
    pub public_keys: Vec<PublicKey>,
}

pub struct MultiSigVerifier {
    // Multi-signature verification logic
}

impl MultiSigVerifier {
    pub fn verify_multisig(&self, 
        signatures: &[Vec<u8>], 
        sign_doc: &SignDoc,
        multisig_info: &MultiSigInfo
    ) -> Result<(), MultiSigError> {
        // Verify threshold number of valid signatures
        // Support different signature ordering
        // Handle missing signatures
    }
    
    pub fn create_multisig_sign_doc(&self, 
        tx: &CosmosTx,
        multisig_info: &MultiSigInfo
    ) -> SignDoc {
        // Create signing document for multi-sig accounts
        // Include multi-sig public key information
    }
}
```

**Implementation Tasks:**
- Implement threshold signature verification
- Add multi-signature account creation
- Support partial signature collection
- Create multi-sig transaction building utilities

### 3.2 Hardware Wallet Integration (2 days)

**File**: `crates/cosmos-sdk-contract/src/crypto/hw_wallets.rs`

Add hardware wallet compatibility:

```rust
pub struct HardwareWalletSupport {
    // Hardware wallet integration utilities
}

impl HardwareWalletSupport {
    pub fn verify_hw_signature(&self, 
        signature: &[u8], 
        sign_doc: &SignDoc,
        derivation_path: &str
    ) -> Result<PublicKey, HwError> {
        // Verify hardware wallet signatures
        // Support standard Cosmos derivation paths
        // Handle different hardware wallet formats
    }
    
    pub fn create_hw_sign_doc(&self, tx: &CosmosTx) -> HwSignDoc {
        // Create hardware wallet compatible signing document
        // Minimize data size for hardware display
    }
}
```

**Supported Hardware Wallets:**
- Ledger Nano S/X (primary target)
- Trezor (secondary support)
- Standard derivation path: `m/44'/118'/0'/0/0`

### 3.3 Transaction Response Formatting (2 days)

**File**: `crates/cosmos-sdk-contract/src/types/tx_response.rs`

Implement standard Cosmos transaction response format:

```rust
pub struct TxResponse {
    pub height: String,
    pub txhash: String,
    pub codespace: String,
    pub code: u32,
    pub data: String,
    pub raw_log: String,
    pub logs: Vec<ABCIMessageLog>,
    pub info: String,
    pub gas_wanted: String,
    pub gas_used: String,
    pub tx: Option<CosmosTx>,
    pub timestamp: String,
    pub events: Vec<Event>,
}

pub struct ABCIMessageLog {
    pub msg_index: u32,
    pub log: String,
    pub events: Vec<StringEvent>,
}
```

**Implementation Tasks:**
- Convert NEAR transaction results to Cosmos format
- Map NEAR events to ABCI events
- Create transaction hash compatible with Cosmos ecosystem
- Implement proper error code mapping

---

## Week 4: Integration, Testing & Documentation

### 4.1 Public API Implementation (2 days)

**File**: `crates/cosmos-sdk-contract/src/lib.rs`

Add new public methods to the contract:

```rust
impl CosmosContract {
    /// Process a standard Cosmos SDK transaction
    #[payable]
    pub fn broadcast_tx_sync(&mut self, tx_bytes: Base64VecU8) -> TxResponse {
        self.tx_handler.process_cosmos_transaction(tx_bytes.into())
            .unwrap_or_else(|e| TxResponse::error(e))
    }
    
    /// Simulate a transaction without executing it
    pub fn simulate_tx(&self, tx_bytes: Base64VecU8) -> SimulateResponse {
        // Dry-run transaction to estimate gas and validate
    }
    
    /// Get account information by address
    pub fn get_account(&self, address: String) -> Option<CosmosAccount> {
        self.account_manager.get_account(&address)
    }
    
    /// Check transaction by hash
    pub fn get_tx(&self, hash: String) -> Option<TxResponse> {
        // Retrieve transaction by hash from storage
    }
}
```

### 4.2 Comprehensive Testing Suite (3 days)

**File**: `crates/cosmos-sdk-contract/tests/cosmos_tx_tests.rs`

Create extensive test coverage:

```rust
#[cfg(test)]
mod cosmos_tx_tests {
    // Transaction decoding tests
    #[tokio::test]
    async fn test_decode_standard_tx() { }
    
    #[tokio::test]
    async fn test_decode_multisig_tx() { }
    
    // Signature verification tests
    #[tokio::test]
    async fn test_secp256k1_signature_verification() { }
    
    #[tokio::test]
    async fn test_invalid_signature_rejection() { }
    
    // Fee processing tests
    #[tokio::test]
    async fn test_fee_deduction() { }
    
    #[tokio::test]
    async fn test_insufficient_fees() { }
    
    // Integration tests with CosmJS
    #[tokio::test]
    async fn test_cosmjs_transaction_submission() { }
    
    // Hardware wallet tests
    #[tokio::test]
    async fn test_ledger_signature_verification() { }
    
    // Account management tests
    #[tokio::test]
    async fn test_account_creation_and_sequence() { }
    
    // Multi-signature tests
    #[tokio::test]
    async fn test_multisig_threshold_verification() { }
}
```

**Test Categories:**
- **Unit Tests**: Each component in isolation (50+ tests)
- **Integration Tests**: Full transaction processing flow (20+ tests)
- **CosmJS Integration**: Real CosmJS library interaction (10+ tests)
- **Hardware Wallet Tests**: Ledger/Trezor compatibility (10+ tests)
- **Performance Tests**: Transaction throughput benchmarks (5+ tests)

### 4.3 Documentation & Migration Guide (2 days)

**Files**:
- `docs/COSMOS_TX_INTEGRATION.md` - Transaction processing guide
- `docs/WALLET_INTEGRATION.md` - Wallet integration instructions
- `docs/MIGRATION_GUIDE_PHASE2.md` - Migration from Phase 1

**Documentation Sections:**
1. **Transaction Format Guide**: How to construct Cosmos transactions
2. **Wallet Integration**: Step-by-step wallet connection guide
3. **Fee Structure**: Understanding fee mapping NEAR ↔ Cosmos
4. **Multi-Signature Setup**: Creating and using multi-sig accounts
5. **Hardware Wallet Guide**: Ledger/Trezor integration steps
6. **API Reference**: Complete method documentation with examples
7. **Troubleshooting**: Common issues and solutions

### 4.4 Backward Compatibility Validation (1 day)

Ensure existing functionality remains intact:

```rust
#[tokio::test]
async fn test_phase1_message_compatibility() {
    // Verify Phase 1 message router still works
    // Test all existing message types
    // Ensure no breaking changes
}

#[tokio::test]
async fn test_near_method_compatibility() {
    // Verify existing NEAR methods still function
    // Test direct method calls
    // Validate response formats
}
```

---

## Implementation Strategy

### Dependencies to Add

```toml
[dependencies]
# Cryptography
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }
sha2 = "0.10"
ripemd = "0.1"
bech32 = "0.9"

# Serialization
prost = "0.12"
prost-types = "0.12"

# Async/await support (for testing)
tokio = { version = "1.0", features = ["full"] }

# Additional utilities
hex = "0.4"
base64 = "0.21"
```

### Build Configuration

```toml
[build-dependencies]
prost-build = "0.12"
```

### Error Handling Strategy

Create comprehensive error types:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    // Transaction structure errors
    InvalidTxFormat(String),
    InvalidSignature(String),
    InvalidSequence { expected: u64, actual: u64 },
    InsufficientFees { required: String, provided: String },
    
    // Account errors
    AccountNotFound(String),
    InvalidAddress(String),
    
    // Multi-signature errors
    InsufficientSignatures { threshold: u32, provided: u32 },
    InvalidMultiSigSetup(String),
    
    // Processing errors
    MessageProcessingFailed(String),
    GasLimitExceeded { limit: u64, used: u64 },
}
```

### Testing Strategy

1. **Unit Tests**: Test each component independently
2. **Integration Tests**: Test full transaction flow
3. **Real-world Tests**: Test with actual Cosmos tooling
4. **Performance Tests**: Ensure acceptable transaction throughput
5. **Security Tests**: Validate signature verification and replay protection

### Migration Path

For existing users:
1. **Phase 1 APIs**: Continue to work unchanged
2. **New Transaction APIs**: Available in parallel
3. **Gradual Migration**: Documentation and examples provided
4. **Helper Utilities**: Tools to convert between formats

---

## Success Criteria

### Phase 2 Completion Checklist

- [ ] **Core Transaction Processing**
  - [ ] CosmosTx structure fully implemented
  - [ ] Transaction decoding pipeline functional
  - [ ] Signature verification working for secp256k1
  - [ ] Account management system operational

- [ ] **Advanced Features**
  - [ ] Multi-signature support implemented
  - [ ] Hardware wallet compatibility verified
  - [ ] Fee processing adapted to NEAR

- [ ] **Integration**
  - [ ] CosmJS library can submit transactions successfully
  - [ ] Cosmos CLI tools can interact with contract
  - [ ] Hardware wallets (Ledger) can sign transactions

- [ ] **Quality Assurance**
  - [ ] 100+ comprehensive tests passing
  - [ ] Performance overhead < 10%
  - [ ] 100% backward compatibility maintained
  - [ ] Complete documentation provided

- [ ] **Production Readiness**
  - [ ] Security audit completed
  - [ ] Error handling comprehensive
  - [ ] Monitoring and metrics implemented
  - [ ] Deployment scripts updated

### Performance Targets

- **Transaction Processing**: < 2 seconds per transaction
- **Signature Verification**: < 100ms per signature
- **Multi-sig Verification**: < 500ms for 3-of-5 setup
- **Memory Usage**: < 50MB increase over Phase 1
- **Gas Consumption**: < 20% increase over direct method calls

### Compatibility Targets

- **CosmJS**: 100% transaction submission compatibility
- **Keplr Wallet**: Full transaction signing support
- **Ledger Hardware**: Native transaction signing
- **Cosmos CLI**: Full transaction broadcast support
- **Block Explorers**: Standard transaction format display

---

## Next Steps After Phase 2

Upon completion, Phase 2 will enable:
- Full wallet ecosystem integration
- Standard Cosmos tooling compatibility
- Hardware wallet transaction signing
- Multi-signature account management
- ~85% traditional Cosmos chain compatibility

**Phase 3 Preview**: Advanced Cosmos SDK Features
- Module Manager pattern implementation
- Keeper pattern for module interaction
- Enhanced event emission system
- Block processing hooks enhancement
- ~95% traditional Cosmos chain compatibility

This comprehensive Phase 2 implementation will establish Proxima as a fully wallet-compatible Cosmos SDK runtime on NEAR, opening the door to broader ecosystem adoption and integration.