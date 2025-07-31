#!/bin/bash

set -e

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

log "Checking Docker requirements for local wasmd testnet..."

# Check if Docker is installed
if command -v docker >/dev/null 2>&1; then
    success "Docker is installed"
    docker --version
else
    error "Docker is not installed"
    echo "Please install Docker to use the local wasmd testnet"
    echo "Visit: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is available
if command -v docker-compose >/dev/null 2>&1; then
    success "Docker Compose is available"
    docker-compose --version
elif docker compose version >/dev/null 2>&1; then
    success "Docker Compose (plugin) is available"
    docker compose version
else
    error "Docker Compose is not available"
    echo "Please install Docker Compose"
    exit 1
fi

# Check if Docker daemon is running
if docker info >/dev/null 2>&1; then
    success "Docker daemon is running"
else
    error "Docker daemon is not running"
    echo "Please start Docker"
    exit 1
fi

# Check if required ports are available
check_port() {
    local port=$1
    local service=$2
    
    if lsof -i :$port >/dev/null 2>&1; then
        warning "Port $port is in use (needed for $service)"
        echo "  Process using port $port:"
        lsof -i :$port | head -2
        return 1
    else
        success "Port $port is available ($service)"
        return 0
    fi
}

log "Checking port availability..."
ports_ok=true

check_port 26657 "Tendermint RPC" || ports_ok=false
check_port 1317 "Cosmos REST API" || ports_ok=false
check_port 9090 "gRPC" || ports_ok=false
check_port 26656 "P2P" || ports_ok=false

if [ "$ports_ok" = false ]; then
    warning "Some ports are in use. You may need to stop other services or the tests may fail."
    echo "To stop any existing wasmd testnet: ./scripts/local-testnet.sh stop"
else
    success "All required ports are available"
fi

# Check if wasmd image can be pulled
log "Checking wasmd Docker image availability..."
if docker pull cosmwasm/wasmd:latest >/dev/null 2>&1; then
    success "wasmd Docker image is available"
else
    error "Failed to pull wasmd Docker image"
    echo "Please check your internet connection"
    exit 1
fi

success "All Docker requirements are satisfied!"
echo ""
echo "You can now run the local wasmd testnet with:"
echo "  ./scripts/local-testnet.sh start"
echo ""
echo "Or run integration tests that will automatically start it:"
echo "  cargo test test_local_wasmd_testnet_connectivity"