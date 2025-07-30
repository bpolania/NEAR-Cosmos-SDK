#!/bin/bash
set -e

echo "======================================="
echo "Starting IBC Relayer for Testnet"
echo "======================================="

# Load environment variables
if [ -f .env ]; then
    echo "Loading environment variables from .env..."
    set -a
    source .env
    set +a
else
    echo "Warning: .env file not found. Using system environment variables."
fi

# Verify required environment variables
if [ -z "$RELAYER_KEY_NEAR_TESTNET" ]; then
    echo "Error: RELAYER_KEY_NEAR_TESTNET not set"
    echo "Please set this in .env or as environment variable"
    exit 1
fi

if [ -z "$RELAYER_KEY_PROVIDER" ]; then
    echo "Error: RELAYER_KEY_PROVIDER not set"
    echo "Please set this in .env or as environment variable"
    exit 1
fi

# Display configuration
echo ""
echo "Configuration:"
echo "- NEAR Account: $(echo $RELAYER_KEY_NEAR_TESTNET | cut -d: -f1)"
echo "- Cosmos Account: $(echo $RELAYER_KEY_PROVIDER | cut -d: -f1)"
echo "- Log Level: ${RUST_LOG:-info}"
echo "- Metrics Port: ${METRICS_PORT:-9090}"
echo ""

# Check if relayer is built
if [ ! -f "target/release/ibc-relayer" ]; then
    echo "Building relayer in release mode..."
    cargo build --release
fi

# Start the relayer
echo "Starting relayer..."
echo "======================================="
echo ""

exec cargo run --release --bin relayer -- start "$@"