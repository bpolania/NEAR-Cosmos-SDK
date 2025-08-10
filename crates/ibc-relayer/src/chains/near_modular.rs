// NEAR chain implementation for modular IBC contract architecture
// Supports routing IBC operations to multiple specialized contracts

use async_trait::async_trait;
use futures::{Stream, future::join_all};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use near_jsonrpc_client::{JsonRpcClient, methods};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    types::{AccountId, BlockReference},
    views::QueryRequest,
};

use super::{Chain, ChainEvent, IbcPacket};
use crate::config::{ChainConfig, ChainSpecificConfig};

/// Cross-module operations that require coordination between multiple IBC modules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op_type")]
pub enum CrossModuleOp {
    /// Send a packet (requires channel, connection, and client modules)
    SendPacket {
        packet: IbcPacket,
    },
    /// Channel handshake operations
    ChannelHandshake {
        operation: String,
        port_id: String,
        channel_id: String,
        counterparty_port_id: String,
        counterparty_channel_id: String,
        connection_hops: Vec<String>,
        version: String,
    },
    /// Connection handshake operations
    ConnectionHandshake {
        operation: String,
        connection_id: String,
        client_id: String,
        counterparty_connection_id: String,
        counterparty_client_id: String,
    },
    /// Update client across modules
    UpdateClient {
        client_id: String,
        header: Vec<u8>,
    },
}

// Base64 helper functions
fn base64_encode(data: &[u8]) -> String {
    near_primitives::serialize::to_base64(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    near_primitives::serialize::from_base64(data)
        .map_err(|e| format!("Base64 decode error: {}", e).into())
}

/// Module types in the IBC ecosystem
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IbcModuleType {
    Client,
    Connection,
    Channel,
    Transfer,
    Router, // Main router contract
}

/// Information about a deployed IBC module
#[derive(Clone, Debug)]
pub struct ModuleInfo {
    pub contract_id: AccountId,
    pub module_type: IbcModuleType,
    pub version: String,
    pub methods: Vec<String>,
}

/// Registry of all IBC modules in the modular architecture
#[derive(Clone, Debug)]
pub struct ModuleRegistry {
    pub router_contract: AccountId,
    pub modules: HashMap<IbcModuleType, ModuleInfo>,
}

impl ModuleRegistry {
    /// Create from configuration
    pub fn from_config(
        router: AccountId,
        module_addresses: &HashMap<String, String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut modules = HashMap::new();
        
        // Map configuration to module types
        for (module_name, address) in module_addresses {
            let module_type = match module_name.as_str() {
                "ibc_client" => IbcModuleType::Client,
                "ibc_connection" => IbcModuleType::Connection,
                "ibc_channel" => IbcModuleType::Channel,
                "ibc_transfer" => IbcModuleType::Transfer,
                _ => continue, // Skip unknown modules
            };
            
            let contract_id = address.parse::<AccountId>()
                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid contract ID for {}: {}", module_name, e))) as Box<dyn std::error::Error + Send + Sync>)?;
            
            modules.insert(module_type.clone(), ModuleInfo {
                contract_id,
                module_type,
                version: "1.0.0".to_string(), // Will be queried later
                methods: vec![], // Will be populated on demand
            });
        }
        
        Ok(Self {
            router_contract: router,
            modules,
        })
    }
    
    /// Discover modules by querying the router contract
    pub async fn discover_modules(
        router_contract: &AccountId,
        rpc_client: &JsonRpcClient,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Query router for registered modules
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id: router_contract.clone(),
                method_name: "get_modules".to_string(),
                args: json!({}).to_string().into_bytes().into(),
            },
        };
        
        let response = rpc_client.call(request).await
            .map_err(|e| format!("Failed to query modules: {}", e))?;
        
        match response.kind {
            QueryResponseKind::CallResult(result) => {
                let modules_data: HashMap<String, String> = 
                    serde_json::from_slice(&result.result)?;
                
                Ok(Self::from_config(router_contract.clone(), &modules_data)?)
            }
            _ => Err("Unexpected response from module query".into()),
        }
    }
    
    /// Discover modules with caching support
    pub async fn discover_with_cache(
        router_contract: &AccountId,
        rpc_client: &JsonRpcClient,
        cache_duration: Duration,
        cached_registry: Option<(Self, Instant)>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Check if we have a valid cached registry
        if let Some((registry, cached_at)) = cached_registry {
            if cached_at.elapsed() < cache_duration {
                println!("📦 Using cached module registry (age: {:?})", cached_at.elapsed());
                return Ok(registry);
            }
        }
        
        // Otherwise, discover modules
        println!("🔍 Discovering modules from router contract");
        Self::discover_modules(router_contract, rpc_client).await
    }
    
    /// Update a specific module's info (for hot-swapping)
    pub fn update_module(
        &mut self,
        module_type: IbcModuleType,
        new_contract_id: AccountId,
    ) -> Result<(), String> {
        if let Some(module) = self.modules.get_mut(&module_type) {
            println!("🔄 Hot-swapping {:?} module: {} -> {}", 
                     module_type, module.contract_id, new_contract_id);
            module.contract_id = new_contract_id;
            Ok(())
        } else {
            Err(format!("Module {:?} not found", module_type))
        }
    }
}

