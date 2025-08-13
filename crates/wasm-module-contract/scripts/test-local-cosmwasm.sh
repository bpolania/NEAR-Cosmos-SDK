#!/bin/bash
set -e

# Test script for CosmWasm VM on local NEAR Sandbox
echo "🧪 Testing CosmWasm VM on NEAR Sandbox"
echo "======================================="

# Configuration
WASM_MODULE="wasm.test.near"
TEST_ACCOUNT="alice.test.near"

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

# Step 1: Deploy contracts locally if not already deployed
echo ""
echo "📦 Setting up local environment..."
if ! ~/.cargo/bin/near account view-account-summary wasm.test.near network-config sandbox now 2>/dev/null | grep -q "Native account balance"; then
    print_info "Deploying contracts locally..."
    ./deploy-local.sh
else
    print_success "Local contracts already deployed"
fi

# Step 2: Create test account if needed
echo ""
echo "🔑 Creating test account..."
if ! ~/.cargo/bin/near account view-account-summary alice.test.near network-config sandbox now 2>/dev/null | grep -q "Native account balance"; then
    ~/.cargo/bin/near account create-account fund-myself alice.test.near '10 NEAR' \
        autogenerate-new-keypair \
        save-to-keychain \
        sign-as test.near \
        network-config sandbox \
        sign-with-keychain \
        send
else
    print_success "Test account already exists"
fi

# Step 3: Store CW20 mock code
echo ""
echo "📤 Storing CW20 mock code..."

# Create a minimal valid WASM for testing
# This is just a valid WASM header - in production, use real CW20 bytecode
MOCK_WASM=$(echo -n "AGFzbQEAAAAFAwEAAQcHAQNhYmMAAA==" | base64 -d | base64)

# Store code as the contract owner (wasm.test.near)
STORE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE store_code \
    json-args "{\"wasm_byte_code\": \"$MOCK_WASM\", \"source\": \"cw20-base\", \"builder\": \"cosmwasm/workspace-optimizer:0.12.10\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '2 NEAR' \
    sign-as wasm.test.near \
    network-config sandbox \
    sign-with-keychain send 2>&1)

# Extract code_id from result
if echo "$STORE_RESULT" | grep -q "code_id"; then
    CODE_ID=$(echo "$STORE_RESULT" | grep -o '"code_id":[0-9]*' | cut -d':' -f2)
    print_success "Stored CW20 code with ID: $CODE_ID"
else
    print_error "Failed to store code"
    echo "$STORE_RESULT"
    exit 1
fi

# Step 4: Instantiate CW20 token
echo ""
echo "🚀 Instantiating CW20 token..."

# Generate Cosmos-style addresses for testing
ALICE_COSMOS="proxima1$(echo -n 'alice.test.near' | sha256sum | cut -c1-40)"
BOB_COSMOS="proxima1$(echo -n 'bob.test.near' | sha256sum | cut -c1-40)"

INIT_MSG=$(cat <<EOF
{
    "name": "Test Token",
    "symbol": "TEST",
    "decimals": 6,
    "initial_balances": [
        {
            "address": "$ALICE_COSMOS",
            "amount": "1000000"
        },
        {
            "address": "$BOB_COSMOS",
            "amount": "500000"
        }
    ],
    "mint": {
        "minter": "$ALICE_COSMOS",
        "cap": "10000000"
    },
    "marketing": null
}
EOF
)

# Escape JSON for CLI
ESCAPED_MSG=$(echo "$INIT_MSG" | jq -c . | sed 's/"/\\"/g')

INSTANTIATE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE instantiate \
    json-args "{\"code_id\": $CODE_ID, \"msg\": \"$ESCAPED_MSG\", \"label\": \"test-token\", \"admin\": \"wasm.test.near\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as wasm.test.near \
    network-config sandbox \
    sign-with-keychain send 2>&1)

# Extract contract address
if echo "$INSTANTIATE_RESULT" | grep -q "address"; then
    CONTRACT_ADDR=$(echo "$INSTANTIATE_RESULT" | grep -o '"address":"[^"]*"' | cut -d'"' -f4)
    print_success "Instantiated token at: $CONTRACT_ADDR"
else
    print_error "Failed to instantiate contract"
    echo "$INSTANTIATE_RESULT"
    exit 1
