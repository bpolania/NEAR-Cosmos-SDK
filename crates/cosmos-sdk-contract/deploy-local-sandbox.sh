#!/bin/bash

# Deploy to Local NEAR Sandbox
# Based on recommended approach from NEAR documentation
set -e

echo "🚀 Deploying to Local NEAR Sandbox"
echo "==================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
SANDBOX_HOME="${NEAR_SANDBOX_HOME:-$HOME/.near-sandbox}"
SANDBOX_PORT="${NEAR_SANDBOX_PORT:-3030}"
SANDBOX_RPC="http://localhost:$SANDBOX_PORT"

# Account names for local deployment
PARENT_ACCOUNT="test.near"
ROUTER_ACCOUNT="cosmos-router.test.near"
WASM_MODULE_ACCOUNT="wasm-module.test.near"

echo -e "${BLUE}Configuration:${NC}"
echo "  Sandbox Home: $SANDBOX_HOME"
echo "  Sandbox RPC: $SANDBOX_RPC"
echo "  Parent Account: $PARENT_ACCOUNT"
echo "  Router Account: $ROUTER_ACCOUNT"
echo "  Wasm Module Account: $WASM_MODULE_ACCOUNT"
echo ""

# Step 1: Check if sandbox is running
echo -e "${YELLOW}Step 1: Checking NEAR Sandbox status...${NC}"
if ! curl -s $SANDBOX_RPC/status > /dev/null 2>&1; then
    echo "Starting NEAR Sandbox..."
    # Clean up any existing sandbox data for a fresh start
    rm -rf "$SANDBOX_HOME"
    mkdir -p "$SANDBOX_HOME"
    
    # Initialize sandbox with localnet configuration
    near-sandbox --home "$SANDBOX_HOME" init
    
    # Start sandbox in the background
    near-sandbox --home "$SANDBOX_HOME" run > "$SANDBOX_HOME/sandbox.log" 2>&1 &
    SANDBOX_PID=$!
    echo "Sandbox PID: $SANDBOX_PID"
    
    # Wait for sandbox to be ready
    echo "Waiting for sandbox to be ready..."
    for i in {1..30}; do
        if curl -s $SANDBOX_RPC/status > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Sandbox is running${NC}"
            break
        fi
        sleep 1
        echo -n "."
    done
    echo ""
    
    if ! curl -s $SANDBOX_RPC/status > /dev/null 2>&1; then
        echo -e "${RED}✗ Failed to start sandbox${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Sandbox is already running${NC}"
fi
echo ""

# Step 2: Build contracts using cargo-near
echo -e "${YELLOW}Step 2: Building contracts with cargo-near...${NC}"

echo "Building router contract..."
cargo near build

# Check if wasm-module-contract exists
if [ -d "../wasm-module-contract" ]; then
    echo "Building wasm module contract..."
    cd ../wasm-module-contract
    cargo near build
    cd ../cosmos-sdk-contract
else
    echo -e "${YELLOW}Warning: wasm-module-contract not found, skipping...${NC}"
fi

echo -e "${GREEN}✓ Contracts built${NC}"
if [ -f "target/near/cosmos_sdk_contract.wasm" ]; then
    echo "  Router: $(ls -lh target/near/cosmos_sdk_contract.wasm | awk '{print $5}')"
fi
if [ -f "../wasm-module-contract/target/near/wasm_module_contract.wasm" ]; then
    echo "  Wasm Module: $(ls -lh ../wasm-module-contract/target/near/wasm_module_contract.wasm | awk '{print $5}')"
fi
echo ""

# Step 3: Create accounts on sandbox
echo -e "${YELLOW}Step 3: Creating accounts on sandbox...${NC}"

# Set NEAR_ENV to use sandbox
export NEAR_ENV=sandbox
export NEAR_CLI_LOCALNET_NETWORK_ID=sandbox
export NEAR_NODE_URL=$SANDBOX_RPC
export NEAR_CLI_LOCALNET_RPC_SERVER_URL=$SANDBOX_RPC
export NEAR_WALLET_URL=$SANDBOX_RPC
export NEAR_HELPER_URL=$SANDBOX_RPC
export NEAR_EXPLORER_URL=$SANDBOX_RPC

# Create accounts
echo "Creating router account..."
near create-account $ROUTER_ACCOUNT --masterAccount $PARENT_ACCOUNT --initialBalance 10 --nodeUrl $SANDBOX_RPC || true

