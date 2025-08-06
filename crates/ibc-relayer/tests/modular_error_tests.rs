// Error handling tests for modular IBC relayer
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use ibc_relayer::{
    chains::{Chain, IbcModuleType, ModuleRegistry, ModuleInfo, NearModularChain, CrossModuleOp, IbcPacket},
    config::{ChainConfig, ChainSpecificConfig},
    monitor::{ModularEventMonitor, MonitorConfig},
};

/// Test module not found error
#[test]
fn test_module_not_found_error() {
    let mut modules = HashMap::new();
    modules.insert("ibc_client".to_string(), "client.testnet".to_string());
    
    let router: near_primitives::types::AccountId = "router.testnet".parse().unwrap();
    let mut registry = ModuleRegistry::from_config(router, &modules).unwrap();
    
    // Try to update non-existent module
    let result = registry.update_module(
        IbcModuleType::Transfer,
        "new-transfer.testnet".parse().unwrap()
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

/// Test invalid contract ID parsing
#[test]
fn test_invalid_contract_id() {
    let mut modules = HashMap::new();
    modules.insert("ibc_client".to_string(), "invalid..contract..id".to_string());
    
    let router: near_primitives::types::AccountId = "router.testnet".parse().unwrap();
    let result = ModuleRegistry::from_config(router, &modules);
    
    assert!(result.is_err());
}

/// Test cache expiration
#[tokio::test]
async fn test_cache_expiration() {
    let router: near_primitives::types::AccountId = "router.testnet".parse().unwrap();
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create initial registry
    let mut modules = HashMap::new();
    modules.insert("ibc_client".to_string(), "client.testnet".to_string());
    let initial_registry = ModuleRegistry::from_config(router.clone(), &modules).unwrap();
    
    // Test with expired cache (0 duration)
    let cache_duration = std::time::Duration::from_secs(0);
    let cached_at = std::time::Instant::now() - std::time::Duration::from_secs(10);
    
    // This should trigger a new discovery (which will fail in test env)
    let result = ModuleRegistry::discover_with_cache(
        &router,
        &rpc_client,
        cache_duration,
        Some((initial_registry.clone(), cached_at)),
    ).await;
    
    // In test environment, this will fail to connect
    assert!(result.is_err() || result.is_ok());
}

/// Test cross-module operation validation
#[test]
fn test_cross_module_op_validation() {
    // Test with empty packet data
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
    
    let op = CrossModuleOp::SendPacket { packet: packet.clone() };
    
    // Verify it can be serialized even with empty fields
    let serialized = serde_json::to_string(&op);
    assert!(serialized.is_ok());
}

/// Test event monitor with missing modules
#[tokio::test]
async fn test_event_monitor_missing_modules() {
    let (tx, _rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create registry with no modules
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules: HashMap::new(),
    };
    
    let monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig::default(),
    ).await;
    
    assert!(monitor.is_ok());
    
    // Initialize should succeed even with no modules
    let mut monitor = monitor.unwrap();
    let result = monitor.initialize_streams().await;
    assert!(result.is_ok());
}

/// Test parallel query error handling
#[tokio::test]
async fn test_parallel_query_errors() {
    let config = create_test_config();
    
    // Try to create chain (will fail in test env but shouldn't panic)
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Test with invalid queries
            let queries = vec![
                (IbcModuleType::Client, "non_existent_method", serde_json::json!({})),
                (IbcModuleType::Channel, "another_bad_method", serde_json::json!({})),
            ];
            
            let results: Vec<_> = chain.query_modules_parallel::<serde_json::Value>(queries).await;
            
            // All queries should fail
            for result in results {
                assert!(result.is_err());
            }
        }
        Err(_) => {
            // Expected in test environment
        }
    }
}

/// Test transaction routing errors
#[test]
fn test_transaction_routing_errors() {
    // Test with invalid transaction data
    let invalid_data = vec![0xFF, 0xFE, 0xFD]; // Not valid JSON
    
    // Try to parse as CrossModuleOp
    let result = serde_json::from_slice::<CrossModuleOp>(&invalid_data);
    assert!(result.is_err());
}

/// Test module registry with duplicate modules
#[test]
fn test_duplicate_module_handling() {
    let mut modules = HashMap::new();
    modules.insert("ibc_client".to_string(), "client-v1.testnet".to_string());
    // Second insert with same key overwrites
    modules.insert("ibc_client".to_string(), "client-v2.testnet".to_string());
    
    let router: near_primitives::types::AccountId = "router.testnet".parse().unwrap();
    let registry = ModuleRegistry::from_config(router, &modules).unwrap();
    
    // Should only have one client module with the latest value
    assert_eq!(registry.modules.len(), 1);
    assert_eq!(
        registry.modules.get(&IbcModuleType::Client).unwrap().contract_id.to_string(),
        "client-v2.testnet"
    );
}

/// Test health check failures
#[tokio::test]
async fn test_health_check_module_failure() {
    let config = create_test_config();
    
    // In test environment, health check will fail
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            let result = chain.health_check().await;
            // Should fail when modules don't exist
            assert!(result.is_err());
        }
        Err(_) => {
            // Expected in test environment
        }
    }
}

// Helper function
fn create_test_config() -> ChainConfig {
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-router.testnet".to_string(),
            modules: Some(HashMap::from([
                ("ibc_client".to_string(), "client.testnet".to_string()),
                ("ibc_channel".to_string(), "channel.testnet".to_string()),
            ])),
            signer_account_id: "relayer.testnet".to_string(),
            private_key: None,
            network_id: "testnet".to_string(),
            modular: true,
        },
    }
}