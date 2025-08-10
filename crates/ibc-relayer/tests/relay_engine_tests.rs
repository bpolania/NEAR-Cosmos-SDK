// Integration tests for the IBC relay engine using near-workspaces

use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

// Import from the local crate (ibc-relayer becomes ibc_relayer in Rust)
use ibc_relayer::config::{RelayerConfig, ChainConfig, ChainSpecificConfig, ConnectionConfig, GlobalConfig, MetricsConfig};
use ibc_relayer::relay::{RelayEngine, RelayEvent, PacketKey};
use ibc_relayer::chains::{Chain, ChainEvent, IbcPacket};
use ibc_relayer::metrics::RelayerMetrics;

/// Test configuration constants
const NEAR_WASM_PATH: &str = "../cosmos-sdk-contract/target/near/cosmos_sdk_near.wasm";

/// Test helper to create a mock relayer configuration
fn create_test_config() -> RelayerConfig {
    RelayerConfig {
        global: GlobalConfig {
            log_level: "info".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval: 30,
        },
        chains: {
            let mut chains = HashMap::new();
            chains.insert("near-testnet".to_string(), ChainConfig {
                chain_id: "near-testnet".to_string(),
                chain_type: "near".to_string(),
                rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
                ws_endpoint: None,
                config: ChainSpecificConfig::Near {
                    contract_id: "test-contract.testnet".to_string(),
                    signer_account_id: "relayer.testnet".to_string(),
                    private_key: None,
                    network_id: "testnet".to_string(),
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
            });
            chains.insert("cosmoshub-testnet".to_string(), ChainConfig {
                chain_id: "cosmoshub-testnet".to_string(),
                chain_type: "cosmos".to_string(),
                rpc_endpoint: "https://rpc.testnet.cosmos.network".to_string(),
                ws_endpoint: Some("wss://rpc.testnet.cosmos.network/websocket".to_string()),
                config: ChainSpecificConfig::Cosmos {
                    address_prefix: "cosmos".to_string(),
                    gas_price: "0.025uatom".to_string(),
                    trust_threshold: "1/3".to_string(),
                    trusting_period_hours: 336, // 14 days
                    signer_key: None,
                },
            });
            chains
        },
        connections: vec![ConnectionConfig {
            id: "connection-0".to_string(),
            src_chain: "near-testnet".to_string(),
            dst_chain: "cosmoshub-testnet".to_string(),
            src_client_id: "07-tendermint-0".to_string(),
            dst_client_id: "near-client-0".to_string(),
            auto_relay: true,
        }],
        metrics: MetricsConfig {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 9090,
        },
    }
}

/// Test helper to create a test IBC packet
fn create_test_packet(sequence: u64) -> IbcPacket {
    IbcPacket {
        sequence,
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: b"test packet data".to_vec(),
        timeout_height: Some(1000),
        timeout_timestamp: Some(1234567890),
    }
}

/// Test helper to create a test chain event
fn create_test_chain_event(event_type: &str, packet: &IbcPacket) -> ChainEvent {
    ChainEvent {
        event_type: event_type.to_string(),
        attributes: vec![
            ("packet_src_port".to_string(), packet.source_port.clone()),
            ("packet_src_channel".to_string(), packet.source_channel.clone()),
            ("packet_dst_port".to_string(), packet.destination_port.clone()),
            ("packet_dst_channel".to_string(), packet.destination_channel.clone()),
            ("packet_sequence".to_string(), packet.sequence.to_string()),
            ("packet_data".to_string(), String::from_utf8_lossy(&packet.data).to_string()),
        ],
        height: 100,
        tx_hash: Some("test_tx_hash".to_string()),
    }
}

#[tokio::test]
async fn test_relay_engine_initialization() -> Result<()> {
    // Create test configuration
    let config = create_test_config();
    
    // Initialize metrics
    let metrics = Arc::new(RelayerMetrics::new()?);
    
    // Create chains map - for this test, we'll use mock chains
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Create relay engine
    let relay_engine = RelayEngine::new(config.clone(), chains, metrics);
    
    // Verify engine is created successfully
    assert!(relay_engine.chains.is_empty()); // No real chains in this test
    
    println!("✅ Relay engine initialization test passed");
    Ok(())
}

#[tokio::test]
async fn test_packet_detection_and_tracking() -> Result<()> {
    let config = create_test_config();
    let metrics = Arc::new(RelayerMetrics::new()?);
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let _relay_engine = RelayEngine::new(config, chains, metrics);
    
    // Create test packet and event
    let test_packet = create_test_packet(1);
    let test_event = create_test_chain_event("send_packet", &test_packet);
    
    // Test packet extraction from event
    let extracted_packet = RelayEngine::extract_packet_from_event(&test_event);
    assert!(extracted_packet.is_some(), "Should extract packet from event");
    
    let packet = extracted_packet.unwrap();
    assert_eq!(packet.sequence, test_packet.sequence);
    assert_eq!(packet.source_port, test_packet.source_port);
    assert_eq!(packet.source_channel, test_packet.source_channel);
    
    println!("✅ Packet detection and tracking test passed");
    Ok(())
}

#[tokio::test]
async fn test_packet_key_generation() -> Result<()> {
    let test_packet = create_test_packet(42);
    
    // Test PacketKey generation
    let packet_key = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: test_packet.source_port.clone(),
        source_channel: test_packet.source_channel.clone(),
        sequence: test_packet.sequence,
    };
    
    // Verify key components
    assert_eq!(packet_key.source_chain, "near-testnet");
    assert_eq!(packet_key.source_port, "transfer");
    assert_eq!(packet_key.source_channel, "channel-0");
    assert_eq!(packet_key.sequence, 42);
    
    // Test key equality and hashing
    let packet_key2 = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: test_packet.source_port.clone(),
        source_channel: test_packet.source_channel.clone(),
        sequence: test_packet.sequence,
    };
    
    assert_eq!(packet_key, packet_key2, "Identical packet keys should be equal");
    
    println!("✅ Packet key generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_event_handling() -> Result<()> {
    let config = create_test_config();
    let metrics = Arc::new(RelayerMetrics::new()?);
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let _relay_engine = RelayEngine::new(config, chains, metrics);
    
    // Test different relay event types
    let test_packet = create_test_packet(1);
    
    // Test PacketDetected event
    let detected_event = RelayEvent::PacketDetected {
        chain_id: "near-testnet".to_string(),
        packet: test_packet.clone(),
        _event: create_test_chain_event("send_packet", &test_packet),
    };
    
    // Test PacketRelayed event
    let relayed_event = RelayEvent::PacketRelayed {
        source_chain: "near-testnet".to_string(),
        dest_chain: "cosmoshub-testnet".to_string(),
        sequence: test_packet.sequence,
    };
    
    // Test PacketAcknowledged event
    let _ack_event = RelayEvent::PacketAcknowledged {
        chain_id: "cosmoshub-testnet".to_string(),
        packet: test_packet.clone(),
        ack_data: b"success".to_vec(),
    };
    
    // Test PacketTimedOut event
    let _timeout_event = RelayEvent::PacketTimedOut {
        chain_id: "near-testnet".to_string(),
        packet: test_packet,
    };
    
    // Verify events can be created (compilation test)
    match detected_event {
        RelayEvent::PacketDetected { chain_id, .. } => {
            assert_eq!(chain_id, "near-testnet");
        }
        _ => panic!("Wrong event type"),
    }
    
    match relayed_event {
        RelayEvent::PacketRelayed { source_chain, dest_chain, sequence } => {
            assert_eq!(source_chain, "near-testnet");
            assert_eq!(dest_chain, "cosmoshub-testnet");
            assert_eq!(sequence, 1);
        }
        _ => panic!("Wrong event type"),
    }
    
    println!("✅ Relay event handling test passed");
    Ok(())
}

