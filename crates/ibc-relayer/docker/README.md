# Local Wasmd Testnet for Integration Testing

This directory contains Docker configuration for running a local `wasmd` testnet for integration testing. This eliminates the need to depend on external public testnets, providing a reliable and controlled testing environment.

## Quick Start

```bash
# Start the local testnet
./scripts/local-testnet.sh start

# Check status
./scripts/local-testnet.sh status

# Run connectivity tests
./scripts/local-testnet.sh test

# Stop the testnet
./scripts/local-testnet.sh stop
```

## Features

- **Deterministic accounts**: Pre-configured test accounts with known mnemonics
- **High balances**: Each account starts with 100000000000000000000 stake and token
- **Fast startup**: Usually ready in 10-30 seconds
- **Full APIs**: RPC, REST, and gRPC endpoints exposed
- **CORS enabled**: Ready for web-based testing

## Endpoints

When running, the testnet exposes:

- **RPC**: http://localhost:26657
- **REST API**: http://localhost:1317  
- **gRPC**: localhost:9090
- **WebSocket**: ws://localhost:26657/websocket

## Test Accounts

All accounts use deterministic mnemonics for reproducible testing:

| Account | Address | Mnemonic |
|---------|---------|----------|
| Validator | wasm1qqxqfzagzm7r8m4zq9gfmk9g5k7nfcasg8jhxm | abandon abandon ... art |
| Test1 | wasm1qy5ldxnqc5x2sffm4m4fl2nzm6q6lzfcx8x4sy | abandon abandon ... about |
| Relayer | wasm1qvw7ks35q7r2qm4m7w7k2lr8m8x6sl2cy8d0mn | abandon abandon ... above |

Each account starts with:
- **100000000000000000000** stake tokens
- **100000000000000000000** custom tokens

## Chain Configuration

- **Chain ID**: wasmd-testnet
- **Address Prefix**: wasm
- **Gas Price**: 0.025stake
- **Keyring Backend**: test (for easy CLI access)

## Integration with Tests

Tests can use the local testnet through the `testnet` module:

```rust
use ibc_relayer::testnet::test_utils;

#[tokio::test]
async fn my_test() {
    // This will start the testnet if not already running
    let testnet = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet");
    
    // Use testnet endpoints
    let rpc_url = &testnet.rpc_endpoint; // http://localhost:26657
    let chain_id = &testnet.chain_id;    // wasmd-testnet
    
    // Get pre-configured accounts
    let accounts = testnet.get_test_accounts();
    let validator_addr = &accounts.validator.address;
}
```

## Docker Commands

The setup uses Docker Compose for easy management:

```bash
# Start in background
docker-compose up -d wasmd

# View logs
docker-compose logs -f wasmd

# Stop
docker-compose stop wasmd

# Clean up (removes all data)
docker-compose down -v
```

## Troubleshooting

### Testnet won't start
- Ensure Docker is running
- Check if ports 26657, 1317, 9090 are available
- View logs: `./scripts/local-testnet.sh logs`

### Connectivity issues
- Run: `./scripts/local-testnet.sh test`
- Check firewall settings
- Verify Docker networking

### Clean start
```bash
./scripts/local-testnet.sh clean
./scripts/local-testnet.sh start
```

## Configuration Files

- `docker-compose.yml`: Main Docker Compose configuration
- `wasmd/init.sh`: Testnet initialization script
- `../config/local-testnet.toml`: Relayer configuration for local testing

## Benefits Over Public Testnets

**Reliability**: No external dependencies, always available
**Speed**: Fast startup, no network latency
**Control**: Full control over accounts, balances, timing
**Isolation**: Tests don't interfere with each other
**CI/CD**: Perfect for automated testing pipelines
**Cost**: No need for testnet tokens or faucets

This local testnet approach provides a robust foundation for developing and testing IBC functionality without the uncertainty of external testnet availability.