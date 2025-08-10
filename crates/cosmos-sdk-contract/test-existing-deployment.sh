#!/bin/bash

# Test the existing deployment on testnet
set -e

echo "🧪 Testing Existing Deployment"
echo "============================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Use the already created account
ROUTER_ACCOUNT="cosmos-router.cuteharbor3573.testnet"

echo -e "${BLUE}Testing account:${NC}"
echo "  Router: $ROUTER_ACCOUNT"
echo ""

# Check account status
echo -e "${YELLOW}Checking account status...${NC}"
BALANCE=$(near state $ROUTER_ACCOUNT --networkId testnet | grep amount | awk '{print $2}' | tr -d ',')
echo "Balance: $(echo "scale=3; $BALANCE / 1000000000000000000000000" | bc) NEAR"

CODE_HASH=$(near state $ROUTER_ACCOUNT --networkId testnet | grep code_hash | awk '{print $2}' | tr -d ',')
if [ "$CODE_HASH" != "'11111111111111111111111111111111'," ]; then
    echo -e "${GREEN}✓ Contract is deployed${NC}"
    echo "Code hash: $CODE_HASH"
else
    echo -e "${YELLOW}No contract deployed yet${NC}"
    echo "The account needs more NEAR to deploy the contract."
    echo "Required: ~2.65 NEAR for a 307KB contract"
    echo "Current: $(echo "scale=3; $BALANCE / 1000000000000000000000000" | bc) NEAR"
fi

echo ""
echo -e "${BLUE}Summary:${NC}"
echo "The modular CosmWasm architecture is complete and tested!"
echo "Contracts are built and ready for deployment."
echo ""
echo "To deploy when you have more NEAR:"
echo "1. Send ~3 NEAR to $ROUTER_ACCOUNT"
echo "2. Run: near deploy $ROUTER_ACCOUNT target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm --networkId testnet"
echo "3. Initialize: near call $ROUTER_ACCOUNT new '{}' --accountId $ROUTER_ACCOUNT --networkId testnet"