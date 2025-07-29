#!/bin/bash

echo "🚀 Setting up testnet deployment configuration..."

# Set environment variables for keys
export RELAYER_KEY_PROVIDER="cosmos162ca2a24f0d288439231d29170a101e554b7e6:d600357797a65160742b73279fb55f55faf83258f841e8411d5503b95f079791"
export RELAYER_KEY_NEAR_TESTNET="relayer.testnet:ed25519:5K8HtSNHQDFvEpALHMy4QN9CvZaT6Q4MpX2YmRe3JdKtF4QdF3uXH8p9RcVjKbM6S4DdNy1F2X4QhVtK8MmN9pL"

echo "✅ Environment variables set:"
echo "   RELAYER_KEY_PROVIDER (Cosmos provider testnet)"
echo "   RELAYER_KEY_NEAR_TESTNET (NEAR testnet)"
echo ""

echo "🔗 Testing network connectivity..."

echo "📡 Cosmos provider testnet status:"
curl -s "https://rpc.provider-sentry-01.ics-testnet.polypore.xyz/status" | jq '.result.node_info.network, .result.sync_info.latest_block_height'

echo ""
echo "📡 NEAR testnet status:"
curl -s "https://rpc.testnet.near.org/status" | jq '.chain_id, .sync_info.latest_block_height'

echo ""
echo "✅ Testnet deployment configuration ready!"
echo ""
echo "🔑 Next steps:"
echo "   1. Fund Cosmos address: cosmos162ca2a24f0d288439231d29170a101e554b7e6"
echo "   2. Create NEAR testnet account: relayer.testnet"
echo "   3. Deploy contract to cosmos-sdk-demo.testnet"
echo "   4. Start relayer: cargo run -- start"