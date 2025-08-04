# Error Handling Guide

This guide provides comprehensive information about error handling in Proxima, including all ABCI error codes, common error scenarios, and troubleshooting solutions.

## Overview

Proxima follows Cosmos SDK ABCI (Application Blockchain Interface) standards for error reporting. All transaction methods return standardized error codes with detailed error messages and appropriate codespaces for error categorization.

## ABCI Error Code Reference

### Standard ABCI Response Codes

| Code | Constant | Name | Description | Codespace |
|------|----------|------|-------------|-----------|
| 0 | OK | Success | Transaction executed successfully | - |
| 1 | INTERNAL_ERROR | Internal Error | Unexpected internal processing error | sdk |
| 2 | TX_DECODE_ERROR | Transaction Decode Error | Failed to decode transaction format | sdk |
| 3 | INVALID_SEQUENCE | Invalid Sequence | Account sequence number mismatch | sdk |
| 4 | UNAUTHORIZED | Unauthorized | Signature verification failed | sdk |
| 5 | INSUFFICIENT_FUNDS | Insufficient Funds | Account balance too low for transaction | sdk |
| 6 | UNKNOWN_REQUEST | Unknown Request | Unknown method or transaction not found | sdk |
| 7 | INVALID_ADDRESS | Invalid Address | Malformed or invalid account address | sdk |
| 8 | INVALID_PUBKEY | Invalid Public Key | Invalid or malformed public key | sdk |
| 9 | UNKNOWN_ADDRESS | Unknown Address | Account does not exist | sdk |
| 10 | INSUFFICIENT_FEE | Insufficient Fee | Transaction fee below minimum required | sdk |
| 11 | MEMO_TOO_LARGE | Memo Too Large | Transaction memo exceeds size limit | sdk |
| 12 | OUT_OF_GAS | Out of Gas | Transaction exceeded gas limit | sdk |
| 13 | TX_TOO_LARGE | Transaction Too Large | Transaction size exceeds limits | sdk |
| 14 | INVALID_COINS | Invalid Coins | Invalid coin denomination or amount | sdk |
| 15 | INVALID_REQUEST | Invalid Request | Generic invalid request parameters | sdk |
| 16 | TIMEOUT | Timeout | Transaction timeout or deadline exceeded | sdk |

### Proxima-Specific Error Mappings

| Proxima Error | ABCI Code | Typical Scenarios |
|---------------|-----------|------------------|
| DecodingError | 2 | Invalid JSON, missing fields, malformed protobuf |
| SignatureError | 4 | Wrong private key, invalid signature format |
| ValidationError | 15 | Invalid message parameters, business logic violations |
| AccountError | 9 | Account not found, invalid account state |
| FeeError | 10 | Insufficient fee, unsupported fee denomination |
| GasLimitExceeded | 12 | Transaction complexity exceeds gas limit |
| SequenceMismatch | 3 | Concurrent transactions, replay attacks |
| MessageExecution | 1 | Internal message processing failures |
| TransactionNotFound | 6 | Transaction hash lookup failures |

## Error Response Structure

All error responses follow this structure:

```json
{
  "height": "0",
  "txhash": "",
  "code": 2,                           // Non-zero error code
  "data": "",                          // Usually empty for errors
  "raw_log": "Detailed error message", // Human-readable error description
  "info": "Transaction failed with code 2",
  "gas_wanted": "0",
  "gas_used": "0", 
  "events": [],                        // Usually empty for errors
  "codespace": "sdk"                   // Error categorization
}
```

## Common Error Scenarios

### 1. Transaction Decoding Errors (Code 2)

**Symptoms**:
- Code: 2
- Raw log contains: "decode", "decoding", "invalid format"
- Codespace: "sdk"

**Common Causes**:
- **Invalid JSON**: Malformed JSON structure
- **Missing Required Fields**: TxBody, AuthInfo, or signatures missing
- **Wrong Data Types**: String where number expected, etc.
- **Invalid Base64 Encoding**: tx_bytes parameter incorrectly encoded

