#!/bin/bash
set -e

# Testnet deployment script for wasm module with router integration
echo "🚀 Deploying WASM Module to NEAR Testnet"
echo "========================================="

# Configuration
ROUTER_ACCOUNT="${ROUTER_ACCOUNT:-cosmos-sdk-demo-1754812961.testnet}"
WASM_MODULE_ACCOUNT="${WASM_MODULE_ACCOUNT:-wasm-module-$(date +%s).testnet}"
NETWORK="testnet"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Step 1: Build the wasm module contract
echo ""
echo "📦 Building wasm module contract..."
if [ -f "target/near/wasm_module_contract.wasm" ]; then
    print_info "WASM already built, checking if rebuild needed..."
    # Check if source is newer than build
    if [ "src/lib.rs" -nt "target/near/wasm_module_contract.wasm" ]; then
        print_info "Source files changed, rebuilding..."
        cargo near build
    else
        print_status "Using existing build"
    fi
else
    cargo near build
fi

# Step 2: Check if router is deployed
echo ""
echo "🔍 Checking router deployment..."
if ~/.cargo/bin/near state "$ROUTER_ACCOUNT" --networkId "$NETWORK" 2>/dev/null | grep -q "code_hash"; then
    print_status "Router contract found at $ROUTER_ACCOUNT"
else
    print_error "Router contract not found at $ROUTER_ACCOUNT"
    echo "Please ensure the router is deployed first or set ROUTER_ACCOUNT environment variable"
    exit 1
fi

# Step 3: Create wasm module account
echo ""
echo "🔑 Creating wasm module account: $WASM_MODULE_ACCOUNT"

# Check if account already exists
if ~/.cargo/bin/near account view-account-summary "$WASM_MODULE_ACCOUNT" network-config "$NETWORK" now 2>/dev/null | grep -q "Native account balance"; then
    print_info "Account $WASM_MODULE_ACCOUNT already exists"
else
    print_info "Creating new account $WASM_MODULE_ACCOUNT..."
    
    # Get parent account (the part after the first dot)
    PARENT_ACCOUNT="${WASM_MODULE_ACCOUNT#*.}"
    
    # Create the account
    ~/.cargo/bin/near account create-account fund-myself "$WASM_MODULE_ACCOUNT" '10 NEAR' \
        autogenerate-new-keypair \
        save-to-keychain \
        sign-as "$PARENT_ACCOUNT" \
        network-config "$NETWORK" \
        sign-with-keychain \
        send
    
    print_status "Account created: $WASM_MODULE_ACCOUNT"
fi

# Step 4: Deploy wasm module contract
echo ""
echo "🚀 Deploying wasm module contract..."
~/.cargo/bin/near contract deploy "$WASM_MODULE_ACCOUNT" \
    use-file target/near/wasm_module_contract.wasm \
    without-init-call \
    network-config "$NETWORK" \
    sign-with-keychain send

print_status "WASM module deployed to $WASM_MODULE_ACCOUNT"

# Step 5: Initialize wasm module
echo ""
echo "🔧 Initializing wasm module..."
~/.cargo/bin/near contract call-function as-transaction "$WASM_MODULE_ACCOUNT" new \
    json-args "{\"owner\": \"$WASM_MODULE_ACCOUNT\", \"router_contract\": \"$ROUTER_ACCOUNT\"}" \
    prepaid-gas '30 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$WASM_MODULE_ACCOUNT" \
    network-config "$NETWORK" \
    sign-with-keychain send

print_status "WASM module initialized"

# Step 6: Register wasm module with router (if not already registered)
echo ""
echo "📝 Registering wasm module with router..."

# Check if already registered
MODULES=$(~/.cargo/bin/near contract call-function as-read-only "$ROUTER_ACCOUNT" get_modules \
    json-args '{}' \
    network-config "$NETWORK" now 2>/dev/null)

if echo "$MODULES" | grep -q "\"wasm\""; then
    print_info "WASM module already registered with router"
else
    print_info "Registering with router..."
    
    # Need to call as router owner - this might require manual intervention
    echo ""
    print_info "To register the wasm module with the router, the router owner needs to run:"
    echo ""
    echo "~/.cargo/bin/near contract call-function as-transaction $ROUTER_ACCOUNT register_module \\"
    echo "    json-args '{\"module_type\": \"wasm\", \"contract_id\": \"$WASM_MODULE_ACCOUNT\", \"version\": \"0.1.0\"}' \\"
    echo "    prepaid-gas '30 Tgas' attached-deposit '0 NEAR' \\"
    echo "    sign-as <ROUTER_OWNER_ACCOUNT> network-config $NETWORK sign-with-keychain send"
    echo ""
fi

# Step 7: Verify deployment
echo ""
echo "✅ Verifying deployment..."

# Check wasm module health
echo "  - WASM module health check:"
HEALTH=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE_ACCOUNT" health_check \
    json-args '{}' \
    network-config "$NETWORK" now)
echo "$HEALTH" | jq '.'

# Check module metadata
echo ""
echo "  - WASM module metadata:"
METADATA=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE_ACCOUNT" get_metadata \
    json-args '{}' \
    network-config "$NETWORK" now)
echo "$METADATA" | jq '.'

echo ""
echo "✨ Deployment complete!"
echo ""
echo "📋 Summary:"
echo "  - Network: $NETWORK"
echo "  - Router Contract: $ROUTER_ACCOUNT"
echo "  - WASM Module: $WASM_MODULE_ACCOUNT"
echo ""
echo "🧪 Test storing CosmWasm code:"
echo "  ~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE_ACCOUNT store_code \\"
echo "    json-args '{\"wasm_byte_code\": \"AGFzbQEAAAA=\", \"source\": \"test\"}' \\"
echo "    prepaid-gas '100 Tgas' attached-deposit '0 NEAR' \\"
echo "    sign-as $WASM_MODULE_ACCOUNT network-config $NETWORK sign-with-keychain send"
echo ""
echo "📝 Note: If the module needs to be registered with the router, contact the router owner."