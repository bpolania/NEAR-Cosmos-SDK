// Integration tests for real-time event monitoring
use std::collections::HashMap;
use std::sync::Arc;

use ibc_relayer::{
    chains::{Chain, ChainEvent},
    monitor::{EventMonitor, MonitorConfig},
    relay::RelayEvent,
};

// Mock chain implementation for testing event monitoring
struct MockChain {
    chain_id: String,
    mock_events: Vec<ChainEvent>,
    event_index: std::sync::Mutex<usize>,
}

impl MockChain {
    fn new(chain_id: String, events: Vec<ChainEvent>) -> Self {
        Self {
            chain_id,
            mock_events: events,
            event_index: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Chain for MockChain {
    async fn chain_id(&self) -> String {
        self.chain_id.clone()
    }

    async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(1000)
    }

    async fn query_packet_commitment(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    async fn query_packet_acknowledgment(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    async fn query_packet_receipt(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(false)
    }

    async fn query_next_sequence_recv(
        &self,
        _: &str,
        _: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(1)
    }

    async fn get_events(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // Return mock events for the specified block range
        let events_per_block = 2;
        let mut result = Vec::new();
        
        for height in from_height..=to_height {
            let start_idx = (height - 1) as usize * events_per_block;
            let end_idx = start_idx + events_per_block;
            
            for (i, event) in self.mock_events.iter().enumerate() {
                if i >= start_idx && i < end_idx && i < self.mock_events.len() {
                    let mut event_copy = event.clone();
                    event_copy.height = height;
                    result.push(event_copy);
                }
            }
        }
        
        Ok(result)
    }

    async fn subscribe_events(
        &self,
    ) -> Result<
        Box<dyn futures::Stream<Item = ChainEvent> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Return empty stream for testing
        Ok(Box::new(futures::stream::empty()))
    }

    async fn submit_transaction(
        &self,
        _: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok("mock_tx_hash".to_string())
    }

    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

// Helper function to create test events
fn create_test_events() -> Vec<ChainEvent> {
    vec![
        ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_data".to_string(), "dGVzdA==".to_string()), // "test" in base64
                ("packet_timeout_height".to_string(), "1000".to_string()),
            ],
            height: 100,
            tx_hash: Some("send_tx_hash".to_string()),
        },
        ChainEvent {
            event_type: "recv_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
            ],
            height: 101,
            tx_hash: Some("recv_tx_hash".to_string()),
        },
        ChainEvent {
            event_type: "acknowledge_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_ack".to_string(), "AQ==".to_string()), // [1] in base64
            ],
            height: 102,
            tx_hash: Some("ack_tx_hash".to_string()),
        },
        ChainEvent {
            event_type: "timeout_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "2".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ],
            height: 103,
            tx_hash: Some("timeout_tx_hash".to_string()),
        },
    ]
}

#[tokio::test]
async fn test_event_monitor_creation() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    chains.insert(
        "near-testnet".to_string(),
        Arc::new(MockChain::new("near-testnet".to_string(), create_test_events()))
    );
    chains.insert(
        "cosmoshub-testnet".to_string(),
        Arc::new(MockChain::new("cosmoshub-testnet".to_string(), create_test_events()))
    );

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel(100);
    let config = MonitorConfig::default();

    let (monitor, _shutdown_sender) = EventMonitor::new(chains, event_sender, config);

    // Verify monitor was created successfully
    // Note: Since chains field is private, we just check creation succeeded
    drop(monitor); // Monitor created successfully
}

#[tokio::test]
async fn test_event_parsing_send_packet() {
    use ibc_relayer::monitor::EventMonitor;
    use ibc_relayer::chains::ChainEvent;
    
    let chain_event = ChainEvent {
        event_type: "send_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "5".to_string()),
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
            ("packet_dst_port".to_string(), "transfer".to_string()),
            ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ("packet_data".to_string(), "aGVsbG8=".to_string()), // "hello" in base64
            ("packet_timeout_height".to_string(), "2000".to_string()),
            ("packet_timeout_timestamp".to_string(), "1234567890".to_string()),
        ],
        height: 150,
        tx_hash: Some("test_tx_hash".to_string()),
    };
    
    let result = EventMonitor::parse_send_packet_event("near-testnet", &chain_event).unwrap();
    assert!(result.is_some());
    
    if let Some(RelayEvent::PacketDetected { chain_id, packet, _event }) = result {
        assert_eq!(chain_id, "near-testnet");
        assert_eq!(packet.sequence, 5);
        assert_eq!(packet.source_port, "transfer");
        assert_eq!(packet.source_channel, "channel-0");
        assert_eq!(packet.destination_port, "transfer");
        assert_eq!(packet.destination_channel, "channel-1");
        assert_eq!(packet.data, b"hello");
        assert_eq!(packet.timeout_height, Some(2000));
        assert_eq!(packet.timeout_timestamp, Some(1234567890));
    } else {
        panic!("Expected PacketDetected event");
    }
}

