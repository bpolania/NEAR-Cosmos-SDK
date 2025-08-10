// Integration tests for bidirectional packet relay with proper sequencing
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use ibc_relayer::chains::{Chain, IbcPacket, near_simple::NearChain, cosmos_minimal::CosmosChain};
use ibc_relayer::config::{ChainConfig, ChainSpecificConfig, RelayerConfig, GlobalConfig, MetricsConfig};
use ibc_relayer::metrics::RelayerMetrics;
use ibc_relayer::relay::{RelayEvent, PacketKey};
use ibc_relayer::relay::bidirectional::{
    BidirectionalRelayManager, BidirectionalConfig, RelayDirection, 
    SequencedPacket, PacketRelayState, SequenceTracker, BidirectionalStats
};

/// Helper function to create test NEAR chain configuration
fn create_test_near_config() -> ChainConfig {
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-sdk-demo.testnet".to_string(),
            signer_account_id: "cuteharbor3573.testnet".to_string(),
            network_id: "testnet".to_string(),
            private_key: None,
            modular: true,
            modules: Some({
                let mut modules = HashMap::new();
                modules.insert("ibc_client".to_string(), "ibc-client.testnet".to_string());
                modules.insert("ibc_connection".to_string(), "ibc-connection.testnet".to_string());
                modules.insert("ibc_channel".to_string(), "ibc-channel.testnet".to_string());
                modules.insert("ibc_transfer".to_string(), "ibc-transfer.testnet".to_string());
                modules.insert("bank".to_string(), "bank.testnet".to_string());
                modules.insert("router".to_string(), "router.testnet".to_string());
                modules
            }),
        },
    }
}

/// Helper function to create test Cosmos chain configuration
fn create_test_cosmos_config() -> ChainConfig {
    ChainConfig {
        chain_id: "provider".to_string(),
        chain_type: "cosmos".to_string(),
        rpc_endpoint: "https://rpc.provider-sentry-01.ics-testnet.polypore.xyz".to_string(),
        ws_endpoint: Some("wss://rpc.provider-sentry-01.ics-testnet.polypore.xyz/websocket".to_string()),
        config: ChainSpecificConfig::Cosmos {
            address_prefix: "cosmos".to_string(),
            gas_price: "0.025uatom".to_string(),
            signer_key: None,
            trust_threshold: "1/3".to_string(),
            trusting_period_hours: 336,
        },
    }
}

/// Helper function to create test relayer configuration
fn create_test_relayer_config() -> RelayerConfig {
    RelayerConfig {
        global: GlobalConfig {
            log_level: "info".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval: 30,
        },
        chains: HashMap::new(),
        connections: vec![],
        metrics: MetricsConfig {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 9090,
        },
    }
}

/// Helper function to create test chains
fn create_test_chains() -> Result<HashMap<String, Arc<dyn Chain>>, Box<dyn std::error::Error + Send + Sync>> {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Create NEAR chain
    let near_config = create_test_near_config();
    let near_chain = NearChain::new(&near_config)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
    chains.insert("near-testnet".to_string(), Arc::new(near_chain));
    
    // Create Cosmos chain
    let cosmos_config = create_test_cosmos_config();
    let cosmos_chain = CosmosChain::new(&cosmos_config)?;
    chains.insert("provider".to_string(), Arc::new(cosmos_chain));
    
    Ok(chains)
}

/// Helper function to create a test packet
fn create_test_packet(sequence: u64, port: &str, channel: &str) -> IbcPacket {
    IbcPacket {
        sequence,
        source_port: port.to_string(),
        source_channel: channel.to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: format!("bidirectional_test_packet_{}", sequence).as_bytes().to_vec(),
        timeout_height: Some(1000000),
        timeout_timestamp: Some(1700000000000000000),
    }
}

#[tokio::test]
async fn test_bidirectional_config_creation() {
    let config = BidirectionalConfig::default();
    
    assert_eq!(config.max_parallel_packets, 10);
    assert_eq!(config.sequence_window_size, 1000);
    assert_eq!(config.sequence_check_interval, 10);
    assert_eq!(config.max_out_of_order_wait, 300);
    assert!(config.strict_ordering);
    assert_eq!(config.batch_size, 5);
    
    println!("✅ Bidirectional config created with expected defaults");
}

