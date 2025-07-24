// Minimal Cosmos chain implementation for IBC relayer
// Focuses on transaction submission for NEAR → Cosmos packet relay

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{Chain, ChainEvent};
use crate::config::{ChainConfig, ChainSpecificConfig};

/// Minimal Cosmos chain implementation
/// Only implements transaction submission for initial packet relay
pub struct CosmosChain {
    chain_id: String,
    rpc_endpoint: String,
    address_prefix: String,
    gas_price: String,
    client: Client,
}

impl CosmosChain {
    /// Create a new Cosmos chain instance
    pub fn new(config: &ChainConfig) -> Result<Self, Box<dyn std::error::Error>> {
        match &config.config {
            ChainSpecificConfig::Cosmos {
                address_prefix,
                gas_price,
                ..
            } => {
                let client = Client::new();
                
                Ok(Self {
                    chain_id: config.chain_id.clone(),
                    rpc_endpoint: config.rpc_endpoint.clone(),
                    address_prefix: address_prefix.clone(),
                    gas_price: gas_price.clone(),
                    client,
                })
            }
            _ => Err("Invalid config type for Cosmos chain".into()),
        }
    }

    /// Submit a RecvPacket transaction to Cosmos chain
    /// This is the core functionality needed for NEAR → Cosmos relay
    async fn submit_recv_packet_tx(
        &self,
        packet_data: &[u8],
        proof: &[u8],
        proof_height: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Construct IBC RecvPacket message
        let msg = json!({
            "@type": "/ibc.core.channel.v1.MsgRecvPacket",
            "packet": {
                "sequence": 1, // TODO: Extract from packet_data
                "source_port": "transfer",
                "source_channel": "channel-0",
                "destination_port": "transfer", 
                "destination_channel": "channel-1",
                "data": general_purpose::STANDARD.encode(packet_data),
                "timeout_height": {
                    "revision_number": 0,
                    "revision_height": proof_height + 1000
                },
                "timeout_timestamp": 0
            },
            "proof_commitment": general_purpose::STANDARD.encode(proof),
            "proof_height": {
                "revision_number": 0,
                "revision_height": proof_height
            },
            "signer": "cosmos1relayer..." // TODO: Use configured signer
        });

        // Construct transaction
        let tx = json!({
            "body": {
                "messages": [msg],
                "memo": "IBC packet relay from NEAR",
                "timeout_height": "0",
                "extension_options": [],
                "non_critical_extension_options": []
            },
            "auth_info": {
                "signer_infos": [],
                "fee": {
                    "amount": [{
                        "denom": "uatom", // TODO: Extract from gas_price
                        "amount": "5000"
                    }],
                    "gas_limit": "200000",
                    "payer": "",
                    "granter": ""
                }
            },
            "signatures": [] // TODO: Add actual signature
        });

        // Submit transaction via RPC
        let response = self.client
            .post(&format!("{}/cosmos/tx/v1beta1/txs", self.rpc_endpoint))
            .json(&json!({
                "tx_bytes": general_purpose::STANDARD.encode(serde_json::to_vec(&tx)?),
                "mode": "BROADCAST_MODE_SYNC"
            }))
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        if let Some(tx_hash) = result["tx_response"]["txhash"].as_str() {
            Ok(tx_hash.to_string())
        } else {
            Err(format!("Transaction failed: {:?}", result).into())
        }
    }

    /// Query Tendermint status for basic connectivity check
    async fn query_status(&self) -> Result<TendermintStatus, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/status", self.rpc_endpoint))
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        Ok(TendermintStatus {
            chain_id: result["result"]["node_info"]["network"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            latest_block_height: result["result"]["sync_info"]["latest_block_height"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0),
            latest_block_time: result["result"]["sync_info"]["latest_block_time"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        })
    }
}

#[async_trait]
impl Chain for CosmosChain {
    /// Get the chain ID
    async fn chain_id(&self) -> String {
        self.chain_id.clone()
    }

    /// Get the latest block height from Tendermint
    async fn get_latest_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let status = self.query_status().await?;
        Ok(status.latest_block_height)
    }

    /// Query packet commitment - STUB for minimal implementation
    /// Returns None since we don't need to query Cosmos state yet
    async fn query_packet_commitment(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement when we need Cosmos → NEAR relay
        Ok(None)
    }

    /// Query packet acknowledgment - STUB for minimal implementation
    async fn query_packet_acknowledgment(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement when we need acknowledgment processing
        Ok(None)
    }

    /// Query packet receipt - STUB for minimal implementation
    async fn query_packet_receipt(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _sequence: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement for unordered channels
        Ok(false)
    }

