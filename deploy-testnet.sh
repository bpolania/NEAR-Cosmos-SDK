#!/bin/bash

# Deployment script for Cosmos-on-NEAR to testnet

set -e

echo "🚀 Deploying Cosmos-on-NEAR to NEAR testnet..."

# Load environment from .env file if it exists
if [ -f ".env" ]; then
    echo "📊 Loading environment from .env file..."
    set -a  # Export all variables
    source .env
    set +a  # Stop exporting
fi

# Environment Configuration
export NEAR_ENV=testnet
export NEAR_CLI_TESTNET_RPC_SERVER_URL=https://rpc.testnet.near.org
export NEAR_CLI_TESTNET_WALLET_URL=https://wallet.testnet.near.org

# Configuration will be set from NEAR_ACCOUNT_ID after loading .env
WASM_FILE="cosmos_on_near/tinygo_contract.wasm"

# Check if Go contract is built
if [ ! -f "$WASM_FILE" ]; then
    echo "📦 Building Go contract with TinyGo..."
    cd cosmos_on_near
    tinygo build -target=wasi -o tinygo_contract.wasm ./cmd/tinygo_main.go
    cd ..
    echo "✅ Built $WASM_FILE"
fi

# Check WASM file size
WASM_SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')
echo "📏 Contract size: $WASM_SIZE"

# Check for required environment variables
echo "🔐 Checking authentication configuration..."

if [ -z "$NEAR_ACCOUNT_ID" ]; then
    echo "❌ Error: NEAR_ACCOUNT_ID environment variable not set"
    echo "Please set your NEAR account ID:"
    echo "  export NEAR_ACCOUNT_ID=your-account.testnet"
    exit 1
fi

if [ -z "$NEAR_PRIVATE_KEY" ]; then
    echo "❌ Error: NEAR_PRIVATE_KEY environment variable not set"
    echo "Please set your private key:"
    echo "  export NEAR_PRIVATE_KEY=ed25519:your-private-key-here"
    exit 1
fi

# Create credentials directory and file
CREDS_DIR="$HOME/.near-credentials/testnet"
CREDS_FILE="$CREDS_DIR/$NEAR_ACCOUNT_ID.json"

mkdir -p "$CREDS_DIR"

# Create credentials file from environment variables
cat > "$CREDS_FILE" << EOF
{
  "account_id": "$NEAR_ACCOUNT_ID",
  "public_key": "",
  "private_key": "$NEAR_PRIVATE_KEY"
}
EOF

echo "✅ Credentials configured for $NEAR_ACCOUNT_ID"

# Use the account from environment variable
CONTRACT_NAME="$NEAR_ACCOUNT_ID"

# Deploy the contract
echo "📤 Deploying contract to $CONTRACT_NAME..."
near deploy $CONTRACT_NAME "$WASM_FILE"

echo "✅ Contract deployed successfully!"
echo ""
echo "✅ This Go contract has custom NEAR bindings with TinyGo-compatible WASM."
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