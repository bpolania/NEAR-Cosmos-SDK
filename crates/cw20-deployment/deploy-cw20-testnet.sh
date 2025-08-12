#!/bin/bash
set -e

echo "🚀 Deploying CW20 Token to NEAR via WASM Module"
echo "================================================"

# Configuration
WASM_MODULE="wasm-module.cosmos-sdk-demo-1754812961.testnet"
DEPLOYER_ACCOUNT="${1:-cosmos-sdk-demo-1754812961.testnet}"

# Token parameters
TOKEN_NAME="${TOKEN_NAME:-Test Token}"
TOKEN_SYMBOL="${TOKEN_SYMBOL:-TEST}"
TOKEN_DECIMALS="${TOKEN_DECIMALS:-6}"
INITIAL_BALANCE="${INITIAL_BALANCE:-1000000000000}" # 1 million tokens with 6 decimals
MINTER_CAP="${MINTER_CAP:-2000000000000}" # 2 million token cap

echo ""
echo "Configuration:"
echo "  WASM Module: $WASM_MODULE"
echo "  Deployer: $DEPLOYER_ACCOUNT"
echo "  Token Name: $TOKEN_NAME"
echo "  Token Symbol: $TOKEN_SYMBOL"
echo "  Decimals: $TOKEN_DECIMALS"
echo ""

# Step 1: Check if CW20 WASM is ready
echo "📦 Checking CW20 contract..."
if [ ! -f contracts/cw20_base.wasm.b64 ]; then
    echo "CW20 contract not found. Running download script..."
    ./download-cw20.sh
fi

# Read the base64 encoded WASM
WASM_BASE64=$(cat contracts/cw20_base.wasm.b64 | tr -d '\n')
echo "✅ CW20 WASM ready ($(echo -n "$WASM_BASE64" | wc -c) base64 chars)"

# Step 2: Store the CW20 code in the WASM module
echo ""
echo "📤 Storing CW20 code in WASM module..."

STORE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction "$WASM_MODULE" store_code \
    json-args "{\"wasm_byte_code\": \"$WASM_BASE64\", \"source\": \"https://github.com/CosmWasm/cw-plus/releases/download/v1.1.2/cw20_base.wasm\", \"builder\": \"cosmwasm/rust-optimizer:0.12.13\"}" \
    prepaid-gas '200 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$DEPLOYER_ACCOUNT" \
    network-config testnet \
    sign-with-keychain send 2>&1)

# Extract code_id from the result
CODE_ID=$(echo "$STORE_RESULT" | grep -o '"code_id":[0-9]*' | cut -d':' -f2 | head -1)

if [ -z "$CODE_ID" ]; then
    echo "❌ Failed to store code. Output:"
    echo "$STORE_RESULT"
    exit 1
fi

echo "✅ Code stored with ID: $CODE_ID"

# Step 3: Instantiate the CW20 token contract
echo ""
echo "🔧 Instantiating CW20 token contract..."

# Create instantiate message for CW20
INSTANTIATE_MSG=$(cat <<EOF
{
  "name": "$TOKEN_NAME",
  "symbol": "$TOKEN_SYMBOL",
  "decimals": $TOKEN_DECIMALS,
  "initial_balances": [
    {
      "address": "$DEPLOYER_ACCOUNT",
      "amount": "$INITIAL_BALANCE"
    }
  ],
  "mint": {
    "minter": "$DEPLOYER_ACCOUNT",
    "cap": "$MINTER_CAP"
  },
  "marketing": {
    "project": "NEAR-Cosmos-SDK Test",
    "description": "Test CW20 token deployed via WASM module",
    "marketing": "$DEPLOYER_ACCOUNT"
  }
}
EOF
)

# Escape the JSON for passing as argument
INSTANTIATE_MSG_ESCAPED=$(echo "$INSTANTIATE_MSG" | jq -c . | sed 's/"/\\"/g')

