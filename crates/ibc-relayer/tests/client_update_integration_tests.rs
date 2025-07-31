// Integration tests for light client update mechanisms
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use anyhow::Result;

use ibc_relayer::relay::{ClientUpdateManager, ClientUpdateConfig, ClientStatus};
use ibc_relayer::chains::{Chain, ChainEvent, IbcPacket};
use ibc_relayer::config::{RelayerConfig, ConnectionConfig};

// Mock chain implementation for testing
#[derive(Debug)]
struct MockChain {
    chain_id: String,
    height: Arc<std::sync::Mutex<u64>>,
    should_fail: Arc<std::sync::Mutex<bool>>,
    call_count: Arc<std::sync::Mutex<u64>>,
}

impl MockChain {
    fn new(chain_id: String, height: u64) -> Self {
        Self {
            chain_id,
            height: Arc::new(std::sync::Mutex::new(height)),
            should_fail: Arc::new(std::sync::Mutex::new(false)),
            call_count: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    fn set_height(&self, new_height: u64) {
        *self.height.lock().unwrap() = new_height;
    }

    fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().unwrap() = fail;
    }

    fn get_call_count(&self) -> u64 {
        *self.call_count.lock().unwrap()
    }

    fn reset_call_count(&self) {
        *self.call_count.lock().unwrap() = 0;
    }
}

#[async_trait::async_trait]
impl Chain for MockChain {
    async fn chain_id(&self) -> String {
        self.chain_id.clone()
    }

    async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        *self.call_count.lock().unwrap() += 1;
        
        if *self.should_fail.lock().unwrap() {
            return Err("Mock chain failure".into());
        }
        
        Ok(*self.height.lock().unwrap())
    }

    async fn query_packet_commitment(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    async fn query_packet_acknowledgment(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    async fn query_packet_receipt(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(false)
    }

    async fn query_next_sequence_recv(
        &self,
        _port_id: &str,
        _channel_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(1)
    }

    async fn get_events(
        &self,
        _from_height: u64,
        _to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }

    async fn subscribe_events(
        &self,
    ) -> Result<
        Box<dyn futures::Stream<Item = ChainEvent> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        Ok(Box::new(futures::stream::empty()))
    }

    async fn submit_transaction(
        &self,
        _data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if *self.should_fail.lock().unwrap() {
            return Err("Mock transaction failure".into());
        }
        Ok(format!("mock_tx_{}", fastrand::u64(1000..9999)))
    }

    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if *self.should_fail.lock().unwrap() {
            return Err("Mock health check failure".into());
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_client_update_manager_initialization() {
    let config = ClientUpdateConfig {
        update_interval: 1, // 1 second for fast testing
        max_client_age_hours: 1,
        max_block_lag: 10,
        enable_pruning: true,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    chains.insert("near-testnet".to_string(), Arc::new(MockChain::new("near-testnet".to_string(), 1000)) as Arc<dyn Chain>);
    chains.insert("cosmoshub-testnet".to_string(), Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 2000)) as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Test initialization
    assert_eq!(manager.client_mappings_count(), 1);
    assert_eq!(manager.chains_count(), 2);
    assert!(manager.has_client_mapping("cosmoshub-testnet"));
    assert_eq!(manager.get_client_id("cosmoshub-testnet").unwrap(), "07-tendermint-0");
}

#[tokio::test]
async fn test_client_status_reporting() {
    let config = ClientUpdateConfig::default();
    
    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 2000));
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let manager = ClientUpdateManager::new(config, chains, client_mappings);
    
    let statuses = manager.get_status().await;
    assert_eq!(statuses.len(), 1);
    
    let status = &statuses[0];
    assert_eq!(status.chain_id, "cosmoshub-testnet");
    assert_eq!(status.client_id, "07-tendermint-0");
    assert_eq!(status.source_height, 2000);
    assert_eq!(status.last_updated_height, 0); // Never updated
    assert!(status.needs_update); // Should need update due to large lag
    assert_eq!(status.block_lag, 2000);
}

#[tokio::test]
async fn test_client_mapping_management() {
    let config = ClientUpdateConfig::default();
    let chains = HashMap::new();
    let client_mappings = HashMap::new();
    
    let mut manager = ClientUpdateManager::new(config, chains, client_mappings);
    
    // Test adding client mappings
    manager.add_client_mapping("test-chain".to_string(), "07-tendermint-1".to_string());
    assert!(manager.has_client_mapping("test-chain"));
    assert_eq!(manager.get_client_id("test-chain").unwrap(), "07-tendermint-1");
    
    // Test removing client mappings
    manager.remove_client_mapping("test-chain");
    assert!(!manager.has_client_mapping("test-chain"));
}

#[tokio::test]
async fn test_client_update_detection() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 50, // Lower threshold for testing
        enable_pruning: false,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 100));
    let near_chain = Arc::new(MockChain::new("near-testnet".to_string(), 1000));
    
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);
    chains.insert("near-testnet".to_string(), near_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let mut manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Initial status - should need update
    let initial_status = manager.get_status().await;
    assert_eq!(initial_status.len(), 1);
    assert!(initial_status[0].needs_update);
    assert_eq!(initial_status[0].block_lag, 100);

    // Simulate updating the client
    manager.set_last_update_height("07-tendermint-0".to_string(), 80);

    let updated_status = manager.get_status().await;
    assert_eq!(updated_status[0].last_updated_height, 80);
    assert_eq!(updated_status[0].block_lag, 20); // 100 - 80
    assert!(!updated_status[0].needs_update); // Within threshold now
}

