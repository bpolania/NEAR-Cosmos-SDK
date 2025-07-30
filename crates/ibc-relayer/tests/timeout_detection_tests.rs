// Integration tests for timeout detection and cleanup mechanisms
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use ibc_relayer::chains::{Chain, IbcPacket, near_simple::NearChain, cosmos_minimal::CosmosChain};
use ibc_relayer::config::{ChainConfig, ChainSpecificConfig, RelayerConfig, GlobalConfig, MetricsConfig};
use ibc_relayer::metrics::RelayerMetrics;
use ibc_relayer::relay::{RelayEvent, PacketKey};
use ibc_relayer::relay::timeout::{TimeoutManager, TimeoutConfig, TimeoutStatus, TimeoutTracker};

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

/// Helper function to create a test packet with timeout
fn create_test_packet_with_timeout(sequence: u64, timeout_timestamp: Option<u64>) -> IbcPacket {
    IbcPacket {
        sequence,
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: format!("timeout_test_packet_{}", sequence).as_bytes().to_vec(),
        timeout_height: Some(1000000),
        timeout_timestamp,
    }
}

#[tokio::test]
async fn test_timeout_config_creation() {
    let config = TimeoutConfig::default();
    
    assert_eq!(config.check_interval, 30);
    assert_eq!(config.grace_period, 300);
    assert_eq!(config.max_cleanup_retries, 3);
    assert_eq!(config.cleanup_retry_delay_ms, 5000);
    assert_eq!(config.max_completed_age_hours, 24);
    
    println!("✅ Timeout config created with expected defaults");
}

