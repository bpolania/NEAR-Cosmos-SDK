# Proxima Public API Reference

This document provides comprehensive reference documentation for all public API methods available in the Proxima smart contract for Cosmos SDK transaction processing on NEAR Protocol.

## Overview

Proxima provides a Cosmos SDK RPC-compatible interface with 7 public methods that handle transaction broadcasting, simulation, lookup, and configuration management. All methods return ABCI-compatible responses following Cosmos SDK standards.

## Base Types

### TxResponse Structure
All transaction methods return a `TxResponse` with the following structure:

```json
{
  "height": "string",           // Block height (0 for pending, actual height for committed)
  "txhash": "string",          // Transaction hash (empty for simulations)
  "code": number,              // ABCI response code (0 = success, >0 = error)
  "data": "string",            // Base64-encoded response data
  "raw_log": "string",         // Human-readable log output
  "logs": [...],               // Structured message logs with events
  "info": "string",            // Additional information
  "gas_wanted": "string",      // Requested gas limit
  "gas_used": "string",        // Actual gas consumed
  "events": [...],             // ABCI events with base64-encoded attributes
  "codespace": "string"        // Error categorization ("sdk", "app", etc.)
}
```

### TxProcessingConfig Structure
Configuration methods use `TxProcessingConfig`:

```json
{
  "chain_id": "string",        // Chain identifier for signature verification
  "max_gas_per_tx": number,    // Maximum gas limit per transaction
  "gas_price": number,         // Gas price in NEAR tokens (u128)
  "verify_signatures": boolean, // Enable signature verification
  "check_sequences": boolean   // Enable sequence number validation
}
```

---

## Transaction Broadcasting Methods

### broadcast_tx_sync()

**Purpose**: Submit Cosmos SDK transactions with immediate ABCI-compatible responses

**Method Signature**:
```rust
pub fn broadcast_tx_sync(&mut self, tx_bytes: Base64VecU8) -> TxResponse
```

**Parameters**:
- `tx_bytes`: Base64-encoded Cosmos SDK transaction (CosmosTx format)

**Returns**: `TxResponse` with immediate processing results

**Behavior**:
- Processes transaction immediately and synchronously
- Validates transaction format, signatures, and fees
- Executes all messages in the transaction
- Returns complete ABCI response with events and gas usage

**Example Request**:
```json
{
  "tx_bytes": "CowBCokBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEmkKLWNvc21vczFoZjBobmI3MnBuZWFzbXIzdWpnZmQ1dTNnZzVsODZ5dXE3dXdzZBItY29zbW9zMXZsc2ZsbnNjZjJrdXM3cjV0OWN2NXNyMHB6Y3k5d3BsM3FrYWRhGgkKBXVuZWFyEgA="
}
```

**Example Success Response**:
```json
{
  "height": "0",
  "txhash": "",
  "code": 0,
  "data": "ChYKFGNvc21vcy5iYW5rLnYxYmV0YTEuTXNnU2VuZA==",
  "raw_log": "Transaction executed successfully",
  "info": "Transaction processed",
  "gas_wanted": "200000",
  "gas_used": "87543",
  "events": [
    {
      "type": "transfer",
      "attributes": [
        {"key": "cmVjaXBpZW50", "value": "Y29zbW9zMXZsc2ZsbnNjZjJrdXM3cjV0OWN2NXNyMHB6Y3k5d3BsM3FrYWRh"},
        {"key": "YW1vdW50", "value": "MTAwMHVuZWFy"}
      ]
    }
  ],
  "codespace": ""
}
```

**Error Response Example**:
```json
{
  "height": "0",
  "txhash": "",
  "code": 2,
  "data": "",
  "raw_log": "Transaction decoding failed: invalid format",
  "info": "Transaction failed with code 2",
  "gas_wanted": "0",
  "gas_used": "0",
  "events": [],
  "codespace": "sdk"
}
```

---

### simulate_tx()

**Purpose**: Simulate transactions for gas estimation and validation without execution

**Method Signature**:
```rust
pub fn simulate_tx(&mut self, tx_bytes: Base64VecU8) -> TxResponse
```

**Parameters**:
- `tx_bytes`: Base64-encoded Cosmos SDK transaction

**Returns**: `TxResponse` with simulation results (no state changes)

**Behavior**:
- Validates transaction format and signatures
- Estimates gas consumption without executing state changes
- Returns gas estimation and validation results
- Does not modify contract state

**Example Response**:
```json
{
  "height": "0",
  "txhash": "",
  "code": 0,
  "data": "",
  "raw_log": "Transaction simulation completed",
  "info": "Simulation successful",
  "gas_wanted": "200000",
  "gas_used": "87543",
  "events": [],
  "codespace": ""
}
```

**Use Cases**:
- Gas estimation before broadcasting
- Transaction validation
- Fee calculation
- Testing transaction structure

---

### broadcast_tx_async()

**Purpose**: Async transaction broadcasting with immediate response

**Method Signature**:
```rust
pub fn broadcast_tx_async(&mut self, tx_bytes: Base64VecU8) -> TxResponse
```

**Parameters**:
- `tx_bytes`: Base64-encoded Cosmos SDK transaction

