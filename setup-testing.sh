#!/bin/bash

# Setup script for testing Cosmos-on-NEAR

set -e

echo "🚀 Setting up testing environment for Cosmos-on-NEAR..."

# Check if Homebrew is installed (macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
    if ! command -v brew &> /dev/null; then
        echo "❌ Homebrew not found. Please install Homebrew first:"
        echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        exit 1
    fi
    
    echo "📦 Installing TinyGo via Homebrew..."
    brew install tinygo
    
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "📦 Installing TinyGo for Linux..."
    # Download and install TinyGo for Linux
    TINYGO_VERSION="0.36.0"
    wget https://github.com/tinygo-org/tinygo/releases/download/v${TINYGO_VERSION}/tinygo_${TINYGO_VERSION}_amd64.deb
    sudo dpkg -i tinygo_${TINYGO_VERSION}_amd64.deb
    rm tinygo_${TINYGO_VERSION}_amd64.deb
    
else
    echo "❌ Unsupported OS. Please install TinyGo manually:"
    echo "   https://tinygo.org/getting-started/install/"
    exit 1
fi

# Verify TinyGo installation
echo "✅ Verifying TinyGo installation..."
tinygo version

# Check if NEAR CLI is installed
echo "🔍 Checking NEAR CLI..."
if ! command -v near &> /dev/null; then
    echo "📦 Installing NEAR CLI..."
    npm install -g near-cli
else
    echo "✅ NEAR CLI already installed"
fi

# Verify NEAR CLI
near --version

echo ""
echo "🎉 Setup complete! You can now:"
echo "   1. Build the contract: cd cosmos_on_near && ./build.sh"
echo "   2. Run tests: ./test-integration.sh"
echo "   3. Deploy to testnet: ./deploy-testnet.sh"