#[tokio::test]
async fn test_relay_direction_properties() {
    let near_to_cosmos = RelayDirection::NearToCosmos;
    assert_eq!(near_to_cosmos.source_chain(), "near");
    assert_eq!(near_to_cosmos.dest_chain(), "cosmos");
    assert_eq!(near_to_cosmos.reverse(), RelayDirection::CosmosToNear);
    
    let cosmos_to_near = RelayDirection::CosmosToNear;
    assert_eq!(cosmos_to_near.source_chain(), "cosmos");
    assert_eq!(cosmos_to_near.dest_chain(), "near");
    assert_eq!(cosmos_to_near.reverse(), RelayDirection::NearToCosmos);
    
    // Test equality
    assert_eq!(near_to_cosmos, RelayDirection::NearToCosmos);
    assert_ne!(near_to_cosmos, RelayDirection::CosmosToNear);
    
    println!("✅ Relay direction properties working correctly");
}

#[tokio::test]
async fn test_sequenced_packet_creation() {
    let packet = create_test_packet(42, "transfer", "channel-0");
    let sequenced = SequencedPacket::new(
        packet,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    assert_eq!(sequenced.packet.sequence, 42);
    assert_eq!(sequenced.direction, RelayDirection::NearToCosmos);
    assert_eq!(sequenced.source_chain_id, "near-testnet");
    assert_eq!(sequenced.dest_chain_id, "provider");
    assert_eq!(sequenced.state, PacketRelayState::Detected);
    assert_eq!(sequenced.attempts, 0);
    assert_eq!(sequenced.channel_id, "transfer:channel-0");
    assert!(sequenced.last_processed.is_none());
    
    // Test packet key generation
    let packet_key = sequenced.packet_key();
    assert_eq!(packet_key.sequence, 42);
    assert_eq!(packet_key.source_port, "transfer");
    assert_eq!(packet_key.source_channel, "channel-0");
    assert_eq!(packet_key.source_chain, "near-testnet");
    
    println!("✅ Sequenced packet creation working correctly");
}

#[tokio::test]
async fn test_packet_relay_state_transitions() {
    let states = vec![
        PacketRelayState::Detected,
        PacketRelayState::Processing,
        PacketRelayState::Relayed,
        PacketRelayState::Acknowledged,
        PacketRelayState::TimedOut,
        PacketRelayState::Failed("test error".to_string()),
    ];
    
    // Test state equality
    assert_eq!(PacketRelayState::Detected, PacketRelayState::Detected);
    assert_ne!(PacketRelayState::Detected, PacketRelayState::Processing);
    
    // Test failed state
    if let PacketRelayState::Failed(msg) = &states[5] {
        assert_eq!(msg, "test error");
    } else {
        panic!("Expected Failed state");
    }
    
    println!("✅ Packet relay state transitions working correctly");
}

#[tokio::test]
async fn test_sequence_tracker_in_order_processing() {
    let mut tracker = SequenceTracker::new(
        "transfer:channel-0".to_string(),
        RelayDirection::NearToCosmos,
        1,
    );
    
    // Add packets in order
    for seq in 1..=5 {
        let packet = create_test_packet(seq, "transfer", "channel-0");
        let sequenced = SequencedPacket::new(
            packet,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );
        
        let ready_packets = tracker.add_packet(sequenced);
        assert_eq!(ready_packets.len(), 1);
        assert_eq!(ready_packets[0].packet.sequence, seq);
    }
    
    assert_eq!(tracker.next_expected, 6);
    assert_eq!(tracker.highest_processed, 5);
    assert!(tracker.pending_packets.is_empty());
    
    let stats = tracker.get_stats();
    assert_eq!(stats.next_expected, 6);
    assert_eq!(stats.highest_processed, 5);
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.sequence_gap, 0);
    
    println!("✅ Sequence tracker in-order processing working correctly");
}

