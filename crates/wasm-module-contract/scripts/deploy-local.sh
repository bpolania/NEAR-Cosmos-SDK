#!/bin/bash
set -e

# Local deployment script for wasm module with router integration
echo "🚀 Deploying WASM Module to NEAR Sandbox"
echo "========================================="

# Configuration
SANDBOX_HOME="${HOME}/.near-sandbox"
ROUTER_ACCOUNT="test.near"
WASM_MODULE_ACCOUNT="wasm.test.near"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Step 1: Check if sandbox is running
echo "🔍 Checking NEAR Sandbox status..."
if ! pgrep -f "near-sandbox" > /dev/null; then
    echo "Starting NEAR Sandbox..."
    near-sandbox --home "$SANDBOX_HOME" run > "$SANDBOX_HOME/sandbox.log" 2>&1 &
    SANDBOX_PID=$!
    echo "Sandbox PID: $SANDBOX_PID"
    sleep 3
else
    print_status "NEAR Sandbox is already running"
fi

# Step 2: Build the contracts
echo ""
echo "📦 Building contracts..."

# Build router contract if needed
if [ ! -f "../cosmos-sdk-contract/target/near/cosmos_sdk_contract.wasm" ]; then
    print_info "Building router contract..."
    cd ../cosmos-sdk-contract
    echo "" | cargo near build non-reproducible-wasm
    cd - > /dev/null
fi

# Build wasm module contract
print_info "Building wasm module contract..."
echo "" | cargo near build non-reproducible-wasm

# Step 3: Deploy router contract if not already deployed
echo ""
echo "🚀 Deploying router contract..."
if ~/.cargo/bin/near state test.near --nodeUrl http://127.0.0.1:3030 2>/dev/null | grep -q "Account is empty"; then
    print_info "Router account is empty, deploying..."
    ~/.cargo/bin/near contract deploy test.near \
        use-file ../cosmos-sdk-contract/target/near/cosmos_sdk_contract.wasm \
        without-init-call \
        network-config sandbox \
        sign-with-plaintext-private-key \
        --signer-public-key ed25519:96cdnpGBiu1ACKZ3vb5LXGxxuJMzn6Eg1h6hhfbDo3Ei \
        --signer-private-key ed25519:2DkfiSzUXSAoVmgVvukMS1u8aboNMuM3Q83ZG5f3yhK14KKr4GNT2H3AQKN4ZKKQp6mubWpSDy6DQwtgQ7fTmpn6 \
        send
    
    # Initialize router
    ~/.cargo/bin/near contract call-function as-transaction test.near new \
        json-args '{}' \
        prepaid-gas '30 Tgas' \
        attached-deposit '0 NEAR' \
        sign-as test.near \
        network-config sandbox \
        sign-with-plaintext-private-key \
        --signer-public-key ed25519:96cdnpGBiu1ACKZ3vb5LXGxxuJMzn6Eg1h6hhfbDo3Ei \
        --signer-private-key ed25519:2DkfiSzUXSAoVmgVvukMS1u8aboNMuM3Q83ZG5f3yhK14KKr4GNT2H3AQKN4ZKKQp6mubWpSDy6DQwtgQ7fTmpn6 \
        send
else
    print_status "Router contract already deployed"
fi

# Step 4: Create wasm module account
echo ""
echo "🔑 Creating wasm module account..."
if ! ~/.cargo/bin/near account view-account-summary wasm.test.near network-config sandbox now 2>/dev/null | grep -q "Native account balance"; then
    ~/.cargo/bin/near account create-account fund-myself wasm.test.near '1 NEAR' \
        autogenerate-new-keypair \
        save-to-keychain \
        sign-as test.near \
        network-config sandbox \
        sign-with-keychain \
        send
else
    print_status "WASM module account already exists"
fi

# Step 5: Deploy wasm module contract
echo ""
echo "🚀 Deploying wasm module contract..."
# Get the key from keychain
WASM_KEY=$(~/.cargo/bin/near account list-keys wasm.test.near network-config sandbox now | grep ed25519 | head -1 | awk '{print $3}')
echo "  Using key: $WASM_KEY"

~/.cargo/bin/near contract deploy wasm.test.near \
    use-file target/near/wasm_module_contract.wasm \
    without-init-call \
    network-config sandbox \
    sign-with-keychain send

# Step 6: Initialize wasm module
echo ""
echo "🔧 Initializing wasm module..."
~/.cargo/bin/near contract call-function as-transaction wasm.test.near new \
    json-args '{"owner": "wasm.test.near", "router_contract": "test.near"}' \
    prepaid-gas '30 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as wasm.test.near \
    network-config sandbox \
    sign-with-keychain send

# Step 7: Register wasm module with router
echo ""
echo "📝 Registering wasm module with router..."
~/.cargo/bin/near contract call-function as-transaction test.near register_module \
    json-args '{"module_type": "wasm", "contract_id": "wasm.test.near", "version": "0.1.0"}' \
    prepaid-gas '30 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as test.near \
    network-config sandbox \
    sign-with-plaintext-private-key \
    --signer-public-key ed25519:96cdnpGBiu1ACKZ3vb5LXGxxuJMzn6Eg1h6hhfbDo3Ei \
    --signer-private-key ed25519:2DkfiSzUXSAoVmgVvukMS1u8aboNMuM3Q83ZG5f3yhK14KKr4GNT2H3AQKN4ZKKQp6mubWpSDy6DQwtgQ7fTmpn6 \
    send

# Step 8: Verify deployment
echo ""
echo "✅ Verifying deployment..."

# Check wasm module health
echo "  - WASM module health check:"
~/.cargo/bin/near contract call-function as-read-only wasm.test.near health_check \
    json-args '{}' \
    network-config sandbox now

# Check router modules
echo ""
echo "  - Router registered modules:"
~/.cargo/bin/near contract call-function as-read-only test.near get_modules \
    json-args '{}' \
    network-config sandbox now

# Check wasm module metadata
echo ""
echo "  - WASM module metadata:"
~/.cargo/bin/near contract call-function as-read-only wasm.test.near get_metadata \
    json-args '{}' \
    network-config sandbox now

echo ""
echo "✨ Deployment complete!"
echo ""
echo "📋 Summary:"
echo "  - Router Contract: test.near"
echo "  - WASM Module: wasm.test.near"
echo ""
echo "🧪 Test storing CosmWasm code:"
echo "  ~/.cargo/bin/near contract call-function as-transaction wasm.test.near store_code \\"
echo "    json-args '{\"wasm_byte_code\": \"AGFzbQEAAAA=\", \"source\": \"test\"}' \\"
echo "    prepaid-gas '100 Tgas' attached-deposit '0 NEAR' \\"
echo "    sign-as wasm.test.near network-config sandbox sign-with-keychain send"