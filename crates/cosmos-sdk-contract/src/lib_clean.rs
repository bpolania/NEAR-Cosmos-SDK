// Clean library file that exports common modules without contract implementations
// This allows different binary targets to use the modules without symbol conflicts

pub type Balance = u128;

// Export all modules for use by different contract types
pub mod modules;
pub mod types;
pub mod handler;
pub mod crypto;
pub mod contracts;

// Re-export commonly used types
pub use modules::bank::BankModule;
pub use modules::gov::GovernanceModule;
pub use modules::staking::StakingModule;
pub use modules::wasm::{WasmModule, CodeID, ContractAddress, InstantiateResponse, ExecuteResponse};
pub use modules::ibc::client::tendermint::{TendermintLightClientModule, Header, Height};
pub use modules::ibc::connection::{ConnectionModule, ConnectionEnd, Counterparty, Version};
pub use modules::ibc::connection::types::{MerklePrefix};
pub use modules::ibc::channel::{ChannelModule, ChannelEnd, Order, Packet, Acknowledgement};
pub use modules::ibc::channel::types::{PacketCommitment, PacketReceipt};
pub use modules::ibc::transfer::{TransferModule, FungibleTokenPacketData, DenomTrace};

pub use handler::{CosmosMessageHandler, HandleResponse, HandleResult, route_cosmos_message, success_result, create_event, validate_cosmos_address, CosmosTransactionHandler, TxProcessingConfig, TxResponse};
pub use types::cosmos_messages::*;

// For testing
#[cfg(test)]
mod lib_tests;