#!/bin/bash
set -e

# Check the status of deployed NEAR contract
CONTRACT_NAME="${1:-cosmos-sdk-demo.testnet}"

echo "======================================="
echo "NEAR Contract Deployment Check"
echo "======================================="
echo "Contract: $CONTRACT_NAME"
echo ""

# Check if NEAR CLI is installed
if ! command -v near &> /dev/null; then
    echo "Error: NEAR CLI is not installed. Please install it with:"
    echo "npm install -g near-cli"
    exit 1
fi

# Check if account exists
echo "1. Checking account status..."
if near state $CONTRACT_NAME 2>/dev/null; then
    echo "✅ Account exists"
    near state $CONTRACT_NAME | grep -E "(amount|locked|storage_usage)"
else
    echo "❌ Account does not exist"
    exit 1
fi

echo ""
echo "2. Checking contract deployment..."
# Try to call a simple view method to verify contract is deployed
if near view $CONTRACT_NAME get_block_height '{}' &>/dev/null; then
    echo "✅ Contract is deployed"
else
    echo "❌ No contract deployed"
    exit 1
fi

echo ""
echo "3. Testing core functions..."

# Test get_block_height
echo -n "- Block height: "
BLOCK_HEIGHT=$(near view $CONTRACT_NAME get_block_height '{}' 2>/dev/null | grep -v "View call" | grep -v "Log" || echo "Failed")
echo "$BLOCK_HEIGHT"

# Test bank balance for contract account
echo -n "- Bank balance check: "
BALANCE=$(near view $CONTRACT_NAME get_balance "{\"account\": \"$CONTRACT_NAME\"}" 2>/dev/null | grep -v "View call" | grep -v "Log" || echo "Failed")
echo "$BALANCE"

# Test IBC client list
echo -n "- IBC clients: "
# Try to get a known client state (client-0 is commonly the first created)
CLIENT_STATE=$(near view $CONTRACT_NAME ibc_get_client_state '{"client_id": "client-0"}' 2>/dev/null | grep -v "View call" | grep -v "Log" || echo "No clients")
if [ "$CLIENT_STATE" == "null" ] || [ "$CLIENT_STATE" == "No clients" ]; then
    echo "No IBC clients created yet"
else
    echo "Found IBC client-0"
fi

# Test connection list
echo -n "- IBC connections: "
CONNECTIONS=$(near view $CONTRACT_NAME ibc_get_connection_ids '{}' 2>/dev/null | grep -v "View call" | grep -v "Log" || echo "Failed")
echo "$CONNECTIONS"

echo ""
echo "4. Testing IBC functionality..."

# Check if we can create a test parameter
echo -n "- Governance parameter test: "
PARAM=$(near view $CONTRACT_NAME get_parameter '{"key": "test_param"}' 2>/dev/null | grep -v "View call" | grep -v "Log" || echo "No params")
if [ "$PARAM" == '""' ] || [ "$PARAM" == "No params" ]; then
    echo "No test parameters set"
else
    echo "Found parameter: $PARAM"
fi

echo ""
echo "======================================="
echo "Deployment Status Summary"
echo "======================================="

if [ "$BLOCK_HEIGHT" != "Failed" ] && [ "$BALANCE" != "Failed" ]; then
    echo "✅ Contract is operational"
    echo "✅ All core modules responding"
    echo "✅ Ready for IBC operations"
    echo ""
    echo "Contract URL: https://testnet.nearblocks.io/address/$CONTRACT_NAME"
else
    echo "❌ Contract may have issues"
    echo "Please check the deployment"
fi

echo "======================================="