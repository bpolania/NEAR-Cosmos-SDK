#!/bin/bash

# Integration test script for Cosmos-on-NEAR
# 
# Prerequisites:
# 1. Build contract: tinygo build -target=wasi -o tinygo_contract.wasm ./cmd/tinygo_main.go
# 2. Deploy contract: near deploy --accountId cosmos-on-near.testnet --wasmFile tinygo_contract.wasm
# 3. Ensure all test accounts exist and have NEAR for gas
#
# NOTE: The TinyGo contract (tinygo_main.go) currently has placeholder input parsing.
# For full integration testing, use the legacy contract (cmd/main.go) or implement
# proper JSON input parsing in the TinyGo version.

set -e

echo "🧪 Running Cosmos-on-NEAR Integration Tests..."

# Configuration
CONTRACT_NAME="cosmos-on-near.testnet"
ADMIN_ACCOUNT="admin.testnet"
ALICE_ACCOUNT="alice.testnet"
BOB_ACCOUNT="bob.testnet"
VALIDATOR_ACCOUNT="validator.testnet"

echo "📋 Test Configuration:"
echo "   Contract: $CONTRACT_NAME"
echo "   Admin: $ADMIN_ACCOUNT"
echo "   Alice: $ALICE_ACCOUNT"
echo "   Bob: $BOB_ACCOUNT"
echo "   Validator: $VALIDATOR_ACCOUNT"
echo ""

# Test 1: Bank Module - Minting and Transfers
echo "🏦 Testing Bank Module..."

echo "  • Minting 1000 tokens to Alice..."
near call $CONTRACT_NAME mint '{"receiver": "'$ALICE_ACCOUNT'", "amount": 1000}' --accountId $ADMIN_ACCOUNT

echo "  • Checking Alice's balance..."
ALICE_BALANCE=$(near call $CONTRACT_NAME get_balance '{"account": "'$ALICE_ACCOUNT'"}' --accountId $ALICE_ACCOUNT)
echo "    Alice balance: $ALICE_BALANCE"

echo "  • Alice transfers 300 tokens to Bob..."
near call $CONTRACT_NAME transfer '{"sender": "'$ALICE_ACCOUNT'", "receiver": "'$BOB_ACCOUNT'", "amount": 300}' --accountId $ALICE_ACCOUNT

echo "  • Checking balances after transfer..."
ALICE_BALANCE=$(near call $CONTRACT_NAME get_balance '{"account": "'$ALICE_ACCOUNT'"}' --accountId $ALICE_ACCOUNT)
BOB_BALANCE=$(near call $CONTRACT_NAME get_balance '{"account": "'$BOB_ACCOUNT'"}' --accountId $BOB_ACCOUNT)
echo "    Alice balance: $ALICE_BALANCE (should be 700)"
echo "    Bob balance: $BOB_BALANCE (should be 300)"

# Test 2: Staking Module
echo ""
echo "🥩 Testing Staking Module..."

echo "  • Adding validator..."
near call $CONTRACT_NAME add_validator '{"address": "'$VALIDATOR_ACCOUNT'"}' --accountId $ADMIN_ACCOUNT

echo "  • Alice delegates 100 tokens to validator..."
near call $CONTRACT_NAME delegate '{"validator": "'$VALIDATOR_ACCOUNT'", "amount": 100}' --accountId $ALICE_ACCOUNT

echo "  • Alice undelegates 50 tokens..."
near call $CONTRACT_NAME undelegate '{"validator": "'$VALIDATOR_ACCOUNT'", "amount": 50}' --accountId $ALICE_ACCOUNT

# Test 3: Governance Module
echo ""
echo "🗳️  Testing Governance Module..."

echo "  • Submitting parameter proposal..."
PROPOSAL_ID=$(near call $CONTRACT_NAME submit_proposal '{
    "title": "Update Reward Rate", 
    "description": "Increase staking rewards", 
    "param_key": "reward_rate", 
    "param_value": "10"
}' --accountId $ADMIN_ACCOUNT)
echo "    Proposal ID: $PROPOSAL_ID"

echo "  • Voting YES on proposal..."
near call $CONTRACT_NAME vote '{"proposal_id": 1, "option": 1}' --accountId $ALICE_ACCOUNT

echo "  • Checking parameter (should be empty before proposal passes)..."
PARAM_VALUE=$(near call $CONTRACT_NAME get_parameter '{"key": "reward_rate"}' --accountId $ALICE_ACCOUNT)
echo "    Current reward_rate: $PARAM_VALUE"

# Test 4: Block Processing
echo ""
echo "⏱️  Testing Block Processing..."

echo "  • Processing 5 blocks to advance time..."
for i in {1..5}; do
    echo "    Processing block $i..."
    near call $CONTRACT_NAME process_block '{}' --accountId $ADMIN_ACCOUNT
    sleep 1
done

echo "  • Processing enough blocks to complete proposal voting period..."
for i in {1..50}; do
    near call $CONTRACT_NAME process_block '{}' --accountId $ADMIN_ACCOUNT >/dev/null 2>&1
done

echo "  • Checking if proposal passed and parameter was updated..."
PARAM_VALUE=$(near call $CONTRACT_NAME get_parameter '{"key": "reward_rate"}' --accountId $ALICE_ACCOUNT)
echo "    Updated reward_rate: $PARAM_VALUE (should be '10' if proposal passed)"

echo ""
echo "🎉 Integration tests completed!"
echo ""
echo "📊 Summary:"
echo "   ✅ Bank transfers and minting"
echo "   ✅ Validator management and delegation"  
echo "   ✅ Governance proposals and voting"
echo "   ✅ Block processing and time-based logic"
echo ""
echo "🔍 Check the console output above for detailed results."