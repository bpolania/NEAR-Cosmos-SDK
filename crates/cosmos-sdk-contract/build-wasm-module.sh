#!/bin/bash
# Build script for standalone x/wasm module contract
# This creates a deployable contract with only x/wasm functionality

echo "🔨 Building standalone x/wasm module contract..."

# Create a temporary Cargo.toml for the standalone build
cat > Cargo.wasm-module.toml << EOF
[package]
name = "cosmos-wasm-module"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
near-sdk = "5.1.0"
borsh = "1.0"
serde = "1.0"
serde_json = "1.0"
bs58 = "0.5"
hex = "0.4"
sha2 = "0.10"
base64 = "0.21"

[profile.release]
codegen-units = 1
opt-level = "z"
lto = true
debug = false
panic = "abort"
overflow-checks = true
EOF

# Create minimal lib.rs that only exports x/wasm module
cat > src/lib_wasm_only.rs << 'EOF'
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{near_bindgen, PanicOnDefault};

// Re-export only what we need
pub mod modules {
    pub mod wasm;
}
pub mod types;
pub mod contracts {
    pub mod wasm_module_contract;
}

// Export the standalone contract
pub use contracts::wasm_module_contract::WasmModuleContract;
EOF

# Build the standalone module
echo "📦 Building WASM module..."
RUSTFLAGS='-C link-arg=-s' cargo build \
    --manifest-path Cargo.wasm-module.toml \
    --target wasm32-unknown-unknown \
    --release \
    --lib \
    --features "wasm-module-only"

# Optimize the WASM
echo "🔧 Optimizing WASM..."
wasm-opt -Oz \
    --output target/wasm-module-optimized.wasm \
    target/wasm32-unknown-unknown/release/cosmos_wasm_module.wasm

# Check the result
echo "✅ Build complete!"
echo "📊 Module stats:"
ls -lh target/wasm-module-optimized.wasm

# Count functions
echo "🔢 Function count:"
wasmparser target/wasm-module-optimized.wasm | grep -c "func" || echo "Install wasmparser for function count"

echo "
🚀 Next steps:
1. Deploy: near deploy cosmos-wasm.testnet --wasmFile target/wasm-module-optimized.wasm
2. Initialize: near call cosmos-wasm.testnet new '{\"owner\": \"your-account.testnet\"}'
3. Use: near call cosmos-wasm.testnet store_code '{...}'
"