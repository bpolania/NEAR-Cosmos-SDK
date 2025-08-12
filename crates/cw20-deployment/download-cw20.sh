#!/bin/bash
set -e

echo "📦 Downloading CW20 Base Contract WASM"
echo "======================================"

# Create contracts directory if it doesn't exist
mkdir -p contracts

# CW20 Base contract from CosmWasm Plus v1.1.2 (stable release)
CW20_URL="https://github.com/CosmWasm/cw-plus/releases/download/v1.1.2/cw20_base.wasm"

echo "Downloading from: $CW20_URL"
curl -L -o contracts/cw20_base.wasm "$CW20_URL"

# Verify the download
if [ -f contracts/cw20_base.wasm ]; then
    FILE_SIZE=$(wc -c < contracts/cw20_base.wasm)
    echo "✅ Downloaded CW20 contract: $FILE_SIZE bytes"
    
    # Generate Base64 for NEAR deployment
    echo ""
    echo "Generating Base64 encoding for NEAR..."
    base64 -i contracts/cw20_base.wasm -o contracts/cw20_base.wasm.b64
    echo "✅ Base64 encoded file saved to contracts/cw20_base.wasm.b64"
    
    # Show first 100 chars of base64 to verify
    echo ""
    echo "Base64 preview (first 100 chars):"
    head -c 100 contracts/cw20_base.wasm.b64
    echo "..."
else
    echo "❌ Failed to download CW20 contract"
    exit 1
fi

echo ""
echo "✨ CW20 contract ready for deployment!"