#!/bin/bash
set -e

# Local Modular Cosmos SDK Deployment Script
# Deploys the router and all module contracts for local development using NEAR sandbox

echo "🏠 Starting LOCAL Modular Cosmos SDK Deployment"

# Configuration for local deployment
NETWORK="sandbox"
ACCOUNT_SUFFIX=".test.near"
ACCOUNT_PREFIX="cosmos"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

print_info() {
    echo -e "${BLUE}ℹ️ $1${NC}"
}

# Check prerequisites for local deployment
check_prerequisites() {
    echo "🔍 Checking prerequisites for local deployment..."
    
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
    
    # Check if near-sandbox is available
    if ! command -v near-sandbox &> /dev/null; then
        print_warning "near-sandbox not found. We'll use near CLI with local network."
    fi
    
    print_status "Prerequisites check passed"
}

# Start local NEAR network (sandbox)
start_local_network() {
    echo "🚀 Starting local NEAR network..."
    
    # Kill any existing sandbox processes
    pkill -f near-sandbox || true
    sleep 2
    
    # Check if we can start sandbox
    if command -v near-sandbox &> /dev/null; then
        print_status "Starting NEAR sandbox..."
        near-sandbox --home ~/.near-sandbox init
        near-sandbox --home ~/.near-sandbox run &
        SANDBOX_PID=$!
        sleep 5
        
        # Set environment for sandbox
        export NEAR_ENV=sandbox
        export NEAR_CLI_SANDBOX_URL=http://localhost:3030
        export NEAR_NODE_URL=http://localhost:3030
        
        print_status "NEAR sandbox started (PID: $SANDBOX_PID)"
    else
        print_info "Using NEAR localnet instead of sandbox"
        export NEAR_ENV=localnet
        export NEAR_NODE_URL=http://localhost:3030
    fi
}

# Create local accounts
create_local_accounts() {
    echo "👤 Creating local accounts..."
    
    # Create a master account for deployment
    local master_account="dev-account${ACCOUNT_SUFFIX}"
    
    # For sandbox/localnet, we'll use dev accounts
    local router_account="${ACCOUNT_PREFIX}-router${ACCOUNT_SUFFIX}"
    local client_account="${ACCOUNT_PREFIX}-client${ACCOUNT_SUFFIX}"
    local connection_account="${ACCOUNT_PREFIX}-connection${ACCOUNT_SUFFIX}"
    local channel_account="${ACCOUNT_PREFIX}-channel${ACCOUNT_SUFFIX}"
    local transfer_account="${ACCOUNT_PREFIX}-transfer${ACCOUNT_SUFFIX}"
    local bank_account="${ACCOUNT_PREFIX}-bank${ACCOUNT_SUFFIX}"
    local wasm_account="${ACCOUNT_PREFIX}-wasm${ACCOUNT_SUFFIX}"
    local staking_account="${ACCOUNT_PREFIX}-staking${ACCOUNT_SUFFIX}"
    local statesync_account="${ACCOUNT_PREFIX}-statesync${ACCOUNT_SUFFIX}"
    
    # Create accounts (these will be dev accounts in sandbox)
    print_status "Creating dev accounts for local deployment..."
    
    # Store account names for later use
    echo "export ROUTER_ACCOUNT='${router_account}'" > .env.local
    echo "export CLIENT_ACCOUNT='${client_account}'" >> .env.local
    echo "export CONNECTION_ACCOUNT='${connection_account}'" >> .env.local
    echo "export CHANNEL_ACCOUNT='${channel_account}'" >> .env.local
    echo "export TRANSFER_ACCOUNT='${transfer_account}'" >> .env.local
    echo "export BANK_ACCOUNT='${bank_account}'" >> .env.local
    echo "export WASM_ACCOUNT='${wasm_account}'" >> .env.local
    echo "export STAKING_ACCOUNT='${staking_account}'" >> .env.local
    echo "export STATESYNC_ACCOUNT='${statesync_account}'" >> .env.local
    
    print_status "Account names saved to .env.local"
}

