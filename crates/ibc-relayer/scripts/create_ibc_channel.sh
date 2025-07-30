#!/bin/bash
set -e

echo "Creating IBC channel for token transfers..."

CONTRACT_ID="cosmos-sdk-demo.testnet"
SIGNER_ID="cuteharbor3573.testnet"
CONNECTION_ID="connection-0"

echo "Step 1: Initialize transfer channel on NEAR side..."

# Channel parameters for ICS-20 token transfer
PORT_ID="transfer"
COUNTERPARTY_PORT_ID="transfer"
CHANNEL_VERSION="ics20-1"
ORDER=0  # 0 = Unordered (typical for token transfers)

# Initialize the channel
echo "Calling ibc_chan_open_init..."
RESULT=$(near call $CONTRACT_ID ibc_chan_open_init "{
  \"port_id\": \"$PORT_ID\",
  \"order\": $ORDER,
  \"connection_hops\": [\"$CONNECTION_ID\"],
  \"counterparty_port_id\": \"$COUNTERPARTY_PORT_ID\",
  \"version\": \"$CHANNEL_VERSION\"
}" --accountId $SIGNER_ID --gas 300000000000000)

echo "Channel init result: $RESULT"

# Check if channel was created
if [[ "$RESULT" == *"channel-"* ]]; then
    echo "✅ IBC channel initialized successfully!"
    
    # Extract channel ID from result
    CHANNEL_ID=$(echo "$RESULT" | grep -o "channel-[0-9]*" | head -1)
    echo "Channel ID: $CHANNEL_ID"
    echo "Expected channel: channel-0"
    
    # Verify channel state
    echo "Verifying channel details..."
    CHANNEL_DETAILS=$(near view $CONTRACT_ID ibc_get_channel "{\"port_id\": \"$PORT_ID\", \"channel_id\": \"$CHANNEL_ID\"}")
    echo "Channel details: $CHANNEL_DETAILS"
    
    echo ""
    echo "🎉 IBC channel $CHANNEL_ID created successfully!"
    echo "Configuration:"
    echo "- Port: $PORT_ID"
    echo "- Channel: $CHANNEL_ID" 
    echo "- Connection: $CONNECTION_ID"
    echo "- Version: $CHANNEL_VERSION"
    echo "- Order: Unordered"
    echo ""
    echo "Next steps:"
    echo "1. Complete channel handshake on Cosmos provider side"
    echo "2. Test token transfers between NEAR and Cosmos"
else
    echo "❌ Failed to create IBC channel"
    echo "Result: $RESULT"
    exit 1
fi