INSTANTIATE_RESULT=$(~/.cargo/bin/near contract call-function as-transaction "$WASM_MODULE" instantiate \
    json-args "{\"code_id\": $CODE_ID, \"msg\": \"$INSTANTIATE_MSG_ESCAPED\", \"label\": \"cw20-$TOKEN_SYMBOL-$(date +%s)\", \"admin\": \"$DEPLOYER_ACCOUNT\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$DEPLOYER_ACCOUNT" \
    network-config testnet \
    sign-with-keychain send 2>&1)

# Extract contract address from the result
CONTRACT_ADDRESS=$(echo "$INSTANTIATE_RESULT" | grep -o '"contract_address":"[^"]*"' | cut -d'"' -f4 | head -1)

if [ -z "$CONTRACT_ADDRESS" ]; then
    echo "❌ Failed to instantiate contract. Output:"
    echo "$INSTANTIATE_RESULT"
    exit 1
fi

echo "✅ Token contract instantiated at: $CONTRACT_ADDRESS"

# Step 4: Query token info to verify deployment
echo ""
echo "📊 Verifying token deployment..."

# Query token info
TOKEN_INFO_MSG='{"token_info":{}}'
TOKEN_INFO=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" query \
    json-args "{\"contract\": \"$CONTRACT_ADDRESS\", \"msg\": \"$TOKEN_INFO_MSG\"}" \
    network-config testnet now 2>&1)

echo "Token Info:"
echo "$TOKEN_INFO" | grep -A10 "Function execution return value" || echo "$TOKEN_INFO"

# Query balance
BALANCE_MSG="{\"balance\":{\"address\":\"$DEPLOYER_ACCOUNT\"}}"
BALANCE_MSG_ESCAPED=$(echo "$BALANCE_MSG" | sed 's/"/\\"/g')
BALANCE=$(~/.cargo/bin/near contract call-function as-read-only "$WASM_MODULE" query \
    json-args "{\"contract\": \"$CONTRACT_ADDRESS\", \"msg\": \"$BALANCE_MSG_ESCAPED\"}" \
    network-config testnet now 2>&1)

echo ""
echo "Deployer Balance:"
echo "$BALANCE" | grep -A10 "Function execution return value" || echo "$BALANCE"

# Step 5: Save deployment info
echo ""
echo "💾 Saving deployment info..."
cat > contracts/cw20-deployment.json <<EOF
{
  "code_id": $CODE_ID,
  "contract_address": "$CONTRACT_ADDRESS",
  "token_name": "$TOKEN_NAME",
  "token_symbol": "$TOKEN_SYMBOL",
  "decimals": $TOKEN_DECIMALS,
  "deployer": "$DEPLOYER_ACCOUNT",
  "wasm_module": "$WASM_MODULE",
  "deployed_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "network": "testnet"
}
EOF

echo "✅ Deployment info saved to contracts/cw20-deployment.json"

echo ""
echo "✨ CW20 Token Successfully Deployed!"
echo "====================================="
echo "  Code ID: $CODE_ID"
echo "  Contract: $CONTRACT_ADDRESS"
echo "  Token: $TOKEN_NAME ($TOKEN_SYMBOL)"
echo "  Initial Supply: $INITIAL_BALANCE (with $TOKEN_DECIMALS decimals)"
echo ""
echo "🧪 Test Commands:"
echo ""
echo "# Check token info:"
echo "near contract call-function as-read-only $WASM_MODULE query \\"
echo "  json-args '{\"contract\": \"$CONTRACT_ADDRESS\", \"msg\": \"{\\\"token_info\\\":{}}\"}}' \\"
echo "  network-config testnet now"
echo ""
echo "# Check balance:"
echo "near contract call-function as-read-only $WASM_MODULE query \\"
echo "  json-args '{\"contract\": \"$CONTRACT_ADDRESS\", \"msg\": \"{\\\"balance\\\":{\\\"address\\\":\\\"$DEPLOYER_ACCOUNT\\\"}}\"}}' \\"
echo "  network-config testnet now"