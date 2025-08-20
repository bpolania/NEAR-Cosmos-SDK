#!/bin/bash
set -e

echo "🧪 Testing Real CW20 Contract on Testnet"
echo "========================================="

# Configuration
CONTRACT_ID="wasm-module.cuteharbor3573.testnet"
CW20_WASM="../cw20-deployment/contracts/cw20_base.wasm"

# Check if file exists
if [ ! -f "$CW20_WASM" ]; then
    echo "❌ CW20 WASM file not found at: $CW20_WASM"
    exit 1
fi

# Convert WASM to base64
echo "📦 Encoding CW20 WASM file..."
WASM_BASE64=$(base64 -i "$CW20_WASM" | tr -d '\n')
echo "✅ WASM encoded (size: $(echo -n "$WASM_BASE64" | wc -c) bytes)"

# Step 1: Store the CW20 code
echo ""
echo "📤 Storing CW20 contract code..."
STORE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction "$CONTRACT_ID" store_code \
    json-args "{\"wasm_byte_code\": \"$WASM_BASE64\", \"instantiate_permission\": {\"everybody\": {}}}" \
    prepaid-gas '300 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$CONTRACT_ID" \
    network-config testnet \
    sign-with-keychain send 2>&1)

# Extract code_id from the output
CODE_ID=$(echo "$STORE_RESULT" | grep -o '"code_id":[0-9]*' | cut -d':' -f2)

if [ -z "$CODE_ID" ]; then
    echo "❌ Failed to extract code_id from store_code response"
    echo "Response: $STORE_RESULT"
    exit 1
fi

echo "✅ Code stored with ID: $CODE_ID"

# Step 2: Query code info
echo ""
echo "🔍 Querying code info..."
~/.cargo/bin/near contract call-function as-read-only "$CONTRACT_ID" get_code_info \
    json-args "{\"code_id\": $CODE_ID}" \
    network-config testnet now

# Step 3: Instantiate the CW20 contract
echo ""
echo "🚀 Instantiating CW20 contract..."

# Create instantiation message for CW20
INIT_MSG=$(cat <<EOF
{
  "name": "Test Token",
  "symbol": "TEST",
  "decimals": 6,
  "initial_balances": [
    {
      "address": "proxima1nqfwazp5hz6nccke8gyjgr9y7srw40mmjpc6ed",
      "amount": "1000000000"
    }
  ],
  "mint": {
    "minter": "proxima1nqfwazp5hz6nccke8gyjgr9y7srw40mmjpc6ed",
    "cap": "10000000000"
  }
}
EOF
)

# Escape the JSON for use in args
INIT_MSG_ESCAPED=$(echo "$INIT_MSG" | jq -Rs .)

INSTANTIATE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction "$CONTRACT_ID" instantiate \
    json-args "{\"code_id\": $CODE_ID, \"msg\": $INIT_MSG_ESCAPED, \"label\": \"CW20 Test Token\", \"admin\": \"$CONTRACT_ID\"}" \
    prepaid-gas '300 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$CONTRACT_ID" \
    network-config testnet \
    sign-with-keychain send 2>&1)

# Extract contract address
CONTRACT_ADDR=$(echo "$INSTANTIATE_RESULT" | grep -o '"address":"[^"]*"' | cut -d':' -f2 | tr -d '"')

if [ -z "$CONTRACT_ADDR" ]; then
    echo "❌ Failed to extract contract address"
    echo "Response: $INSTANTIATE_RESULT"
    exit 1
fi

echo "✅ Contract instantiated at: $CONTRACT_ADDR"

# Step 4: Query token info
echo ""
echo "📊 Querying token info..."
~/.cargo/bin/near contract call-function as-read-only "$CONTRACT_ID" query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"token_info\\\": {}}\"}" \
    network-config testnet now

# Step 5: Query balance
echo ""
echo "💰 Querying initial balance..."
~/.cargo/bin/near contract call-function as-read-only "$CONTRACT_ID" query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"balance\\\": {\\\"address\\\": \\\"proxima1nqfwazp5hz6nccke8gyjgr9y7srw40mmjpc6ed\\\"}}\"}" \
    network-config testnet now

echo ""
echo "✨ CW20 test completed successfully!"
echo "Contract deployed at: $CONTRACT_ADDR"