/// NEAR chain implementation supporting modular IBC architecture
pub struct NearModularChain {
    chain_id: String,
    module_registry: Arc<RwLock<ModuleRegistry>>,
    rpc_client: JsonRpcClient,
    network_id: String,
}

impl NearModularChain {
    /// Create new instance with modular support
    pub async fn new(config: &ChainConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match &config.config {
            ChainSpecificConfig::Near { contract_id, modules, network_id, .. } => {
                let rpc_client = JsonRpcClient::connect(&config.rpc_endpoint);
                let router_account_id = contract_id.parse::<AccountId>()
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid router contract ID: {}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
                
                // Initialize module registry
                let module_registry = if let Some(module_config) = modules {
                    ModuleRegistry::from_config(router_account_id, module_config)?
                } else {
                    // Auto-discover modules
                    ModuleRegistry::discover_modules(&router_account_id, &rpc_client).await?
                };
                
                Ok(Self {
                    chain_id: config.chain_id.clone(),
                    module_registry: Arc::new(RwLock::new(module_registry)),
                    rpc_client,
                    network_id: network_id.clone(),
                })
            }
            _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid config type for NEAR chain")) as Box<dyn std::error::Error + Send + Sync>),
        }
    }
    
    /// Call a specific IBC module
    pub async fn call_module(
        &self,
        module_type: IbcModuleType,
        method_name: &str,
        args: serde_json::Value,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let registry = self.module_registry.read().await;
        
        let module = registry.modules.get(&module_type)
            .ok_or_else(|| format!("Module {:?} not found", module_type))?;
        
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id: module.contract_id.clone(),
                method_name: method_name.to_string(),
                args: args.to_string().into_bytes().into(),
            },
        };
        
        let response = self.rpc_client.call(request).await
            .map_err(|e| format!("Module call failed: {}", e))?;
        
        match response.kind {
            QueryResponseKind::CallResult(result) => Ok(result.result),
            _ => Err("Unexpected response from module call".into()),
        }
    }
    
    /// Call router for cross-module operations
    async fn call_router(
        &self,
        method_name: &str,
        args: serde_json::Value,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let registry = self.module_registry.read().await;
        
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id: registry.router_contract.clone(),
                method_name: method_name.to_string(),
                args: args.to_string().into_bytes().into(),
            },
        };
        
        let response = self.rpc_client.call(request).await
            .map_err(|e| format!("Router call failed: {}", e))?;
        
        match response.kind {
            QueryResponseKind::CallResult(result) => Ok(result.result),
            _ => Err("Unexpected response from router call".into()),
        }
    }
    
    /// Execute a cross-module operation through the router
    pub async fn execute_cross_module_op(
        &self,
        operation: CrossModuleOp,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let args = json!({
            "operation": operation,
        });
        
        self.call_router("execute_cross_module_op", args).await
    }
    
    /// Submit a transaction that may span multiple modules
    pub async fn submit_cross_module_transaction(
        &self,
        operation: CrossModuleOp,
        signer_account_id: &str,
        _private_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would:
        // 1. Build a transaction calling the router contract
        // 2. Sign it with the provided key
        // 3. Submit it to the network
        // 4. Return the transaction hash
        
        // For now, simulate with a view call
        let args = json!({
            "operation": operation,
            "signer": signer_account_id,
        });
        
        let result = self.call_router("simulate_cross_module_tx", args).await?;
        Ok(serde_json::from_slice(&result)?)
    }
    
    /// Query multiple modules in parallel for improved performance
    pub async fn query_modules_parallel<T>(
        &self,
        queries: Vec<(IbcModuleType, &str, serde_json::Value)>,
    ) -> Vec<Result<T, Box<dyn std::error::Error + Send + Sync>>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let futures = queries.into_iter().map(|(module_type, method, args)| {
            let self_clone = self.clone();
            async move {
                let result = self_clone.call_module(module_type, method, args).await?;
                serde_json::from_slice::<T>(&result)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        });
        
        join_all(futures).await
    }
    
    /// Get comprehensive channel state by querying multiple modules in parallel
    pub async fn get_channel_state_parallel(
        &self,
        port_id: &str,
        channel_id: &str,
    ) -> Result<ChannelStateInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Define queries for different modules
        let queries = vec![
            // Channel info from channel module
            (
                IbcModuleType::Channel,
                "get_channel",
                json!({ "port_id": port_id, "channel_id": channel_id })
            ),
            // Connection info from connection module
            (
                IbcModuleType::Connection,
                "get_connection",
                json!({ "connection_id": "connection-0" }) // Would get from channel
            ),
            // Client state from client module
            (
                IbcModuleType::Client,
                "get_client_state",
                json!({ "client_id": "07-tendermint-0" }) // Would get from connection
            ),
        ];
        
        // Execute queries in parallel
        let results: Vec<_> = self.query_modules_parallel::<serde_json::Value>(queries).await;
        
        // Combine results
        Ok(ChannelStateInfo {
            channel: results[0].as_ref().ok().cloned(),
            connection: results[1].as_ref().ok().cloned(),
            client: results[2].as_ref().ok().cloned(),
        })
    }
}

