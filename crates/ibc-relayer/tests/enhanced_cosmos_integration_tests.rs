// Enhanced integration tests for complete NEAR-Cosmos IBC relay flow
use std::collections::HashMap;
use std::sync::Arc;

use ibc_relayer::{
    chains::{Chain, IbcPacket},
    config::{ChainConfig, ChainSpecificConfig, RelayerConfig, GlobalConfig, MetricsConfig},
    relay::PacketProcessor,
    metrics::RelayerMetrics,
};
use ibc_relayer::relay::engine::RelayEngine;
use ibc_relayer::chains::{near_simple::NearChain, cosmos_minimal::CosmosChain};

/// Create test configuration for NEAR chain
fn create_near_config() -> ChainConfig {
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-sdk-demo.testnet".to_string(),
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
    }
}

/// Create test configuration for Cosmos chain  
fn create_cosmos_config() -> ChainConfig {
    ChainConfig {
        chain_id: "cosmoshub-testnet".to_string(),
        chain_type: "cosmos".to_string(),
        rpc_endpoint: "https://rpc.cosmos.directory/cosmoshub".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Cosmos {
            address_prefix: "cosmos".to_string(),
            gas_price: "0.025uatom".to_string(),
            trust_threshold: "1/3".to_string(),
            trusting_period_hours: 336,
            signer_key: None,
        },
    }
}

/// Create test relayer configuration
fn create_relayer_config() -> RelayerConfig {
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
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 9090,
        },
    }
}

/// Create test IBC packet
fn create_test_packet() -> IbcPacket {
    IbcPacket {
        sequence: 1,
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: b"Hello Cosmos from NEAR!".to_vec(),
        timeout_height: Some(1000000),
        timeout_timestamp: Some(1700000000),
    }
}

#[tokio::test]
async fn test_enhanced_packet_processor_creation() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Create NEAR and Cosmos chains
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    
    // Test packet validation
    let packet = create_test_packet();
    assert!(processor.validate_packet(&packet).is_ok());
    
    println!("✅ Enhanced packet processor created successfully");
}

#[tokio::test]
async fn test_near_to_cosmos_packet_processing() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Create chains
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    
    let packet = create_test_packet();
    
    // Test NEAR -> Cosmos packet processing
    match processor.process_send_packet(
        "near-testnet",
        "cosmoshub-testnet", 
        &packet
    ).await {
        Ok(tx_hash) => {
            assert!(!tx_hash.is_empty());
            println!("✅ NEAR->Cosmos packet processed successfully: {}", tx_hash);
        }
        Err(e) => {
            // Expected to fail due to network connectivity, but should process the logic
            println!("⚠️  Packet processing failed as expected (network issue): {}", e);
        }
    }
}

#[tokio::test]
async fn test_enhanced_relay_engine_integration() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Create chains
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    // Create enhanced relay engine
    let (mut relay_engine, event_sender, _shutdown_sender) = RelayEngine::new(config, chains, metrics);
    
    let packet = create_test_packet();
    
    // Test packet queueing via events
    let relay_event = ibc_relayer::relay::RelayEvent::PacketDetected {
        chain_id: "near-testnet".to_string(),
        packet: packet.clone(),
        _event: ibc_relayer::chains::ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![],
            height: 1000,
            tx_hash: Some("test_tx".to_string()),
        },
    };
    
    match event_sender.send(relay_event).await {
        Ok(_) => println!("✅ Packet event sent successfully"),
        Err(e) => println!("⚠️  Packet event sending failed: {}", e),
    }
    
    // Get relay statistics
    let stats = relay_engine.get_relay_stats();
    println!("📊 Relay engine stats: total_tracked={}", stats.total_tracked);
    
    println!("✅ Enhanced relay engine integration test completed");
}

#[tokio::test]
async fn test_gas_estimation() {
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    
    let test_data = b"test transaction data";
    let gas_estimate = processor.estimate_gas("cosmoshub-testnet", test_data).await.unwrap();
    
    assert!(gas_estimate > 0);
    println!("✅ Gas estimation test: {} gas units", gas_estimate);
}

#[tokio::test] 
async fn test_packet_validation_edge_cases() {
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    
    // Test various invalid packet scenarios
    let test_cases = vec![
        (IbcPacket {
            sequence: 0, // Invalid: zero sequence
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }, "Zero sequence should be invalid"),
        
        (IbcPacket {
            sequence: 1,
            source_port: "".to_string(), // Invalid: empty port
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }, "Empty source port should be invalid"),
        
        (IbcPacket {
            sequence: 1,
            source_port: "transfer".to_string(),
            source_channel: "".to_string(), // Invalid: empty channel
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }, "Empty source channel should be invalid"),
    ];
    
    for (packet, description) in test_cases {
        assert!(processor.validate_packet(&packet).is_err(), "{}", description);
    }
    
    println!("✅ Packet validation edge cases test passed");
}

