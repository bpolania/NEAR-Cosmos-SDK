#!/bin/bash

# Deploy to local NEAR sandbox
set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print functions
print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️ $1${NC}"
}

# Check if sandbox is running
check_sandbox() {
    if ! ps aux | grep -q "[n]ear-sandbox.*run"; then
        print_error "near-sandbox is not running. Starting it..."
        near-sandbox --home ~/.near-sandbox run > ~/.near-sandbox/sandbox.log 2>&1 &
        print_info "Waiting for sandbox to start..."
        sleep 5
    else
        print_status "near-sandbox is already running"
    fi
}

# Create test account
create_test_account() {
    local account_name=$1
    print_info "Creating account: $account_name"
    
    # Create account with initial balance
    NEAR_ENV=localnet near create-account $account_name --useAccount test.near --initialBalance 100 || {
        print_warning "Account $account_name might already exist"
    }
}

# Deploy contract
deploy_contract() {
    local wasm_file=$1
    local account_id=$2
    
    print_info "Deploying contract to $account_id..."
    
    # Deploy the contract
    NEAR_ENV=localnet near deploy $account_id $wasm_file --force
}

# Initialize contract
init_contract() {
    local account_id=$1
    
    print_info "Initializing contract on $account_id..."
    
    NEAR_ENV=localnet near call $account_id new '{}' --accountId $account_id
}

# Register modules
register_modules() {
    local router_account=$1
    
    print_info "Registering modules..."
    
    # Register bank module
    NEAR_ENV=localnet near call $router_account register_module \
        '{"module_type": "bank", "contract_id": "'$router_account'", "version": "1.0.0"}' \
        --accountId $router_account
    
    # Register IBC modules
    NEAR_ENV=localnet near call $router_account register_module \
        '{"module_type": "ibc_client", "contract_id": "'$router_account'", "version": "1.0.0"}' \
        --accountId $router_account
        
    NEAR_ENV=localnet near call $router_account register_module \
        '{"module_type": "ibc_transfer", "contract_id": "'$router_account'", "version": "1.0.0"}' \
        --accountId $router_account
}

# Test deployment
test_deployment() {
    local account_id=$1
    
    print_info "Testing deployment..."
    
    # Test health check
    NEAR_ENV=localnet near view $account_id health_check
    
    # Test module listing
    NEAR_ENV=localnet near view $account_id get_modules
    
    # Test basic function
    NEAR_ENV=localnet near view $account_id test_function
}

# Main deployment flow
main() {
    echo "🚀 Deploying Modular Cosmos SDK to Local Sandbox"
    echo "================================================"
    
    # Set localnet environment
    export NEAR_ENV=localnet
    export NODE_ENV=localnet
    export NEAR_NODE_URL=http://localhost:3030
    
    # Check if sandbox is running
    check_sandbox
    
    # Build the contract first
    print_status "Building contract..."
    cargo build --target wasm32-unknown-unknown --release
    
    # Copy the wasm file
    mkdir -p target/near
    cp target/wasm32-unknown-unknown/release/cosmos_sdk_contract.wasm target/near/modular_router.wasm
    
    # Create account names
    ROUTER_ACCOUNT="cosmos-router.test.near"
    
    # Create accounts
    create_test_account $ROUTER_ACCOUNT
    
    # Deploy contracts
    deploy_contract target/near/modular_router.wasm $ROUTER_ACCOUNT
    
    # Initialize contracts
    init_contract $ROUTER_ACCOUNT
    
    # Register modules
    register_modules $ROUTER_ACCOUNT
    
    # Test deployment
    test_deployment $ROUTER_ACCOUNT
    
    print_status "🎉 Deployment completed successfully!"
    echo ""
    echo "📝 Deployed contracts:"
    echo "   Router: $ROUTER_ACCOUNT"
    echo ""
    echo "🔧 Next steps:"
    echo "   - View health: NEAR_ENV=localnet near view $ROUTER_ACCOUNT health_check"
    echo "   - View modules: NEAR_ENV=localnet near view $ROUTER_ACCOUNT get_modules"
    echo "   - Test function: NEAR_ENV=localnet near view $ROUTER_ACCOUNT test_function"
}

# Run main function
main "$@"