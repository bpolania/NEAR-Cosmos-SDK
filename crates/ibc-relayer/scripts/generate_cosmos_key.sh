#!/bin/bash
set -e

# Simple script to generate a Cosmos test key using openssl
echo "🔑 Generating test Cosmos key for provider testnet..."

# Generate 32 random bytes for private key
PRIVATE_KEY=$(openssl rand -hex 32)

# Generate a mock address for demonstration (cosmos1...)
# In real usage, this would be derived from the public key
ADDRESS="cosmos1$(openssl rand -hex 20 | cut -c1-38)"

echo "✅ Generated test Cosmos key:"
echo "   Private Key: $PRIVATE_KEY"
echo "   Address:     $ADDRESS"
echo ""
echo "💰 To fund this address, visit:"
echo "   https://faucet.cosmoskit.com/ (if available)"
echo "   Or ask in Cosmos Discord for testnet tokens"
echo ""
echo "🔐 To add to keystore, run:"
echo "   cargo run --bin key-manager add provider --key-type cosmos"
echo "   Then enter the private key and address above when prompted"
echo ""
echo "📋 For environment variable setup:"
echo "   export RELAYER_KEY_PROVIDER=\"$ADDRESS:$PRIVATE_KEY\""