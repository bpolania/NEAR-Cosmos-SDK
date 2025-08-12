#!/bin/bash

# Deploy Test CW20 Token Script
# Deploys a fresh CW20 token for testing with various features enabled

set -e

# Configuration
WASM_MODULE="wasm-module.cosmos-sdk-demo-1754812961.testnet"
SIGNER="cosmos-sdk-demo-1754812961.testnet"
NEAR_CLI="$HOME/.cargo/bin/near"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================"
echo "CW20 Test Token Deployment"
echo "========================================${NC}"
echo ""

# Step 1: Check if CW20 code is already stored
echo -e "${YELLOW}Checking for existing CW20 code...${NC}"
CODE_INFO=$($NEAR_CLI contract call-function as-read-only \
    "$WASM_MODULE" get_code_info \
    json-args '{"code_id": 4}' \
    network-config testnet \
    now 2>/dev/null || echo "")

if echo "$CODE_INFO" | grep -q "cw20-base"; then
    echo -e "${GREEN}✓ CW20 code already stored with code_id: 4${NC}"
    CODE_ID=4
else
    echo -e "${YELLOW}Storing CW20 code...${NC}"
    # Would need to store the code here if not already present
    echo "Error: CW20 code not found. Please run deploy-cw20-testnet.sh first"
    exit 1
fi

# Step 2: Generate unique token name with timestamp
TIMESTAMP=$(date +%s)
TOKEN_NAME="TestToken$TIMESTAMP"
TOKEN_SYMBOL="TEST$((TIMESTAMP % 10000))"
CONTRACT_LABEL="test-token-$TIMESTAMP"

echo ""
echo -e "${YELLOW}Deploying new test token:${NC}"
echo "  Name: $TOKEN_NAME"
echo "  Symbol: $TOKEN_SYMBOL"
echo "  Label: $CONTRACT_LABEL"
echo ""

# Step 3: Create instantiation message with various features
INIT_MSG=$(cat <<EOF
{
    "name": "$TOKEN_NAME",
    "symbol": "$TOKEN_SYMBOL",
    "decimals": 6,
    "initial_balances": [
        {
            "address": "$SIGNER",
            "amount": "1000000000"
        },
        {
            "address": "alice.testnet",
            "amount": "500000000"
        },
        {
            "address": "bob.testnet",
            "amount": "250000000"
        }
    ],
    "mint": {
        "minter": "$SIGNER",
        "cap": "10000000000"
    },
    "marketing": {
        "project": "CW20 Test Project",
        "description": "A test token for CW20 functionality on NEAR",
        "marketing": "$SIGNER",
        "logo": null
    }
}
EOF
)

# Escape the JSON for the command
ESCAPED_MSG=$(echo "$INIT_MSG" | jq -c . | sed 's/"/\\"/g')

# Step 4: Instantiate the token
echo -e "${YELLOW}Instantiating token contract...${NC}"
RESULT=$($NEAR_CLI contract call-function as-transaction \
    "$WASM_MODULE" instantiate \
    json-args "{\"code_id\": $CODE_ID, \"msg\": \"$ESCAPED_MSG\", \"_funds\": null, \"label\": \"$CONTRACT_LABEL\", \"admin\": \"$SIGNER\"}" \
    prepaid-gas '100 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$SIGNER" \
    network-config testnet \
    sign-with-keychain send 2>&1)

# Extract contract address
CONTRACT_ADDR=$(echo "$RESULT" | grep -o '"address":"[^"]*"' | cut -d'"' -f4)

if [ -z "$CONTRACT_ADDR" ]; then
    echo "Error: Failed to instantiate contract"
    echo "$RESULT"
    exit 1
fi

echo -e "${GREEN}✓ Token deployed successfully!${NC}"
echo ""
echo -e "${BLUE}Contract Details:${NC}"
echo "  Address: $CONTRACT_ADDR"
echo "  Name: $TOKEN_NAME"
echo "  Symbol: $TOKEN_SYMBOL"
echo "  Total Supply: 1,750,000,000 (with 6 decimals)"
echo "  Minter: $SIGNER"
echo "  Cap: 10,000,000,000"
echo ""

# Step 5: Verify deployment
echo -e "${YELLOW}Verifying deployment...${NC}"

# Query token info
TOKEN_INFO=$($NEAR_CLI contract call-function as-read-only \
    "$WASM_MODULE" query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"token_info\\\": {}}\"}" \
    network-config testnet \
    now 2>/dev/null || echo "")

if echo "$TOKEN_INFO" | grep -q "query_result"; then
    echo -e "${GREEN}✓ Token info query successful${NC}"
else
    echo "Warning: Could not verify token info"
fi

# Query balance
BALANCE=$($NEAR_CLI contract call-function as-read-only \
    "$WASM_MODULE" query \
    json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"{\\\"balance\\\": {\\\"address\\\": \\\"$SIGNER\\\"}}\"}" \
    network-config testnet \
    now 2>/dev/null || echo "")

if echo "$BALANCE" | grep -q "query_result"; then
    echo -e "${GREEN}✓ Balance query successful${NC}"
else
    echo "Warning: Could not verify balance"
fi

# Step 6: Save deployment info
DEPLOYMENT_FILE="deployments/test-token-$TIMESTAMP.json"
mkdir -p deployments

cat > "$DEPLOYMENT_FILE" <<EOF
{
    "timestamp": "$TIMESTAMP",
    "contract_address": "$CONTRACT_ADDR",
    "token_name": "$TOKEN_NAME",
    "token_symbol": "$TOKEN_SYMBOL",
    "code_id": $CODE_ID,
    "deployer": "$SIGNER",
    "initial_supply": "1750000000",
    "decimals": 6,
    "minter": "$SIGNER",
    "cap": "10000000000"
}
EOF

echo ""
echo -e "${GREEN}Deployment info saved to: $DEPLOYMENT_FILE${NC}"
echo ""
echo -e "${BLUE}========================================"
echo "Deployment Complete!"
echo "========================================${NC}"
echo ""
echo "You can now test this token with:"
echo "  export TEST_TOKEN_ADDR=\"$CONTRACT_ADDR\""
echo "  ./test-cw20-operations.sh"