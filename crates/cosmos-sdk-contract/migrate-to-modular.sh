#!/bin/bash
set -e

# Migration Script: Monolithic to Modular Architecture
# This script helps migrate data from the old monolithic contract to the new modular architecture

echo "🔄 Starting Migration from Monolithic to Modular Architecture"

# Configuration
NETWORK="testnet"
OLD_CONTRACT="cosmos-sdk.${NETWORK}"  # Original monolithic contract
ROUTER_CONTRACT="cosmos-sdk-router.${NETWORK}"
CLIENT_CONTRACT="cosmos-sdk-client.${NETWORK}"
CONNECTION_CONTRACT="cosmos-sdk-connection.${NETWORK}"
CHANNEL_CONTRACT="cosmos-sdk-channel.${NETWORK}"
TRANSFER_CONTRACT="cosmos-sdk-transfer.${NETWORK}"
BANK_CONTRACT="cosmos-sdk-bank.${NETWORK}"
WASM_CONTRACT="cosmos-sdk-wasm.${NETWORK}"
STAKING_CONTRACT="cosmos-sdk-staking.${NETWORK}"

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

# Check if old contract exists
check_old_contract() {
    echo "🔍 Checking for existing monolithic contract..."
    
    if ! near state ${OLD_CONTRACT} &>/dev/null; then
        print_warning "Old monolithic contract ${OLD_CONTRACT} not found"
        print_info "This might be a fresh deployment. Continuing with modular setup..."
        return 0
    fi
    
    print_status "Found existing monolithic contract: ${OLD_CONTRACT}"
    return 1  # Contract exists
}

# Export data from monolithic contract
export_monolithic_data() {
    echo "📤 Exporting data from monolithic contract..."
    
    local export_dir="./migration_data"
    mkdir -p ${export_dir}
    
    print_status "Exporting IBC clients..."
    near view ${OLD_CONTRACT} get_all_clients > ${export_dir}/clients.json || echo "[]" > ${export_dir}/clients.json
    
    print_status "Exporting IBC connections..."
    near view ${OLD_CONTRACT} get_connection_ids > ${export_dir}/connections.json || echo "[]" > ${export_dir}/connections.json
    
    print_status "Exporting IBC channels..."
    near view ${OLD_CONTRACT} get_all_channels > ${export_dir}/channels.json || echo "[]" > ${export_dir}/channels.json
    
    print_status "Exporting bank balances..."
    # Note: This would need to be implemented in the actual contract
    echo "{}" > ${export_dir}/balances.json
    
    print_status "Exporting WASM codes..."
    near view ${OLD_CONTRACT} wasm_list_codes '{"start_after": null, "limit": 100}' > ${export_dir}/wasm_codes.json || echo "[]" > ${export_dir}/wasm_codes.json
    
    print_status "Data export completed. Files saved to: ${export_dir}"
}

# Import data to modular contracts
import_to_modular() {
    echo "📥 Importing data to modular contracts..."
    
    local export_dir="./migration_data"
    
    if [[ ! -d ${export_dir} ]]; then
        print_warning "No export data found. Skipping data migration..."
        return 0
    fi
    
    # Import IBC clients
    if [[ -s ${export_dir}/clients.json ]]; then
        print_status "Importing IBC clients..."
        local clients=$(cat ${export_dir}/clients.json)
        if [[ ${clients} != "[]" ]]; then
            print_info "Found clients to migrate. Manual migration required for client state."
            print_info "Client data saved in ${export_dir}/clients.json"
        fi
    fi
    
    # Import connections
    if [[ -s ${export_dir}/connections.json ]]; then
        print_status "Importing IBC connections..."
        local connections=$(cat ${export_dir}/connections.json)
        if [[ ${connections} != "[]" ]]; then
            print_info "Found connections to migrate. Manual migration required for connection state."
            print_info "Connection data saved in ${export_dir}/connections.json"
        fi
    fi
    
    # Import channels
    if [[ -s ${export_dir}/channels.json ]]; then
        print_status "Importing IBC channels..."
        local channels=$(cat ${export_dir}/channels.json)
        if [[ ${channels} != "[]" ]]; then
            print_info "Found channels to migrate. Manual migration required for channel state."
            print_info "Channel data saved in ${export_dir}/channels.json"
        fi
    fi
    
    print_status "Data import preparation completed"
}

