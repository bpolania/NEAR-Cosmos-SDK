// Light client update mechanisms for automatic header submission and client management
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};
use serde_json::json;
use base64::Engine;

use crate::chains::{Chain, ChainEvent};
use crate::config::RelayerConfig;

/// Configuration for light client updates
#[derive(Debug, Clone)]
pub struct ClientUpdateConfig {
    /// How often to check for new headers (seconds)
    pub update_interval: u64,
    /// Maximum age of client state before requiring update (hours)
    pub max_client_age_hours: u64,
    /// How many blocks behind the chain tip to keep client updated
    pub max_block_lag: u64,
    /// Enable automatic consensus state pruning
    pub enable_pruning: bool,
    /// Consensus state trust period (hours)
    pub trust_period_hours: u64,
}

impl Default for ClientUpdateConfig {
    fn default() -> Self {
        Self {
            update_interval: 60, // Check every minute
            max_client_age_hours: 2, // Update if client is 2 hours behind
            max_block_lag: 100, // Keep within 100 blocks of chain tip
            enable_pruning: true,
            trust_period_hours: 336, // 14 days default trust period
        }
    }
}

/// Light client update manager
pub struct ClientUpdateManager {
    config: ClientUpdateConfig,
    chains: HashMap<String, Arc<dyn Chain>>,
    client_mappings: HashMap<String, String>, // chain_id -> client_id
    last_update_heights: HashMap<String, u64>, // client_id -> last_updated_height
    last_prune_time: HashMap<String, Instant>, // client_id -> last_pruned_time
}

impl ClientUpdateManager {
    /// Create a new client update manager
    pub fn new(
        config: ClientUpdateConfig,
        chains: HashMap<String, Arc<dyn Chain>>,
        client_mappings: HashMap<String, String>,
    ) -> Self {
        Self {
            config,
            chains,
            client_mappings,
            last_update_heights: HashMap::new(),
            last_prune_time: HashMap::new(),
        }
    }

    /// Start the automatic client update service
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔄 Starting automatic light client update service");
        info!("📊 Update interval: {}s, Max client age: {}h, Max block lag: {}", 
              self.config.update_interval, self.config.max_client_age_hours, self.config.max_block_lag);

        let mut update_interval = interval(Duration::from_secs(self.config.update_interval));

