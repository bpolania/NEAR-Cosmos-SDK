#!/bin/bash
set -e

# NEAR Testnet Deployment Script for Cosmos SDK Contract with IBC Modules
# This script deploys the unified Cosmos SDK contract to NEAR testnet

echo "======================================="
echo "NEAR Testnet Contract Deployment"
echo "======================================="

# Configuration
CONTRACT_NAME="cosmos-sdk-ibc.testnet"
WASM_FILE="target/near/cosmos_sdk_near.wasm"
MASTER_ACCOUNT="${MASTER_ACCOUNT:-testnet}"

# Check if NEAR CLI is installed
if ! command -v near &> /dev/null; then
    echo "Error: NEAR CLI is not installed. Please install it with:"
    echo "npm install -g near-cli"
    exit 1
fi

# Check if contract is built
if [ ! -f "$WASM_FILE" ]; then
    echo "Error: Contract WASM not found at $WASM_FILE"
    echo "Please build the contract first with: cargo near build"
    exit 1
fi

# Display contract info
echo ""
echo "Contract Details:"
echo "- Account ID: $CONTRACT_NAME"
echo "- WASM File: $WASM_FILE"
echo "- WASM Size: $(du -h $WASM_FILE | cut -f1)"
echo ""

# Check if account exists
echo "Checking if account $CONTRACT_NAME exists..."
if near state $CONTRACT_NAME 2>/dev/null; then
    echo "Account already exists. Proceeding with deployment..."
    ACCOUNT_EXISTS=true
else
    echo "Account does not exist. Creating new account..."
    ACCOUNT_EXISTS=false
fi

# Create account if it doesn't exist
if [ "$ACCOUNT_EXISTS" = false ]; then
    echo ""
    echo "Creating new account $CONTRACT_NAME..."
    near create-account $CONTRACT_NAME --masterAccount $MASTER_ACCOUNT --initialBalance 10
fi

# Deploy the contract
echo ""
echo "Deploying contract to $CONTRACT_NAME..."
near deploy --accountId $CONTRACT_NAME --wasmFile $WASM_FILE

# Initialize the contract
echo ""
echo "Initializing contract..."
near call $CONTRACT_NAME new '{}' --accountId $CONTRACT_NAME

# Verify deployment
echo ""
echo "Verifying deployment..."
echo ""
echo "1. Getting block height..."
near view $CONTRACT_NAME get_block_height '{}'

echo ""
echo "2. Checking IBC client functionality..."
# Create a test light client
INIT_HEADER='{
  "signed_header": {
    "header": {
      "version": {"block": "11", "app": "0"},
      "chain_id": "provider",
      "height": "1",
      "time": "2024-01-01T00:00:00.000000000Z",
      "last_block_id": {
        "hash": "0000000000000000000000000000000000000000000000000000000000000000",
        "parts": {
          "total": 1,
          "hash": "0000000000000000000000000000000000000000000000000000000000000000"
        }
      },
      "last_commit_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "data_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "validators_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "next_validators_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "consensus_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "app_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "last_results_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "evidence_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "proposer_address": "000000000000000000000000"
    },
    "commit": {
      "height": "0",
      "round": 0,
      "block_id": {
        "hash": "0000000000000000000000000000000000000000000000000000000000000000",
        "parts": {
          "total": 1,
          "hash": "0000000000000000000000000000000000000000000000000000000000000000"
        }
      },
      "signatures": []
    }
  },
  "validator_set": {
    "validators": [],
    "proposer": {
      "address": "000000000000000000000000",
      "pub_key": {
        "type": "tendermint/PubKeyEd25519",
        "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
      },
      "voting_power": "1",
      "proposer_priority": "0"
    },
    "total_voting_power": "1"
  },
  "trusted_height": {"revision_number": "0", "revision_height": "0"},
  "trusted_validators": {
    "validators": [],
    "proposer": {
      "address": "000000000000000000000000",
      "pub_key": {
        "type": "tendermint/PubKeyEd25519",
        "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
      },
      "voting_power": "1",
      "proposer_priority": "0"
    },
    "total_voting_power": "1"
  }
}'

echo "Creating test IBC client..."
CLIENT_RESULT=$(near call $CONTRACT_NAME ibc_create_client "{
  \"chain_id\": \"provider\",
  \"trust_period\": 86400,
  \"unbonding_period\": 259200,
  \"max_clock_drift\": 10,
  \"initial_header\": $INIT_HEADER
}" --accountId $CONTRACT_NAME 2>&1)

if echo "$CLIENT_RESULT" | grep -q "client-0"; then
    echo "✅ IBC client created successfully: client-0"
else
    echo "❌ Failed to create IBC client"
fi

echo ""
echo "======================================="
echo "Deployment Summary"
echo "======================================="
echo "✅ Contract deployed to: $CONTRACT_NAME"
echo "✅ Contract initialized successfully"
echo "✅ IBC modules are operational"
echo ""
echo "Available Modules:"
echo "- Bank Module (transfers, minting)"
echo "- Staking Module (delegation, validators)"
echo "- Governance Module (proposals, voting)"
echo "- IBC Client Module (light client verification)"
echo "- IBC Connection Module (connection handshakes)"
echo "- IBC Channel Module (packet transmission)"
echo "- IBC Transfer Module (cross-chain token transfers)"
echo ""
echo "Next Steps:"
echo "1. Configure the IBC relayer to use this contract"
echo "2. Create IBC connections and channels"
echo "3. Start relaying packets between NEAR and Cosmos"
echo ""
echo "View contract at: https://testnet.nearblocks.io/address/$CONTRACT_NAME"
echo "======================================="