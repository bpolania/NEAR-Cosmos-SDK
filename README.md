# Cosmos-on-NEAR

A Cosmos-inspired application-layer runtime implemented as NEAR smart contracts written in Go with near-sdk-go.

## Overview

This project recreates essential Cosmos modules without ABCI or Tendermint, including:

- **Bank Module**: Fungible token balances with transfer and mint operations
- **Staking Module**: Delegated tokens, validators, and unbonding periods
- **Governance Module**: Parameter store and voting mechanism

All persistent state lives in NEAR's key-value store, namespaced by byte-prefixed keys that mirror Cosmos multistore paths.

## Architecture

```
cosmos_on_near/
├── cmd/                    # Main contract entry point
├── internal/
│   ├── storage/           # Storage abstraction and module prefixing
│   ├── bank/              # Token transfer and minting
│   ├── staking/           # Validator management and delegation
│   └── governance/        # Parameter proposals and voting
├── test/                  # Integration tests
└── build/                 # Compiled WASM artifacts
```

## Requirements

- Go 1.23.7+
- TinyGo 0.36.0 (for WASM compilation)
- near-cli (for deployment)

## Building

```bash
# Build with TinyGo for WASM target
./build.sh

# This generates build/main.wasm
```

## Module Details

### Bank Module
- `Balance` struct with Borsh serialization
- `Transfer(receiver, amount)` - Transfer tokens between accounts
- `Mint(receiver, amount)` - Create new tokens
- All operations emit NEAR logs

### Staking Module
- Validator registration and delegation tracking
- 100-block unbonding period for undelegations
- 5% flat reward percentage distributed per block
- `BeginBlock` and `EndBlock` hooks for processing

### Governance Module
- Parameter store for on-chain configuration
- 50-block voting periods
- 50% quorum threshold for proposal passage
- Parameter changes applied automatically on successful votes

### Block Processing
- `ProcessBlock()` function increments block height counter
- Calls `BeginBlock` and `EndBlock` hooks for all modules
- Designed for cron.cat integration for regular execution

## TinyGo Limitations

- All imports must be TinyGo-compatible
- No networking or OS calls allowed
- Limited reflection support
- Standard Go tests don't work (require TinyGo runtime)

## NEAR Gas Considerations

- Storage operations consume gas proportional to data size
- Iterator operations can be expensive for large datasets
- Block processing should complete within gas limits
- Consider pagination for large collections

## Development Notes

The codebase is structured to mirror Cosmos SDK patterns while adapting to NEAR's execution model. Key differences:

1. **No ABCI**: Direct function calls instead of ABCI messages
2. **Single Contract**: All modules in one contract vs. separate modules
3. **NEAR Storage**: Key-value store instead of IAVL trees
4. **Block Simulation**: Manual block increment vs. Tendermint consensus

## Deployment

```bash
# Deploy to NEAR testnet
near deploy --accountId your-account.testnet --wasmFile build/main.wasm

# Initialize validator
near call your-account.testnet add_validator '{"address": "validator.testnet"}' --accountId your-account.testnet

# Process a block (for testing)
near call your-account.testnet process_block '{}' --accountId your-account.testnet
```

## Testing

Due to TinyGo requirements, traditional Go tests cannot run. Testing should be done through:

1. NEAR CLI integration tests
2. Local NEAR node deployment
3. Manual function calls

Example integration test:
```bash
# Mint tokens
near call contract.testnet mint '{"receiver": "alice.testnet", "amount": 1000}' --accountId admin.testnet

# Check balance
near call contract.testnet get_balance '{"account": "alice.testnet"}' --accountId alice.testnet

# Delegate to validator
near call contract.testnet delegate '{"validator": "validator.testnet", "amount": 100}' --accountId alice.testnet
```