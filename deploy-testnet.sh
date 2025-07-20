#!/bin/bash

# Deployment script for Cosmos-on-NEAR to testnet

set -e

echo "🚀 Deploying Cosmos-on-NEAR to NEAR testnet..."

# Configuration
CONTRACT_NAME="cosmos-on-near.testnet"
WASM_FILE="tinygo_contract.wasm"

# Check if TinyGo contract is built
if [ ! -f "$WASM_FILE" ]; then
    echo "📦 Building TinyGo contract first..."
    cd cosmos_on_near
    tinygo build -target=wasi -o ../tinygo_contract.wasm ./cmd/tinygo_main.go
    cd ..
    echo "✅ Built $WASM_FILE"
fi

# Check WASM file size
WASM_SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')
echo "📏 Contract size: $WASM_SIZE"

# Login to NEAR (if not already logged in)
echo "🔐 Checking NEAR CLI login status..."
if ! near state $CONTRACT_NAME >/dev/null 2>&1; then
    echo "Please login to NEAR CLI first:"
    echo "  near login"
    exit 1
fi

# Deploy the contract
echo "📤 Deploying contract to $CONTRACT_NAME..."
near deploy --accountId $CONTRACT_NAME --wasmFile "$WASM_FILE"

echo "✅ Contract deployed successfully!"
echo ""
echo "⚠️  NOTE: This TinyGo contract has placeholder input parsing."
echo "   Function calls will execute but with default/system values."
echo "   For full functionality, implement proper JSON input parsing."
echo ""
echo "🔧 You can test basic contract calls:"
echo "   # Add a validator (uses system default)"
echo "   near call $CONTRACT_NAME add_validator '{}' --accountId $CONTRACT_NAME"
echo ""
echo "   # Mint some tokens (uses system default)"
echo "   near call $CONTRACT_NAME mint '{}' --accountId $CONTRACT_NAME"
echo ""
echo "   # Process a block"
echo "   near call $CONTRACT_NAME process_block '{}' --accountId $CONTRACT_NAME"
echo ""
echo "🧪 Run integration tests (may have limited functionality):"
echo "   ./test-integration.sh"