**Example Error**:
```json
{
  "code": 2,
  "raw_log": "Transaction decoding failed: invalid format",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Correct transaction structure
const validTx = {
  body: {
    messages: [...],      // Required: array of messages
    memo: "",            // Required: string (can be empty)
    timeout_height: 0,   // Required: number
    extension_options: [], // Required: array (can be empty)
    non_critical_extension_options: [] // Required: array
  },
  auth_info: {
    signer_infos: [...], // Required: array of signer info
    fee: {...}          // Required: fee object
  },
  signatures: [...]     // Required: array of signature bytes
};

// ✅ Proper Base64 encoding
const txBytes = base64Encode(JSON.stringify(validTx));
```

### 2. Signature Verification Errors (Code 4)

**Symptoms**:
- Code: 4
- Raw log contains: "signature", "unauthorized", "verification failed"
- Codespace: "sdk"

**Common Causes**:
- **Wrong Private Key**: Signing with incorrect key for account
- **Invalid Signature Format**: Signature not 65 bytes for secp256k1
- **Chain ID Mismatch**: Wrong chain_id in sign document
- **Account Number Mismatch**: Incorrect account_number in sign document

**Example Error**:
```json
{
  "code": 4,
  "raw_log": "Signature verification failed: invalid signature",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Correct signing process
const signDoc = {
  body_bytes: encodeToProtobuf(txBody),
  auth_info_bytes: encodeToProtobuf(authInfo),
  chain_id: "proxima-testnet-1",        // Must match chain configuration
  account_number: "42"                  // Must match account's number
};

// ✅ Proper secp256k1 signing
const hash = sha256(canonicalJSON(signDoc));
const signature = secp256k1.sign(hash, privateKey); // 65 bytes
```

### 3. Sequence Number Errors (Code 3)

**Symptoms**:
- Code: 3
- Raw log contains: "sequence", "expected", "got"
- Codespace: "sdk"

**Common Causes**:
- **Concurrent Transactions**: Multiple transactions with same sequence
- **Stale Sequence**: Using old sequence number
- **Sequence Gap**: Skipping sequence numbers

**Example Error**:
```json
{
  "code": 3,
  "raw_log": "Sequence mismatch: expected 5, got 3",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Query current sequence before each transaction
async function sendTransaction(tx) {
  const account = await queryAccount(senderAddress);
  
  // Update sequence in transaction
  tx.auth_info.signer_infos[0].sequence = account.sequence;
  
  const result = await broadcastTxSync(tx);
  
  if (result.code === 0) {
    // Increment local sequence counter for next transaction
    account.sequence++;
  }
  
  return result;
}

// ✅ Handle concurrent transactions
let sequenceCounter = await getAccountSequence(address);
const promises = transactions.map(async (tx, index) => {
  tx.auth_info.signer_infos[0].sequence = sequenceCounter + index;
  return broadcastTxSync(tx);
});
```

### 4. Insufficient Funds Errors (Code 5)

**Symptoms**:
- Code: 5
- Raw log contains: "insufficient", "funds", "balance"
- Codespace: "sdk"

**Common Causes**:
- **Low Account Balance**: Not enough tokens for transfer + fees
- **Fee Calculation**: Underestimating transaction fees
- **Multiple Pending**: Multiple transactions depleting balance

**Example Error**:
```json
{
  "code": 5,
  "raw_log": "Insufficient funds: account balance 1000, required 1500",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Check balance before transaction
async function validateBalance(address, amount, fee) {
  const balance = await getAccountBalance(address);
  const required = BigInt(amount) + BigInt(fee.amount[0].amount);
  
  if (balance < required) {
    throw new Error(`Insufficient funds: have ${balance}, need ${required}`);
  }
}

// ✅ Use simulation for accurate fee estimation
const simulation = await simulateTx(txBytes);
const estimatedFee = Math.ceil(simulation.gas_used * gasPrice * 1.2); // 20% buffer
```

### 5. Gas Limit Exceeded (Code 12)