**Returns**: `TxResponse` with immediate response

**Behavior**:
- On NEAR, behaves identically to `broadcast_tx_sync()` for compatibility
- Processes transaction immediately
- Provided for Cosmos SDK RPC interface compatibility

---

### broadcast_tx_commit()

**Purpose**: Transaction broadcasting with block commitment and height inclusion

**Method Signature**:
```rust
pub fn broadcast_tx_commit(&mut self, tx_bytes: Base64VecU8) -> TxResponse
```

**Parameters**:
- `tx_bytes`: Base64-encoded Cosmos SDK transaction

**Returns**: `TxResponse` with block height information

**Behavior**:
- Processes transaction identically to `broadcast_tx_sync()`
- Sets `height` field to current block height
- Indicates transaction is committed to the blockchain

**Example Response Difference**:
```json
{
  "height": "12345",  // Current block height instead of "0"
  "code": 0,
  // ... rest of response
}
```

---

## Transaction Lookup Methods

### get_tx()

**Purpose**: Transaction lookup by hash with proper error handling

**Method Signature**:
```rust
pub fn get_tx(&self, hash: String) -> TxResponse
```

**Parameters**:
- `hash`: Transaction hash string (any format)

**Returns**: `TxResponse` with transaction details or error

**Behavior**:
- Currently returns "transaction not found" error (placeholder implementation)
- Maintains proper ABCI error response format
- Ready for future transaction storage implementation

**Example Request**:
```json
{
  "hash": "A1B2C3D4E5F6789..."
}
```

**Response (Not Found)**:
```json
{
  "height": "0",
  "txhash": "",
  "code": 6,
  "data": "",
  "raw_log": "Transaction not found",
  "info": "Transaction failed with code 6",
  "gas_wanted": "0",
  "gas_used": "0",
  "events": [],
  "codespace": "sdk"
}
```

---

## Configuration Management Methods

### get_tx_config()

**Purpose**: Retrieve current transaction processing configuration

**Method Signature**:
```rust
pub fn get_tx_config(&self) -> TxProcessingConfig
```

**Parameters**: None

**Returns**: `TxProcessingConfig` with current settings

**Example Response**:
```json
{
  "chain_id": "proxima-testnet-1",
  "max_gas_per_tx": 2000000,
  "gas_price": 1,
  "verify_signatures": true,
  "check_sequences": true
}
```

---

### update_tx_config()

**Purpose**: Runtime configuration management for chain parameters and gas limits

**Method Signature**:
```rust
pub fn update_tx_config(&mut self, config: TxProcessingConfig)
```

**Parameters**:
- `config`: New configuration settings

**Returns**: None (void method)

**Behavior**:
- Updates transaction processing parameters at runtime
- Changes take effect immediately for new transactions
- Does not affect transactions already in progress

**Example Request**:
```json
{
  "config": {
    "chain_id": "proxima-mainnet-1",
    "max_gas_per_tx": 5000000,
    "gas_price": 2,
    "verify_signatures": true,
    "check_sequences": true
  }
}
```

**Configuration Parameters**:
- `chain_id`: Used for signature verification and transaction validation
- `max_gas_per_tx`: Maximum gas limit allowed per transaction
- `gas_price`: Gas price in NEAR tokens (affects fee calculation)
- `verify_signatures`: Enable/disable signature verification (testing vs production)
- `check_sequences`: Enable/disable sequence number validation

---

## ABCI Error Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | OK | Transaction successful |
| 1 | INTERNAL_ERROR | Internal processing error |
| 2 | TX_DECODE_ERROR | Transaction decoding failed |
| 3 | INVALID_SEQUENCE | Sequence number mismatch |
| 4 | UNAUTHORIZED | Signature verification failed |
| 5 | INSUFFICIENT_FUNDS | Insufficient balance for transaction |
| 6 | UNKNOWN_REQUEST | Unknown request or transaction not found |
| 7 | INVALID_ADDRESS | Invalid account address |
| 10 | INSUFFICIENT_FEE | Transaction fee too low |
| 12 | OUT_OF_GAS | Gas limit exceeded |
| 15 | INVALID_REQUEST | Invalid request parameters |

## Performance Characteristics

### Rate Limits
- No built-in rate limiting (implement at infrastructure level)
- Each method processes synchronously
- Recommend max 100 requests/second per client

### Gas Consumption
- `broadcast_tx_sync()`: ~200,000-2,000,000 gas depending on message complexity
- `simulate_tx()`: ~50,000-500,000 gas (simulation overhead)
- `get_tx()`: ~10,000 gas (minimal processing)
- Configuration methods: ~5,000 gas

### Response Times
- Transaction broadcasting: 100-500ms
- Simulation: 50-200ms
- Configuration methods: <50ms
- Transaction lookup: <50ms

## Best Practices

1. **Always simulate before broadcasting** to estimate gas and validate transactions
2. **Handle ABCI error codes** appropriately in client applications
3. **Use appropriate broadcast method** based on your needs (sync vs commit)
4. **Monitor gas usage** and adjust limits as needed
5. **Update configuration** during low-traffic periods when possible