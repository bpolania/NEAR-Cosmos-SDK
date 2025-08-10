// Chain factory for creating appropriate chain implementations
// Handles both monolithic and modular NEAR contracts

use std::boxed::Box;
use async_trait::async_trait;

use super::{Chain, NearChain, NearModularChain, CosmosChain};
use crate::config::{ChainConfig, ChainSpecificConfig};

/// Factory for creating chain instances based on configuration
pub struct ChainFactory;

impl ChainFactory {
    /// Create a chain instance from configuration
    pub async fn create_chain(config: &ChainConfig) -> Result<Box<dyn Chain>, Box<dyn std::error::Error + Send + Sync>> {
        match &config.config {
            ChainSpecificConfig::Near { modular, .. } => {
                if *modular {
                    // Create modular NEAR chain for new architecture
                    let chain = NearModularChain::new(config).await?;
                    Ok(Box::new(chain))
                } else {
                    // Create traditional monolithic NEAR chain
                    let chain = NearChain::new(config)?;
                    Ok(Box::new(chain))
                }
            }
            ChainSpecificConfig::Cosmos { .. } => {
                let chain = CosmosChain::new(config)?;
                Ok(Box::new(chain))
            }
        }
    }
    
    /// Auto-detect chain architecture type
    pub async fn auto_detect_and_create(
        config: &ChainConfig
    ) -> Result<Box<dyn Chain>, Box<dyn std::error::Error + Send + Sync>> {
        match &config.config {
            ChainSpecificConfig::Near { contract_id, .. } => {
                // Try to detect if this is a modular or monolithic contract
                // by querying for module registry methods
                let is_modular = Self::detect_modular_architecture(config, contract_id).await?;
                
                if is_modular {
                    println!("🔍 Detected modular NEAR contract architecture for {}", contract_id);
                    let chain = NearModularChain::new(config).await?;
                    Ok(Box::new(chain))
                } else {
                    println!("🔍 Detected monolithic NEAR contract architecture for {}", contract_id);
                    let chain = NearChain::new(config)?;
                    Ok(Box::new(chain))
                }
            }
            ChainSpecificConfig::Cosmos { .. } => {
                let chain = CosmosChain::new(config)?;
                Ok(Box::new(chain))
            }
        }
    }
    
    /// Detect if a NEAR contract uses modular architecture
    async fn detect_modular_architecture(
        config: &ChainConfig,
        contract_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        use near_jsonrpc_client::{JsonRpcClient, methods};
        use near_primitives::{types::BlockReference, views::QueryRequest};
        use serde_json::json;
        
        let rpc_client = JsonRpcClient::connect(&config.rpc_endpoint);
        let account_id = contract_id.parse()
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid account ID: {}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Try to call a method that only exists in modular contracts
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id,
                method_name: "get_modules".to_string(),
                args: json!({}).to_string().into_bytes().into(),
            },
        };
        
        match rpc_client.call(request).await {
            Ok(_) => Ok(true), // Method exists, this is modular
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("MethodNotFound") || error_msg.contains("method not found") {
                    Ok(false) // Method doesn't exist, this is monolithic
                } else {
                    // Other errors (network, etc.) - default to monolithic
                    eprintln!("⚠️  Could not detect architecture type, defaulting to monolithic: {}", e);
                    Ok(false)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChainConfig, ChainSpecificConfig};
    
    #[tokio::test]
    async fn test_create_monolithic_chain() {
        let config = ChainConfig {
            chain_id: "near-testnet".to_string(),
            chain_type: "near".to_string(),
            rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
            ws_endpoint: None,
            config: ChainSpecificConfig::Near {
                contract_id: "cosmos-sdk-demo.testnet".to_string(),
                modules: None,
                signer_account_id: "relayer.testnet".to_string(),
                private_key: None,
                network_id: "testnet".to_string(),
                modular: false,
            },
        };
        
        let chain = ChainFactory::create_chain(&config).await.unwrap();
        assert_eq!(chain.chain_id().await, "near-testnet");
    }
    
    #[tokio::test]
    async fn test_create_modular_chain() {
        let config = ChainConfig {
            chain_id: "near-testnet".to_string(),
            chain_type: "near".to_string(),
            rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
            ws_endpoint: None,
            config: ChainSpecificConfig::Near {
                contract_id: "cosmos-router.testnet".to_string(),
                modules: None, // Auto-discover
                signer_account_id: "relayer.testnet".to_string(),
                private_key: None,
                network_id: "testnet".to_string(),
                modular: true,
            },
        };
        
        // This will fail with real network but shouldn't panic
        let _result = ChainFactory::create_chain(&config).await;
    }
}