**Symptoms**:
- Code: 12
- Raw log contains: "out of gas", "gas limit", "exceeded"
- Codespace: "sdk"

**Common Causes**:
- **Complex Transactions**: Multi-message transactions requiring more gas
- **Underestimated Gas**: Setting gas limit too low
- **Contract Complexity**: Complex message processing

**Example Error**:
```json
{
  "code": 12,
  "raw_log": "Out of gas: limit 200000, used 250000",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Use simulation for gas estimation
const simulation = await simulateTx(txBytes);
const recommendedGas = Math.ceil(simulation.gas_used * 1.3); // 30% buffer

// Update gas limit in transaction
tx.auth_info.fee.gas_limit = recommendedGas;

// ✅ Set reasonable maximums for complex transactions
const gasLimits = {
  simple_transfer: 200000,
  staking_delegate: 300000,
  governance_vote: 250000,
  multi_message: 800000,
  complex_contract: 2000000
};
```

### 6. Invalid Address Errors (Code 7)

**Symptoms**:
- Code: 7
- Raw log contains: "invalid address", "malformed", "bech32"
- Codespace: "sdk"

**Common Causes**:
- **Wrong Format**: Not using bech32 format
- **Invalid Checksum**: Corrupted address string
- **Wrong Prefix**: Using wrong bech32 prefix

**Example Error**:
```json
{
  "code": 7,
  "raw_log": "Invalid address: malformed bech32 address",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Validate addresses before use
function validateCosmosAddress(address) {
  const bech32Regex = /^cosmos1[0-9a-z]{38}$/;
  if (!bech32Regex.test(address)) {
    throw new Error(`Invalid Cosmos address format: ${address}`);
  }
  
  // Additional bech32 checksum validation
  try {
    bech32.decode(address);
  } catch (e) {
    throw new Error(`Invalid bech32 checksum: ${address}`);
  }
}

// ✅ Generate proper Cosmos addresses
function generateCosmosAddress(publicKey) {
  const hash = sha256(publicKey).slice(0, 20); // First 20 bytes
  return bech32.encode('cosmos', hash);
}
```

### 7. Transaction Not Found (Code 6)

**Symptoms**:
- Code: 6
- Raw log contains: "not found", "unknown request"
- Codespace: "sdk"

**Common Causes**:
- **Invalid Hash**: Transaction hash doesn't exist
- **Storage Limitations**: Transaction storage not implemented
- **Wrong Network**: Querying wrong chain

**Example Error**:
```json
{
  "code": 6,
  "raw_log": "Transaction not found",
  "codespace": "sdk"
}
```

**Solutions**:
```javascript
// ✅ Handle not found gracefully
async function getTxWithRetry(hash, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    const result = await getTx(hash);
    
    if (result.code === 0) {
      return result; // Found
    } else if (result.code === 6) {
      // Wait and retry for recent transactions
      await new Promise(resolve => setTimeout(resolve, 1000));
    } else {
      throw new Error(`Transaction query failed: ${result.raw_log}`);
    }
  }
  
  throw new Error(`Transaction ${hash} not found after ${maxRetries} retries`);
}
```

## Error Handling Best Practices

### 1. Implement Proper Error Detection

```javascript
function handleTxResponse(response) {
  if (response.code === 0) {
    // Success
    console.log('Transaction successful:', response.txhash);
    return response;
  }
  
  // Error handling based on code
  switch (response.code) {
    case 2:
      throw new TxDecodingError(response.raw_log);
    case 3:
      throw new SequenceError(response.raw_log);
    case 4:
      throw new SignatureError(response.raw_log);
    case 5:
      throw new InsufficientFundsError(response.raw_log);
    case 12:
      throw new OutOfGasError(response.raw_log);
    default:
      throw new TxError(response.code, response.raw_log);
  }
}
```

### 2. Implement Retry Logic for Recoverable Errors

