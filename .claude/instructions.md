# Project Instructions for Claude Code

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Cosmos-inspired application-layer runtime implemented as NEAR smart contracts in Rust using the official NEAR SDK. The project recreates essential Cosmos modules (bank, staking, governance, IBC) following proper Cosmos SDK conventions in a unified contract without ABCI or Tendermint.

## Build Commands

```bash
# Build WASM contract with cargo-near
cd cosmos_sdk_near
cargo near build

# Run tests
cargo test

# Deploy to NEAR testnet
near deploy --accountId your-account.testnet --wasmFile target/near/cosmos_sdk_near.wasm
```

## Development Requirements

- Rust 1.86.0 (for NEAR compatibility) 
- cargo-near (for proper NEAR contract building)
- near-cli (for deployment and testing)

## Architecture

```
cosmos_sdk_near/           # Unified Cosmos SDK NEAR Implementation
├── src/
│   ├── lib.rs             # Main contract entry point
│   └── modules/           # Cosmos SDK Modules
│       ├── bank/          # Token operations (transfer, mint)
│       ├── staking/       # Delegation and validator management
│       ├── gov/           # Governance proposals and voting
│       └── ibc/           # Inter-Blockchain Communication
│           └── client/
│               └── tendermint/  # 07-tendermint light client (ICS-07)
├── tests/
│   ├── integration_tests.rs     # Main contract tests
│   └── ibc_integration_tests.rs # IBC functionality tests
└── target/near/           # Compiled WASM artifacts
```

## Key Components

### Storage Pattern
- NEAR SDK collections (LookupMap, UnorderedMap) for efficient storage
- Module storage prefixing with single-byte keys to avoid collisions
- All data uses Borsh serialization for efficiency and NEAR compatibility
- Storage operations consume gas proportional to data size

### Module Structure
- All modules follow proper Cosmos SDK conventions
- Each module is properly namespaced under `/modules/`
- Unified contract contains all modules instead of separate contracts
- Implements BeginBlock/EndBlock hooks for block processing
- State changes emit NEAR logs for transparency

### Block Processing
- `process_block()` simulates block boundaries
- Designed for cron.cat integration for regular execution
- Handles unbonding releases and reward distribution
- Cross-module coordination during block processing

### IBC Integration
- Complete 07-tendermint light client implementation
- Ed25519 signature verification and IAVL Merkle proof validation
- Cross-chain state verification capabilities
- Foundation for full IBC Connection and Channel modules

### Testing Framework
- Uses near-workspaces for comprehensive integration testing
- Real NEAR sandbox environment testing (equivalent of Hardhat for NEAR)
- All modules tested with cross-module interactions
- Both unit tests and integration tests included

## Development Workflow

1. Make changes to Rust code in `cosmos_sdk_near/src/`
2. Build with `cargo near build`
3. Run tests with `cargo test`
4. Deploy with near-cli for integration testing
5. Commit changes following commit guidelines

## Deployment Status

- **Current Deployment**: `demo.cuteharbor3573.testnet`
- **Contract**: `cosmos_sdk_near.wasm` (unified contract with all modules)
- **Network**: NEAR Testnet
- **All Tests**: ✅ Passing (12 main integration tests + 9 IBC tests)

## Important Notes

- Use Rust 1.86.0 for NEAR compatibility (newer versions cause WASM issues)
- All functions are properly integrated (no dead code warnings allowed)
- Storage uses efficient single-byte keys for NEAR collections
- Iterator operations can be expensive for large datasets
- Contract deployed to `demo.cuteharbor3573.testnet` for testing
- No networking or OS calls allowed in smart contracts

## Testing Limitations

- Use near-workspaces integration testing framework instead of standard unit tests
- Deploy to NEAR sandbox for realistic testing environment
- Test contract functions through near-workspaces API calls

## Commit Guidelines

- Use clear, descriptive commit messages
- Do not include Claude Code attribution or co-authorship
- Focus on what was implemented, not who implemented it
- Commits should be atomic and focused on single features

## Testing Commands

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_tests
cargo test --test ibc_integration_tests

# Build contract
cargo near build
```