#!/bin/bash
set -e

echo "🔨 Building Minimal CW20 Contract"
echo "=================================="

# Create a minimal Rust project for CW20
mkdir -p contracts/minimal-cw20
cd contracts/minimal-cw20

# Check if we already have the project
if [ ! -f "Cargo.toml" ]; then
    echo "Creating new CosmWasm project..."
    
    # Create Cargo.toml
    cat > Cargo.toml << 'EOF'
[package]
name = "minimal-cw20"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[profile.release]
opt-level = 3
debug = false
rpath = false
lto = true
debug-assertions = false
codegen-units = 1
panic = 'abort'
incremental = false
overflow-checks = true

[dependencies]
cosmwasm-std = "1.1"
cosmwasm-storage = "1.1"
schemars = "0.8"
serde = { version = "1.0", default-features = false, features = ["derive"] }
thiserror = "1.0"
cw2 = "1.0"
cw20 = "1.0"

[dev-dependencies]
cosmwasm-schema = "1.1"
EOF

    # Create minimal contract
    mkdir -p src
    cat > src/lib.rs << 'EOF'
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, BalanceResponse, TokenInfoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balance: Uint128,
}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("name", msg.name)
        .add_attribute("symbol", msg.symbol))
}

#[entry_point]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Cw20ExecuteMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("method", "execute"))
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: Cw20QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw20QueryMsg::TokenInfo {} => to_binary(&TokenInfoResponse {
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            decimals: 6,
            total_supply: Uint128::new(1000000),
        }),
        Cw20QueryMsg::Balance { address: _ } => to_binary(&BalanceResponse {
            balance: Uint128::new(1000000),
        }),
        _ => to_binary(&"{}"),
    }
}
EOF

    echo "✅ Project created"
else
    echo "✅ Project already exists"
fi

# Build the contract
echo ""
echo "Building WASM..."

# Install wasm target if not present
rustup target add wasm32-unknown-unknown 2>/dev/null || true

# Build
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --lib

# Copy and optimize
cp target/wasm32-unknown-unknown/release/minimal_cw20.wasm ../minimal_cw20_raw.wasm

echo ""
echo "WASM built: $(ls -lh ../minimal_cw20_raw.wasm | awk '{print $5}')"

# Generate base64
cd ..
base64 -i minimal_cw20_raw.wasm -o minimal_cw20.wasm.b64

echo "✅ Base64 generated: $(wc -c < minimal_cw20.wasm.b64) bytes"
echo ""
echo "✨ Minimal CW20 contract ready!"