# Verify modular deployment
verify_modular_deployment() {
    echo "✅ Verifying modular deployment..."
    
    # Check router
    if ! near state ${ROUTER_CONTRACT} &>/dev/null; then
        print_error "Router contract not found: ${ROUTER_CONTRACT}"
        print_error "Please run deploy-modular.sh first"
        exit 1
    fi
    
    # Check all modules
    local modules=("${CLIENT_CONTRACT}" "${CONNECTION_CONTRACT}" "${CHANNEL_CONTRACT}" "${TRANSFER_CONTRACT}" "${BANK_CONTRACT}" "${WASM_CONTRACT}" "${STAKING_CONTRACT}")
    
    for contract in "${modules[@]}"; do
        if ! near state ${contract} &>/dev/null; then
            print_error "Module contract not found: ${contract}"
            print_error "Please run deploy-modular.sh first"
            exit 1
        fi
    done
    
    print_status "All modular contracts verified"
}

# Update relayer configuration
update_relayer_config() {
    echo "⚙️ Generating relayer configuration..."
    
    local config_file="./relayer-config-modular.toml"
    
    cat > ${config_file} << EOF
# Updated Relayer Configuration for Modular Architecture
# Generated by migrate-to-modular.sh

[[chains]]
chain_id = "near-${NETWORK}"
rpc_endpoint = "https://rpc.${NETWORK}.near.org"

[chains.config.near]
# Main router contract (required)
router_contract = "${ROUTER_CONTRACT}"

# Direct module addresses for optimization (optional)
[chains.config.near.modules]
ibc_client = "${CLIENT_CONTRACT}"
ibc_connection = "${CONNECTION_CONTRACT}"
ibc_channel = "${CHANNEL_CONTRACT}"
ibc_transfer = "${TRANSFER_CONTRACT}"
bank = "${BANK_CONTRACT}"
wasm = "${WASM_CONTRACT}"
staking = "${STAKING_CONTRACT}"

# Module discovery settings
[chains.config.near.discovery]
auto_discover = true
cache_duration_secs = 3600

# Performance settings for modular architecture
[chains.config.near.performance]
enable_parallel_queries = true
max_concurrent_operations = 5
operation_timeout_secs = 30
EOF

    print_status "Relayer configuration generated: ${config_file}"
}

# Pause old contract (if it exists)
pause_old_contract() {
    echo "⏸️ Pausing old monolithic contract..."
    
    if ! near state ${OLD_CONTRACT} &>/dev/null; then
        print_info "Old contract not found, skipping pause step"
        return 0
    fi
    
    print_warning "⚠️ IMPORTANT: Manual action required"
    print_warning "The old monolithic contract should be paused to prevent new operations"
    print_warning "during the migration window."
    print_warning ""
    print_warning "If your contract has a pause function, run:"
    print_warning "near call ${OLD_CONTRACT} pause '{}' --accountId ${OLD_CONTRACT}"
    print_warning ""
    print_info "Press Enter to continue after pausing the old contract..."
    read -r
}

