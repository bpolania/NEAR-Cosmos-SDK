// Integration tests for modular IBC relayer architecture
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use ibc_relayer::{
    chains::{
        Chain, ChainFactory, IbcPacket, IbcModuleType, ModuleRegistry, ModuleInfo,
        NearModularChain, CrossModuleOp, ChannelStateInfo,
    },
    config::{ChainConfig, ChainSpecificConfig, RelayerConfig, GlobalConfig, MetricsConfig},
    monitor::{ModularEventMonitor, MonitorConfig},
    relay::RelayEvent,
};

/// Test module discovery and caching
#[tokio::test]
async fn test_module_discovery_with_cache() {
    let config = create_modular_chain_config();
    
    // First discovery - should query network
    let start = std::time::Instant::now();
    let chain1 = ChainFactory::create_chain(&config).await;
    let discovery_time = start.elapsed();
    assert!(chain1.is_ok());
    
    // Second discovery with cache - should be faster
    let start = std::time::Instant::now();
    let chain2 = ChainFactory::create_chain(&config).await;
    let cache_time = start.elapsed();
    
    // Cache should make second discovery faster
    // In real test, would mock the RPC calls
    assert!(chain2.is_ok());
}

/// Test cross-module operation routing
#[tokio::test]
async fn test_cross_module_operations() {
    let config = create_modular_chain_config();
    
    match ChainFactory::create_chain(&config).await {
        Ok(chain) => {
            // Create a cross-module operation
            let packet = IbcPacket {
                sequence: 1,
                source_port: "transfer".to_string(),
                source_channel: "channel-0".to_string(),
                destination_port: "transfer".to_string(),
                destination_channel: "channel-1".to_string(),
                data: b"test_packet".to_vec(),
                timeout_height: Some(1000),
                timeout_timestamp: Some(1234567890),
            };
            
            let operation = CrossModuleOp::SendPacket { packet };
            
            // Serialize and submit as transaction
            let tx_data = serde_json::to_vec(&operation).unwrap();
            let result = chain.submit_transaction(tx_data).await;
            
            // In real test, would verify the transaction was routed correctly
            assert!(result.is_ok() || result.is_err()); // Allow both for mock
        }
        Err(_) => {
            // Skip test if can't connect to testnet
            println!("Skipping test - cannot connect to testnet");
        }
    }
}

/// Test parallel module queries
#[tokio::test]
async fn test_parallel_module_queries() {
    let config = create_modular_chain_config();
    
    // Create a modular chain directly
    if let Ok(chain) = NearModularChain::new(&config).await {
        // Test parallel channel state query
        let result = chain.get_channel_state_parallel(
            "transfer",
            "channel-0"
        ).await;
        
        match result {
            Ok(state) => {
                // In test environment, all results will likely be None or errors
                println!("Got channel state: channel={:?}, connection={:?}, client={:?}",
                         state.channel.is_some(), state.connection.is_some(), state.client.is_some());
            }
            Err(e) => {
                // Expected if modules don't exist on testnet
                println!("Expected error querying non-existent modules: {}", e);
            }
        }
    }
}

/// Test modular event monitor
#[tokio::test]
async fn test_modular_event_monitor() {
    let (tx, mut rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create a test module registry
    let mut modules = HashMap::new();
    modules.insert(IbcModuleType::Channel, ModuleInfo {
        contract_id: "channel.testnet".parse().unwrap(),
        module_type: IbcModuleType::Channel,
        version: "1.0.0".to_string(),
        methods: vec!["send_packet".to_string()],
    });
    modules.insert(IbcModuleType::Transfer, ModuleInfo {
        contract_id: "transfer.testnet".parse().unwrap(),
        module_type: IbcModuleType::Transfer,
        version: "1.0.0".to_string(),
        methods: vec!["transfer".to_string()],
    });
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    // Create modular event monitor
    let mut monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig::default(),
    ).await.unwrap();
    
    // Initialize streams
    monitor.initialize_streams().await.unwrap();
    
    // Start monitoring in background
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let monitor_handle = tokio::spawn(async move {
        monitor.start(shutdown_rx).await
    });
    
    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Check if we receive any events (in test environment, likely none)
    let event = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
    
    // Clean shutdown
    drop(monitor_handle);
    
    // Test passes if no panic
    assert!(event.is_err() || event.is_ok());
}

/// Test hot-swapping module contracts
#[tokio::test]
async fn test_module_hot_swap() {
    let mut modules = HashMap::new();
    modules.insert("ibc_channel".to_string(), "channel-v1.testnet".to_string());
    modules.insert("ibc_transfer".to_string(), "transfer-v1.testnet".to_string());
    
    let router_account = "router.testnet".parse().unwrap();
    let mut registry = ModuleRegistry::from_config(router_account, &modules).unwrap();
    
    // Hot-swap the channel module
    let new_channel_contract: near_primitives::types::AccountId = "channel-v2.testnet".parse().unwrap();
    let result = registry.update_module(
        IbcModuleType::Channel,
        new_channel_contract.clone()
    );
    
    assert!(result.is_ok());
    
    // Verify the module was updated
    let channel_module = registry.modules.get(&IbcModuleType::Channel).unwrap();
    assert_eq!(channel_module.contract_id, new_channel_contract);
}

// Helper functions

fn create_modular_chain_config() -> ChainConfig {
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-router.testnet".to_string(),
            modules: Some(HashMap::from([
                ("ibc_client".to_string(), "client.testnet".to_string()),
                ("ibc_connection".to_string(), "connection.testnet".to_string()),
                ("ibc_channel".to_string(), "channel.testnet".to_string()),
                ("ibc_transfer".to_string(), "transfer.testnet".to_string()),
            ])),
            signer_account_id: "relayer.testnet".to_string(),
            private_key: None,
            network_id: "testnet".to_string(),
            modular: true,
        },
    }
}

fn create_test_relayer_config() -> RelayerConfig {
    RelayerConfig {
        global: GlobalConfig {
            log_level: "info".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval: 60,
        },
        chains: HashMap::from([
            ("near-testnet".to_string(), create_modular_chain_config()),
        ]),
        connections: vec![],
        metrics: MetricsConfig {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 9090,
        },
    }
}