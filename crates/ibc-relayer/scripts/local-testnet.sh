#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/../docker"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

show_help() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  start     Start the local wasmd testnet"
    echo "  stop      Stop the local wasmd testnet"
    echo "  restart   Restart the local wasmd testnet"
    echo "  status    Show testnet status"
    echo "  logs      Show testnet logs"
    echo "  clean     Stop and remove all data"
    echo "  accounts  Show test account information"
    echo "  test      Run connectivity tests"
    echo "  help      Show this help message"
}

start_testnet() {
    log "Starting local wasmd testnet..."
    cd "$DOCKER_DIR"
    
    if docker-compose ps wasmd | grep -q "Up"; then
        warning "Testnet is already running"
        return 0
    fi
    
    docker-compose up -d wasmd
    
    log "Waiting for testnet to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:26657/status > /dev/null 2>&1; then
            success "Local wasmd testnet is running!"
            echo ""
            echo "RPC endpoint: http://localhost:26657"
            echo "REST API: http://localhost:1317"
            echo "gRPC: localhost:9090"
            echo ""
            show_accounts
            return 0
        fi
        sleep 2
        echo -n "."
    done
    
    error "Testnet failed to start within 60 seconds"
    docker-compose logs wasmd
    return 1
}

stop_testnet() {
    log "Stopping local wasmd testnet..."
    cd "$DOCKER_DIR"
    docker-compose stop wasmd
    success "Testnet stopped"
}

restart_testnet() {
    stop_testnet
    sleep 2
    start_testnet
}

show_status() {
    cd "$DOCKER_DIR"
    if docker-compose ps wasmd | grep -q "Up"; then
        success "Testnet is running"
        if curl -s http://localhost:26657/status > /dev/null 2>&1; then
            local height=$(curl -s http://localhost:26657/status | jq -r '.result.sync_info.latest_block_height')
            echo "Latest block height: $height"
        fi
    else
        warning "Testnet is not running"
    fi
}

show_logs() {
    cd "$DOCKER_DIR"
    docker-compose logs -f wasmd
}

clean_testnet() {
    log "Cleaning local wasmd testnet..."
    cd "$DOCKER_DIR"
    docker-compose down -v
    docker-compose rm -f
    success "Testnet cleaned"
}

show_accounts() {
    echo "Test Accounts (deterministic):"
    echo "=============================="
    echo "Validator: wasm1qqxqfzagzm7r8m4zq9gfmk9g5k7nfcasg8jhxm"
    echo "Test1:     wasm1qy5ldxnqc5x2sffm4m4fl2nzm6q6lzfcx8x4sy"  
    echo "Relayer:   wasm1qvw7ks35q7r2qm4m7w7k2lr8m8x6sl2cy8d0mn"
    echo ""
    echo "All accounts have 100000000000000000000 stake and token"
    echo "Keyring backend: test (for easy CLI access)"
}

test_connectivity() {
    log "Testing connectivity to local wasmd testnet..."
    
    # Test RPC
    if curl -s http://localhost:26657/status > /dev/null; then
        success "RPC endpoint responding"
        local chain_id=$(curl -s http://localhost:26657/status | jq -r '.result.node_info.network')
        echo "Chain ID: $chain_id"
    else
        error "RPC endpoint not responding"
        return 1
    fi
    
    # Test REST API
    if curl -s http://localhost:1317/cosmos/base/tendermint/v1beta1/node_info > /dev/null; then
        success "REST API responding"
    else
        error "REST API not responding"
        return 1
    fi
    
    success "All connectivity tests passed!"
}

case "${1:-help}" in
    start)
        start_testnet
        ;;
    stop)
        stop_testnet
        ;;
    restart)
        restart_testnet
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs
        ;;
    clean)
        clean_testnet
        ;;
    accounts)
        show_accounts
        ;;
    test)
        test_connectivity
        ;;
    help|*)
        show_help
        ;;
esac