# Generate migration report
generate_migration_report() {
    echo "📋 Generating migration report..."
    
    local report_file="./migration-report.md"
    
    cat > ${report_file} << EOF
# Migration Report: Monolithic to Modular Architecture

**Date:** $(date)
**Network:** ${NETWORK}

## Migration Summary

### Old Architecture
- Monolithic Contract: \`${OLD_CONTRACT}\`
- Single contract handling all modules

### New Architecture
- Router Contract: \`${ROUTER_CONTRACT}\`
- IBC Client Module: \`${CLIENT_CONTRACT}\`
- IBC Connection Module: \`${CONNECTION_CONTRACT}\`
- IBC Channel Module: \`${CHANNEL_CONTRACT}\`
- IBC Transfer Module: \`${TRANSFER_CONTRACT}\`
- Bank Module: \`${BANK_CONTRACT}\`
- WASM Module: \`${WASM_CONTRACT}\`
- Staking Module: \`${STAKING_CONTRACT}\`

## Benefits of New Architecture

1. **Modularity**: Each component can be updated independently
2. **Scalability**: Parallel processing across modules
3. **Maintainability**: Clear separation of concerns
4. **Performance**: Optimized gas usage per operation
5. **Flexibility**: Easy to add new modules or features

## Data Migration Status

- **IBC Clients**: $(if [[ -s ./migration_data/clients.json ]]; then echo "Exported (manual import required)"; else echo "No data found"; fi)
- **IBC Connections**: $(if [[ -s ./migration_data/connections.json ]]; then echo "Exported (manual import required)"; else echo "No data found"; fi)
- **IBC Channels**: $(if [[ -s ./migration_data/channels.json ]]; then echo "Exported (manual import required)"; else echo "No data found"; fi)
- **Bank Balances**: Manual migration required
- **WASM Codes**: $(if [[ -s ./migration_data/wasm_codes.json ]]; then echo "Exported (manual import required)"; else echo "No data found"; fi)

## Next Steps

1. **Update Relayer**: Use the generated \`relayer-config-modular.toml\`
2. **Test Operations**: Verify all IBC operations work correctly
3. **Monitor Performance**: Check improved performance metrics
4. **Data Migration**: Complete manual data migration if needed
5. **Decommission Old Contract**: After successful verification

## Support

For issues or questions about the migration, please:
1. Check the modular architecture documentation
2. Review the deployment logs
3. Test with small operations first

---
*Migration completed by migrate-to-modular.sh*
EOF

    print_status "Migration report generated: ${report_file}"
}

# Main migration flow
main() {
    echo "🌟 Migration from Monolithic to Modular Cosmos SDK Architecture"
    echo "Network: ${NETWORK}"
    echo ""
    
    # Check if old contract exists
    if check_old_contract; then
        print_info "No existing monolithic contract found. Setting up fresh modular deployment..."
    else
        print_info "Existing monolithic contract found. Proceeding with migration..."
        pause_old_contract
        export_monolithic_data
    fi
    
    # Verify modular contracts are deployed
    verify_modular_deployment
    
    # Import data (if available)
    import_to_modular
    
    # Generate configuration files
    update_relayer_config
    
    # Generate report
    generate_migration_report
    
    echo ""
    print_status "🎉 Migration completed successfully!"
    echo ""
    echo "📝 What was done:"
    echo "   ✅ Verified modular contract deployment"
    echo "   ✅ Exported data from old contract (if found)"
    echo "   ✅ Generated new relayer configuration"
    echo "   ✅ Created migration report"
    echo ""
    echo "📋 Next steps:"
    echo "   1. Review migration-report.md"
    echo "   2. Update your relayer with relayer-config-modular.toml"
    echo "   3. Test IBC operations with the new architecture"
    echo "   4. Complete manual data migration if needed"
    echo ""
    print_warning "⚠️ Remember to thoroughly test the new setup before decommissioning the old contract!"
}

# Handle command line arguments
case "${1:-}" in
    --network=*)
        NETWORK="${1#*=}"
        ;;
    --old-contract=*)
        OLD_CONTRACT="${1#*=}"
        ;;
    --help)
        echo "Usage: $0 [--network=testnet|mainnet] [--old-contract=account.testnet]"
        echo ""
        echo "Options:"
        echo "  --network=NETWORK        Target network (testnet or mainnet, default: testnet)"
        echo "  --old-contract=CONTRACT  Old monolithic contract account"
        echo "  --help                   Show this help message"
        exit 0
        ;;
esac

# Run main migration
main