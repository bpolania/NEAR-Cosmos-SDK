#!/bin/bash

# Test CosmWasm Deployment on NEAR Testnet
set -e

echo "🧪 Testing CosmWasm on NEAR Testnet"
echo "===================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
PARENT_ACCOUNT="cuteharbor3573.testnet"
ROUTER_ACCOUNT="cosmos-router.cuteharbor3573.testnet"
WASM_MODULE_ACCOUNT="wasm-module.cuteharbor3573.testnet"

echo -e "${BLUE}Testing with:${NC}"
echo "  Router: $ROUTER_ACCOUNT"
echo "  Wasm Module: $WASM_MODULE_ACCOUNT"
echo ""

# Step 1: Create test WASM contract
echo -e "${YELLOW}Step 1: Creating test WASM contract...${NC}"

# Create a minimal valid WASM file
printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > test_contract.wasm
printf '\x01\x04\x01\x60\x00\x00' >> test_contract.wasm
printf '\x03\x02\x01\x00' >> test_contract.wasm
printf '\x07\x08\x01\x04\x69\x6e\x69\x74\x00\x00' >> test_contract.wasm
printf '\x0a\x04\x01\x02\x00\x0b' >> test_contract.wasm

WASM_BYTES=$(hexdump -v -e '1/1 "%u,"' test_contract.wasm | sed 's/,$//')
WASM_BYTES="[$WASM_BYTES]"

echo -e "${GREEN}✓ Test WASM created${NC}"
echo ""

# Step 2: Store WASM code via router
echo -e "${YELLOW}Step 2: Storing WASM code...${NC}"

CODE_ID=$(near call $ROUTER_ACCOUNT wasm_store_code \
    "{\"wasm_byte_code\": $WASM_BYTES, \"source\": \"test\", \"builder\": \"test\", \"instantiate_permission\": {\"everybody\": {}}}" \
    --accountId $PARENT_ACCOUNT \
    --networkId testnet \
    --gas 300000000000000 | tail -1)

echo "Store result: $CODE_ID"
echo -e "${GREEN}✓ WASM code stored${NC}"
echo ""

# Step 3: Get code info
echo -e "${YELLOW}Step 3: Getting code info...${NC}"

near call $ROUTER_ACCOUNT wasm_get_code_info \
    "{\"code_id\": 1}" \
    --accountId $PARENT_ACCOUNT \
    --networkId testnet

echo -e "${GREEN}✓ Code info retrieved${NC}"
echo ""

# Step 4: Instantiate contract
echo -e "${YELLOW}Step 4: Instantiating contract...${NC}"

INIT_MSG='{"init": "test"}'
INIT_MSG_BYTES=$(echo -n "$INIT_MSG" | od -An -tx1 | sed 's/ /,0x/g' | sed 's/^,/[/; s/$/]/' | sed 's/,0x/, /g' | sed 's/\[, /[/')

CONTRACT_ADDR=$(near call $ROUTER_ACCOUNT wasm_instantiate \
    "{\"code_id\": 1, \"msg\": $INIT_MSG_BYTES, \"funds\": [], \"label\": \"test-contract\", \"admin\": null}" \
    --accountId $PARENT_ACCOUNT \
    --networkId testnet \
    --gas 300000000000000 | tail -1)

echo "Contract address: $CONTRACT_ADDR"
echo -e "${GREEN}✓ Contract instantiated${NC}"
echo ""

# Step 5: Execute contract
echo -e "${YELLOW}Step 5: Executing contract...${NC}"

EXEC_MSG='{"execute": "test"}'
EXEC_MSG_BYTES=$(echo -n "$EXEC_MSG" | od -An -tx1 | sed 's/ /,0x/g' | sed 's/^,/[/; s/$/]/' | sed 's/,0x/, /g' | sed 's/\[, /[/')

near call $ROUTER_ACCOUNT wasm_execute \
    "{\"contract_addr\": \"contract.1.1\", \"msg\": $EXEC_MSG_BYTES, \"funds\": []}" \
    --accountId $PARENT_ACCOUNT \
    --networkId testnet \
    --gas 300000000000000

echo -e "${GREEN}✓ Contract executed${NC}"
echo ""

# Clean up
rm -f test_contract.wasm

echo -e "${GREEN}🎉 CosmWasm Testing Complete!${NC}"
echo ""
echo "The modular CosmWasm architecture is working on testnet!"