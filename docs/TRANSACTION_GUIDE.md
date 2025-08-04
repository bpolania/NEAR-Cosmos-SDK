# Cosmos SDK Transaction Building Guide

This guide explains how to construct valid Cosmos SDK transactions for use with the Proxima smart contract on NEAR Protocol.

## Overview

Proxima processes transactions in standard Cosmos SDK format, enabling compatibility with existing Cosmos tooling while running on NEAR Protocol. This guide covers the complete transaction structure and construction process.

## Transaction Structure

A complete Cosmos SDK transaction consists of three main components:

```rust
pub struct CosmosTx {
    pub body: TxBody,           // Transaction content and messages
    pub auth_info: AuthInfo,    // Authentication and fee information
    pub signatures: Vec<Vec<u8>> // Cryptographic signatures
}
```

### 1. Transaction Body (TxBody)

Contains the actual transaction content:

```rust
pub struct TxBody {
    pub messages: Vec<Any>,     // List of messages to execute
    pub memo: String,           // Optional memo (max 256 characters)
    pub timeout_height: u64,    // Block height timeout (0 = no timeout)
    pub extension_options: Vec<Any>,    // Extension options (usually empty)
    pub non_critical_extension_options: Vec<Any> // Non-critical extensions
}
```

**Example TxBody**:
```json
{
  "messages": [
    {
      "@type": "/cosmos.bank.v1beta1.MsgSend",
      "from_address": "cosmos1sender...",
      "to_address": "cosmos1recipient...",
      "amount": [
        {
          "denom": "unear",
          "amount": "1000000"
        }
      ]
    }
  ],
  "memo": "Transfer from Alice to Bob",
  "timeout_height": 0,
  "extension_options": [],
  "non_critical_extension_options": []
}
```

### 2. Authentication Info (AuthInfo)

Contains signature and fee information:

```rust
pub struct AuthInfo {
    pub signer_infos: Vec<SignerInfo>, // Signer information
    pub fee: Fee                       // Transaction fee
}
```

#### SignerInfo Structure:
```rust
pub struct SignerInfo {
    pub public_key: Option<Any>,    // Public key (can be None)
    pub mode_info: ModeInfo,        // Signing mode information
    pub sequence: u64               // Account sequence number
}

pub struct ModeInfo {
    pub mode: SignMode,            // Signing mode (Direct, Textual, etc.)
    pub multi: Option<MultiInfo>   // Multi-signature info (if applicable)
}
```

#### Fee Structure:
```rust
pub struct Fee {
    pub amount: Vec<Coin>,    // Fee amount in various denominations
    pub gas_limit: u64,       // Maximum gas allowed
    pub payer: String,        // Fee payer address (optional)
    pub granter: String       // Fee granter address (optional)
}

pub struct Coin {
    pub denom: String,        // Denomination (e.g., "unear", "uatom")
    pub amount: String        // Amount as string (supports large numbers)
}
```

**Example AuthInfo**:
```json
{
  "signer_infos": [
    {
      "public_key": null,
      "mode_info": {
        "mode": "Direct",
        "multi": null
      },
      "sequence": 5
    }
  ],
  "fee": {
    "amount": [
      {
        "denom": "unear",
        "amount": "1000"
      }
    ],
    "gas_limit": 200000,
    "payer": "",
    "granter": ""
  }
}
```

### 3. Signatures

Array of signature bytes from all required signers:

```rust
pub signatures: Vec<Vec<u8>>  // Each signature is 65 bytes for secp256k1
```

## Supported Message Types

Proxima supports all Phase 1 Cosmos SDK message types:

### Bank Messages
- **MsgSend**: Transfer tokens between accounts
- **MsgMultiSend**: Transfer tokens to multiple recipients

### Staking Messages  
- **MsgDelegate**: Delegate tokens to a validator
- **MsgUndelegate**: Undelegate tokens from a validator
- **MsgRedelegate**: Redelegate tokens to a different validator

### Governance Messages
- **MsgSubmitProposal**: Submit a governance proposal
- **MsgVote**: Vote on a governance proposal
- **MsgDeposit**: Deposit tokens to a proposal

### Example Messages

#### MsgSend (Token Transfer)
```json
{
  "@type": "/cosmos.bank.v1beta1.MsgSend",
  "from_address": "cosmos1sender123...",
  "to_address": "cosmos1recipient456...",
  "amount": [
    {
      "denom": "unear",
      "amount": "1000000"
    }
  ]
}
```

#### MsgDelegate (Staking)
```json
{
  "@type": "/cosmos.staking.v1beta1.MsgDelegate", 
  "delegator_address": "cosmos1delegator...",
  "validator_address": "cosmosvaloper1validator...",
  "amount": {
    "denom": "unear",
    "amount": "5000000"
  }
}
```

