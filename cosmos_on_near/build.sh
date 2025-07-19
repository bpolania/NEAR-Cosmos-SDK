#!/bin/bash

# Build script for Cosmos-on-NEAR smart contracts using TinyGo

set -e

echo "Building Cosmos-on-NEAR contracts with TinyGo..."

# Check if TinyGo is installed
if ! command -v tinygo &> /dev/null; then
    echo "TinyGo is not installed. Please install TinyGo version 0.36.0"
    echo "Visit: https://tinygo.org/getting-started/install/"
    exit 1
fi

# Check TinyGo version
TINYGO_VERSION=$(tinygo version | head -n1 | cut -d' ' -f3)
echo "Using TinyGo version: $TINYGO_VERSION"

# Create build directory
mkdir -p build

# Build the main contract
echo "Building main contract..."
tinygo build -o build/main.wasm -target=wasi -gc=leaking ./cmd/main.go

echo "Build completed successfully!"
echo "Output: build/main.wasm"

# Show file size
ls -lh build/main.wasm