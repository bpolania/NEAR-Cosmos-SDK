#!/bin/bash

# Get NEAR tokens for testnet
set -e

echo "💰 Getting NEAR Testnet Tokens"
echo "=============================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ACCOUNT="cuteharbor3573.testnet"

echo -e "${BLUE}Current Balance:${NC}"
BALANCE=$(near state $ACCOUNT --networkId testnet | grep amount | awk '{print $2}' | tr -d ',')
echo "$(echo "scale=3; $BALANCE / 1000000000000000000000000" | bc) NEAR"
echo ""

echo -e "${YELLOW}Option 1: NEAR Testnet Faucet${NC}"
echo "Visit: https://near-faucet.io/"
echo "1. Enter your account: $ACCOUNT"
echo "2. Complete the captcha"
echo "3. Receive 20 NEAR instantly"
echo ""

echo -e "${YELLOW}Option 2: MyNearWallet Faucet${NC}"
echo "Visit: https://testnet.mynearwallet.com/"
echo "1. Sign in with your testnet account"
echo "2. Click on 'Get Testnet Tokens'"
echo "3. Receive tokens"
echo ""

echo -e "${YELLOW}Option 3: Discord Faucet${NC}"
echo "1. Join NEAR Discord: https://discord.gg/near"
echo "2. Go to #testnet-faucet channel"
echo "3. Type: /faucet $ACCOUNT"
echo ""

echo -e "${YELLOW}Option 4: Use near-cli (if faucet service is enabled)${NC}"
echo "Try running:"
echo -e "${GREEN}near create-account temp-\$(date +%s).testnet --useFaucet --networkId testnet${NC}"
echo ""

echo -e "${BLUE}After getting tokens, verify balance:${NC}"
echo "near state $ACCOUNT --networkId testnet"
echo ""

# Try the CLI faucet
echo -e "${YELLOW}Attempting CLI faucet...${NC}"
TEMP_ACCOUNT="faucet-$(date +%s).testnet"
if near create-account $TEMP_ACCOUNT --useFaucet --networkId testnet 2>/dev/null; then
    echo -e "${GREEN}✓ Created faucet account: $TEMP_ACCOUNT${NC}"
    echo "Transferring funds to main account..."
    near send $TEMP_ACCOUNT $ACCOUNT 19 --networkId testnet || echo "Transfer failed"
else
    echo "CLI faucet not available. Please use one of the web options above."
fi