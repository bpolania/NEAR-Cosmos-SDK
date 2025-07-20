# Cosmos-on-NEAR

A Cosmos-inspired application-layer runtime implemented as NEAR smart contracts using Rust and the official NEAR SDK.

## Overview

This project recreates essential Cosmos modules without ABCI or Tendermint, including:

- **Bank Module**: Fungible token balances with transfer and mint operations
- **Staking Module**: Delegated tokens, validators, and unbonding periods
- **Governance Module**: Parameter store and voting mechanism

All persistent state lives in NEAR's key-value store, namespaced by byte-prefixed keys that mirror Cosmos multistore paths.

## Architecture

```
cosmos_on_near_rust/
├── src/
│   ├── lib.rs             # Main contract entry point
│   ├── bank.rs            # Token transfer and minting
│   ├── staking.rs         # Validator management and delegation
│   └── governance.rs      # Parameter proposals and voting
├── target/near/           # Compiled WASM artifacts
└── Cargo.toml            # Rust dependencies
```

## Requirements

- Rust 1.86.0 (for NEAR-compatible WASM compilation)
- cargo-near (for proper NEAR contract building)
- near-cli (for deployment)

## Building

```bash
# Set Rust version to 1.86.0 for NEAR compatibility
rustup override set 1.86.0

# Build with cargo-near for proper NEAR contract
cargo near build

# Output will be in target/near/cosmos_on_near_rust.wasm
```

## Deployment

```bash
# Build the contract
cd cosmos_on_near_rust
cargo near build

# Deploy to NEAR testnet
near deploy --accountId your-account.testnet --wasmFile target/near/cosmos_on_near_rust.wasm

# Initialize contract
near call your-account.testnet new '{}' --accountId your-account.testnet
```

**Note**: This project uses the official NEAR SDK for Rust with cargo-near for reliable WASM compilation and deployment.

## Module Details

### Bank Module
- `Balance` struct with efficient binary serialization
- `Transfer(sender, receiver, amount)` - Transfer tokens between accounts
- `Mint(receiver, amount)` - Create new tokens
- All operations emit NEAR logs via custom runtime bindings

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

## Technical Implementation

### Testing Strategy
The contract has been thoroughly tested on NEAR testnet with all modules functioning correctly:

- ✅ Bank Module: Transfer and minting operations
- ✅ Staking Module: Validator registration, delegation, and rewards
- ✅ Governance Module: Proposal submission and voting
- ✅ Block Processing: Cross-module integration and state management

For integration testing, consider using [near-workspaces](https://github.com/near/workspaces-rs) - the Rust equivalent of Hardhat for NEAR contracts.

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


## Status

### ✅ **Production Ready**

The Rust implementation has been successfully deployed and tested:

- **✅ NEAR SDK Integration**: Uses official NEAR SDK for Rust with cargo-near
- **✅ All Modules Functional**: Bank, staking, and governance modules fully operational
- **✅ Testnet Deployment**: Successfully deployed and tested on `demo.cuteharbor3573.testnet`
- **✅ Cross-Module Integration**: Block processing and state management verified

### Ready for Production
The contract is ready for:
1. ✅ NEAR testnet deployment (completed)
2. ✅ Integration testing with real NEAR environment (completed)
3. 🔄 Production deployment with cron.cat automation

The core architecture and business logic have been proven through comprehensive testing on live NEAR testnet, making this a robust Cosmos-inspired runtime for NEAR Protocol.

## LATEST DEPLOY

**Contract Address:** `cuteharbor3573.testnet`  
**Transaction Hash:** `12RKM38nmfz5ZaW59rS2d4a1BvdbeonMkiZj6UUknP5G`  
**Network:** NEAR Testnet  
**Explorer:** https://testnet.nearblocks.io/txns/12RKM38nmfz5ZaW59rS2d4a1BvdbeonMkiZj6UUknP5G