if [ -d "../wasm-module-contract" ]; then
    echo "Creating wasm module account..."
    near create-account $WASM_MODULE_ACCOUNT --masterAccount $PARENT_ACCOUNT --initialBalance 10 --nodeUrl $SANDBOX_RPC || true
fi

echo -e "${GREEN}✓ Accounts created${NC}"
echo ""

# Step 4: Deploy contracts
echo -e "${YELLOW}Step 4: Deploying contracts to sandbox...${NC}"

# Find the correct wasm file location
if [ -f "target/near/cosmos_sdk_contract.wasm" ]; then
    ROUTER_WASM="target/near/cosmos_sdk_contract.wasm"
elif [ -f "target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm" ]; then
    ROUTER_WASM="target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm"
else
    echo -e "${RED}✗ Router contract WASM not found${NC}"
    exit 1
fi

echo "Deploying router contract..."
near deploy --accountId $ROUTER_ACCOUNT --wasmFile $ROUTER_WASM --nodeUrl $SANDBOX_RPC

if [ -d "../wasm-module-contract" ]; then
    if [ -f "../wasm-module-contract/target/near/wasm_module_contract.wasm" ]; then
        WASM_MODULE_WASM="../wasm-module-contract/target/near/wasm_module_contract.wasm"
    elif [ -f "../wasm-module-contract/target/wasm32-unknown-unknown/release/wasm_module_contract.wasm" ]; then
        WASM_MODULE_WASM="../wasm-module-contract/target/wasm32-unknown-unknown/release/wasm_module_contract.wasm"
    fi
    
    if [ -n "$WASM_MODULE_WASM" ]; then
        echo "Deploying wasm module contract..."
        near deploy --accountId $WASM_MODULE_ACCOUNT --wasmFile $WASM_MODULE_WASM --nodeUrl $SANDBOX_RPC
    fi
fi

echo -e "${GREEN}✓ Contracts deployed${NC}"
echo ""

# Step 5: Initialize contracts
echo -e "${YELLOW}Step 5: Initializing contracts...${NC}"

echo "Initializing router..."
near call $ROUTER_ACCOUNT new '{}' --accountId $PARENT_ACCOUNT --nodeUrl $SANDBOX_RPC

if [ -d "../wasm-module-contract" ] && [ -n "$WASM_MODULE_WASM" ]; then
    echo "Initializing wasm module..."
    near call $WASM_MODULE_ACCOUNT new "{\"router_contract\": \"$ROUTER_ACCOUNT\"}" --accountId $PARENT_ACCOUNT --nodeUrl $SANDBOX_RPC
fi

echo -e "${GREEN}✓ Contracts initialized${NC}"
echo ""

# Step 6: Test deployment
echo -e "${YELLOW}Step 6: Testing deployment...${NC}"

echo "Testing router health check..."
near view $ROUTER_ACCOUNT health_check '{}' --nodeUrl $SANDBOX_RPC

if [ -d "../wasm-module-contract" ] && [ -n "$WASM_MODULE_WASM" ]; then
    echo ""
    echo "Testing wasm module health check..."
    near view $WASM_MODULE_ACCOUNT health_check '{}' --nodeUrl $SANDBOX_RPC
fi

echo ""
echo -e "${GREEN}🎉 Local Deployment Complete!${NC}"
echo ""
echo -e "${BLUE}Deployed Contracts:${NC}"
echo "  Router: $ROUTER_ACCOUNT"
if [ -d "../wasm-module-contract" ] && [ -n "$WASM_MODULE_WASM" ]; then
    echo "  Wasm Module: $WASM_MODULE_ACCOUNT"
fi
echo ""
echo -e "${BLUE}Sandbox Information:${NC}"
echo "  RPC Endpoint: $SANDBOX_RPC"
echo "  Home Directory: $SANDBOX_HOME"
echo "  Log File: $SANDBOX_HOME/sandbox.log"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo "1. Test CosmWasm functionality:"
echo "   near call $ROUTER_ACCOUNT <method> '<args>' --accountId $PARENT_ACCOUNT --nodeUrl $SANDBOX_RPC"
echo ""
echo "2. View sandbox logs:"
echo "   tail -f $SANDBOX_HOME/sandbox.log"
echo ""
echo "3. Stop sandbox when done:"
echo "   pkill -f near-sandbox"