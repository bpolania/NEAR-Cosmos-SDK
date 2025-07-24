// Chain-related types
#![allow(dead_code)]

use async_trait::async_trait;
use futures::Stream;

pub mod near_simple;
pub mod cosmos_minimal;

// Re-export for easier access (will be used by relay engine)
#[allow(unused_imports)]
pub use near_simple::NearChain;
#[allow(unused_imports)]
pub use cosmos_minimal::CosmosChain;

/// Generic chain interface for IBC operations
#[async_trait]
pub trait Chain: Send + Sync {
    /// Get the chain ID
    async fn chain_id(&self) -> String;

    /// Get the latest block height
    async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Query packet commitment
    async fn query_packet_commitment(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;

    /// Query packet acknowledgment
    async fn query_packet_acknowledgment(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;

    /// Query packet receipt (for unordered channels)
    async fn query_packet_receipt(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    /// Query next sequence receive
    async fn query_next_sequence_recv(
        &self,
        port_id: &str,
        channel_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get events in a block range
    async fn get_events(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>>;

    /// Monitor for new events (streaming)
    async fn subscribe_events(
        &self,
    ) -> Result<
        Box<dyn Stream<Item = ChainEvent> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    >;

    /// Submit a transaction
    async fn submit_transaction(
        &self,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Health check
    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_types_compile() {
        // This test ensures all types can be constructed
        let event = ChainEvent {
            event_type: "test".to_string(),
            attributes: vec![],
            height: 0,
            tx_hash: None,
        };
        
        let packet = IbcPacket {
            sequence: 0,
            source_port: "".to_string(),
            source_channel: "".to_string(),
            destination_port: "".to_string(),
            destination_channel: "".to_string(),
            data: vec![],
            timeout_height: None,
            timeout_timestamp: None,
        };
        
        // Verify types can be used
        assert_eq!(event.event_type, "test");
        assert_eq!(packet.sequence, 0);
        
        // Test Chain trait with async implementation
        struct TestChain;
        
        #[async_trait]
        impl Chain for TestChain {
            async fn chain_id(&self) -> String { "test".to_string() }
            async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> { Ok(1) }
            async fn query_packet_commitment(&self, _: &str, _: &str, _: u64) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> { Ok(None) }
            async fn query_packet_acknowledgment(&self, _: &str, _: &str, _: u64) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> { Ok(None) }
            async fn query_packet_receipt(&self, _: &str, _: &str, _: u64) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> { Ok(false) }
            async fn query_next_sequence_recv(&self, _: &str, _: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> { Ok(1) }
            async fn get_events(&self, _: u64, _: u64) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> { Ok(vec![]) }
            async fn subscribe_events(&self) -> Result<Box<dyn Stream<Item = ChainEvent> + Send + Unpin>, Box<dyn std::error::Error + Send + Sync>> { 
                Ok(Box::new(futures::stream::empty()))
            }
            async fn submit_transaction(&self, _: Vec<u8>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> { Ok("test".to_string()) }
            async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Ok(()) }
        }
        
        let _chain: Box<dyn Chain> = Box::new(TestChain);
    }
}