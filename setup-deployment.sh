#!/bin/bash

# Setup script for NEAR deployment environment

set -e

echo "🔧 Setting up NEAR deployment environment..."

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo "📝 Creating .env file from template..."
    cp .env.example .env
    echo "✅ Created .env file"
    echo ""
    echo "🔑 Please edit .env file and add your NEAR credentials:"
    echo "   1. Set NEAR_ACCOUNT_ID to your testnet account"
    echo "   2. Set NEAR_PRIVATE_KEY to your private key"
    echo ""
    echo "💡 To get your private key:"
    echo "   near login  # Login first"
    echo "   cat ~/.near-credentials/testnet/your-account.testnet.json"
    echo ""
    exit 1
fi

# Load environment variables
echo "📊 Loading environment variables from .env..."
set -a  # Export all variables
source .env
set +a  # Stop exporting

# Validate required variables
if [ -z "$NEAR_ACCOUNT_ID" ]; then
    echo "❌ Error: NEAR_ACCOUNT_ID not set in .env file"
    exit 1
fi

if [ -z "$NEAR_PRIVATE_KEY" ]; then
    echo "❌ Error: NEAR_PRIVATE_KEY not set in .env file"
    exit 1
fi

echo "✅ Environment configured for account: $NEAR_ACCOUNT_ID"

# Export for deployment script
export NEAR_ACCOUNT_ID
export NEAR_PRIVATE_KEY
export CONTRACT_NAME

echo "🚀 Ready for deployment! Run:"
echo "   ./deploy-testnet.sh"