#!/bin/sh
set -e

CHAIN_ID="wasmd-testnet"
MONIKER="testnode"
KEYRING_BACKEND="test"
HOME_DIR="/root/.wasmd"

echo "Starting wasmd initialization..."
echo "Chain ID: $CHAIN_ID"
echo "Moniker: $MONIKER"
echo "Home dir: $HOME_DIR"

# Check if already initialized
if [ -d "$HOME_DIR/config" ] && [ -f "$HOME_DIR/config/genesis.json" ]; then
    echo "Node already initialized, starting wasmd..."
else
    echo "Initializing new node..."
fi

echo "Initializing wasmd testnet..."

# Initialize node
wasmd init $MONIKER --chain-id $CHAIN_ID --home $HOME_DIR

# Create test accounts
echo "Creating test accounts..."

# Simply create new accounts (non-deterministic but working)
echo "Creating validator key..."
wasmd keys add validator --keyring-backend $KEYRING_BACKEND --home $HOME_DIR --output json > /tmp/validator.json 2>&1 || {
    echo "Error creating validator key:"
    cat /tmp/validator.json
    exit 1
}

echo "Creating test1 key..."
wasmd keys add test1 --keyring-backend $KEYRING_BACKEND --home $HOME_DIR --output json > /tmp/test1.json 2>&1 || {
    echo "Error creating test1 key:"
    cat /tmp/test1.json
    exit 1
}

echo "Creating relayer key..."
wasmd keys add relayer --keyring-backend $KEYRING_BACKEND --home $HOME_DIR --output json > /tmp/relayer.json 2>&1 || {
    echo "Error creating relayer key:"
    cat /tmp/relayer.json
    exit 1
}

# Get addresses
VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend $KEYRING_BACKEND --home $HOME_DIR 2>/dev/null || echo "")
TEST1_ADDR=$(wasmd keys show test1 -a --keyring-backend $KEYRING_BACKEND --home $HOME_DIR 2>/dev/null || echo "")
RELAYER_ADDR=$(wasmd keys show relayer -a --keyring-backend $KEYRING_BACKEND --home $HOME_DIR 2>/dev/null || echo "")

echo "Validator address: $VALIDATOR_ADDR"
echo "Test1 address: $TEST1_ADDR"
echo "Relayer address: $RELAYER_ADDR"

# Only proceed if we have addresses
if [ -n "$VALIDATOR_ADDR" ]; then
    # Add genesis accounts with large balances
    wasmd genesis add-genesis-account $VALIDATOR_ADDR 100000000000000000000stake,100000000000000000000token --home $HOME_DIR
    
    if [ -n "$TEST1_ADDR" ]; then
        wasmd genesis add-genesis-account $TEST1_ADDR 100000000000000000000stake,100000000000000000000token --home $HOME_DIR
    fi
    
    if [ -n "$RELAYER_ADDR" ]; then
        wasmd genesis add-genesis-account $RELAYER_ADDR 100000000000000000000stake,100000000000000000000token --home $HOME_DIR
    fi

    # Generate genesis transaction
    wasmd genesis gentx validator 1000000000000000000stake --chain-id $CHAIN_ID --keyring-backend $KEYRING_BACKEND --home $HOME_DIR
else
    echo "ERROR: Could not create validator address"
    exit 1
fi

# Collect genesis transactions
wasmd genesis collect-gentxs --home $HOME_DIR

# Validate genesis
wasmd genesis validate-genesis --home $HOME_DIR

# Update configuration for testing
sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/' $HOME_DIR/config/config.toml
sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' $HOME_DIR/config/config.toml
sed -i 's/enable = false/enable = true/' $HOME_DIR/config/app.toml
sed -i 's/swagger = false/swagger = true/' $HOME_DIR/config/app.toml
sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/' $HOME_DIR/config/app.toml

echo "Starting wasmd..."
exec wasmd start --home $HOME_DIR