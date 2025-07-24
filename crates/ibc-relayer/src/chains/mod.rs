// Chain-related types
#![allow(dead_code)]

/// Generic chain interface for IBC operations
pub trait Chain: Send + Sync {
    // Empty trait - only used as a type constraint in tests
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
        
        // Test Chain trait
        struct TestChain;
        impl Chain for TestChain {}
        let _chain: Box<dyn Chain> = Box::new(TestChain);
    }
}