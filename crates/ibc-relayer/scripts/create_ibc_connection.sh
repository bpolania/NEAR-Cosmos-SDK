#!/bin/bash
set -e

echo "Creating IBC connection between NEAR and Cosmos provider..."

CONTRACT_ID="cosmos-sdk-demo.testnet"
SIGNER_ID="cuteharbor3573.testnet"
CLIENT_ID="07-tendermint-0"

echo "Step 1: Initialize connection on NEAR side..."

# Connection parameters
CONNECTION_ID="connection-0" 
COUNTERPARTY_CLIENT_ID="07-tendermint-0"  # This would be the client ID on Cosmos side
COUNTERPARTY_CONNECTION_ID=""  # Empty for init step

# Initialize the connection
echo "Calling ibc_conn_open_init..."
RESULT=$(near call $CONTRACT_ID ibc_conn_open_init "{
  \"client_id\": \"$CLIENT_ID\",
  \"counterparty_client_id\": \"$COUNTERPARTY_CLIENT_ID\",
  \"counterparty_prefix\": [105, 98, 99],
  \"version\": {
    \"identifier\": \"1\",
    \"features\": [\"ORDER_ORDERED\", \"ORDER_UNORDERED\"]
  },
  \"delay_period\": 0
}" --accountId $SIGNER_ID --gas 300000000000000)

echo "Connection init result: $RESULT"

# Check if connection was created
if [[ "$RESULT" == *"connection-"* ]]; then
    echo "✅ IBC connection initialized successfully!"
    
    # Extract connection ID from result if possible
    echo "Verifying connection state..."
    CONNECTION_STATE=$(near view $CONTRACT_ID ibc_get_connection_state '{\"connection_id\": \"connection-0\"}')
    echo "Connection state: $CONNECTION_STATE"
    
    echo ""
    echo "🎉 IBC connection-0 created successfully!"
    echo "Next steps:"
    echo "1. Create corresponding connection on Cosmos provider chain"
    echo "2. Complete connection handshake (try/ack/confirm)"
    echo "3. Create IBC channel for token transfers"
else
    echo "❌ Failed to create IBC connection"
    echo "Result: $RESULT"
    exit 1
fi