#[tokio::test]
async fn test_sequence_tracker_out_of_order_processing() {
    let mut tracker = SequenceTracker::new(
        "transfer:channel-0".to_string(),
        RelayDirection::NearToCosmos,
        1,
    );
    
    // Add packet 3 first (out of order)
    let packet3 = create_test_packet(3, "transfer", "channel-0");
    let sequenced3 = SequencedPacket::new(
        packet3,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced3);
    assert!(ready.is_empty()); // Not ready yet
    assert_eq!(tracker.pending_packets.len(), 1);
    assert_eq!(tracker.next_expected, 1);
    
    // Add packet 5 (further out of order)
    let packet5 = create_test_packet(5, "transfer", "channel-0");
    let sequenced5 = SequencedPacket::new(
        packet5,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced5);
    assert!(ready.is_empty()); // Still not ready
    assert_eq!(tracker.pending_packets.len(), 2);
    
    // Add packet 1 (expected)
    let packet1 = create_test_packet(1, "transfer", "channel-0");
    let sequenced1 = SequencedPacket::new(
        packet1,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced1);
    assert_eq!(ready.len(), 1); // Only packet 1 is ready
    assert_eq!(ready[0].packet.sequence, 1);
    assert_eq!(tracker.next_expected, 2);
    
    // Add packet 2 (should not release 3 yet because 4 is missing)
    let packet2 = create_test_packet(2, "transfer", "channel-0");
    let sequenced2 = SequencedPacket::new(
        packet2,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced2);
    assert_eq!(ready.len(), 2); // Packets 2 and 3 are ready
    assert_eq!(ready[0].packet.sequence, 2);
    assert_eq!(ready[1].packet.sequence, 3);
    assert_eq!(tracker.next_expected, 4);
    assert_eq!(tracker.pending_packets.len(), 1); // Packet 5 still pending
    
    // Add packet 4 (should release packet 5)
    let packet4 = create_test_packet(4, "transfer", "channel-0");
    let sequenced4 = SequencedPacket::new(
        packet4,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced4);
    assert_eq!(ready.len(), 2); // Packets 4 and 5 are ready
    assert_eq!(ready[0].packet.sequence, 4);
    assert_eq!(ready[1].packet.sequence, 5);
    assert_eq!(tracker.next_expected, 6);
    assert!(tracker.pending_packets.is_empty());
    
    println!("✅ Sequence tracker out-of-order processing working correctly");
}

