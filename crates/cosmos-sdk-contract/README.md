# Cosmos SDK Contract

A comprehensive Cosmos SDK implementation on NEAR Protocol with full IBC infrastructure and CosmWasm compatibility.

## Overview

This contract implements essential Cosmos SDK modules as NEAR smart contracts, including:

- **Bank Module**: Token balances, transfers, and mint operations
- **Staking Module**: Delegated tokens, validators, and unbonding periods  
- **Governance Module**: Parameter store and voting mechanism
- **IBC Infrastructure**: Light client (ICS-07), connections (ICS-03), channels (ICS-04), and token transfers (ICS-20)
- **CosmWasm Runtime**: Full compatibility layer for existing Cosmos smart contracts

## CosmWasm Compatibility

### Modular Architecture

The project now supports a modular CosmWasm architecture with two deployment approaches:

#### 1. Monolithic Deployment
Single contract containing all modules (traditional approach):
```bash
cargo near build
near deploy your-account.testnet --wasmFile target/near/cosmos_sdk_near.wasm
```

#### 2. Modular Deployment  
Separate contracts for each module with cross-contract communication:

**Router Contract**: Central message routing and coordination
**Module Contracts**: Independent deployments for each Cosmos SDK module
- `wasm-module-contract`: Standalone x/wasm module for CosmWasm contract deployment
- `ibc-client-contract`: IBC light client operations
- `bank-contract`: Token operations
- `staking-contract`: Delegation and validation

### Building and Deploying

#### Build All Contracts
```bash
# Build router and all module contracts
./scripts/deploy-modular.sh

# Or build standalone wasm module only  
./build-wasm-module.sh
```

#### Deploy Modular Architecture
```bash
# Deploy complete modular system
./scripts/deploy-modular.sh --network=testnet --prefix=cosmos-sdk

# Deploy to custom network
./scripts/deploy-modular.sh --network=mainnet --prefix=my-cosmos
```

### Available CosmWasm Methods

The x/wasm module provides complete Cosmos SDK compatible functionality:

#### Code Management
- `store_code()` - Store WASM bytecode and return CodeID
- `get_code_info()` - Retrieve code metadata by CodeID
- `list_codes()` - List all stored codes with pagination

#### Contract Lifecycle  
- `instantiate()` - Deploy contract instance from stored code
- `execute()` - Execute messages on deployed contracts
- `query()` - Query contract state (read-only)
- `get_contract_info()` - Retrieve contract metadata
- `list_contracts_by_code()` - List contract instances by CodeID

#### Advanced Features
- **Access Control**: Configurable instantiation permissions (Everybody, Nobody, OnlyAddress, AnyOfAddresses)
- **Cross-Contract Communication**: Router-mediated calls between modules
- **State Isolation**: Each contract maintains independent state
- **Gas Optimization**: Efficient storage patterns and execution

### Deployment Scripts

The project includes comprehensive deployment automation:

- `deploy-modular.sh` - Deploy complete modular architecture
- `deploy-local.sh` - Local development deployment  
- `migrate-to-modular.sh` - Migrate from monolithic to modular
- `build-wasm-module.sh` - Build standalone x/wasm module
- `verify-modular-architecture.sh` - Validate deployment

### Configuration

Deployment configuration is handled through environment variables and script parameters:

```bash
# Network selection
export NEAR_ENV=testnet  # or mainnet

# Account prefix for module contracts
export ACCOUNT_PREFIX=cosmos-sdk

# Router contract (central coordinator)  
export ROUTER_CONTRACT=cosmos-sdk-router.testnet
```

## Architecture Benefits

### Modular Design
- **Independent Scaling**: Each module can be upgraded independently
- **Resource Optimization**: Deploy only needed modules
- **Development Flexibility**: Teams can work on modules independently
- **Testing Isolation**: Test modules in isolation before integration

### CosmWasm Ecosystem Access
- **Contract Migration**: Existing CosmWasm contracts work with minimal changes
- **Proven Security**: Access to hundreds of audited Cosmos contracts
- **Developer Familiarity**: Standard CosmWasm development patterns
- **Tool Compatibility**: Works with existing Cosmos SDK tooling

## Testing

The project includes comprehensive test coverage:

```bash
# Run all tests
cargo test

# Test specific components
cargo test cosmwasm_compatibility
cargo test wasm_module
cargo test modular_architecture
```

Test categories:
- **Unit Tests**: Individual module functionality (226+ tests)
- **Integration Tests**: Cross-module interactions (41+ tests)  
- **Performance Tests**: Load testing and benchmarks (4+ tests)
- **Compatibility Tests**: CosmWasm contract migration validation

## Development

### Prerequisites
- Rust 1.86.0+ with `wasm32-unknown-unknown` target
- `cargo-near` for NEAR contract building
- `near-cli` for deployment

### Local Development
```bash
# Build for development
cargo near build

# Run tests with coverage
cargo test --verbose

# Deploy locally
./scripts/deploy-local.sh
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Add comprehensive tests
4. Ensure all tests pass
5. Submit a pull request

## Production Deployment

For production deployments:

1. Use `deploy-modular.sh` for new installations
2. Use `migrate-to-modular.sh` for upgrades from monolithic
3. Verify deployment with `verify-modular-architecture.sh`
4. Monitor using health check endpoints

## Support

- **Documentation**: Complete API reference in `/docs`
- **Examples**: Integration examples in multiple languages
- **Issues**: Report bugs and feature requests on GitHub
- **Community**: Join the NEAR and Cosmos SDK communities for support