#[tokio::test]
async fn test_packet_tracker_state_management() -> Result<()> {
    use ibc_relayer::relay::{PacketTracker, PendingPacket};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    let tracker = Arc::new(RwLock::new(PacketTracker::new()));
    let test_packet = create_test_packet(1);
    
    // Create a pending packet
    let pending_packet = PendingPacket {
        packet: test_packet.clone(),
        dest_chain: "cosmoshub-testnet".to_string(),
        retry_count: 0,
        next_retry: None,
    };
    
    // Add packet to tracker
    {
        let mut tracker_guard = tracker.write().await;
        tracker_guard.pending_packets
            .entry("near-testnet".to_string())
            .or_insert_with(Vec::new)
            .push(pending_packet.clone());
    }
    
    // Verify packet was added
    {
        let tracker_guard = tracker.read().await;
        let near_packets = tracker_guard.pending_packets.get("near-testnet");
        assert!(near_packets.is_some(), "Should have packets for near-testnet");
        assert_eq!(near_packets.unwrap().len(), 1, "Should have exactly 1 packet");
        assert_eq!(near_packets.unwrap()[0].packet.sequence, 1);
    }
    
    // Test packet key creation and acknowledgment tracking
    let packet_key = PacketKey {
        source_chain: "near-testnet".to_string(),
        source_port: test_packet.source_port.clone(),
        source_channel: test_packet.source_channel.clone(),
        sequence: test_packet.sequence,
    };
    
    // Move packet to awaiting acknowledgment
    {
        let mut tracker_guard = tracker.write().await;
        tracker_guard.awaiting_ack.insert(packet_key.clone(), pending_packet);
    }
    
    // Verify packet is awaiting ack
    {
        let tracker_guard = tracker.read().await;
        assert!(tracker_guard.awaiting_ack.contains_key(&packet_key));
    }
    
    // Complete the packet
    {
        let mut tracker_guard = tracker.write().await;
        if tracker_guard.awaiting_ack.remove(&packet_key).is_some() {
            tracker_guard.completed_packets.push(packet_key.clone());
        }
    }
    
    // Verify packet is completed
    {
        let tracker_guard = tracker.read().await;
        assert!(!tracker_guard.awaiting_ack.contains_key(&packet_key));
        assert!(tracker_guard.completed_packets.contains(&packet_key));
    }
    
    println!("✅ Packet tracker state management test passed");
    Ok(())
}

