#!/bin/bash
set -e

# Testnet deployment script for modular Cosmos SDK contract
echo "🚀 NEAR Testnet Deployment Script"
echo "=================================="

# Configuration
ACCOUNT_ID="${1:-bpolania.testnet}"
CONTRACT_NAME="${2:-cosmos-router-test}"
FULL_CONTRACT_ID="${CONTRACT_NAME}.${ACCOUNT_ID}"

echo "Account: $ACCOUNT_ID"
echo "Contract: $FULL_CONTRACT_ID"
echo ""

# Step 1: Build the contract
echo "📦 Building contract with cargo-near..."
echo "" | cargo near build non-reproducible-wasm

# Step 2: Check if subaccount exists
echo ""
echo "🔍 Checking if contract account exists..."
if ~/.cargo/bin/near account view-account-summary "$FULL_CONTRACT_ID" network-config testnet now 2>/dev/null; then
    echo "✅ Contract account exists"
else
    echo "📝 Creating contract subaccount..."
    ~/.cargo/bin/near account create-account fund-myself "$FULL_CONTRACT_ID" '5 NEAR' \
        use-auto-generation save-to-folder ~/.near-credentials/testnet \
        sign-as "$ACCOUNT_ID" network-config testnet sign-with-keychain send
fi

# Step 3: Deploy the contract
echo ""
echo "🚀 Deploying contract to testnet..."
~/.cargo/bin/near contract deploy "$FULL_CONTRACT_ID" \
    use-file target/near/cosmos_sdk_contract.wasm \
    without-init-call \
    network-config testnet \
    sign-with-keychain send

# Step 4: Initialize the contract
echo ""
echo "🔧 Initializing contract..."
~/.cargo/bin/near contract call-function as-transaction "$FULL_CONTRACT_ID" new \
    json-args '{}' \
    prepaid-gas '30 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$FULL_CONTRACT_ID" \
    network-config testnet \
    sign-with-keychain send

# Step 5: Test the contract
echo ""
echo "✅ Testing contract methods..."

echo "  - Testing health_check..."
~/.cargo/bin/near contract call-function as-read-only "$FULL_CONTRACT_ID" health_check \
    json-args '{}' \
    network-config testnet now

echo ""
echo "  - Testing get_metadata..."
~/.cargo/bin/near contract call-function as-read-only "$FULL_CONTRACT_ID" get_metadata \
    json-args '{}' \
    network-config testnet now

echo ""
echo "  - Testing test_function..."
~/.cargo/bin/near contract call-function as-read-only "$FULL_CONTRACT_ID" test_function \
    json-args '{}' \
    network-config testnet now

echo ""
echo "✨ Deployment complete!"
echo "Contract deployed at: $FULL_CONTRACT_ID"
echo "Explorer: https://explorer.testnet.near.org/accounts/$FULL_CONTRACT_ID"