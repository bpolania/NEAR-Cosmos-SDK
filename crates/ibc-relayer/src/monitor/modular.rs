// Modular event monitor for aggregating events from multiple IBC module contracts
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use futures::{Stream, StreamExt, stream};
use near_jsonrpc_client::JsonRpcClient;
use near_primitives::types::AccountId;

use crate::chains::{ChainEvent, IbcModuleType, ModuleRegistry};
use crate::relay::RelayEvent;
use super::{EventMonitor, MonitorConfig};

/// Event monitor specifically designed for modular IBC architecture
/// Aggregates events from multiple module contracts
pub struct ModularEventMonitor {
    /// Module registry containing all IBC module addresses
    module_registry: Arc<ModuleRegistry>,
    /// Individual event streams for each module
    module_streams: HashMap<IbcModuleType, Box<dyn Stream<Item = ChainEvent> + Send + Sync + Unpin>>,
    /// Event sender for relay engine
    event_sender: mpsc::Sender<RelayEvent>,
    /// RPC client for querying
    rpc_client: JsonRpcClient,
    /// Chain ID
    chain_id: String,
    /// Monitoring configuration
    config: MonitorConfig,
    /// Last known heights for each module (for polling)
    last_heights: HashMap<AccountId, u64>,
}

impl ModularEventMonitor {
    /// Create a new modular event monitor
    pub async fn new(
        chain_id: String,
        module_registry: Arc<ModuleRegistry>,
        event_sender: mpsc::Sender<RelayEvent>,
        rpc_client: JsonRpcClient,
        config: MonitorConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let module_streams = HashMap::new();
        let last_heights = HashMap::new();
        
        Ok(Self {
            module_registry,
            module_streams,
            event_sender,
            rpc_client,
            chain_id,
            config,
            last_heights,
        })
    }
    
    /// Initialize event streams for all modules
    pub async fn initialize_streams(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔄 Initializing event streams for {} modules", self.module_registry.modules.len());
        
        // Try to create WebSocket streams for each module if streaming is preferred
        if self.config.prefer_streaming {
            for (module_type, module_info) in &self.module_registry.modules {
                match self.create_module_stream(&module_info.contract_id).await {
                    Ok(stream) => {
                        println!("📡 Created event stream for {:?} module at {}", 
                                 module_type, module_info.contract_id);
                        self.module_streams.insert(module_type.clone(), stream);
                    }
                    Err(e) => {
                        println!("⚠️  Failed to create stream for {:?} module: {}", module_type, e);
                        // Will fall back to polling for this module
                    }
                }
            }
        }
        
        // Initialize last heights for polling fallback
        for module_info in self.module_registry.modules.values() {
            self.last_heights.insert(module_info.contract_id.clone(), 0);
        }
        
        Ok(())
    }
    