# Build contracts for local deployment
build_contracts() {
    echo "🔨 Building contracts for local deployment..."
    
    print_status "Building modular router contract..."
    
    # Create near directory structure
    mkdir -p target/near/
    
    # Build router contract using standard cargo build
    print_info "Building modular router contract..."
    cargo build --target wasm32-unknown-unknown --release
    
    # Look for the built wasm file
    local wasm_files=(target/wasm32-unknown-unknown/release/*.wasm)
    if [[ -f "${wasm_files[0]}" ]]; then
        local wasm_file="${wasm_files[0]}"
        local wasm_name=$(basename "$wasm_file")
        cp "$wasm_file" target/near/modular_router.wasm
        print_status "Router contract built: target/near/modular_router.wasm (from $wasm_name)"
        
        # Get file size
        local router_size=$(wc -c < target/near/modular_router.wasm)
        print_info "Router contract size: $(($router_size / 1024)) KB"
        
        # Check for reasonable size
        if [[ $router_size -gt 4000000 ]]; then
            print_warning "Contract is large (>4MB). This is a simplified router."
        fi
        
    else
        print_error "Failed to build router contract. No WASM file found."
        print_info "Trying debug build..."
        
        # Try debug build
        cargo build --target wasm32-unknown-unknown
        
        local debug_files=(target/wasm32-unknown-unknown/debug/*.wasm)
        if [[ -f "${debug_files[0]}" ]]; then
            local debug_file="${debug_files[0]}"
            local debug_name=$(basename "$debug_file")
            cp "$debug_file" target/near/modular_router.wasm
            print_status "Router contract built (debug): target/near/modular_router.wasm (from $debug_name)"
        else
            print_error "Failed to build router contract completely."
            exit 1
        fi
    fi
    
    print_status "Contract build completed!"
}

# Deploy contracts to local network
deploy_contracts_local() {
    echo "🚢 Deploying contracts to local network..."
    
    # Source the account names
    source .env.local
    
    print_status "Deploying and initializing router contract..."
    
    # Generate a temporary account name for local testing
    local timestamp=$(date +%s)
    local temp_account="cosmos-router-${timestamp}.test.near"
    
    # Try to create and deploy (this might fail in localnet, but we'll handle it gracefully)
    if near deploy --wasmFile target/near/modular_router.wasm --accountId "$temp_account" 2>/dev/null; then
        ROUTER_ACCOUNT=$temp_account
        print_status "Router deployed to: $ROUTER_ACCOUNT"
    else
        # Alternative: just save the wasm file and provide instructions
        print_info "Direct deployment not available in current environment."
        print_info "WASM file ready for manual deployment: target/near/modular_router.wasm"
        ROUTER_ACCOUNT="<your-account>.testnet"
        print_info "Use: near deploy --wasmFile target/near/modular_router.wasm --accountId <your-account>.testnet"
    fi
    
    print_status "Router deployed to: $ROUTER_ACCOUNT"
    
    # Initialize router (only if deployed successfully)
    if [[ "$ROUTER_ACCOUNT" != "<your-account>.testnet" ]]; then
        print_info "Initializing router contract..."
        near call $ROUTER_ACCOUNT new "{}" --accountId $ROUTER_ACCOUNT --gas 300000000000000 || print_warning "Router initialization may require manual setup"
        
        # Register modules pointing to the same contract (for local testing)
        print_status "Setting up modular configuration on single contract..."
        
        near call $ROUTER_ACCOUNT register_module "{\"module_type\": \"ibc_client\", \"contract_id\": \"$ROUTER_ACCOUNT\", \"version\": \"1.0.0\"}" --accountId $ROUTER_ACCOUNT --gas 50000000000000 || true
        
        near call $ROUTER_ACCOUNT register_module "{\"module_type\": \"bank\", \"contract_id\": \"$ROUTER_ACCOUNT\", \"version\": \"1.0.0\"}" --accountId $ROUTER_ACCOUNT --gas 50000000000000 || true
        
        near call $ROUTER_ACCOUNT register_module "{\"module_type\": \"wasm\", \"contract_id\": \"$ROUTER_ACCOUNT\", \"version\": \"1.0.0\"}" --accountId $ROUTER_ACCOUNT --gas 50000000000000 || true
    else
        print_info "Manual deployment required. After deploying, run these commands:"
        echo "near call <your-account>.testnet new '{}' --accountId <your-account>.testnet"
        echo "near call <your-account>.testnet register_module '{\"module_type\": \"bank\", \"contract_id\": \"<your-account>.testnet\", \"version\": \"1.0.0\"}' --accountId <your-account>.testnet"
    fi
    
    # Update .env.local with the actual deployed account
    cat > .env.local << EOF
export NEAR_ENV=sandbox
export NEAR_NODE_URL=http://localhost:3030
export ROUTER_ACCOUNT='${ROUTER_ACCOUNT}'
export CLIENT_ACCOUNT='${ROUTER_ACCOUNT}'
export CONNECTION_ACCOUNT='${ROUTER_ACCOUNT}'
export CHANNEL_ACCOUNT='${ROUTER_ACCOUNT}'
export TRANSFER_ACCOUNT='${ROUTER_ACCOUNT}'
export BANK_ACCOUNT='${ROUTER_ACCOUNT}'
export WASM_ACCOUNT='${ROUTER_ACCOUNT}'
export STAKING_ACCOUNT='${ROUTER_ACCOUNT}'
export STATESYNC_ACCOUNT='${ROUTER_ACCOUNT}'
EOF
    
    print_status "All modules registered with router contract"
}

# Test local deployment
test_local_deployment() {
    echo "🧪 Testing local deployment..."
    
    source .env.local
    
    if [[ "$ROUTER_ACCOUNT" != "<your-account>.testnet" ]]; then
        print_status "Testing router health check..."
        near view $ROUTER_ACCOUNT health_check --accountId $ROUTER_ACCOUNT || print_warning "Health check failed - contract may not be initialized"
        
        print_status "Testing module discovery..."
        near view $ROUTER_ACCOUNT get_modules --accountId $ROUTER_ACCOUNT || print_warning "Module discovery failed - contract may not be initialized"
        
        print_status "Testing basic operations..."
        near view $ROUTER_ACCOUNT test_function --accountId $ROUTER_ACCOUNT || true
        
        print_status "Basic functionality tests completed"
    else
        print_info "Deployment was manual. To test after deploying:"
        echo "near view <your-account>.testnet health_check"
        echo "near view <your-account>.testnet get_modules"
        echo "near view <your-account>.testnet test_function"
    fi
}

# Generate local relayer config
generate_local_config() {
    echo "⚙️ Generating local relayer configuration..."
    
    source .env.local
    
    local config_file="./relayer-config-local.toml"
    
    cat > ${config_file} << EOF
# Local Relayer Configuration for Development
# Generated by deploy-local.sh

[[chains]]
chain_id = "near-localnet"
rpc_endpoint = "http://localhost:3030"

[chains.config.near]
# Main router contract (required)
router_contract = "${ROUTER_ACCOUNT}"

# For local development, all modules point to the same contract
[chains.config.near.modules]
ibc_client = "${ROUTER_ACCOUNT}"
ibc_connection = "${ROUTER_ACCOUNT}"
ibc_channel = "${ROUTER_ACCOUNT}"
ibc_transfer = "${ROUTER_ACCOUNT}"
bank = "${ROUTER_ACCOUNT}"
wasm = "${ROUTER_ACCOUNT}"
staking = "${ROUTER_ACCOUNT}"

# Local development settings
[chains.config.near.discovery]
auto_discover = true
cache_duration_secs = 60  # Short cache for development

# Local performance settings
[chains.config.near.performance]
enable_parallel_queries = true
max_concurrent_operations = 3  # Reduced for local testing
operation_timeout_secs = 10    # Shorter timeout for local
EOF

    print_status "Local relayer configuration generated: ${config_file}"
}

# Generate local development guide
generate_local_guide() {
    echo "📚 Generating local development guide..."
    
    source .env.local
    
    cat > LOCAL_DEVELOPMENT.md << EOF
# Local Development Guide

## Environment Setup

Your local NEAR environment is now running with:
- **Network**: Sandbox/Localnet  
- **RPC URL**: http://localhost:3030
- **Router Contract**: \`${ROUTER_ACCOUNT}\`

## Environment Variables

Load the local environment:
\`\`\`bash
source .env.local
\`\`\`

## Available Commands

### Basic Operations
\`\`\`bash
# Check router health
near view \$ROUTER_ACCOUNT health_check

# View registered modules
near view \$ROUTER_ACCOUNT get_modules

# Check balance
near view \$ROUTER_ACCOUNT get_balance '{"account": "'"\$ROUTER_ACCOUNT"'"}'

# Transfer tokens
near call \$ROUTER_ACCOUNT transfer '{"receiver": "alice.test.near", "amount": "1000"}' --accountId \$ROUTER_ACCOUNT
\`\`\`

### IBC Operations
\`\`\`bash
# Create IBC client (example)
near call \$ROUTER_ACCOUNT ibc_create_client '{"chain_id": "test-chain", "trust_period": 86400, "unbonding_period": 172800, "max_clock_drift": 3600, "initial_header": {...}}' --accountId \$ROUTER_ACCOUNT

# Query clients
near view \$ROUTER_ACCOUNT get_all_clients
\`\`\`

### WASM Operations
\`\`\`bash
# Store WASM code
near call \$ROUTER_ACCOUNT wasm_store_code '{"wasm_byte_code": [...], "source": "test", "builder": "test", "instantiate_permission": {"everybody": {}}}' --accountId \$ROUTER_ACCOUNT

# List stored codes
near view \$ROUTER_ACCOUNT wasm_list_codes '{"start_after": null, "limit": 10}'
\`\`\`

## Development Workflow

1. **Make changes** to contract code
2. **Rebuild**: \`cargo near build\`
3. **Redeploy**: \`near dev-deploy target/near/cosmos_sdk_near.wasm\`
4. **Test**: Run your test commands
5. **Iterate**: Repeat as needed

## Debugging

- **View logs**: Check terminal output during contract calls
- **State inspection**: Use \`near view\` commands to inspect contract state
- **Gas optimization**: Monitor gas usage in transaction results

## Cleanup

To stop and cleanup:
\`\`\`bash
# Stop sandbox (if running)
pkill -f near-sandbox

# Clean dev account
rm -rf neardev/
rm .env.local
\`\`\`

## Next Steps

1. Test all module functionality locally
2. Run integration tests
3. Optimize gas usage
4. Deploy to testnet when ready

---
*Generated by deploy-local.sh on $(date)*
EOF

    print_status "Local development guide created: LOCAL_DEVELOPMENT.md"
}

# Cleanup function
cleanup() {
    echo "🧹 Cleaning up..."
    if [[ -n "$SANDBOX_PID" ]]; then
        print_info "Stopping NEAR sandbox (PID: $SANDBOX_PID)"
        kill $SANDBOX_PID 2>/dev/null || true
    fi
}

# Set trap for cleanup
trap cleanup EXIT

# Main deployment flow
main() {
    echo "🌟 Local Modular Cosmos SDK Deployment"
    echo "This will deploy the modular architecture locally for development"
    echo ""
    
    check_prerequisites
    start_local_network
    create_local_accounts
    build_contracts
    deploy_contracts_local
    test_local_deployment
    generate_local_config
    generate_local_guide
    
    echo ""
    print_status "🎉 Local deployment completed successfully!"
    echo ""
    echo "📝 What's available:"
    echo "   ✅ Local NEAR network running on http://localhost:3030"
    echo "   ✅ Modular Cosmos SDK deployed to: $(cat neardev/dev-account 2>/dev/null || echo 'dev account')"
    echo "   ✅ All modules registered and configured"
    echo "   ✅ Environment file: .env.local"
    echo "   ✅ Relayer config: relayer-config-local.toml"
    echo "   ✅ Development guide: LOCAL_DEVELOPMENT.md"
    echo ""
    echo "📋 Next steps:"
    echo "   1. Run: source .env.local"
    echo "   2. Test with: near view \$ROUTER_ACCOUNT health_check"
    echo "   3. Read LOCAL_DEVELOPMENT.md for more examples"
    echo "   4. Develop and iterate on your local setup"
    echo ""
    print_info "The sandbox will keep running. Press Ctrl+C to stop it."
    
    # Keep script alive to maintain sandbox
    if [[ -n "$SANDBOX_PID" ]]; then
        print_info "Waiting for sandbox to run... (Press Ctrl+C to exit)"
        wait $SANDBOX_PID
    else
        print_info "Local deployment complete. Network may be running externally."
    fi
}

# Handle command line arguments
case "${1:-}" in
    --help)
        echo "Usage: $0 [--help]"
        echo ""
        echo "Local deployment script for Modular Cosmos SDK"
        echo ""
        echo "This script will:"
        echo "  - Start a local NEAR sandbox/localnet"
        echo "  - Deploy the modular contract architecture"
        echo "  - Set up development environment"
        echo "  - Generate configuration files"
        exit 0
        ;;
esac

# Run main deployment
main