        loop {
            update_interval.tick().await;
            
            debug!("🔍 Checking for required client updates...");
            
            // Update all registered clients
            for (chain_id, client_id) in &self.client_mappings.clone() {
                if let Err(e) = self.update_client_if_needed(chain_id, client_id).await {
                    error!("Failed to update client {} for chain {}: {}", client_id, chain_id, e);
                }
            }

            // Prune expired consensus states if enabled
            if self.config.enable_pruning {
                for (chain_id, client_id) in &self.client_mappings.clone() {
                    if let Err(e) = self.prune_client_if_needed(chain_id, client_id).await {
                        error!("Failed to prune client {} for chain {}: {}", client_id, chain_id, e);
                    }
                }
            }
        }
    }

    /// Update a client if it needs updating
    async fn update_client_if_needed(
        &mut self,
        chain_id: &str,
        client_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source_chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;

        let target_chain = self.get_target_chain_for_client(client_id)?;

        // Get current heights
        let source_height = source_chain.get_latest_height().await?;
        let last_updated_height = self.last_update_heights.get(client_id).copied().unwrap_or(0);

        debug!("📊 Client {} status: source_height={}, last_updated={}", 
               client_id, source_height, last_updated_height);

        // Check if update is needed based on block lag
        let block_lag = source_height.saturating_sub(last_updated_height);
        let needs_update = block_lag > self.config.max_block_lag;

        if needs_update {
            info!("🔄 Updating client {} (block lag: {})", client_id, block_lag);
            
            // Get the latest header from source chain
            let header = self.fetch_header_for_client(source_chain.as_ref(), source_height).await?;
            
            // Submit update to target chain
            let success = self.submit_client_update(target_chain.as_ref(), client_id, &header).await?;
            
            if success {
                self.last_update_heights.insert(client_id.to_string(), source_height);
                info!("✅ Successfully updated client {} to height {}", client_id, source_height);
            } else {
                warn!("⚠️ Client update failed for {}", client_id);
            }
        } else {
            debug!("✓ Client {} is up to date (lag: {} blocks)", client_id, block_lag);
        }

        Ok(())
    }

    /// Prune expired consensus states for a client
    async fn prune_client_if_needed(
        &mut self,
        chain_id: &str,
        client_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = Instant::now();
        let last_prune = self.last_prune_time.get(client_id).copied()
            .unwrap_or(now - Duration::from_secs(3600)); // Default to 1 hour ago

        // Only prune once per hour to avoid excessive operations
        if now.duration_since(last_prune) < Duration::from_secs(3600) {
            return Ok(());
        }

        let target_chain = self.get_target_chain_for_client(client_id)?;
        
        debug!("🧹 Checking for expired consensus states in client {}", client_id);
        
        // Get the current client state to determine trust period
        let client_state = self.query_client_state(target_chain.as_ref(), client_id).await?;
        
        if let Some(client_info) = client_state {
            let trust_period_seconds = self.config.trust_period_hours * 3600;
            let current_height = target_chain.get_latest_height().await?;
            
            // Prune consensus states older than trust period
            let mut pruned_count = 0;
            for height in 1..current_height {
                if self.prune_consensus_state_at_height(target_chain.as_ref(), client_id, height, trust_period_seconds).await? {
                    pruned_count += 1;
                }
            }
            
            if pruned_count > 0 {
                info!("🧹 Pruned {} expired consensus states from client {}", pruned_count, client_id);
            }
        }

        self.last_prune_time.insert(client_id.to_string(), now);
        Ok(())
    }

    /// Fetch a header from the source chain for client update
    async fn fetch_header_for_client(
        &self,
        source_chain: &dyn Chain,
        height: u64,
    ) -> Result<ClientHeader, Box<dyn std::error::Error + Send + Sync>> {
        // For Cosmos chains, we need to fetch the Tendermint header
        debug!("📡 Fetching header at height {} from {}", height, source_chain.chain_id().await);
        
        // This is a simplified header structure - in production you'd fetch real Tendermint headers
        Ok(ClientHeader {
            height,
            chain_id: source_chain.chain_id().await,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            // In real implementation, would include validator set, signatures, etc.
            header_data: format!("header_data_for_height_{}", height).into_bytes(),
        })
    }

    /// Submit a client update transaction
    async fn submit_client_update(
        &self,
        target_chain: &dyn Chain,
        client_id: &str,
        header: &ClientHeader,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!("📤 Submitting client update for {} to height {}", client_id, header.height);
        
        // Create UpdateClient message
        let update_msg = json!({
            "@type": "/ibc.core.client.v1.MsgUpdateClient",
            "client_id": client_id,
            "client_message": {
                "@type": "/ibc.lightclients.tendermint.v1.Header",
                "signed_header": {
                    "header": {
                        "chain_id": header.chain_id,
                        "height": header.height.to_string(),
                        "time": header.timestamp.to_string(),
                        "app_hash": base64::engine::general_purpose::STANDARD.encode(&header.header_data)
                    }
                }
            }
        });

        // Submit transaction
        let tx_data = serde_json::to_vec(&update_msg)?;
        let tx_hash = target_chain.submit_transaction(tx_data).await?;
        
        info!("✅ Client update transaction submitted: {}", tx_hash);
        Ok(true)
    }

    /// Query client state from target chain
    async fn query_client_state(
        &self,
        target_chain: &dyn Chain,
        client_id: &str,
    ) -> Result<Option<ClientStateInfo>, Box<dyn std::error::Error + Send + Sync>> {
        // This would query the actual client state from the chain
        // For now, return a mock response
        debug!("🔍 Querying client state for {}", client_id);
        
        Ok(Some(ClientStateInfo {
            client_id: client_id.to_string(),
            latest_height: target_chain.get_latest_height().await?,
            trust_period: self.config.trust_period_hours * 3600,
        }))
    }

    /// Prune a consensus state at a specific height if expired
    async fn prune_consensus_state_at_height(
        &self,
        target_chain: &dyn Chain,
        client_id: &str,
        height: u64,
        trust_period_seconds: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Check if consensus state exists and is expired
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // In a real implementation, you would:
        // 1. Query the consensus state timestamp
        // 2. Check if it's older than trust_period
        // 3. Submit a pruning transaction if needed
        
        // For now, simulate pruning logic
        if height % 1000 == 0 { // Only prune every 1000th height for demo
            debug!("🧹 Pruning consensus state at height {} for client {}", height, client_id);
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Get the target chain where the client is deployed
    fn get_target_chain_for_client(
        &self,
        client_id: &str,
    ) -> Result<Arc<dyn Chain>, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, you'd maintain a mapping of client_id -> target_chain
        // For now, assume NEAR is the target for all clients
        for (_chain_id, chain) in &self.chains {
            let chain_id = futures::executor::block_on(chain.chain_id());
            if chain_id.contains("near") {
                return Ok(chain.clone());
            }
        }
        
        Err("No target chain found for client".into())
    }

    /// Add a new client mapping
    pub fn add_client_mapping(&mut self, chain_id: String, client_id: String) {
        info!("📝 Registered client mapping: {} -> {}", chain_id, client_id);
        self.client_mappings.insert(chain_id, client_id);
    }

    /// Get the number of client mappings
    pub fn client_mappings_count(&self) -> usize {
        self.client_mappings.len()
    }

    /// Get a reference to all client mappings
    pub fn client_mappings(&self) -> &HashMap<String, String> {
        &self.client_mappings
    }

    /// Get the number of chains
    pub fn chains_count(&self) -> usize {
        self.chains.len()
    }

    /// Check if a client mapping exists for a chain
    pub fn has_client_mapping(&self, chain_id: &str) -> bool {
        self.client_mappings.contains_key(chain_id)
    }

    /// Get a client ID for a chain
    pub fn get_client_id(&self, chain_id: &str) -> Option<&String> {
        self.client_mappings.get(chain_id)
    }

    /// Set last update height for testing
    #[doc(hidden)]
    pub fn set_last_update_height(&mut self, client_id: String, height: u64) {
        self.last_update_heights.insert(client_id, height);
    }

    /// Set last prune time for testing
    #[doc(hidden)]
    pub fn set_last_prune_time(&mut self, client_id: String, time: std::time::Instant) {
        self.last_prune_time.insert(client_id, time);
    }

    /// Check if last prune time exists for testing
    #[doc(hidden)]
    pub fn has_last_prune_time(&self, client_id: &str) -> bool {
        self.last_prune_time.contains_key(client_id)
    }

    /// Remove a client mapping
    pub fn remove_client_mapping(&mut self, chain_id: &str) {
        if let Some(client_id) = self.client_mappings.remove(chain_id) {
            info!("🗑️ Removed client mapping: {} -> {}", chain_id, client_id);
            self.last_update_heights.remove(&client_id);
            self.last_prune_time.remove(&client_id);
        }
    }

    /// Get status of all managed clients
    pub async fn get_status(&self) -> Vec<ClientStatus> {
        let mut statuses = Vec::new();
        
        for (chain_id, client_id) in &self.client_mappings {
            let source_chain = match self.chains.get(chain_id) {
                Some(chain) => chain,
                None => continue,
            };
            
            let source_height = source_chain.get_latest_height().await.unwrap_or(0);
            let last_updated = self.last_update_heights.get(client_id).copied().unwrap_or(0);
            let block_lag = source_height.saturating_sub(last_updated);
            
            let status = ClientStatus {
                chain_id: chain_id.clone(),
                client_id: client_id.clone(),
                source_height,
                last_updated_height: last_updated,
                block_lag,
                needs_update: block_lag > self.config.max_block_lag,
            };
            
            statuses.push(status);
        }
        
        statuses
    }
}

