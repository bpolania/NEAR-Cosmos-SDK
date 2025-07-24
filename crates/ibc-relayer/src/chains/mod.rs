use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;

pub mod near;
pub mod cosmos;

/// Generic chain interface for IBC operations
#[async_trait]
pub trait Chain: Send + Sync {
    /// Chain identifier
    fn chain_id(&self) -> &str;
    
    /// Get current block height
    async fn get_height(&self) -> Result<u64>;
    
    /// Get block header at specific height
    async fn get_header(&self, height: u64) -> Result<Value>;
    
    /// Submit a transaction to the chain
    async fn submit_tx(&self, tx: Vec<u8>) -> Result<String>;
    
    /// Query chain state
    async fn query(&self, path: &str, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Subscribe to events
    async fn subscribe_events(&self) -> Result<Box<dyn EventStream>>;
    
    // IBC-specific operations
    
    /// Create IBC client
    async fn create_client(&self, client_state: Value, consensus_state: Value) -> Result<String>;
    
    /// Update IBC client
    async fn update_client(&self, client_id: &str, header: Value) -> Result<()>;
    
    /// Send IBC packet
    async fn send_packet(&self, packet: IbcPacket) -> Result<u64>;
    
    /// Receive IBC packet
    async fn recv_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()>;
    
    /// Acknowledge IBC packet
    async fn ack_packet(&self, packet: IbcPacket, ack: Vec<u8>, proof: Vec<u8>) -> Result<()>;
    
    /// Handle packet timeout
    async fn timeout_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()>;
}

/// Event stream interface
#[async_trait]
pub trait EventStream: Send {
    /// Get next event
    async fn next_event(&mut self) -> Result<Option<ChainEvent>>;
}

/// Chain event types
#[derive(Debug, Clone)]
pub struct ChainEvent {
    pub event_type: String,
    pub attributes: Vec<(String, String)>,
    pub height: u64,
    pub tx_hash: Option<String>,
}

/// IBC packet structure
#[derive(Debug, Clone)]
pub struct IbcPacket {
    pub sequence: u64,
    pub source_port: String,
    pub source_channel: String,
    pub destination_port: String,
    pub destination_channel: String,
    pub data: Vec<u8>,
    pub timeout_height: Option<u64>,
    pub timeout_timestamp: Option<u64>,
}

/// Create chain instance based on configuration
pub fn create_chain(config: &crate::config::ChainConfig) -> Result<Box<dyn Chain>> {
    match config.chain_type.as_str() {
        "near" => {
            if let crate::config::ChainSpecificConfig::Near { .. } = &config.config {
                Ok(Box::new(near::NearChain::new(config)?))
            } else {
                anyhow::bail!("Invalid NEAR chain configuration")
            }
        }
        "cosmos" => {
            if let crate::config::ChainSpecificConfig::Cosmos { .. } = &config.config {
                Ok(Box::new(cosmos::CosmosChain::new(config)?))
            } else {
                anyhow::bail!("Invalid Cosmos chain configuration")
            }
        }
        _ => anyhow::bail!("Unsupported chain type: {}", config.chain_type),
    }
}