    /// Query next sequence receive - STUB for minimal implementation
    async fn query_next_sequence_recv(
        &self,
        _port_id: &str,
        _channel_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement when we need bidirectional relay
        Ok(1)
    }

    /// Get events in a block range - STUB for minimal implementation
    /// Returns empty since we don't monitor Cosmos events yet
    async fn get_events(
        &self,
        _from_height: u64,
        _to_height: u64,
    ) -> Result<Vec<ChainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement when we need Cosmos → NEAR relay
        Ok(vec![])
    }

    /// Monitor for new events - STUB for minimal implementation
    /// Returns empty stream since we don't monitor Cosmos events yet
    async fn subscribe_events(
        &self,
    ) -> Result<
        Box<dyn Stream<Item = ChainEvent> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // TODO: Implement Tendermint WebSocket event streaming
        let stream = futures::stream::empty();
        Ok(Box::new(stream))
    }

    /// Submit a transaction - CORE FUNCTIONALITY for minimal implementation
    /// This handles RecvPacket transactions for NEAR → Cosmos relay
    async fn submit_transaction(
        &self,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Parse the transaction data to determine type
        // For now, assume it's a RecvPacket transaction
        
        // TODO: Parse actual packet data and proof from input
        let packet_data = &data[..data.len().min(100)]; // Mock packet data
        let proof = &[0u8; 32]; // Mock proof
        let proof_height = self.get_latest_height().await? - 1;
        
        self.submit_recv_packet_tx(packet_data, proof, proof_height).await
    }

    /// Health check - Verify connection to Tendermint RPC
    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status = self.query_status().await?;
        
        // Verify we can connect and get a reasonable response
        if status.chain_id.is_empty() || status.latest_block_height == 0 {
            return Err("Cosmos chain health check failed: invalid status response".into());
        }
        
        println!("Cosmos chain health check: OK (chain_id: {}, height: {})", 
                 status.chain_id, status.latest_block_height);
        Ok(())
    }
}

/// Tendermint status response structure
#[derive(Debug, Deserialize)]
struct TendermintStatus {
    chain_id: String,
    latest_block_height: u64,
    latest_block_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChainConfig;

    #[tokio::test]
    async fn test_cosmos_chain_creation() {
        let config = ChainConfig {
            chain_id: "cosmoshub-testnet".to_string(),
            chain_type: "cosmos".to_string(),
            rpc_endpoint: "https://rpc.testnet.cosmos.network".to_string(),
            ws_endpoint: None,
            config: ChainSpecificConfig::Cosmos {
                address_prefix: "cosmos".to_string(),
                gas_price: "0.025uatom".to_string(),
                trust_threshold: "1/3".to_string(),
                trusting_period_hours: 336,
                signer_key: None,
            },
        };

        let chain = CosmosChain::new(&config).unwrap();
        assert_eq!(chain.chain_id, "cosmoshub-testnet");
        assert_eq!(chain.address_prefix, "cosmos");
        assert_eq!(chain.gas_price, "0.025uatom");
    }

    #[tokio::test]
    async fn test_cosmos_chain_methods() {
        let config = ChainConfig {
            chain_id: "cosmoshub-testnet".to_string(),
            chain_type: "cosmos".to_string(),
            rpc_endpoint: "https://rpc.testnet.cosmos.network".to_string(),
            ws_endpoint: None,
            config: ChainSpecificConfig::Cosmos {
                address_prefix: "cosmos".to_string(),
                gas_price: "0.025uatom".to_string(),
                trust_threshold: "1/3".to_string(),
                trusting_period_hours: 336,
                signer_key: None,
            },
        };

        let chain = CosmosChain::new(&config).unwrap();
        
        // Test basic methods
        assert_eq!(chain.chain_id().await, "cosmoshub-testnet");
        
        // Test stub methods (should return defaults for minimal implementation)
        assert_eq!(chain.query_packet_commitment("transfer", "channel-0", 1).await.unwrap(), None);
        assert_eq!(chain.query_packet_acknowledgment("transfer", "channel-0", 1).await.unwrap(), None);
        assert_eq!(chain.query_packet_receipt("transfer", "channel-0", 1).await.unwrap(), false);
        assert_eq!(chain.query_next_sequence_recv("transfer", "channel-0").await.unwrap(), 1);
        
        // Test event methods (should return empty for minimal implementation)
        let events = chain.get_events(1000, 1010).await.unwrap();
        assert!(events.is_empty());
        
        // Note: Health check and transaction submission tests would require actual RPC endpoint
        // These are tested in integration tests or with mock servers
    }
}