#[tokio::test]
async fn test_client_update_with_chain_height_changes() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 30,
        enable_pruning: false,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 100));
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let mut manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Set initial client height
    manager.set_last_update_height("07-tendermint-0".to_string(), 90);

    // Chain height is 100, client is at 90, lag is 10 - should not need update
    let status1 = manager.get_status().await;
    assert!(!status1[0].needs_update);
    assert_eq!(status1[0].block_lag, 10);

    // Increase chain height significantly
    cosmos_chain.set_height(150);

    // Now lag is 60 (150 - 90), should need update
    let status2 = manager.get_status().await;
    assert!(status2[0].needs_update);
    assert_eq!(status2[0].block_lag, 60);
    assert_eq!(status2[0].source_height, 150);
}

#[tokio::test]
async fn test_client_update_failure_handling() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 10,
        enable_pruning: false,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 100));
    let near_chain = Arc::new(MockChain::new("near-testnet".to_string(), 1000));
    
    // Make the near chain fail (this would be the target chain for updates)
    near_chain.set_should_fail(true);
    
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);
    chains.insert("near-testnet".to_string(), near_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let mut manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Test that status reporting still works even with chain failures
    let status = manager.get_status().await;
    assert_eq!(status.len(), 1);
    assert!(status[0].needs_update);
    
    // Test that update attempts are resilient to failures
    // In a real test, we'd mock the update_client_if_needed method
    // For now, we verify the chains can be queried correctly
    assert!(cosmos_chain.get_latest_height().await.is_ok());
    assert!(near_chain.get_latest_height().await.is_err()); // Should fail as expected
}

#[tokio::test]
async fn test_client_pruning_logic() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 10,
        enable_pruning: true,
        trust_period_hours: 1, // Very short for testing
    };

    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 100));
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let mut manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Set pruning time to past to trigger pruning check
    let past_time = Instant::now() - Duration::from_secs(7200); // 2 hours ago
    manager.set_last_prune_time("07-tendermint-0".to_string(), past_time);

    // This would normally trigger pruning logic
    // In a real implementation, we'd have more sophisticated pruning tests
    assert!(manager.has_last_prune_time("07-tendermint-0"));
}

#[tokio::test]
async fn test_multiple_clients_management() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 50,
        enable_pruning: false,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    chains.insert("chain-a".to_string(), Arc::new(MockChain::new("chain-a".to_string(), 1000)) as Arc<dyn Chain>);
    chains.insert("chain-b".to_string(), Arc::new(MockChain::new("chain-b".to_string(), 2000)) as Arc<dyn Chain>);
    chains.insert("chain-c".to_string(), Arc::new(MockChain::new("chain-c".to_string(), 1500)) as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("chain-a".to_string(), "07-tendermint-0".to_string());
    client_mappings.insert("chain-b".to_string(), "07-tendermint-1".to_string());
    client_mappings.insert("chain-c".to_string(), "07-tendermint-2".to_string());

    let manager = ClientUpdateManager::new(config, chains, client_mappings);

    let statuses = manager.get_status().await;
    assert_eq!(statuses.len(), 3);

    // Verify each client status
    let chain_a_status = statuses.iter().find(|s| s.chain_id == "chain-a").unwrap();
    let chain_b_status = statuses.iter().find(|s| s.chain_id == "chain-b").unwrap();
    let chain_c_status = statuses.iter().find(|s| s.chain_id == "chain-c").unwrap();

    assert_eq!(chain_a_status.source_height, 1000);
    assert_eq!(chain_b_status.source_height, 2000);
    assert_eq!(chain_c_status.source_height, 1500);

    assert_eq!(chain_a_status.client_id, "07-tendermint-0");
    assert_eq!(chain_b_status.client_id, "07-tendermint-1");
    assert_eq!(chain_c_status.client_id, "07-tendermint-2");

    // All should need updates initially
    assert!(chain_a_status.needs_update);
    assert!(chain_b_status.needs_update);
    assert!(chain_c_status.needs_update);
}

