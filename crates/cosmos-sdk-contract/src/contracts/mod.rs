/// Standard CosmWasm Contract Implementations
/// 
/// This module contains production-ready implementations of standard CosmWasm contracts
/// that demonstrate how existing Cosmos ecosystem contracts can run on NEAR using our
/// compatibility layer.

// Only include non-bindgen components in the library to avoid symbol conflicts
pub mod cw20_base;

// Comment out near_bindgen contracts to avoid symbol conflicts during library compilation
// These will be used directly in their respective binary targets

// pub mod router_contract;
// pub mod bank_contract;
// pub mod wasm_module_contract;
// pub mod ibc_client_contract;
// pub mod ibc_channel_contract;
// pub mod ibc_connection_contract;
// pub mod ibc_transfer_contract;
// pub mod staking_contract;
// pub mod state_sync_contract;

// Only export non-conflicting types
pub use cw20_base::Cw20Contract;

// Note: Individual contracts are implemented in their respective binary targets