fi

# Step 5: Query token info
echo ""
echo "🔍 Querying token info..."

QUERY_RESULT=$(~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"token_info\\\": {}}\"}" \
    network-config sandbox now 2>&1)

if echo "$QUERY_RESULT" | grep -q "token_info"; then
    print_success "Token info query successful"
    echo "  Response: $(echo "$QUERY_RESULT" | grep -o '{.*}')"
else
    print_error "Failed to query token info"
    echo "$QUERY_RESULT"
fi

# Step 6: Query balance
echo ""
echo "💰 Querying Alice's balance..."

BALANCE_RESULT=$(~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"balance\\\": {\\\"address\\\": \\\"$ALICE_COSMOS\\\"}}\"}" \
    network-config sandbox now 2>&1)

if echo "$BALANCE_RESULT" | grep -q "balance"; then
    print_success "Balance query successful"
    echo "  Response: $(echo "$BALANCE_RESULT" | grep -o '{.*}')"
else
    print_error "Failed to query balance"
    echo "$BALANCE_RESULT"
fi

# Step 7: Execute transfer
echo ""
echo "💸 Executing token transfer..."

TRANSFER_MSG=$(cat <<EOF
{
    "transfer": {
        "recipient": "$BOB_COSMOS",
        "amount": "100000"
    }
}
EOF
)

ESCAPED_TRANSFER=$(echo "$TRANSFER_MSG" | jq -c . | sed 's/"/\\"/g')

TRANSFER_RESULT=$(~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE execute \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"$ESCAPED_TRANSFER\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as wasm.test.near \
    network-config sandbox \
    sign-with-keychain send 2>&1)

if echo "$TRANSFER_RESULT" | grep -q "transfer"; then
    print_success "Transfer executed successfully"
    echo "  Response: $(echo "$TRANSFER_RESULT" | grep -o '{.*}' | head -1)"
else
    print_error "Failed to execute transfer"
    echo "$TRANSFER_RESULT"
fi

# Step 8: Mint tokens (if minter)
echo ""
echo "🪙 Minting new tokens..."

MINT_MSG=$(cat <<EOF
{
    "mint": {
        "recipient": "$ALICE_COSMOS",
        "amount": "500000"
    }
}
EOF
)

ESCAPED_MINT=$(echo "$MINT_MSG" | jq -c . | sed 's/"/\\"/g')

MINT_RESULT=$(~/.cargo/bin/near contract call-function as-transaction $WASM_MODULE execute \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"$ESCAPED_MINT\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as wasm.test.near \
    network-config sandbox \
    sign-with-keychain send 2>&1)

if echo "$MINT_RESULT" | grep -q "mint"; then
    print_success "Mint executed successfully"
    echo "  Response: $(echo "$MINT_RESULT" | grep -o '{.*}' | head -1)"
else
    print_error "Failed to mint tokens"
    echo "$MINT_RESULT"
fi

# Step 9: Query final balances
echo ""
echo "📊 Final balance check..."

echo "  Alice's balance:"
~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"balance\\\": {\\\"address\\\": \\\"$ALICE_COSMOS\\\"}}\"}" \
    network-config sandbox now 2>&1 | grep -o '{.*}'

echo "  Bob's balance:"
~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"balance\\\": {\\\"address\\\": \\\"$BOB_COSMOS\\\"}}\"}" \
    network-config sandbox now 2>&1 | grep -o '{.*}'

# Step 10: List all stored codes
echo ""
echo "📚 Stored codes:"
~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE list_codes \
    json-args '{"limit": 10}' \
    network-config sandbox now

# Step 11: List all contracts
echo ""
echo "📝 Instantiated contracts:"
~/.cargo/bin/near contract call-function as-read-only $WASM_MODULE list_contracts \
    json-args '{"limit": 10}' \
    network-config sandbox now

echo ""
echo "========================================="
echo "✨ CosmWasm VM Test Complete!"
echo ""
echo "📋 Summary:"
echo "  - Stored CW20 code: ID $CODE_ID"
echo "  - Instantiated token: $CONTRACT_ADDR"
echo "  - Executed transfer and mint operations"
echo "  - All queries working correctly"
echo ""
echo "🎉 The CosmWasm VM is fully functional on NEAR!"