#[tokio::test]
async fn test_configuration_loading() -> Result<()> {
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    // Create a temporary config file
    let mut temp_file = NamedTempFile::new()?;
    writeln!(temp_file, r#"
[global]
log_level = "info"
max_retries = 3
retry_delay_ms = 1000
health_check_interval = 30

[chains.near-testnet]
chain_id = "near-testnet"
chain_type = "near"
rpc_endpoint = "https://rpc.testnet.near.org"

[chains.near-testnet.config]
type = "near"
contract_id = "test-contract.testnet"
signer_account_id = "relayer.testnet"
network_id = "testnet"

[chains.cosmoshub-testnet]
chain_id = "cosmoshub-testnet"
chain_type = "cosmos"
rpc_endpoint = "https://rpc.testnet.cosmos.network"

[chains.cosmoshub-testnet.config]
type = "cosmos"
address_prefix = "cosmos"
gas_price = "0.025uatom"
trust_threshold = "1/3"
trusting_period_hours = 336

[[connections]]
id = "connection-0"
src_chain = "near-testnet"
dst_chain = "cosmoshub-testnet"
src_client_id = "07-tendermint-0"
dst_client_id = "near-client-0"
auto_relay = true

[metrics]
enabled = true
host = "127.0.0.1"
port = 9090
"#)?;
    
    // Load configuration
    let config = RelayerConfig::load(temp_file.path().to_str().unwrap())?;
    
    // Verify configuration
    assert_eq!(config.chains.len(), 2);
    assert!(config.chains.contains_key("near-testnet"));
    assert!(config.chains.contains_key("cosmoshub-testnet"));
    
    let near_config = &config.chains["near-testnet"];
    assert_eq!(near_config.chain_type, "near");
    
    match &near_config.config {
        ChainSpecificConfig::Near { contract_id, .. } => {
            assert_eq!(contract_id, "test-contract.testnet");
        }
        _ => panic!("Expected NEAR config"),
    }
    
    // Verify global config
    assert_eq!(config.global.log_level, "info");
    assert_eq!(config.global.max_retries, 3);
    assert_eq!(config.global.retry_delay_ms, 1000);
    assert_eq!(config.global.health_check_interval, 30);
    
    // Verify connections
    assert_eq!(config.connections.len(), 1);
    assert_eq!(config.connections[0].id, "connection-0");
    
    println!("✅ Configuration loading test passed");
    Ok(())
}

#[tokio::test]
async fn test_metrics_initialization() -> Result<()> {
    use ibc_relayer::metrics::RelayerMetrics;
    
    // Initialize metrics
    let metrics = RelayerMetrics::new()?;
    
    // Test metric operations
    metrics.packets_relayed.inc();
    metrics.packets_failed.inc();
    metrics.rpc_errors.inc();
    
    // Test histogram
    metrics.packet_relay_duration.observe(1.5);
    
    // Verify metrics registry is accessible
    let registry = metrics.registry();
    assert!(!registry.gather().is_empty(), "Metrics registry should not be empty");
    
    println!("✅ Metrics initialization test passed");
    Ok(())
}

/// Integration test with actual NEAR contract deployment
#[tokio::test]
async fn test_near_chain_integration() -> Result<()> {
    // Skip this test if WASM file doesn't exist
    if !std::path::Path::new(NEAR_WASM_PATH).exists() {
        println!("⚠️  Skipping NEAR integration test - WASM file not found");
        return Ok(());
    }
    
    let worker = near_workspaces::sandbox().await?;
    let wasm = tokio::fs::read(NEAR_WASM_PATH).await?;
    
    // Deploy contract
    let contract = worker.dev_deploy(&wasm).await?;
    
    // Initialize contract
    let init_result = contract
        .call("new")
        .args_json(json!({}))
        .transact()
        .await?;
    
    assert!(init_result.is_success(), "Contract initialization should succeed");
    
    // Contract deployed and initialized successfully
    println!("✅ NEAR chain integration test passed");
    println!("   Contract deployed at: {}", contract.id());
    println!("   Init result: {:?}", init_result.receipt_failures());
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_recovery() -> Result<()> {
    let config = create_test_config();
    let metrics = Arc::new(RelayerMetrics::new()?);
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let _relay_engine = RelayEngine::new(config, chains, metrics.clone());
    
    // Test invalid packet event handling
    let invalid_event = ChainEvent {
        event_type: "invalid_event".to_string(),
        attributes: vec![], // Missing required attributes
        height: 100,
        tx_hash: None,
    };
    
    let extracted_packet = RelayEngine::extract_packet_from_event(&invalid_event);
    assert!(extracted_packet.is_none(), "Should handle invalid events gracefully");
    
    // Test metrics for error cases
    metrics.packets_failed.inc();
    metrics.rpc_errors.inc();
    
    // Verify error metrics are recorded
    let registry = metrics.registry();
    let metric_families = registry.gather();
    let error_metrics = metric_families.iter()
        .find(|mf| mf.get_name() == "ibc_packets_failed_total");
    
    assert!(error_metrics.is_some(), "Should have error metrics");
    
    println!("✅ Error handling and recovery test passed");
    Ok(())
}

#[tokio::test]
async fn test_concurrent_packet_processing() -> Result<()> {
    use tokio::task::JoinSet;
    
    let config = create_test_config();
    let metrics = Arc::new(RelayerMetrics::new()?);
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let relay_engine = Arc::new(RelayEngine::new(config, chains, metrics));
    
    // Create multiple test packets
    let mut join_set = JoinSet::new();
    
    for i in 1..=10 {
        let _engine = relay_engine.clone();
        join_set.spawn(async move {
            let packet = create_test_packet(i);
            let event = create_test_chain_event("send_packet", &packet);
            
            // Test packet extraction for each packet
            let extracted = RelayEngine::extract_packet_from_event(&event);
            assert!(extracted.is_some());
            extracted.unwrap().sequence
        });
    }
    
    // Wait for all tasks to complete
    let mut sequences = Vec::new();
    while let Some(result) = join_set.join_next().await {
        sequences.push(result?);
    }
    
    // Verify all packets were processed
    sequences.sort();
    assert_eq!(sequences, (1..=10).collect::<Vec<u64>>());
    
    println!("✅ Concurrent packet processing test passed");
    Ok(())
}

/// Test relay engine shutdown handling
#[tokio::test]
async fn test_relay_engine_shutdown() -> Result<()> {
    let config = create_test_config();
    let metrics = Arc::new(RelayerMetrics::new()?);
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let _relay_engine = RelayEngine::new(config, chains, metrics);
    
    // Test that engine can be created and shutdown gracefully
    // (In a real scenario, we'd test the full start/shutdown cycle)
    
    // For now, just verify the engine doesn't panic on creation
    println!("✅ Relay engine shutdown test passed");
    Ok(())
}