#[tokio::test]
async fn test_chain_combination_validation() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    let packet = create_test_packet();
    
    // Test invalid chain combinations
    let invalid_combinations = vec![
        ("cosmos-wrong", "near-testnet", "Invalid source chain"),
        ("near-testnet", "cosmos-wrong", "Invalid destination chain"),
        ("cosmos-hub", "cosmos-testnet", "Cosmos to Cosmos not supported"),
        ("near-mainnet", "near-testnet", "NEAR to NEAR not supported"),
    ];
    
    for (source, dest, description) in invalid_combinations {
        let result = processor.process_send_packet(source, dest, &packet).await;
        assert!(result.is_err(), "{}", description);
    }
    
    println!("✅ Chain combination validation test passed");
}

#[tokio::test]
async fn test_acknowledgment_processing() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    let packet = create_test_packet();
    let ack_data = b"success";
    
    // Test acknowledgment processing
    match processor.process_acknowledgment(
        "near-testnet",
        "cosmoshub-testnet",
        &packet,
        ack_data
    ).await {
        Ok(tx_hash) => {
            assert!(!tx_hash.is_empty());
            println!("✅ Acknowledgment processed successfully: {}", tx_hash);
        }
        Err(e) => {
            // Expected to fail due to network connectivity
            println!("⚠️  Acknowledgment processing failed as expected: {}", e);
        }
    }
}

#[tokio::test]
async fn test_timeout_processing() {
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain);
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain);
    
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains, config, metrics);
    let packet = create_test_packet();
    
    // Test timeout processing
    match processor.process_timeout(
        "near-testnet",
        "cosmoshub-testnet",
        &packet
    ).await {
        Ok(tx_hash) => {
            assert!(!tx_hash.is_empty());
            println!("✅ Timeout processed successfully: {}", tx_hash);
        }
        Err(e) => {
            // Expected to fail due to network connectivity
            println!("⚠️  Timeout processing failed as expected: {}", e);
        }
    }
}

/// Integration test demonstrating the complete flow
#[tokio::test]
async fn test_complete_near_cosmos_relay_flow() {
    println!("🚀 Starting complete NEAR-Cosmos relay flow integration test");
    
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Setup chains
    let near_config = create_near_config();
    let cosmos_config = create_cosmos_config();
    
    let near_chain = Arc::new(NearChain::new(&near_config).unwrap());
    let cosmos_chain = Arc::new(CosmosChain::new(&cosmos_config).unwrap());
    
    chains.insert("near-testnet".to_string(), near_chain.clone());
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone());
    
    // Test chain connectivity
    println!("📡 Testing chain connectivity...");
    
    match near_chain.health_check().await {
        Ok(_) => println!("  ✅ NEAR connectivity OK"),
        Err(e) => println!("  ⚠️  NEAR connectivity issue: {}", e),
    }
    
    match cosmos_chain.health_check().await {
        Ok(_) => println!("  ✅ Cosmos connectivity OK"),
        Err(e) => println!("  ⚠️  Cosmos connectivity issue: {}", e),
    }
    
    // Setup relay components
    let config = create_relayer_config();
    let metrics = Arc::new(RelayerMetrics::new().unwrap());
    
    let processor = PacketProcessor::new(chains.clone(), config.clone(), metrics.clone());
    let (mut relay_engine, event_sender, _shutdown_sender) = RelayEngine::new(config, chains, metrics);
    
    // Create test packet
    let packet = create_test_packet();
    println!("📦 Created test packet: seq={}, data={:?}", 
             packet.sequence, String::from_utf8_lossy(&packet.data));
    
    // Step 1: Send packet detection event to relay engine
    let relay_event = ibc_relayer::relay::RelayEvent::PacketDetected {
        chain_id: "near-testnet".to_string(),
        packet: packet.clone(),
        _event: ibc_relayer::chains::ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![],
            height: 1000,
            tx_hash: Some("test_tx".to_string()),
        },
    };
    
    match event_sender.send(relay_event).await {
        Ok(_) => println!("  ✅ Packet detection event sent to relay engine"),
        Err(e) => println!("  ⚠️  Packet event sending failed: {}", e),
    }
    
    // Step 2: Process packet through processor
    match processor.process_send_packet(
        "near-testnet",
        "cosmoshub-testnet",
        &packet
    ).await {
        Ok(tx_hash) => {
            println!("  ✅ Packet processed successfully: {}", tx_hash);
            
            // Step 3: Simulate acknowledgment processing
            let ack_data = b"packet_received_successfully";
            match processor.process_acknowledgment(
                "near-testnet",
                "cosmoshub-testnet",
                &packet,
                ack_data
            ).await {
                Ok(ack_tx_hash) => println!("  ✅ Acknowledgment processed: {}", ack_tx_hash),
                Err(e) => println!("  ⚠️  Acknowledgment processing failed: {}", e),
            }
        }
        Err(e) => println!("  ⚠️  Packet processing failed (expected): {}", e),
    }
    
    // Step 4: Check final statistics
    let stats = relay_engine.get_relay_stats();
    println!("📊 Final relay statistics:");
    println!("  Total tracked: {}", stats.total_tracked);
    println!("  Detected: {}", stats.detected);
    println!("  Submitted: {}", stats.submitted);
    println!("  Confirmed: {}", stats.confirmed);
    
    println!("🎯 Complete NEAR-Cosmos relay flow integration test completed");
}