/// Client header information
#[derive(Debug, Clone)]
pub struct ClientHeader {
    pub height: u64,
    pub chain_id: String,
    pub timestamp: u64,
    pub header_data: Vec<u8>,
}

/// Client state information
#[derive(Debug, Clone)]
pub struct ClientStateInfo {
    pub client_id: String,
    pub latest_height: u64,
    pub trust_period: u64,
}

/// Status of a managed client
#[derive(Debug, Clone)]
pub struct ClientStatus {
    pub chain_id: String,
    pub client_id: String,
    pub source_height: u64,
    pub last_updated_height: u64,
    pub block_lag: u64,
    pub needs_update: bool,
}

/// Helper function to create a client update manager from relayer config
pub fn create_client_update_manager(
    config: &RelayerConfig,
    chains: HashMap<String, Arc<dyn Chain>>,
) -> ClientUpdateManager {
    let client_config = ClientUpdateConfig::default();
    
    // Extract client mappings from config
    let mut client_mappings = HashMap::new();
    for connection in &config.connections {
        // Map source chain to source client
        client_mappings.insert(connection.src_chain.clone(), connection.src_client_id.clone());
        // Map destination chain to destination client
        client_mappings.insert(connection.dst_chain.clone(), connection.dst_client_id.clone());
    }
    
    ClientUpdateManager::new(client_config, chains, client_mappings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use async_trait::async_trait;
    use futures::Stream;
    use crate::chains::{Chain, ChainEvent, IbcPacket};

    // Mock chain for testing
    struct MockChain {
        chain_id: String,
        height: u64,
    }

    #[async_trait]
    impl Chain for MockChain {
        async fn chain_id(&self) -> String {
            self.chain_id.clone()
        }

        async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.height)
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
        ) -> Result<Box<dyn Stream<Item = ChainEvent> + Send + Unpin>, Box<dyn std::error::Error + Send + Sync>> {
            let stream = futures::stream::empty();
            Ok(Box::new(stream))
        }

        async fn submit_transaction(
            &self,
            _data: Vec<u8>,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok("mock_tx_hash".to_string())
        }

        async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_client_update_manager_creation() {
        let config = ClientUpdateConfig::default();
        
        let mut chains = HashMap::new();
        chains.insert("near-testnet".to_string(), Arc::new(MockChain {
            chain_id: "near-testnet".to_string(),
            height: 1000,
        }) as Arc<dyn Chain>);
        chains.insert("cosmoshub-testnet".to_string(), Arc::new(MockChain {
            chain_id: "cosmoshub-testnet".to_string(),
            height: 2000,
        }) as Arc<dyn Chain>);

        let mut client_mappings = HashMap::new();
        client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

        let manager = ClientUpdateManager::new(config, chains, client_mappings);
        
        assert_eq!(manager.client_mappings.len(), 1);
        assert_eq!(manager.chains.len(), 2);
    }

    #[tokio::test]
    async fn test_client_status_reporting() {
        let config = ClientUpdateConfig::default();
        
        let mut chains = HashMap::new();
        chains.insert("near-testnet".to_string(), Arc::new(MockChain {
            chain_id: "near-testnet".to_string(),
            height: 1000,
        }) as Arc<dyn Chain>);
        chains.insert("cosmoshub-testnet".to_string(), Arc::new(MockChain {
            chain_id: "cosmoshub-testnet".to_string(),
            height: 2000,
        }) as Arc<dyn Chain>);

        let mut client_mappings = HashMap::new();
        client_mappings.insert("cosmoshub-testnet".to_string(), "07-tendermint-0".to_string());

        let manager = ClientUpdateManager::new(config, chains, client_mappings);
        
        let statuses = manager.get_status().await;
        assert_eq!(statuses.len(), 1);
        
        let status = &statuses[0];
        assert_eq!(status.chain_id, "cosmoshub-testnet");
        assert_eq!(status.client_id, "07-tendermint-0");
        assert_eq!(status.source_height, 2000);
        assert_eq!(status.last_updated_height, 0);
        assert!(status.needs_update); // Should need update due to large lag
    }

    #[tokio::test]
    async fn test_header_fetching() {
        let config = ClientUpdateConfig::default();
        let chains = HashMap::new();
        let client_mappings = HashMap::new();
        
        let manager = ClientUpdateManager::new(config, chains, client_mappings);
        
        let mock_chain = MockChain {
            chain_id: "test-chain".to_string(),
            height: 100,
        };
        
        let header = manager.fetch_header_for_client(&mock_chain, 100).await.unwrap();
        
        assert_eq!(header.height, 100);
        assert_eq!(header.chain_id, "test-chain");
        assert!(!header.header_data.is_empty());
    }
}