/// Standard CosmWasm Contract Implementations
/// 
/// This module contains production-ready implementations of standard CosmWasm contracts
/// that demonstrate how existing Cosmos ecosystem contracts can run on NEAR using our
/// compatibility layer.

pub mod cw20_base;
pub mod wasm_module_contract;
pub mod router_contract;
pub mod ibc_client_contract;
pub mod ibc_channel_contract;
pub mod ibc_connection_contract;
pub mod ibc_transfer_contract;
pub mod bank_contract;
pub mod staking_contract;

pub use cw20_base::Cw20Contract;
pub use wasm_module_contract::WasmModuleContract;
pub use router_contract::RouterContract;
pub use ibc_client_contract::IbcClientContract;
pub use ibc_channel_contract::IbcChannelContract;
pub use ibc_connection_contract::IbcConnectionContract;
pub use ibc_transfer_contract::IbcTransferContract;
pub use bank_contract::BankContract;
pub use staking_contract::StakingContract;