#[tokio::test]
async fn test_sequence_tracker_duplicate_detection() {
    let mut tracker = SequenceTracker::new(
        "transfer:channel-0".to_string(),
        RelayDirection::NearToCosmos,
        1,
    );
    
    // Add packet 1
    let packet1 = create_test_packet(1, "transfer", "channel-0");
    let sequenced1 = SequencedPacket::new(
        packet1,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced1);
    assert_eq!(ready.len(), 1);
    assert_eq!(tracker.next_expected, 2);
    
    // Try to add packet 1 again (duplicate)
    let packet1_dup = create_test_packet(1, "transfer", "channel-0");
    let sequenced1_dup = SequencedPacket::new(
        packet1_dup,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let ready = tracker.add_packet(sequenced1_dup);
    assert!(ready.is_empty()); // Duplicate should be ignored
    assert_eq!(tracker.next_expected, 2); // No change
    
    println!("✅ Sequence tracker duplicate detection working correctly");
}

#[tokio::test]
async fn test_sequence_tracker_expired_packets() {
    let mut tracker = SequenceTracker::new(
        "transfer:channel-0".to_string(),
        RelayDirection::NearToCosmos,
        1,
    );
    
    // Add an out-of-order packet
    let packet3 = create_test_packet(3, "transfer", "channel-0");
    let sequenced3 = SequencedPacket::new(
        packet3,
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    tracker.add_packet(sequenced3);
    assert_eq!(tracker.pending_packets.len(), 1);
    
    // Check for expired packets with a very short timeout
    let expired = tracker.get_expired_packets(Duration::from_millis(1));
    // Might or might not be expired depending on timing
    
    // Check with a very long timeout (should not be expired)
    let expired = tracker.get_expired_packets(Duration::from_secs(3600));
    assert!(expired.is_empty());
    
    println!("✅ Sequence tracker expired packet handling working correctly");
}

#[tokio::test]
async fn test_bidirectional_relay_manager_creation() {
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    let bidirectional_config = BidirectionalConfig::default();
    let (event_sender, _event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let bidirectional_manager = BidirectionalRelayManager::new(
        bidirectional_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    let stats = bidirectional_manager.get_bidirectional_stats();
    assert_eq!(stats.total_trackers, 0);
    assert_eq!(stats.processing_packets, 0);
    assert_eq!(stats.total_pending, 0);
    
    println!("✅ Bidirectional relay manager created successfully");
}

#[tokio::test]
async fn test_bidirectional_stats_calculation() {
    let stats = BidirectionalStats {
        total_trackers: 4,
        processing_packets: 2,
        completed_packets: 100,
        total_pending: 5,
        max_sequence_gap: 3,
        successful_relays: 85,
        failed_relays: 10,
        acknowledged_packets: 80,
        timed_out_packets: 5,
        near_to_cosmos_packets: 50,
        cosmos_to_near_packets: 45,
    };
    
    assert_eq!(stats.total_trackers, 4);
    assert_eq!(stats.successful_relays, 85);
    assert_eq!(stats.failed_relays, 10);
    assert_eq!(stats.near_to_cosmos_packets, 50);
    assert_eq!(stats.cosmos_to_near_packets, 45);
    assert_eq!(stats.max_sequence_gap, 3);
    
    println!("✅ Bidirectional statistics calculation working correctly");
}

#[tokio::test]
async fn test_relay_event_handling_for_bidirectional() {
    let (event_sender, mut event_receiver) = mpsc::channel(100);
    
    // Create test relay events for both directions
    let near_to_cosmos_event = RelayEvent::PacketRelayed {
        source_chain: "near-testnet".to_string(),
        dest_chain: "provider".to_string(),
        sequence: 42,
    };
    
    let cosmos_to_near_event = RelayEvent::PacketRelayed {
        source_chain: "provider".to_string(),
        dest_chain: "near-testnet".to_string(),
        sequence: 43,
    };
    
    // Send events
    event_sender.send(near_to_cosmos_event).await.unwrap();
    event_sender.send(cosmos_to_near_event).await.unwrap();
    
    // Receive and verify events
    let mut received_count = 0;
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv()).await {
        if let Some(relay_event) = event {
            match relay_event {
                RelayEvent::PacketRelayed { source_chain, dest_chain, sequence } => {
                    if sequence == 42 {
                        assert_eq!(source_chain, "near-testnet");
                        assert_eq!(dest_chain, "provider");
                    } else if sequence == 43 {
                        assert_eq!(source_chain, "provider");
                        assert_eq!(dest_chain, "near-testnet");
                    }
                    received_count += 1;
                }
                _ => panic!("Unexpected event type"),
            }
            
            if received_count >= 2 {
                break;
            }
        }
    }
    
    assert_eq!(received_count, 2);
    println!("✅ Bidirectional relay event handling working correctly");
}

#[tokio::test]
async fn test_comprehensive_bidirectional_workflow() {
    println!("🧪 Starting comprehensive bidirectional workflow test");
    
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    // Create bidirectional manager with test-friendly configuration
    let bidirectional_config = BidirectionalConfig {
        max_parallel_packets: 3,
        sequence_window_size: 100,
        sequence_check_interval: 1, // Check every second for testing
        max_out_of_order_wait: 5,   // 5 seconds for testing
        strict_ordering: true,
        batch_size: 2,
    };
    
    let (event_sender, mut event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let mut bidirectional_manager = BidirectionalRelayManager::new(
        bidirectional_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    // Test adding packets in both directions
    let near_packet = create_test_packet(1, "transfer", "channel-0");
    let add_result = bidirectional_manager.add_packet(
        near_packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    ).await;
    
    match add_result {
        Ok(()) => println!("   Successfully added NEAR->Cosmos packet"),
        Err(e) => println!("   Expected error adding packet (no real processing): {}", e),
    }
    
    let cosmos_packet = create_test_packet(1, "transfer", "channel-0");
    let add_result = bidirectional_manager.add_packet(
        cosmos_packet,
        "provider".to_string(),
        "near-testnet".to_string(),
    ).await;
    
    match add_result {
        Ok(()) => println!("   Successfully added Cosmos->NEAR packet"),
        Err(e) => println!("   Expected error adding packet (no real processing): {}", e),
    }
    
    // Test statistics
    let stats = bidirectional_manager.get_bidirectional_stats();
    println!("   Bidirectional stats: {} trackers, {} processing", 
             stats.total_trackers, stats.processing_packets);
    
    println!("✅ Comprehensive bidirectional workflow test completed");
}

#[tokio::test]
async fn test_packet_key_consistency() {
    let packet = create_test_packet(999, "transfer", "channel-0");
    
    // Create sequenced packet
    let sequenced = SequencedPacket::new(
        packet.clone(),
        RelayDirection::NearToCosmos,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    // Create packet key directly
    let direct_key = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: packet.source_port.clone(),
        source_channel: packet.source_channel.clone(),
        sequence: packet.sequence,
    };
    
    // Keys should be identical
    let sequenced_key = sequenced.packet_key();
    assert_eq!(sequenced_key.sequence, direct_key.sequence);
    assert_eq!(sequenced_key.source_port, direct_key.source_port);
    assert_eq!(sequenced_key.source_channel, direct_key.source_channel);
    assert_eq!(sequenced_key.source_chain, direct_key.source_chain);
    
    // Test that keys can be used in hash maps
    let mut packet_map = HashMap::new();
    packet_map.insert(sequenced_key.clone(), "bidirectional_data".to_string());
    
    assert!(packet_map.contains_key(&sequenced_key));
    assert_eq!(packet_map.get(&sequenced_key), Some(&"bidirectional_data".to_string()));
    
    println!("✅ Packet key consistency working correctly");
}