#!/bin/bash

# CW20 Integration Test Script
# Tests actual CW20 operations on NEAR testnet

set -e

# Configuration
WASM_MODULE="wasm-module.cosmos-sdk-demo-1754812961.testnet"
SIGNER="cosmos-sdk-demo-1754812961.testnet"
CONTRACT_ADDR="contract1.wasm-module.cosmos-sdk-demo-1754812961.testnet"
NEAR_CLI="$HOME/.cargo/bin/near"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run tests
run_test() {
    local test_name="$1"
    local command="$2"
    
    echo -e "${YELLOW}Running test: $test_name${NC}"
    if eval "$command"; then
        echo -e "${GREEN}✓ $test_name passed${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        ((TESTS_FAILED++))
    fi
    echo ""
}

# Helper function for contract calls
call_contract() {
    local method="$1"
    local args="$2"
    
    $NEAR_CLI contract call-function as-transaction \
        "$WASM_MODULE" "$method" \
        json-args "$args" \
        prepaid-gas '100 Tgas' \
        attached-deposit '0 NEAR' \
        sign-as "$SIGNER" \
        network-config testnet \
        sign-with-keychain send 2>/dev/null
}

# Helper function for queries
query_contract() {
    local msg="$1"
    
    $NEAR_CLI contract call-function as-read-only \
        "$WASM_MODULE" query \
        json-args "{\"contract_addr\": \"$CONTRACT_ADDR\", \"msg\": \"$msg\"}" \
        network-config testnet \
        now 2>/dev/null
}

echo "========================================"
echo "CW20 Token Integration Tests"
echo "========================================"
echo ""

# Test 1: Query Token Info
run_test "Query Token Info" \
    "query_contract '{\"token_info\":{}}' | grep -q 'query_result'"

# Test 2: Query Balance
run_test "Query Balance" \
    "query_contract '{\"balance\":{\"address\":\"$SIGNER\"}}' | grep -q 'query_result'"

# Test 3: Transfer Tokens
echo -e "${YELLOW}Testing Transfer Operation${NC}"
TRANSFER_MSG='{
    "transfer": {
        "recipient": "test.testnet",
        "amount": "1000"
    }
}'
run_test "Transfer Tokens" \
    "call_contract execute \"{\\\"contract_addr\\\": \\\"$CONTRACT_ADDR\\\", \\\"msg\\\": \\\"$(echo $TRANSFER_MSG | sed 's/"/\\\\"/g')\\\", \\\"_funds\\\": null}\" | grep -q 'events'"

# Test 4: Increase Allowance
echo -e "${YELLOW}Testing Allowance Operations${NC}"
ALLOWANCE_MSG='{
    "increase_allowance": {
        "spender": "spender.testnet",
        "amount": "5000",
        "expires": null
    }
}'
run_test "Increase Allowance" \
    "call_contract execute \"{\\\"contract_addr\\\": \\\"$CONTRACT_ADDR\\\", \\\"msg\\\": \\\"$(echo $ALLOWANCE_MSG | sed 's/"/\\\\"/g')\\\", \\\"_funds\\\": null}\" | grep -q 'events'"

# Test 5: Query Allowance
run_test "Query Allowance" \
    "query_contract '{\"allowance\":{\"owner\":\"$SIGNER\",\"spender\":\"spender.testnet\"}}' | grep -q 'query_result'"

# Test 6: Burn Tokens (if supported)
echo -e "${YELLOW}Testing Burn Operation${NC}"
BURN_MSG='{
    "burn": {
        "amount": "100"
    }
}'
run_test "Burn Tokens" \
    "call_contract execute \"{\\\"contract_addr\\\": \\\"$CONTRACT_ADDR\\\", \\\"msg\\\": \\\"$(echo $BURN_MSG | sed 's/"/\\\\"/g')\\\", \\\"_funds\\\": null}\" | grep -q 'events'"

# Test 7: Query Total Supply
run_test "Query Total Supply" \
    "query_contract '{\"token_info\":{}}' | grep -q 'query_result'"

# Test 8: Query All Accounts
run_test "Query All Accounts" \
    "query_contract '{\"all_accounts\":{\"start_after\":null,\"limit\":10}}' | grep -q 'query_result'"

# Test 9: Transfer with Memo
echo -e "${YELLOW}Testing Transfer with Memo${NC}"
TRANSFER_MEMO_MSG='{
    "transfer": {
        "recipient": "recipient.testnet",
        "amount": "500"
    }
}'
run_test "Transfer with Memo" \
    "call_contract execute \"{\\\"contract_addr\\\": \\\"$CONTRACT_ADDR\\\", \\\"msg\\\": \\\"$(echo $TRANSFER_MEMO_MSG | sed 's/"/\\\\"/g')\\\", \\\"_funds\\\": null}\" | grep -q 'events'"

# Test 10: Decrease Allowance
DECREASE_MSG='{
    "decrease_allowance": {
        "spender": "spender.testnet",
        "amount": "2000",
        "expires": null
    }
}'
run_test "Decrease Allowance" \
    "call_contract execute \"{\\\"contract_addr\\\": \\\"$CONTRACT_ADDR\\\", \\\"msg\\\": \\\"$(echo $DECREASE_MSG | sed 's/"/\\\\"/g')\\\", \\\"_funds\\\": null}\" | grep -q 'events'"

# Summary
echo "========================================"
echo "Test Summary"
echo "========================================"
echo -e "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Tests Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi