#!/bin/bash
set -e

echo "🔨 Building IBC Relayer..."

cd crates/ibc-relayer

# Build the relayer binary
cargo build --release

echo "✅ Relayer built successfully!"
echo "📦 Binary output: crates/ibc-relayer/target/release/relayer"