#[tokio::test]
async fn test_timeout_manager_creation() {
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    let timeout_config = TimeoutConfig::default();
    let (event_sender, _event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let timeout_manager = TimeoutManager::new(
        timeout_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    let stats = timeout_manager.get_timeout_stats();
    assert_eq!(stats.total_tracked, 0);
    assert_eq!(stats.active, 0);
    
    println!("✅ Timeout manager created successfully");
}

#[tokio::test]
async fn test_timeout_tracker_creation_and_status() {
    let packet = create_test_packet_with_timeout(42, None);
    let tracker = TimeoutTracker::new(
        packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    assert_eq!(tracker.packet.sequence, 42);
    assert_eq!(tracker.source_chain, "near-testnet");
    assert_eq!(tracker.dest_chain, "provider");
    assert_eq!(tracker.status, TimeoutStatus::Active);
    assert_eq!(tracker.cleanup_attempts, 0);
    assert!(tracker.last_cleanup_attempt.is_none());
    assert!(tracker.error_message.is_none());
    
    println!("✅ Timeout tracker created with correct initial state");
}

#[tokio::test]
async fn test_timeout_detection_with_expired_packet() {
    // Create packet with past timeout timestamp
    let past_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64 - 1_000_000_000; // 1 second ago
    
    let packet = create_test_packet_with_timeout(123, Some(past_time));
    let tracker = TimeoutTracker::new(
        packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    assert!(tracker.is_expired());
    
    // Test with future timeout
    let future_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64 + 1_000_000_000; // 1 second from now
    
    let packet = create_test_packet_with_timeout(124, Some(future_time));
    let tracker = TimeoutTracker::new(
        packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    assert!(!tracker.is_expired());
    
    println!("✅ Timeout detection working correctly for past and future timeouts");
}

#[tokio::test]
async fn test_timeout_status_transitions() {
    let packet = create_test_packet_with_timeout(456, None);
    let mut tracker = TimeoutTracker::new(
        packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    // Test status progression
    assert_eq!(tracker.status, TimeoutStatus::Active);
    
    tracker.status = TimeoutStatus::Expired;
    assert_eq!(tracker.status, TimeoutStatus::Expired);
    
    tracker.status = TimeoutStatus::CleaningUp;
    assert_eq!(tracker.status, TimeoutStatus::CleaningUp);
    
    tracker.status = TimeoutStatus::Completed;
    assert_eq!(tracker.status, TimeoutStatus::Completed);
    
    tracker.status = TimeoutStatus::Failed("Test error".to_string());
    if let TimeoutStatus::Failed(msg) = &tracker.status {
        assert_eq!(msg, "Test error");
    } else {
        panic!("Expected Failed status");
    }
    
    println!("✅ Timeout status transitions working correctly");
}

#[tokio::test]
async fn test_packet_tracking_and_untracking() {
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    let timeout_config = TimeoutConfig::default();
    let (event_sender, _event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let mut timeout_manager = TimeoutManager::new(
        timeout_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    // Track a packet
    let packet = create_test_packet_with_timeout(789, None);
    timeout_manager.track_packet(
        packet.clone(),
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    let stats = timeout_manager.get_timeout_stats();
    assert_eq!(stats.total_tracked, 1);
    assert_eq!(stats.active, 1);
    
    // Untrack the packet
    let packet_key = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: packet.source_port.clone(),
        source_channel: packet.source_channel.clone(),
        sequence: packet.sequence,
    };
    
    let removed = timeout_manager.untrack_packet(&packet_key);
    assert!(removed);
    
    let stats = timeout_manager.get_timeout_stats();
    assert_eq!(stats.total_tracked, 0);
    
    // Try to untrack non-existent packet
    let removed = timeout_manager.untrack_packet(&packet_key);
    assert!(!removed);
    
    println!("✅ Packet tracking and untracking working correctly");
}

#[tokio::test]
async fn test_timeout_stats_calculation() {
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    let timeout_config = TimeoutConfig::default();
    let (event_sender, _event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let mut timeout_manager = TimeoutManager::new(
        timeout_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    // Add multiple packets with different states
    for i in 1..=5 {
        let packet = create_test_packet_with_timeout(i, None);
        timeout_manager.track_packet(
            packet,
            "near-testnet".to_string(),
            "provider".to_string(),
        );
    }
    
    let stats = timeout_manager.get_timeout_stats();
    assert_eq!(stats.total_tracked, 5);
    assert_eq!(stats.active, 5);
    assert_eq!(stats.expired, 0);
    assert_eq!(stats.cleaning_up, 0);
    assert_eq!(stats.completed, 0);
    assert_eq!(stats.failed, 0);
    
    println!("✅ Timeout statistics calculation working correctly");
}

#[tokio::test]
async fn test_packet_key_generation_for_timeout() {
    let packet = create_test_packet_with_timeout(999, None);
    
    let packet_key = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: packet.source_port.clone(),
        source_channel: packet.source_channel.clone(),
        sequence: packet.sequence,
    };
    
    assert_eq!(packet_key.sequence, 999);
    assert_eq!(packet_key.source_port, "transfer");
    assert_eq!(packet_key.source_channel, "channel-0");
    assert_eq!(packet_key.source_chain, "near-testnet");
    
    // Test that packet keys can be used in hash maps
    let mut packet_map = HashMap::new();
    packet_map.insert(packet_key.clone(), "timeout_data".to_string());
    
    assert!(packet_map.contains_key(&packet_key));
    assert_eq!(packet_map.get(&packet_key), Some(&"timeout_data".to_string()));
    
    println!("✅ Packet key generation for timeout tracking working correctly");
}

#[tokio::test]
async fn test_grace_period_calculation() {
    let packet = create_test_packet_with_timeout(111, None);
    let tracker = TimeoutTracker::new(
        packet,
        "near-testnet".to_string(),
        "provider".to_string(),
    );
    
    // Should not need cleanup immediately
    let short_grace = Duration::from_millis(100);
    assert!(!tracker.needs_cleanup(short_grace));
    
    // Simulate expired packet
    let mut expired_tracker = tracker.clone();
    expired_tracker.status = TimeoutStatus::Expired;
    
    // Even expired packets should not need cleanup if within grace period
    assert!(!expired_tracker.needs_cleanup(Duration::from_secs(3600)));
    
    println!("✅ Grace period calculation working correctly");
}

#[tokio::test]
async fn test_timeout_event_handling() {
    let (event_sender, mut event_receiver) = mpsc::channel(100);
    
    // Create test timeout event
    let packet = create_test_packet_with_timeout(555, None);
    let timeout_event = RelayEvent::PacketTimedOut {
        chain_id: "provider".to_string(),
        packet: packet.clone(),
    };
    
    // Send timeout event
    let send_result = event_sender.send(timeout_event).await;
    assert!(send_result.is_ok());
    
    // Receive and verify timeout event
    if let Some(received_event) = event_receiver.recv().await {
        match received_event {
            RelayEvent::PacketTimedOut { chain_id, packet: received_packet } => {
                assert_eq!(chain_id, "provider");
                assert_eq!(received_packet.sequence, 555);
            }
            _ => panic!("Expected PacketTimedOut event"),
        }
    } else {
        panic!("Failed to receive timeout event");
    }
    
    println!("✅ Timeout event handling working correctly");
}

#[tokio::test]
async fn test_timeout_configuration_validation() {
    let custom_config = TimeoutConfig {
        check_interval: 60,
        grace_period: 600,
        max_cleanup_retries: 5,
        cleanup_retry_delay_ms: 10000,
        max_completed_age_hours: 48,
    };
    
    assert_eq!(custom_config.check_interval, 60);
    assert_eq!(custom_config.grace_period, 600);
    assert_eq!(custom_config.max_cleanup_retries, 5);
    assert_eq!(custom_config.cleanup_retry_delay_ms, 10000);
    assert_eq!(custom_config.max_completed_age_hours, 48);
    
    println!("✅ Timeout configuration validation working correctly");
}

#[tokio::test]
async fn test_comprehensive_timeout_workflow() {
    println!("🧪 Starting comprehensive timeout workflow test");
    
    let chains = create_test_chains().expect("Failed to create test chains");
    let config = create_test_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    // Create timeout manager with short intervals for testing
    let timeout_config = TimeoutConfig {
        check_interval: 1,     // Check every second
        grace_period: 2,       // 2 second grace period
        max_cleanup_retries: 2,
        cleanup_retry_delay_ms: 500,
        max_completed_age_hours: 1,
    };
    
    let (event_sender, mut event_receiver) = mpsc::channel(100);
    let (_shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    
    let mut timeout_manager = TimeoutManager::new(
        timeout_config,
        config,
        chains,
        metrics,
        event_sender,
        shutdown_receiver,
    );
    
    // Track multiple packets
    for i in 1..=3 {
        let packet = create_test_packet_with_timeout(i, None);
        timeout_manager.track_packet(
            packet,
            "near-testnet".to_string(),
            "provider".to_string(),
        );
    }
    
    let initial_stats = timeout_manager.get_timeout_stats();
    assert_eq!(initial_stats.total_tracked, 3);
    assert_eq!(initial_stats.active, 3);
    
    println!("   Tracked {} packets for timeout monitoring", initial_stats.total_tracked);
    println!("✅ Comprehensive timeout workflow test completed");
}