/// Combined channel state information from multiple modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStateInfo {
    pub channel: Option<serde_json::Value>,
    pub connection: Option<serde_json::Value>,
    pub client: Option<serde_json::Value>,
}

impl Clone for NearModularChain {
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id.clone(),
            module_registry: Arc::clone(&self.module_registry),
            rpc_client: self.rpc_client.clone(),
            network_id: self.network_id.clone(),
        }
    }
}

#[async_trait]
impl Chain for NearModularChain {
    async fn chain_id(&self) -> String {
        self.chain_id.clone()
    }
    
    async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let status = self.rpc_client
            .call(methods::status::RpcStatusRequest)
            .await?;
        
        Ok(status.sync_info.latest_block_height)
    }
    
    /// Query packet commitment from IBC Channel module
    async fn query_packet_commitment(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let args = json!({
            "port_id": port_id,
            "channel_id": channel_id,
            "sequence": sequence,
        });
        
        let result = self.call_module(
            IbcModuleType::Channel,
            "query_packet_commitment",
            args
        ).await?;
        
        let commitment: Option<String> = serde_json::from_slice(&result)?;
        Ok(commitment.map(|c| base64_decode(&c).unwrap_or_default()))
    }
    
    /// Query packet acknowledgment from IBC Channel module
    async fn query_packet_acknowledgment(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let args = json!({
            "port_id": port_id,
            "channel_id": channel_id,
            "sequence": sequence,
        });
        
        let result = self.call_module(
            IbcModuleType::Channel,
            "query_packet_acknowledgment",
            args
        ).await?;
        
        let ack: Option<String> = serde_json::from_slice(&result)?;
        Ok(ack.map(|a| base64_decode(&a).unwrap_or_default()))
    }
    
    
    async fn query_packet_receipt(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let args = json!({
            "port_id": port_id,
            "channel_id": channel_id,
            "sequence": sequence,
        });
        
        let result = self.call_module(
            IbcModuleType::Channel,
            "query_packet_receipt",
            args
        ).await?;
        
        Ok(serde_json::from_slice(&result)?)
    }
    
    async fn query_next_sequence_recv(
        &self,
        port_id: &str,
        channel_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let args = json!({
            "port_id": port_id,
            "channel_id": channel_id,
        });
        
        let result = self.call_module(
            IbcModuleType::Channel,
            "query_next_sequence_recv",
            args
        ).await?;
        
        Ok(serde_json::from_slice(&result)?)
    }
    
    async fn get_events(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        println!("📡 Querying modular NEAR events from blocks {}-{}", from_height, to_height);
        
        let all_events = Vec::new();
        
        // Query events from all module contracts in the range
        let registry = self.module_registry.read().await;
        for _module in registry.modules.values() {
            // Query events from each module contract
            // This would use NEAR's indexer or event query APIs
            // For now, return empty events as placeholder
        }
        
        Ok(all_events)
    }

    async fn subscribe_events(
        &self,
    ) -> Result<
        Box<dyn Stream<Item = ChainEvent> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Would implement WebSocket subscription to multiple module contracts
        // For now, return empty stream
        Ok(Box::new(futures::stream::empty()))
    }
    
    async fn submit_transaction(
        &self,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Try to parse as CrossModuleOp first
        if let Ok(operation) = serde_json::from_slice::<CrossModuleOp>(&data) {
            // This is a cross-module operation
            println!("🔄 Submitting cross-module operation: {:?}", operation);
            
            // In production, would get signer info from config
            // For now, simulate the transaction
            return self.submit_cross_module_transaction(
                operation,
                "relayer.testnet",
                "dummy_key"
            ).await;
        }
        
        // Otherwise, route as generic transaction through router
        let args = json!({
            "transaction_data": base64_encode(&data),
        });
        
        let result = self.call_router("submit_transaction", args).await?;
        Ok(serde_json::from_slice(&result)?)
    }

    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check health of router and all modules
        let registry = self.module_registry.read().await;
        
        // Check router first
        let router_request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::ViewAccount {
                account_id: registry.router_contract.clone(),
            },
        };
        
        self.rpc_client.call(router_request).await
            .map_err(|e| format!("Router health check failed: {}", e))?;
        
        // Check each module
        for (module_type, module) in &registry.modules {
            let module_request = methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request: QueryRequest::ViewAccount {
                    account_id: module.contract_id.clone(),
                },
            };
            
            self.rpc_client.call(module_request).await
                .map_err(|e| format!("Module {:?} health check failed: {}", module_type, e))?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_registry_from_config() {
        let mut modules = HashMap::new();
        modules.insert("ibc_client".to_string(), "client.testnet".to_string());
        modules.insert("ibc_channel".to_string(), "channel.testnet".to_string());
        
        let router = "router.testnet".parse().unwrap();
        let registry = ModuleRegistry::from_config(router, &modules).unwrap();
        
        assert_eq!(registry.modules.len(), 2);
        assert!(registry.modules.contains_key(&IbcModuleType::Client));
        assert!(registry.modules.contains_key(&IbcModuleType::Channel));
    }
    
    #[test]
    fn test_module_registry_update() {
        let mut modules = HashMap::new();
        modules.insert("ibc_channel".to_string(), "channel-v1.testnet".to_string());
        
        let router = "router.testnet".parse().unwrap();
        let mut registry = ModuleRegistry::from_config(router, &modules).unwrap();
        
        // Update module
        let new_contract: AccountId = "channel-v2.testnet".parse().unwrap();
        let result = registry.update_module(IbcModuleType::Channel, new_contract.clone());
        
        assert!(result.is_ok());
        assert_eq!(
            registry.modules.get(&IbcModuleType::Channel).unwrap().contract_id,
            new_contract
        );
    }
    
    #[test]
    fn test_cross_module_op_serialization() {
        let packet = IbcPacket {
            sequence: 1,
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: None,
        };
        
        let op = CrossModuleOp::SendPacket { packet };
        
        // Test serialization
        let serialized = serde_json::to_string(&op).unwrap();
        assert!(serialized.contains("SendPacket"));
        assert!(serialized.contains("op_type"));
        
        // Test deserialization
        let deserialized: CrossModuleOp = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            CrossModuleOp::SendPacket { packet } => {
                assert_eq!(packet.sequence, 1);
                assert_eq!(packet.source_port, "transfer");
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_ibc_module_type_equality() {
        assert_eq!(IbcModuleType::Client, IbcModuleType::Client);
        assert_ne!(IbcModuleType::Client, IbcModuleType::Channel);
        
        // Test hash capability for HashMap
        let mut map = HashMap::new();
        map.insert(IbcModuleType::Client, "test");
        assert_eq!(map.get(&IbcModuleType::Client), Some(&"test"));
    }
}