#[tokio::test]
async fn test_client_update_config_from_relayer_config() {
    use ibc_relayer::relay::create_client_update_manager;
    
    // Create a mock relayer config
    let mut relayer_config = RelayerConfig::default();
    relayer_config.connections = vec![
        ConnectionConfig {
            id: "connection-0".to_string(),
            src_chain: "near-testnet".to_string(),
            dst_chain: "cosmoshub-testnet".to_string(),
            src_client_id: "07-tendermint-0".to_string(),
            dst_client_id: "07-near-0".to_string(),
            auto_relay: true,
        },
        ConnectionConfig {
            id: "connection-1".to_string(),
            src_chain: "osmosis-testnet".to_string(),
            dst_chain: "near-testnet".to_string(),
            src_client_id: "07-tendermint-1".to_string(),
            dst_client_id: "07-near-1".to_string(),
            auto_relay: true,
        },
    ];

    let chains = HashMap::new();
    let manager = create_client_update_manager(&relayer_config, chains);

    // Should have created mappings for all unique chains in connections
    assert_eq!(manager.client_mappings_count(), 3); // 3 unique chains (near-testnet appears twice)
    assert!(manager.has_client_mapping("near-testnet"));
    assert!(manager.has_client_mapping("cosmoshub-testnet"));
    assert!(manager.has_client_mapping("osmosis-testnet"));
}

#[tokio::test]
async fn test_client_update_performance() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 10,
        enable_pruning: false,
        trust_period_hours: 24,
    };

    // Create many chains to test performance
    let mut chains = HashMap::new();
    let mut client_mappings = HashMap::new();
    
    for i in 0..100 {
        let chain_id = format!("chain-{}", i);
        let client_id = format!("07-tendermint-{}", i);
        
        chains.insert(chain_id.clone(), Arc::new(MockChain::new(chain_id.clone(), 1000 + i)) as Arc<dyn Chain>);
        client_mappings.insert(chain_id, client_id);
    }

    let manager = ClientUpdateManager::new(config, chains, client_mappings);

    // Test that status reporting for many clients is reasonably fast
    let start = Instant::now();
    let statuses = manager.get_status().await;
    let duration = start.elapsed();
    
    assert_eq!(statuses.len(), 100);
    assert!(duration < Duration::from_millis(500)); // Should complete within 500ms
    
    println!("Status check for 100 clients took: {:?}", duration);
}

#[tokio::test]
async fn test_client_update_concurrent_access() {
    let config = ClientUpdateConfig {
        update_interval: 1,
        max_client_age_hours: 1,
        max_block_lag: 10,
        enable_pruning: false,
        trust_period_hours: 24,
    };

    let mut chains = HashMap::new();
    let cosmos_chain = Arc::new(MockChain::new("cosmoshub-testnet".to_string(), 1000));
    chains.insert("cosmoshub-testnet".to_string(), cosmos_chain.clone() as Arc<dyn Chain>);

    let mut client_mappings = HashMap::new();
    client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

    let manager = Arc::new(ClientUpdateManager::new(config, chains, client_mappings));

    // Test concurrent status checks
    let mut handles = Vec::new();
    for _ in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone.get_status().await
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        let statuses = handle.await.unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].chain_id, "cosmoshub-testnet");
    }
}

#[tokio::test]
async fn test_client_update_edge_cases() {
    let config = ClientUpdateConfig::default();
    
    // Test with empty chains
    let empty_chains = HashMap::new();
    let empty_mappings = HashMap::new();
    let empty_manager = ClientUpdateManager::new(config.clone(), empty_chains, empty_mappings);
    
    let empty_statuses = empty_manager.get_status().await;
    assert_eq!(empty_statuses.len(), 0);
    
    // Test with chain that has zero height
    let mut chains = HashMap::new();
    chains.insert("zero-chain".to_string(), Arc::new(MockChain::new("zero-chain".to_string(), 0)) as Arc<dyn Chain>);
    
    let mut client_mappings = HashMap::new();
    client_mappings.insert("zero-chain".to_string(), "07-tendermint-0".to_string());
    
    let zero_manager = ClientUpdateManager::new(config, chains, client_mappings);
    let zero_statuses = zero_manager.get_status().await;
    
    assert_eq!(zero_statuses.len(), 1);
    assert_eq!(zero_statuses[0].source_height, 0);
    assert_eq!(zero_statuses[0].block_lag, 0); // 0 - 0
    assert!(!zero_statuses[0].needs_update); // No lag, no update needed
}