```javascript
async function broadcastWithRetry(tx, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const result = await broadcastTxSync(tx);
      
      if (result.code === 0) {
        return result; // Success
      }
      
      // Handle recoverable errors
      if (result.code === 3) { // Sequence error
        // Update sequence and retry
        const account = await queryAccount(senderAddress);
        tx.auth_info.signer_infos[0].sequence = account.sequence;
        continue;
      }
      
      if (result.code === 12) { // Out of gas
        // Increase gas and retry
        const currentGas = tx.auth_info.fee.gas_limit;
        tx.auth_info.fee.gas_limit = Math.ceil(currentGas * 1.5);
        continue;
      }
      
      // Non-recoverable error
      throw new Error(`Transaction failed: ${result.raw_log}`);
      
    } catch (error) {
      if (attempt === maxRetries) {
        throw error;
      }
      
      // Wait before retry
      await new Promise(resolve => setTimeout(resolve, 1000 * attempt));
    }
  }
}
```

### 3. Validate Before Broadcasting

```javascript
async function validateAndBroadcast(tx) {
  // Pre-flight validation
  await validateTxStructure(tx);
  await validateAccountBalance(tx);
  await validateAddresses(tx);
  
  // Simulate first
  const simulation = await simulateTx(tx);
  if (simulation.code !== 0) {
    throw new Error(`Simulation failed: ${simulation.raw_log}`);
  }
  
  // Update gas based on simulation
  tx.auth_info.fee.gas_limit = Math.ceil(simulation.gas_used * 1.2);
  
  // Broadcast
  return await broadcastTxSync(tx);
}
```

### 4. Provide User-Friendly Error Messages

```javascript
function formatErrorForUser(error) {
  const userFriendlyMessages = {
    2: "Transaction format is invalid. Please check your transaction structure.",
    3: "Transaction sequence is outdated. Please refresh and try again.",
    4: "Transaction signature is invalid. Please check your private key.",
    5: "Insufficient funds. Please check your account balance.",
    6: "Transaction not found. It may not exist or hasn't been processed yet.",
    12: "Transaction requires more gas. Please increase the gas limit.",
    15: "Invalid transaction parameters. Please check your input data."
  };
  
  return userFriendlyMessages[error.code] || 
         `Transaction failed: ${error.raw_log}`;
}
```

### 5. Log Errors for Debugging

```javascript
function logTransactionError(tx, error) {
  console.error('Transaction Error Details:', {
    timestamp: new Date().toISOString(),
    errorCode: error.code,
    errorMessage: error.raw_log,
    codespace: error.codespace,
    transactionHash: error.txhash || 'N/A',
    gasWanted: error.gas_wanted,
    gasUsed: error.gas_used,
    senderAddress: tx.body.messages[0]?.from_address,
    messageTypes: tx.body.messages.map(m => m['@type']),
    sequence: tx.auth_info.signer_infos[0]?.sequence
  });
}
```

## Debugging Checklist

When encountering transaction errors, check these items in order:

### 1. Transaction Structure ✅
- [ ] Valid JSON format
- [ ] All required fields present (body, auth_info, signatures)
- [ ] Correct data types for all fields
- [ ] Proper Base64 encoding of tx_bytes

### 2. Account Information ✅
- [ ] Correct account address format (bech32)
- [ ] Account exists and has been used before
- [ ] Current sequence number (query account)
- [ ] Sufficient balance for transfer + fees

### 3. Signature Validation ✅
- [ ] Correct private key for account
- [ ] Proper sign document construction
- [ ] Correct chain_id and account_number
- [ ] Valid secp256k1 signature (65 bytes)

### 4. Fee and Gas ✅
- [ ] Sufficient fee amount
- [ ] Supported fee denomination
- [ ] Reasonable gas limit
- [ ] Gas price within acceptable range

### 5. Message Validation ✅
- [ ] Supported message types
- [ ] Valid message parameters
- [ ] Correct address formats in messages
- [ ] Positive amounts for transfers

This guide covers the most common error scenarios you'll encounter when working with Proxima. For integration examples showing proper error handling, see the [Integration Examples](INTEGRATION_EXAMPLES.md) documentation.