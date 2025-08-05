/// CosmWasm Module
/// 
/// This module implements the Cosmos SDK x/wasm module functionality for NEAR,
/// allowing arbitrary CosmWasm contracts to be deployed and managed.

pub mod types;
pub mod module;

pub use types::*;
pub use module::WasmModule;