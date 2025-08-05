/// Standard CosmWasm Contract Implementations
/// 
/// This module contains production-ready implementations of standard CosmWasm contracts
/// that demonstrate how existing Cosmos ecosystem contracts can run on NEAR using our
/// compatibility layer.

pub mod cw20_base;

pub use cw20_base::Cw20Contract;