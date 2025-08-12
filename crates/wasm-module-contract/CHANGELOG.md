# Changelog

All notable changes to the WASM Module Contract will be documented in this file.

## [0.1.0] - 2025-01-12

### Added
- Initial implementation of standalone WASM module for CosmWasm x/wasm functionality
- Core features:
  - Store CosmWasm contract bytecode with unique code IDs
  - Instantiate contracts from stored code
  - Execute contract methods
  - Query contract state
  - Migrate contracts to new code versions
- Permission management system:
  - Configurable access control for code upload (Nobody, OnlyAddress, Everybody, AnyOfAddresses)
  - Owner-based permission updates
- Storage management:
  - Code storage with checksums and metadata
  - Contract instance tracking
  - Configurable max code size (default: 3MB)
- Integration with modular router architecture
- Comprehensive test coverage:
  - Unit tests for all functionality
  - Integration tests for standalone operation
  - Router integration tests

### Deployment
- Successfully deployed to NEAR testnet at `wasm-module.cosmos-sdk-demo-1754812961.testnet`
- Registered with router contract at `cosmos-sdk-demo-1754812961.testnet`
- Verified operational status with test code storage

### Technical Details
- Built with NEAR SDK 5.0
- Supports Base64-encoded WASM bytecode
- Compatible with CosmWasm contract format
- Gas-efficient storage using NEAR's key-value store