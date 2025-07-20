#!/bin/bash

set -e

echo "🦀 Building Rust test contract..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build the contract
cargo build --target wasm32-unknown-unknown --release

# Copy the WASM file to a convenient location
mkdir -p build
cp target/wasm32-unknown-unknown/release/cosmos_test_contract.wasm build/

echo "✅ Contract built successfully!"
echo "📦 WASM file: build/cosmos_test_contract.wasm"

# Show file size
ls -lh build/cosmos_test_contract.wasm