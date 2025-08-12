# WASM Module Contract

A standalone NEAR smart contract that implements CosmWasm x/wasm module functionality, enabling deployment and execution of CosmWasm contracts on NEAR Protocol.

## Overview

This contract provides a complete implementation of the Cosmos SDK x/wasm module functionality as a modular component in the NEAR-Cosmos-SDK architecture. It handles:

- Storage of CosmWasm contract bytecode
- Contract instantiation and deployment
- Contract execution and queries
- Contract migration
- Permission management for code uploads

## Features

### Core Functionality

- **Store Code**: Upload and store CosmWasm WASM bytecode with unique code IDs
- **Instantiate**: Deploy contract instances from stored code
- **Execute**: Call contract methods with JSON messages
- **Query**: Read contract state without gas costs
- **Migrate**: Upgrade contracts to new code versions
- **Sudo**: Admin-only contract operations

### Permission System

The module supports flexible access control for code uploads:

- `Nobody`: No one can upload code
- `OnlyAddress`: Only a specific address can upload
- `Everybody`: Anyone can upload code
- `AnyOfAddresses`: Any address from a whitelist can upload

### Storage Features

- Maximum code size: 3MB (configurable)
- Automatic checksum generation for uploaded code
- Metadata tracking (source, builder, code hash)
- Efficient contract instance management

## Deployment

### Testnet Deployment

The contract is deployed on NEAR testnet:
- **Contract ID**: `wasm-module.cosmos-sdk-demo-1754812961.testnet`
- **Router**: `cosmos-sdk-demo-1754812961.testnet`
- **Explorer**: [View on Explorer](https://explorer.testnet.near.org/accounts/wasm-module.cosmos-sdk-demo-1754812961.testnet)

### Deploy Your Own

1. Build the contract:
```bash
cargo near build
```

2. Deploy using the provided script:
```bash
./deploy-wasm-testnet.sh <parent-account> <module-name>
```

Or manually:
```bash
near deploy <account-id> target/near/wasm_module_contract.wasm
```

3. Initialize:
```bash
near call <account-id> new '{"owner": "<owner-id>", "router_contract": "<router-id>"}' --accountId <account-id>
```

### Local Development

For local testing with NEAR Sandbox:
```bash
./deploy-local.sh
```

## Usage Examples

### Store CosmWasm Code

```bash
near call <wasm-module-id> store_code '{
  "wasm_byte_code": "<base64-encoded-wasm>",
  "source": "https://github.com/...",
  "builder": "cosmwasm/rust-optimizer:0.12.6"
}' --accountId <your-account> --gas 100000000000000
```

### Instantiate Contract

```bash
near call <wasm-module-id> instantiate '{
  "code_id": 1,
  "msg": "{\"name\":\"My Token\",\"symbol\":\"MTK\"}",
  "label": "my-token-contract",
  "admin": "<admin-account>"
}' --accountId <your-account> --gas 100000000000000
```

### Execute Contract

```bash
near call <wasm-module-id> execute '{
  "contract": "<contract-address>",
  "msg": "{\"transfer\":{\"recipient\":\"<address>\",\"amount\":\"1000\"}}",
  "funds": []
}' --accountId <your-account>
```

### Query Contract

```bash
near view <wasm-module-id> query '{
  "contract": "<contract-address>",
  "msg": "{\"balance\":{\"address\":\"<address>\"}}"
}'
```

## Integration with Router

The WASM module integrates with the modular router architecture:

1. Register with router:
```bash
near call <router-id> register_module '{
  "module_type": "wasm",
  "contract_id": "<wasm-module-id>",
  "version": "0.1.0"
}' --accountId <router-owner>
```

2. Verify registration:
```bash
near view <router-id> get_modules '{}'
```

## Testing

### Run Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# Router integration tests  
cargo test --test router_integration_tests
```

### Test Coverage

- ✅ Store code with various permission levels
- ✅ Instantiate contracts with initialization
- ✅ Execute contract methods
- ✅ Query contract state
- ✅ Contract migration
- ✅ Permission updates
- ✅ Router integration

## Architecture

The module follows a modular design:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    Router    │────▶│ WASM Module  │────▶│  CosmWasm    │
│   Contract   │     │   Contract   │     │   Contract   │
└──────────────┘     └──────────────┘     └──────────────┘
```

- **Router**: Routes CosmWasm-related calls to the WASM module
- **WASM Module**: Manages code storage and contract lifecycle
- **CosmWasm Contracts**: Individual deployed contract instances

## Development

### Prerequisites

- Rust 1.69+
- NEAR CLI
- cargo-near

### Building

```bash
# Debug build
cargo build

# Release build for deployment
cargo near build
```

### Code Structure

```
src/
├── lib.rs          # Main contract implementation
└── types.rs        # Type definitions and data structures

tests/
├── integration_tests.rs       # Standalone module tests
└── router_integration_tests.rs # Router integration tests
```

## License

This project is part of the NEAR-Cosmos-SDK and follows the same license terms.