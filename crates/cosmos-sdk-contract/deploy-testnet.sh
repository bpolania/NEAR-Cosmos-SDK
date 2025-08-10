#!/bin/bash

# Deploy Modular CosmWasm Architecture to NEAR Testnet
set -e

echo "🚀 Deploying to NEAR Testnet"
echo "============================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
PARENT_ACCOUNT="cuteharbor3573.testnet"
ROUTER_ACCOUNT="cosmos-router.cuteharbor3573.testnet"
WASM_MODULE_ACCOUNT="wasm-module.cuteharbor3573.testnet"

echo -e "${BLUE}Configuration:${NC}"
echo "  Parent Account: $PARENT_ACCOUNT"
echo "  Router Account: $ROUTER_ACCOUNT"
echo "  Wasm Module Account: $WASM_MODULE_ACCOUNT"
echo ""

# Step 1: Build contracts
echo -e "${YELLOW}Step 1: Building contracts...${NC}"

echo "Building router contract..."
cargo build --target wasm32-unknown-unknown --release

echo "Building wasm module contract..."
cd ../wasm-module-contract
cargo build --target wasm32-unknown-unknown --release
cd ../cosmos-sdk-contract

echo -e "${GREEN}✓ Contracts built${NC}"
echo "  Router: $(ls -lh target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm | awk '{print $5}')"
echo "  Wasm Module: $(ls -lh ../wasm-module-contract/target/wasm32-unknown-unknown/release/wasm_module_contract.wasm | awk '{print $5}')"
echo ""

# Step 2: Create accounts if they don't exist
echo -e "${YELLOW}Step 2: Creating/checking accounts...${NC}"

# Check if router account exists
if near state $ROUTER_ACCOUNT --networkId testnet 2>/dev/null | grep -q "amount"; then
    echo -e "${GREEN}✓ Router account exists${NC}"
else
    echo "Creating router account..."
    near create-account $ROUTER_ACCOUNT --useAccount $PARENT_ACCOUNT --initialBalance 0.5 --networkId testnet
fi

# Check if wasm module account exists
if near state $WASM_MODULE_ACCOUNT --networkId testnet 2>/dev/null | grep -q "amount"; then
    echo -e "${GREEN}✓ Wasm module account exists${NC}"
else
    echo "Creating wasm module account..."
    near create-account $WASM_MODULE_ACCOUNT --useAccount $PARENT_ACCOUNT --initialBalance 0.5 --networkId testnet
fi

echo ""

# Step 3: Deploy contracts
echo -e "${YELLOW}Step 3: Deploying contracts...${NC}"

echo "Deploying router contract..."
near deploy $ROUTER_ACCOUNT target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm --networkId testnet

echo "Deploying wasm module contract..."
near deploy $WASM_MODULE_ACCOUNT ../wasm-module-contract/target/wasm32-unknown-unknown/release/wasm_module_contract.wasm --networkId testnet

echo -e "${GREEN}✓ Contracts deployed${NC}"
echo ""

# Step 4: Initialize contracts
echo -e "${YELLOW}Step 4: Initializing contracts...${NC}"

echo "Initializing router..."
near call $ROUTER_ACCOUNT new '{}' --accountId $PARENT_ACCOUNT --networkId testnet

echo "Initializing wasm module..."
near call $WASM_MODULE_ACCOUNT new "{\"router_contract\": \"$ROUTER_ACCOUNT\"}" --accountId $PARENT_ACCOUNT --networkId testnet

echo -e "${GREEN}✓ Contracts initialized${NC}"
echo ""

# Step 5: Test deployment
echo -e "${YELLOW}Step 5: Testing deployment...${NC}"

echo "Testing router health check..."
near view $ROUTER_ACCOUNT health_check '{}' --networkId testnet

echo ""
echo "Testing wasm module health check..."
near view $WASM_MODULE_ACCOUNT health_check '{}' --networkId testnet

echo ""
echo -e "${GREEN}🎉 Deployment Complete!${NC}"
echo ""
echo -e "${BLUE}Deployed Contracts:${NC}"
echo "  Router: https://testnet.nearblocks.io/address/$ROUTER_ACCOUNT"
echo "  Wasm Module: https://testnet.nearblocks.io/address/$WASM_MODULE_ACCOUNT"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo "1. Test CosmWasm deployment:"
echo "   ./test-cosmwasm-testnet.sh"
echo ""
echo "2. Deploy actual CosmWasm contracts:"
echo "   - Store WASM bytecode via router"
echo "   - Instantiate contracts"
echo "   - Execute contract methods"