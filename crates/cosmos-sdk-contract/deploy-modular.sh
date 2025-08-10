#!/bin/bash
set -e

# Modular Cosmos SDK Deployment Script
# Deploys the router and all module contracts for the modular architecture

echo "🚀 Starting Modular Cosmos SDK Deployment"

# Configuration
NETWORK="testnet"  # Change to "mainnet" for production
ACCOUNT_PREFIX="cosmos-sdk"
NEAR_ENV=${NETWORK}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    echo "🔍 Checking prerequisites..."
    
    if ! command -v near &> /dev/null; then
        print_error "NEAR CLI is not installed. Please install it first."
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust and Cargo first."
        exit 1
    fi
    
    if ! cargo near --version &> /dev/null; then
        print_error "cargo-near is not installed. Run: cargo install cargo-near"
        exit 1
    fi
    
    print_status "Prerequisites check passed"
}

# Build all contracts
build_contracts() {
    echo "🔨 Building all contracts..."
    
    # Build router contract
    print_status "Building router contract..."
    cargo near build --package cosmos-sdk-contract --features "router"
    
    # Build individual module contracts
    for module in "client" "connection" "channel" "transfer" "bank" "wasm" "staking"; do
        print_status "Building ${module} contract..."
        cargo near build --package cosmos-sdk-contract --features "${module}"
    done
    
    print_status "All contracts built successfully"
}

# Deploy contracts
deploy_contracts() {
    echo "🚢 Deploying contracts to ${NETWORK}..."
    
    local router_account="${ACCOUNT_PREFIX}-router.${NETWORK}"
    local client_account="${ACCOUNT_PREFIX}-client.${NETWORK}"
    local connection_account="${ACCOUNT_PREFIX}-connection.${NETWORK}"
    local channel_account="${ACCOUNT_PREFIX}-channel.${NETWORK}"
    local transfer_account="${ACCOUNT_PREFIX}-transfer.${NETWORK}"
    local bank_account="${ACCOUNT_PREFIX}-bank.${NETWORK}"
    local wasm_account="${ACCOUNT_PREFIX}-wasm.${NETWORK}"
    local staking_account="${ACCOUNT_PREFIX}-staking.${NETWORK}"
    
    # Deploy router contract
    print_status "Deploying router contract to ${router_account}..."
    near deploy ${router_account} target/near/cosmos_sdk_near.wasm
    
    # Initialize router
    near call ${router_account} new '{"owner": "'${router_account}'"}' --accountId ${router_account}
    
    # Deploy module contracts
    print_status "Deploying IBC client module to ${client_account}..."
    near deploy ${client_account} target/near/cosmos_sdk_near.wasm
    near call ${client_account} new '{"owner": "'${client_account}'", "router_contract": "'${router_account}'"}' --accountId ${client_account}
    
    print_status "Deploying IBC connection module to ${connection_account}..."
    near deploy ${connection_account} target/near/cosmos_sdk_near.wasm
    near call ${connection_account} new '{"owner": "'${connection_account}'", "router_contract": "'${router_account}'"}' --accountId ${connection_account}
    
    print_status "Deploying IBC channel module to ${channel_account}..."
    near deploy ${channel_account} target/near/cosmos_sdk_near.wasm
    near call ${channel_account} new '{"owner": "'${channel_account}'", "router_contract": "'${router_account}'"}' --accountId ${channel_account}
    
    print_status "Deploying IBC transfer module to ${transfer_account}..."
    near deploy ${transfer_account} target/near/cosmos_sdk_near.wasm
    near call ${transfer_account} new '{"owner": "'${transfer_account}'", "router_contract": "'${router_account}'"}' --accountId ${transfer_account}
    
    print_status "Deploying bank module to ${bank_account}..."
    near deploy ${bank_account} target/near/cosmos_sdk_near.wasm
    near call ${bank_account} new '{"owner": "'${bank_account}'", "router_contract": "'${router_account}'"}' --accountId ${bank_account}
    
    print_status "Deploying wasm module to ${wasm_account}..."
    near deploy ${wasm_account} target/near/cosmos_sdk_near.wasm
    near call ${wasm_account} new '{"owner": "'${wasm_account}'", "router_contract": "'${router_account}'"}' --accountId ${wasm_account}
    
    print_status "Deploying staking module to ${staking_account}..."
    near deploy ${staking_account} target/near/cosmos_sdk_near.wasm
    near call ${staking_account} new '{"owner": "'${staking_account}'", "router_contract": "'${router_account}'"}' --accountId ${staking_account}
    
    print_status "All contracts deployed successfully"
}

