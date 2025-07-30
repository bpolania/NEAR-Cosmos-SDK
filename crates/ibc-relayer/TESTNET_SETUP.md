# IBC Relayer Testnet Setup Guide

This guide helps you set up and run the IBC relayer between NEAR testnet and Cosmos ICS provider testnet.

## Prerequisites

1. **NEAR Contract**: Already deployed at `cosmos-sdk-demo.testnet`
2. **NEAR Account**: `cuteharbor3573.testnet` (with private key in `.env`)
3. **Cosmos Account**: You need to generate or obtain a Cosmos testnet account

## Environment Setup

1. **Copy environment template**:
   ```bash
   cp .env.example .env
   ```

2. **Update `.env` with your keys**:
   - NEAR key is already configured for `cuteharbor3573.testnet`
   - For Cosmos key, either:
     - Use the test key provided (for testing only)
     - Generate a new key: `./scripts/generate_cosmos_key.sh`

## Running the Relayer

1. **Build the relayer**:
   ```bash
   cargo build --release
   ```

2. **Run tests to verify setup**:
   ```bash
   cargo test testnet_deployment
   ```

3. **Start the relayer**:
   ```bash
   # Load environment variables
   source .env
   
   # Start relayer
   cargo run -- start
   ```

## Creating IBC Infrastructure

Once the relayer is running, you need to create the IBC connection and channel:

1. **Create IBC client on NEAR** (if not exists):
   ```bash
   cargo run -- create-client near-testnet cosmoshub-testnet
   ```

2. **Create IBC connection**:
   ```bash
   cargo run -- create-connection near-testnet cosmoshub-testnet
   ```

3. **Create transfer channel**:
   ```bash
   cargo run -- create-channel connection-0 transfer
   ```

## Monitoring

- **Logs**: Check console output or redirect to file
- **Metrics**: Available at `http://localhost:9090/metrics`
- **Health Check**: `cargo run -- status`

## Testnet Endpoints

- **NEAR Testnet**: https://rpc.testnet.near.org
- **NEAR Explorer**: https://testnet.nearblocks.io/address/cosmos-sdk-demo.testnet
- **Cosmos Provider**: https://rpc.provider-sentry-01.ics-testnet.polypore.xyz
- **Cosmos REST**: https://rest.provider-sentry-01.ics-testnet.polypore.xyz

## Troubleshooting

1. **Key Loading Issues**: 
   - Ensure environment variables are set: `echo $RELAYER_KEY_NEAR_TESTNET`
   - Check key format matches examples in `.env.example`

2. **Connection Issues**:
   - Verify testnet endpoints are accessible
   - Check firewall/network settings
   - Ensure accounts have sufficient balance

3. **Contract Issues**:
   - Verify contract is deployed: `near state cosmos-sdk-demo.testnet`
   - Check contract methods: `./scripts/check_deployment.sh`

## Security Notes

- **Never commit `.env` files** with real private keys
- Use encrypted keystore for production deployments
- Rotate keys regularly
- Monitor account balances and activity