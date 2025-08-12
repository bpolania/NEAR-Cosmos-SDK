#!/bin/bash

# Show Address Mappings
# Demonstrates how NEAR accounts map to Cosmos-style addresses

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================"
echo "NEAR to Cosmos Address Mappings"
echo "========================================${NC}"
echo ""

echo -e "${CYAN}When you interact with CW20 tokens on NEAR, addresses are converted:${NC}"
echo ""

# Common NEAR accounts and their Cosmos equivalents
declare -a NEAR_ACCOUNTS=(
    "cosmos-sdk-demo-1754812961.testnet"
    "alice.testnet"
    "bob.testnet"
    "charlie.testnet"
    "test.testnet"
    "minter.testnet"
    "admin.testnet"
)

echo -e "${YELLOW}Example Address Mappings:${NC}"
echo "----------------------------------------"

for account in "${NEAR_ACCOUNTS[@]}"; do
    # Generate a mock Cosmos address using the same logic as our Rust code
    # (This is a simplified version for demonstration)
    hash=$(echo -n "$account" | sha256sum | cut -d' ' -f1)
    # Take first 40 chars of hash for demo (actual implementation uses bech32)
    cosmos_addr="proxima1${hash:0:38}"
    
    echo -e "${GREEN}NEAR:${NC}   $account"
    echo -e "${BLUE}Cosmos:${NC} $cosmos_addr"
    echo ""
done

echo "----------------------------------------"
echo ""

echo -e "${CYAN}Key Points:${NC}"
echo "• NEAR accounts are deterministically mapped to Cosmos addresses"
echo "• The mapping uses SHA256 hashing for consistency"
echo "• Addresses use the 'proxima' prefix (proxima1...)"
echo "• The same NEAR account always maps to the same Cosmos address"
echo "• Contract addresses are generated similarly from instance IDs"
echo ""

echo -e "${YELLOW}Contract Address Generation:${NC}"
echo "----------------------------------------"
echo "Contract instances get addresses like:"
echo "  proxima1<hash_of_module_and_instance_id>"
echo ""
echo "Example:"
echo "  Instance 1: proxima1qyqszqgpqyqszqgpqyqszqgpqyqszqgp..."
echo "  Instance 2: proxima1szqgpqyqszqgpqyqszqgpqyqszqgpqyq..."
echo ""

echo -e "${GREEN}In CW20 Operations:${NC}"
echo "----------------------------------------"
echo "When you mint tokens to 'alice.testnet', they actually go to:"
echo "  proxima1<alice_hash>..."
echo ""
echo "When you transfer from 'bob.testnet' to 'charlie.testnet':"
echo "  From: proxima1<bob_hash>..."
echo "  To:   proxima1<charlie_hash>..."
echo ""

echo -e "${BLUE}Important Notes:${NC}"
echo "----------------------------------------"
echo "1. The actual implementation uses proper bech32 encoding"
echo "2. Addresses are 20 bytes (40 hex chars) after 'proxima1'"
echo "3. This ensures CosmWasm contracts work correctly"
echo "4. All balance tracking uses Cosmos addresses internally"
echo "5. The NEAR interface translates addresses automatically"
echo ""

echo -e "${CYAN}========================================"
echo "Address System Overview Complete"
echo "========================================${NC}"