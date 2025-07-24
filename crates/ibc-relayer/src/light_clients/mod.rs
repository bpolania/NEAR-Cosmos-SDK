// Light client management for IBC relayer

pub mod near_client;
pub mod tendermint_client;

use anyhow::Result;
use serde_json::Value;

/// Light client interface
pub trait LightClient {
    /// Verify a header against the current trusted state
    fn verify_header(&self, header: &Value) -> Result<bool>;
    
    /// Update the trusted state with a new header
    fn update_state(&mut self, header: Value) -> Result<()>;
    
    /// Get the current trusted height
    fn trusted_height(&self) -> u64;
    
    /// Generate a proof for state at specific height
    fn generate_proof(&self, height: u64, key: &[u8]) -> Result<Vec<u8>>;
}

/// Create light client instance based on client type
pub fn create_light_client(client_type: &str, initial_state: Value) -> Result<Box<dyn LightClient>> {
    match client_type {
        "07-tendermint" => Ok(Box::new(tendermint_client::TendermintLightClient::new(initial_state)?)),
        "07-near" => Ok(Box::new(near_client::NearLightClient::new(initial_state)?)),
        _ => anyhow::bail!("Unsupported light client type: {}", client_type),
    }
}