#### MsgVote (Governance)
```json
{
  "@type": "/cosmos.gov.v1beta1.MsgVote",
  "proposal_id": "1",
  "voter": "cosmos1voter...",
  "option": "Yes"
}
```

## Step-by-Step Transaction Construction

### Step 1: Create Messages
```javascript
const messages = [
  {
    "@type": "/cosmos.bank.v1beta1.MsgSend",
    "from_address": senderAddress,
    "to_address": recipientAddress,
    "amount": [{ "denom": "unear", "amount": "1000000" }]
  }
];
```

### Step 2: Build Transaction Body
```javascript
const txBody = {
  "messages": messages,
  "memo": "My transaction memo",
  "timeout_height": 0,
  "extension_options": [],
  "non_critical_extension_options": []
};
```

### Step 3: Create Fee Structure
```javascript
const fee = {
  "amount": [{ "denom": "unear", "amount": "1000" }],
  "gas_limit": 200000,
  "payer": "",
  "granter": ""
};
```

### Step 4: Build AuthInfo
```javascript
const authInfo = {
  "signer_infos": [
    {
      "public_key": null, // Can be omitted for known accounts
      "mode_info": {
        "mode": "Direct",
        "multi": null
      },
      "sequence": accountSequence
    }
  ],
  "fee": fee
};
```

### Step 5: Create Sign Document
```javascript
const signDoc = {
  "body_bytes": encodeToProtobuf(txBody),
  "auth_info_bytes": encodeToProtobuf(authInfo),
  "chain_id": "proxima-testnet-1",
  "account_number": accountNumber
};
```

### Step 6: Sign Transaction
```javascript
// Sign the transaction using secp256k1
const signature = await signTransaction(signDoc, privateKey);
```

### Step 7: Assemble Final Transaction
```javascript
const completeTx = {
  "body": txBody,
  "auth_info": authInfo,
  "signatures": [signature]
};

// Encode to Base64 for API submission
const txBytes = base64Encode(JSON.stringify(completeTx));
```

## Account Management

### Account Numbers and Sequences

Every Cosmos account has two important identifiers:

- **Account Number**: Permanent identifier assigned when account is first used
- **Sequence Number**: Incremental counter preventing replay attacks

**Key Rules**:
- Account numbers start from 1 (not 0)
- Sequence numbers start from 0 for new accounts  
- Sequence must increment with each transaction
- Concurrent transactions with same sequence will fail

**Example Account State**:
```json
{
  "account_number": 42,
  "sequence": 7,
  "address": "cosmos1abc123...",
  "public_key": "secp256k1_public_key"
}
```

### Address Format

Proxima uses standard Cosmos bech32 addresses:
- **Format**: `cosmos1{20-byte-hash}`
- **Derivation**: From secp256k1 public key
- **Validation**: Checksum verification

**Example**: `cosmos1hf0hnb72pneasmr3ujgfd5u3gg5l86yuq7uwsd`

## Fee Calculation

### Gas and Fees

Transactions require gas to execute, and fees must be paid in supported denominations:

**Gas Estimation**:
- Use `simulate_tx()` to estimate gas consumption
- Add 10-20% buffer for gas price fluctuations
- Consider message complexity (transfers vs staking vs governance)

**Fee Calculation**:
```
fee_amount = gas_used × gas_price
```

**Example Fee Structures**:

**Simple Transfer**:
```json
{
  "amount": [{ "denom": "unear", "amount": "1000" }],
  "gas_limit": 200000
}
```

**Complex Multi-Message Transaction**:
```json
{
  "amount": [{ "denom": "unear", "amount": "5000" }],
  "gas_limit": 800000
}
```

**Multiple Denomination Fees** (if supported):
```json
{
  "amount": [
    { "denom": "unear", "amount": "1000" },
    { "denom": "uatom", "amount": "500" }
  ],
  "gas_limit": 300000
}
```

## Signing Process

### secp256k1 Signature

Proxima uses secp256k1 signatures compatible with Cosmos SDK:

**Sign Document Structure**:
```json
{
  "body_bytes": "base64_encoded_tx_body",
  "auth_info_bytes": "base64_encoded_auth_info", 
  "chain_id": "proxima-testnet-1",
  "account_number": "42"
}
```

**Signature Process**:
1. Create canonical JSON from sign document
2. SHA256 hash the canonical JSON
3. Sign hash with secp256k1 private key
4. Result is 65-byte signature (r + s + recovery_id)

**Signature Verification**:
- Proxima validates signatures against account public keys
- Replay protection via sequence number incrementation
- Chain ID binding prevents cross-chain signature reuse

## Multi-Message Transactions

### Batching Messages

Multiple messages can be included in a single transaction:

