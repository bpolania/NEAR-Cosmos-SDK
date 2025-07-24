#!/bin/bash
set -e

echo "🔨 Building Cosmos SDK Contract..."

cd crates/cosmos-sdk-contract

# Build the contract with cargo-near for proper WASM target
cargo near build

echo "✅ Contract built successfully!"
echo "📦 WASM output: crates/cosmos-sdk-contract/target/near/cosmos-sdk-contract.wasm"