    /// Create an event stream for a specific module contract
    async fn create_module_stream(
        &self,
        contract_id: &AccountId,
    ) -> Result<Box<dyn Stream<Item = ChainEvent> + Send + Sync + Unpin>, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would create a WebSocket connection
        // to monitor events from the specific contract
        // For now, return an empty stream as placeholder
        Ok(Box::new(stream::empty()))
    }
    
    /// Start monitoring all modules
    pub async fn start(
        &mut self,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting modular event monitor for chain: {}", self.chain_id);
        
        // Initialize streams
        self.initialize_streams().await?;
        
        // Create a merged stream from all module streams
        let mut merged_stream = self.create_merged_stream();
        
        // Polling interval for modules without streams
        let mut poll_interval = time::interval(Duration::from_millis(self.config.polling_interval_ms));
        
        loop {
            tokio::select! {
                // Process events from merged stream
                Some(event) = merged_stream.next() => {
                    if let Err(e) = self.process_module_event(event).await {
                        eprintln!("Error processing module event: {}", e);
                    }
                }
                
                // Poll modules that don't have streams
                _ = poll_interval.tick() => {
                    if let Err(e) = self.poll_modules_without_streams().await {
                        eprintln!("Error polling modules: {}", e);
                    }
                }
                
                // Check for shutdown
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        println!("🛑 Stopping modular event monitor");
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a merged stream from all module streams
    fn create_merged_stream(&mut self) -> Box<dyn Stream<Item = ChainEvent> + Send + Sync + Unpin> {
        let streams: Vec<_> = self.module_streams
            .drain()
            .map(|(_, stream)| stream)
            .collect();
        
        // Merge all streams into one
        Box::new(stream::select_all(streams))
    }
    
    /// Poll modules that don't have WebSocket streams
    async fn poll_modules_without_streams(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clone the data we need to avoid borrow checker issues
        let modules_to_poll: Vec<(IbcModuleType, crate::chains::ModuleInfo)> = self.module_registry.modules
            .iter()
            .filter(|(module_type, _)| !self.module_streams.contains_key(module_type))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        for (module_type, module_info) in modules_to_poll {
            if let Err(e) = self.poll_module_events(&module_type, &module_info).await {
                eprintln!("Error polling {:?} module: {}", module_type, e);
            }
        }
        
        Ok(())
    }
    
    /// Poll events from a specific module
    async fn poll_module_events(
        &mut self,
        module_type: &IbcModuleType,
        module_info: &crate::chains::ModuleInfo,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get current block height
        let current_height = self.get_current_height().await?;
        let last_height = self.last_heights.get(&module_info.contract_id).copied().unwrap_or(0);
        
        if current_height > last_height {
            // Query events from this module for new blocks
            let events = self.query_module_events(
                &module_info.contract_id,
                last_height + 1,
                current_height,
            ).await?;
            
            // Process events
            for event in events {
                if let Err(e) = self.process_module_event(event).await {
                    eprintln!("Error processing event from {:?} module: {}", module_type, e);
                }
            }
            
            // Update last height
            self.last_heights.insert(module_info.contract_id.clone(), current_height);
        }
        
        Ok(())
    }
    
    /// Query events from a specific module contract
    async fn query_module_events(
        &self,
        contract_id: &AccountId,
        from_height: u64,
        to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would query the NEAR indexer
        // or use NEAR's event APIs to get events from the specific contract
        // within the height range
        
        println!("📊 Querying events from {} (blocks {}-{})", 
                 contract_id, from_height, to_height);
        
        // Placeholder: return empty vec
        Ok(vec![])
    }
    
    /// Process an event from any module
    async fn process_module_event(
        &self,
        event: ChainEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use the existing event parsing logic from EventMonitor
        match event.event_type.as_str() {
            "send_packet" => {
                if let Some(relay_event) = EventMonitor::parse_send_packet_event(&self.chain_id, &event)? {
                    self.event_sender.send(relay_event).await?;
                }
            }
            "recv_packet" => {
                if let Some(relay_event) = EventMonitor::parse_recv_packet_event(&self.chain_id, &event)? {
                    self.event_sender.send(relay_event).await?;
                }
            }
            "acknowledge_packet" => {
                if let Some(relay_event) = EventMonitor::parse_acknowledge_packet_event(&self.chain_id, &event)? {
                    self.event_sender.send(relay_event).await?;
                }
            }
            "timeout_packet" => {
                if let Some(relay_event) = EventMonitor::parse_timeout_packet_event(&self.chain_id, &event)? {
                    self.event_sender.send(relay_event).await?;
                }
            }
            _ => {
                // Ignore other event types
            }
        }
        
        Ok(())
    }
    
    /// Get current blockchain height
    async fn get_current_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        use near_jsonrpc_client::methods;
        
        let status = self.rpc_client
            .call(methods::status::RpcStatusRequest)
            .await?;
        
        Ok(status.sync_info.latest_block_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_modular_event_monitor_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let rpc_client = JsonRpcClient::connect("https://rpc.testnet.near.org");
        
        let mut modules = HashMap::new();
        modules.insert(IbcModuleType::Channel, crate::chains::ModuleInfo {
            contract_id: "channel.testnet".parse().unwrap(),
            module_type: IbcModuleType::Channel,
            version: "1.0.0".to_string(),
            methods: vec![],
        });
        
        let registry = ModuleRegistry {
            router_contract: "router.testnet".parse().unwrap(),
            modules,
        };
        
        let monitor = ModularEventMonitor::new(
            "near-testnet".to_string(),
            Arc::new(registry),
            tx,
            rpc_client,
            MonitorConfig::default(),
        ).await;
        
        assert!(monitor.is_ok());
    }
}