#[tokio::test]
async fn test_event_parsing_acknowledge_packet() {
    use ibc_relayer::monitor::EventMonitor;
    use ibc_relayer::chains::ChainEvent;
    
    let chain_event = ChainEvent {
        event_type: "acknowledge_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "7".to_string()),
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
            ("packet_dst_port".to_string(), "transfer".to_string()),
            ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ("packet_ack".to_string(), "c3VjY2Vzcw==".to_string()), // "success" in base64
        ],
        height: 200,
        tx_hash: Some("ack_test_hash".to_string()),
    };
    
    let result = EventMonitor::parse_acknowledge_packet_event("cosmoshub-testnet", &chain_event).unwrap();
    assert!(result.is_some());
    
    if let Some(RelayEvent::PacketAcknowledged { chain_id, packet, ack_data }) = result {
        assert_eq!(chain_id, "cosmoshub-testnet");
        assert_eq!(packet.sequence, 7);
        assert_eq!(packet.source_port, "transfer");
        assert_eq!(packet.source_channel, "channel-0");
        assert_eq!(ack_data, b"success");
    } else {
        panic!("Expected PacketAcknowledged event");
    }
}

#[tokio::test]
async fn test_event_parsing_recv_packet() {
    use ibc_relayer::monitor::EventMonitor;
    use ibc_relayer::chains::ChainEvent;
    
    let chain_event = ChainEvent {
        event_type: "recv_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "3".to_string()),
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
        ],
        height: 175,
        tx_hash: Some("recv_test_hash".to_string()),
    };
    
    let result = EventMonitor::parse_recv_packet_event("near-testnet", &chain_event).unwrap();
    assert!(result.is_some());
    
    if let Some(RelayEvent::PacketRelayed { source_chain, dest_chain, sequence }) = result {
        assert_eq!(source_chain, "cosmoshub-testnet"); // Packet came from Cosmos
        assert_eq!(dest_chain, "near-testnet"); // Received on NEAR
        assert_eq!(sequence, 3);
    } else {
        panic!("Expected PacketRelayed event");
    }
}

#[tokio::test]
async fn test_event_parsing_timeout_packet() {
    use ibc_relayer::monitor::EventMonitor;
    use ibc_relayer::chains::ChainEvent;
    
    let chain_event = ChainEvent {
        event_type: "timeout_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "9".to_string()),
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
            ("packet_dst_port".to_string(), "transfer".to_string()),
            ("packet_dst_channel".to_string(), "channel-1".to_string()),
        ],
        height: 250,
        tx_hash: Some("timeout_test_hash".to_string()),
    };
    
    let result = EventMonitor::parse_timeout_packet_event("cosmoshub-testnet", &chain_event).unwrap();
    assert!(result.is_some());
    
    if let Some(RelayEvent::PacketTimedOut { chain_id, packet }) = result {
        assert_eq!(chain_id, "cosmoshub-testnet");
        assert_eq!(packet.sequence, 9);
        assert_eq!(packet.source_port, "transfer");
        assert_eq!(packet.source_channel, "channel-0");
    } else {
        panic!("Expected PacketTimedOut event");
    }
}

#[tokio::test]
async fn test_monitor_configuration() {
    let config = MonitorConfig {
        polling_interval_ms: 500,
        blocks_per_poll: 5,
        prefer_streaming: false,
        max_concurrent_monitors: 5,
    };
    
    assert_eq!(config.polling_interval_ms, 500);
    assert_eq!(config.blocks_per_poll, 5);
    assert!(!config.prefer_streaming);
    assert_eq!(config.max_concurrent_monitors, 5);
}

#[tokio::test]
async fn test_chain_event_querying() {
    let mock_chain = MockChain::new("test-chain".to_string(), create_test_events());
    
    // Test querying events from multiple blocks
    let events = mock_chain.get_events(1, 2).await.unwrap();
    assert_eq!(events.len(), 4); // 2 events per block * 2 blocks
    
    // Verify event heights are set correctly
    assert_eq!(events[0].height, 1);
    assert_eq!(events[1].height, 1);
    assert_eq!(events[2].height, 2);
    assert_eq!(events[3].height, 2);
    
    // Verify event types
    assert_eq!(events[0].event_type, "send_packet");
    assert_eq!(events[1].event_type, "recv_packet");
}

#[tokio::test]
async fn test_invalid_event_parsing() {
    use ibc_relayer::monitor::EventMonitor;
    use ibc_relayer::chains::ChainEvent;
    
    // Test event with missing required attributes
    let invalid_event = ChainEvent {
        event_type: "send_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "1".to_string()),
            // Missing required attributes like port and channel
        ],
        height: 100,
        tx_hash: None,
    };
    
    let result = EventMonitor::parse_send_packet_event("test-chain", &invalid_event).unwrap();
    assert!(result.is_none()); // Should return None for incomplete events
    
    // Test with non-IBC event type
    let non_ibc_event = ChainEvent {
        event_type: "transfer".to_string(), // Not an IBC event
        attributes: vec![],
        height: 100,
        tx_hash: None,
    };
    
    let result = EventMonitor::parse_send_packet_event("test-chain", &non_ibc_event).unwrap();
    assert!(result.is_none());
}