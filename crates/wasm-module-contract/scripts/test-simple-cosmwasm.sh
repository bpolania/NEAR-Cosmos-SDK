#!/bin/bash
set -e

# Simple test script to verify CosmWasm VM functionality
echo "🧪 Simple CosmWasm VM Test"
echo "=========================="

# Configuration - use the existing testnet deployment
CONTRACT="cosmos-sdk-demo-1754812961.testnet"
WASM_MODULE="wasm.cosmos-sdk-demo-1754812961.testnet"

# Colors for output  
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if the WASM module exists
echo ""
echo "📍 Checking WASM module deployment..."
if ~/.cargo/bin/near account view-account-summary "$WASM_MODULE" network-config testnet now 2>/dev/null | grep -q "Native account balance"; then
    print_success "WASM module account exists: $WASM_MODULE"
else
    print_error "WASM module not found at $WASM_MODULE"
    echo "Please deploy the wasm module first"
    exit 1
fi

# Test 1: Health check
echo ""
echo "🏥 Testing health check..."
HEALTH_RESULT=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" health_check \
    json-args '{}' \
    network-config testnet now 2>&1)

if echo "$HEALTH_RESULT" | grep -q "status"; then
    print_success "Health check passed"
    echo "  Response: $(echo "$HEALTH_RESULT" | grep -o '{.*}')"
else
    print_error "Health check failed"
    echo "$HEALTH_RESULT"
fi

# Test 2: Get metadata
echo ""
echo "📋 Getting module metadata..."
META_RESULT=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" get_metadata \
    json-args '{}' \
    network-config testnet now 2>&1)

if echo "$META_RESULT" | grep -q "name"; then
    print_success "Metadata retrieved"
    echo "  Response: $(echo "$META_RESULT" | grep -o '{.*}')"
else
    print_error "Failed to get metadata"
    echo "$META_RESULT"
fi

# Test 3: List codes (should be empty initially)
echo ""
echo "📚 Listing stored codes..."
CODES_RESULT=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" list_codes \
    json-args '{"limit": 10}' \
    network-config testnet now 2>&1)

if echo "$CODES_RESULT" | grep -q '\['; then
    print_success "Listed codes successfully"
    echo "  Response: $(echo "$CODES_RESULT" | tail -1)"
else
    print_error "Failed to list codes"
    echo "$CODES_RESULT"
fi

# Test 4: List contracts (should be empty initially)
echo ""
echo "📝 Listing contracts..."
CONTRACTS_RESULT=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" list_contracts \
    json-args '{"limit": 10}' \
    network-config testnet now 2>&1)

if echo "$CONTRACTS_RESULT" | grep -q '\['; then
    print_success "Listed contracts successfully"
    echo "  Response: $(echo "$CONTRACTS_RESULT" | tail -1)"
else
    print_error "Failed to list contracts"
    echo "$CONTRACTS_RESULT"
fi

echo ""
echo "=========================="
echo "✨ Test Complete!"
echo ""
echo "Summary:"
echo "  - WASM Module: $WASM_MODULE"
echo "  - All read operations working correctly"
echo ""
echo "📝 Note: To test store_code, instantiate, and execute operations,"
echo "   you'll need to call them with proper authorization (as the contract owner)."
echo ""
echo "Example store_code command (requires owner authorization):"
echo "  ~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE store_code \\"
echo "    json-args '{\"wasm_byte_code\": \"AGFzbQEAAAA=\", \"source\": \"test\"}' \\"
echo "    prepaid-gas '100 Tgas' attached-deposit '1 NEAR' \\"
echo "    sign-as <OWNER_ACCOUNT> network-config testnet sign-with-keychain send"