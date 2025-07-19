# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Cosmos-inspired application-layer runtime implemented as NEAR smart contracts in Go using near-sdk-go. The project recreates essential Cosmos modules (bank, staking, governance) without ABCI or Tendermint.

## Build Commands

```bash
# Build WASM contract with TinyGo
./cosmos_on_near/build.sh

# Note: Standard Go build/test won't work due to TinyGo-specific dependencies
```

## Development Requirements

- Go 1.23.7+
- TinyGo 0.36.0 (for WASM compilation)
- near-cli (for deployment and testing)

## Architecture

```
cosmos_on_near/
├── cmd/main.go                    # Contract entry point with function registration
├── internal/
│   ├── storage/                   # Storage abstraction and module prefixing
│   │   ├── storage.go            # NEAR storage wrapper
│   │   ├── module_store.go       # Module-namespaced storage
│   │   └── block_height.go       # Block height management
│   ├── bank/                     # Token operations
│   │   ├── types.go              # Balance struct with Borsh serialization
│   │   └── bank.go               # Transfer/mint functionality
│   ├── staking/                  # Delegation and validation
│   │   ├── types.go              # Validator, Delegation, UnbondingEntry structs
│   │   └── staking.go            # Delegation logic with 100-block unbonding
│   └── governance/               # On-chain parameters
│       ├── types.go              # Proposal and Vote structs
│       └── governance.go         # Voting with 50-block periods
```

## Key Components

### Storage Pattern
- `Store` interface abstracts NEAR storage
- `ModuleStore` prefixes keys with module name to avoid collisions
- All data uses Borsh serialization for efficiency

### Module Structure
- Each module has isolated storage namespace
- Implements BeginBlock/EndBlock hooks for block processing
- State changes emit NEAR logs for transparency

### Block Processing
- `ProcessBlock()` simulates block boundaries
- Designed for cron.cat integration for regular execution
- Handles unbonding releases and reward distribution

## Testing Limitations

- Standard Go tests fail due to TinyGo-specific dependencies
- Use NEAR CLI for integration testing
- Deploy locally for development testing

## Important Notes

- All imports must be TinyGo-compatible
- No networking or OS calls allowed in smart contracts
- Storage operations consume gas proportional to data size
- Iterator operations can be expensive for large datasets

## Development Workflow

1. Make changes to Go code
2. Build with `./build.sh`
3. Deploy with near-cli
4. Test contract functions manually
5. Commit and push to develop branch