# Configure router with module addresses
configure_router() {
    echo "⚙️ Configuring router with module addresses..."
    
    local router_account="${ACCOUNT_PREFIX}-router.${NETWORK}"
    local client_account="${ACCOUNT_PREFIX}-client.${NETWORK}"
    local connection_account="${ACCOUNT_PREFIX}-connection.${NETWORK}"
    local channel_account="${ACCOUNT_PREFIX}-channel.${NETWORK}"
    local transfer_account="${ACCOUNT_PREFIX}-transfer.${NETWORK}"
    local bank_account="${ACCOUNT_PREFIX}-bank.${NETWORK}"
    local wasm_account="${ACCOUNT_PREFIX}-wasm.${NETWORK}"
    local staking_account="${ACCOUNT_PREFIX}-staking.${NETWORK}"
    
    # Register all modules with the router
    print_status "Registering IBC client module..."
    near call ${router_account} register_module '{"module_type": "ibc_client", "contract_id": "'${client_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering IBC connection module..."
    near call ${router_account} register_module '{"module_type": "ibc_connection", "contract_id": "'${connection_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering IBC channel module..."
    near call ${router_account} register_module '{"module_type": "ibc_channel", "contract_id": "'${channel_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering IBC transfer module..."
    near call ${router_account} register_module '{"module_type": "ibc_transfer", "contract_id": "'${transfer_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering bank module..."
    near call ${router_account} register_module '{"module_type": "bank", "contract_id": "'${bank_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering wasm module..."
    near call ${router_account} register_module '{"module_type": "wasm", "contract_id": "'${wasm_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Registering staking module..."
    near call ${router_account} register_module '{"module_type": "staking", "contract_id": "'${staking_account}'", "version": "1.0.0"}' --accountId ${router_account}
    
    print_status "Router configuration completed"
}

# Verify deployment
verify_deployment() {
    echo "🔍 Verifying deployment..."
    
    local router_account="${ACCOUNT_PREFIX}-router.${NETWORK}"
    
    # Check router health
    print_status "Checking router health..."
    near view ${router_account} health_check
    
    # Check registered modules
    print_status "Checking registered modules..."
    near view ${router_account} get_modules
    
    # Test a simple cross-module operation
    print_status "Testing module discovery..."
    near view ${router_account} get_metadata
    
    print_status "Deployment verification completed"
}

# Main deployment flow
main() {
    echo "🌟 Modular Cosmos SDK Deployment for NEAR Protocol"
    echo "Network: ${NETWORK}"
    echo "Account prefix: ${ACCOUNT_PREFIX}"
    echo ""
    
    check_prerequisites
    build_contracts
    deploy_contracts
    configure_router
    verify_deployment
    
    echo ""
    echo "🎉 Modular Cosmos SDK deployment completed successfully!"
    echo ""
    echo "📝 Deployment Summary:"
    echo "   Router: ${ACCOUNT_PREFIX}-router.${NETWORK}"
    echo "   IBC Client: ${ACCOUNT_PREFIX}-client.${NETWORK}"
    echo "   IBC Connection: ${ACCOUNT_PREFIX}-connection.${NETWORK}"
    echo "   IBC Channel: ${ACCOUNT_PREFIX}-channel.${NETWORK}"
    echo "   IBC Transfer: ${ACCOUNT_PREFIX}-transfer.${NETWORK}"
    echo "   Bank Module: ${ACCOUNT_PREFIX}-bank.${NETWORK}"
    echo "   WASM Module: ${ACCOUNT_PREFIX}-wasm.${NETWORK}"
    echo "   Staking Module: ${ACCOUNT_PREFIX}-staking.${NETWORK}"
    echo ""
    echo "🔗 Update your relayer configuration to use:"
    echo "   router_contract: ${ACCOUNT_PREFIX}-router.${NETWORK}"
    echo ""
}

# Handle command line arguments
case "${1:-}" in
    --network=*)
        NETWORK="${1#*=}"
        ;;
    --prefix=*)
        ACCOUNT_PREFIX="${1#*=}"
        ;;
    --help)
        echo "Usage: $0 [--network=testnet|mainnet] [--prefix=account-prefix]"
        echo ""
        echo "Options:"
        echo "  --network=NETWORK    Target network (testnet or mainnet, default: testnet)"
        echo "  --prefix=PREFIX      Account prefix for contracts (default: cosmos-sdk)"
        echo "  --help              Show this help message"
        exit 0
        ;;
esac

# Run main deployment
main