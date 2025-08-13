#!/bin/bash
set -e

# Testnet deployment script for wasm module - following the EXACT pattern that worked
echo "🚀 NEAR Testnet Deployment Script for WASM Module"
echo "=================================================="

# Configuration - use the same pattern as the working cosmos-sdk-contract deployment
ACCOUNT_ID="${1:-bpolania.testnet}"
CONTRACT_NAME="${2:-wasm-module}"
FULL_CONTRACT_ID="${CONTRACT_NAME}.${ACCOUNT_ID}"
ROUTER_ACCOUNT="${ROUTER_ACCOUNT:-cosmos-sdk-demo-1754812961.testnet}"

echo "Parent Account: $ACCOUNT_ID"
echo "WASM Module: $FULL_CONTRACT_ID"
echo "Router: $ROUTER_ACCOUNT"
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
        autogenerate-new-keypair save-to-keychain \
        sign-as "$ACCOUNT_ID" network-config testnet sign-with-keychain send
fi

# Step 3: Deploy the contract
echo ""
echo "🚀 Deploying contract to testnet..."
~/.cargo/bin/near contract deploy "$FULL_CONTRACT_ID" \
    use-file target/near/wasm_module_contract.wasm \
    without-init-call \
    network-config testnet \
    sign-with-keychain send

# Step 4: Initialize the contract
echo ""
echo "🔧 Initializing contract..."
~/.cargo/bin/near contract call-function as-transaction "$FULL_CONTRACT_ID" new \
    json-args "{\"owner\": \"$FULL_CONTRACT_ID\", \"router_contract\": \"$ROUTER_ACCOUNT\"}" \
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
echo "✨ Deployment complete!"
echo "Contract deployed at: $FULL_CONTRACT_ID"
echo "Explorer: https://explorer.testnet.near.org/accounts/$FULL_CONTRACT_ID"
echo ""
echo "📝 To register with router, the router owner needs to run:"
echo "~/.cargo/bin/near contract call-function as-transaction $ROUTER_ACCOUNT register_module \\"
echo "    json-args '{\"module_type\": \"wasm\", \"contract_id\": \"$FULL_CONTRACT_ID\", \"version\": \"0.1.0\"}' \\"
echo "    prepaid-gas '30 Tgas' attached-deposit '0 NEAR' \\"
echo "    sign-as <ROUTER_OWNER> network-config testnet sign-with-keychain send"