```json
{
  "messages": [
    {
      "@type": "/cosmos.bank.v1beta1.MsgSend",
      "from_address": "cosmos1sender...",
      "to_address": "cosmos1recipient1...",
      "amount": [{ "denom": "unear", "amount": "1000000" }]
    },
    {
      "@type": "/cosmos.bank.v1beta1.MsgSend", 
      "from_address": "cosmos1sender...",
      "to_address": "cosmos1recipient2...",
      "amount": [{ "denom": "unear", "amount": "2000000" }]
    },
    {
      "@type": "/cosmos.staking.v1beta1.MsgDelegate",
      "delegator_address": "cosmos1sender...",
      "validator_address": "cosmosvaloper1validator...",
      "amount": { "denom": "unear", "amount": "5000000" }
    }
  ]
}
```

**Benefits**:
- Atomic execution (all messages succeed or all fail)
- Reduced transaction fees (single signature, single sequence increment)
- Consistent state changes

**Considerations**:
- Higher gas consumption
- More complex error debugging
- Longer transaction validation time

## Common Patterns

### Pattern 1: Simple Token Transfer

```javascript
function createTransferTx(from, to, amount, sequence, accountNumber) {
  return {
    body: {
      messages: [{
        "@type": "/cosmos.bank.v1beta1.MsgSend",
        from_address: from,
        to_address: to,
        amount: [{ denom: "unear", amount: amount.toString() }]
      }],
      memo: "",
      timeout_height: 0,
      extension_options: [],
      non_critical_extension_options: []
    },
    auth_info: {
      signer_infos: [{
        public_key: null,
        mode_info: { mode: "Direct", multi: null },
        sequence: sequence
      }],
      fee: {
        amount: [{ denom: "unear", amount: "1000" }],
        gas_limit: 200000,
        payer: "",
        granter: ""
      }
    },
    signatures: [] // To be filled after signing
  };
}
```

### Pattern 2: Delegate and Vote

```javascript
function createDelegateAndVoteTx(delegator, validator, amount, proposalId, sequence) {
  return {
    body: {
      messages: [
        {
          "@type": "/cosmos.staking.v1beta1.MsgDelegate",
          delegator_address: delegator,
          validator_address: validator,
          amount: { denom: "unear", amount: amount.toString() }
        },
        {
          "@type": "/cosmos.gov.v1beta1.MsgVote",
          proposal_id: proposalId.toString(),
          voter: delegator,
          option: "Yes"
        }
      ],
      memo: "Delegate and vote in one transaction",
      timeout_height: 0,
      extension_options: [],
      non_critical_extension_options: []
    },
    // ... auth_info and signatures
  };
}
```

## Validation Rules

### Transaction Validation

Proxima validates transactions according to these rules:

**Format Validation**:
- Valid JSON structure
- Required fields present
- Correct data types

**Business Logic Validation**:
- Sufficient account balance for transfers and fees
- Valid addresses (bech32 format)
- Positive amounts
- Valid validator addresses for staking operations

**Signature Validation**:
- Valid secp256k1 signatures
- Correct account sequence numbers
- Matching chain ID
- Account exists and has sufficient sequence

**Gas Validation**:
- Gas limit within maximum allowed
- Sufficient fee for gas consumption
- Supported fee denominations

### Common Validation Errors

| Error | Code | Description | Solution |
|-------|------|-------------|----------|
| Invalid sequence | 3 | Sequence number mismatch | Query current sequence and increment |
| Insufficient funds | 5 | Not enough balance | Check account balance |
| Unauthorized | 4 | Signature verification failed | Verify private key and signing process |
| TX decode error | 2 | Malformed transaction | Validate JSON structure and required fields |
| Out of gas | 12 | Gas limit exceeded | Increase gas limit or simplify transaction |

## Best Practices

### 1. Always Simulate First
```javascript
// Simulate before broadcasting
const simulation = await client.simulateTx(txBytes);
if (simulation.code === 0) {
  const result = await client.broadcastTxSync(txBytes);
}
```

### 2. Handle Sequence Numbers Properly
```javascript
// Track sequences for concurrent transactions
let accountSequence = await getAccountSequence(address);

for (const tx of pendingTransactions) {
  tx.auth_info.signer_infos[0].sequence = accountSequence++;
  await broadcastTx(tx);
}
```

### 3. Include Appropriate Memos
```javascript
const memo = `Transfer ID: ${transferId} | App: MyApp v1.0`;
```

### 4. Set Reasonable Gas Limits
```javascript
// Add 20% buffer to simulation results
const gasLimit = Math.ceil(simulationResult.gas_used * 1.2);
```

### 5. Validate Addresses
```javascript
function isValidCosmosAddress(address) {
  return /^cosmos1[0-9a-z]{38}$/.test(address);
}
```

This guide provides the foundation for building Cosmos SDK transactions compatible with Proxima. For specific integration examples in different programming languages, see the [Integration Examples](INTEGRATION_EXAMPLES.md) documentation.