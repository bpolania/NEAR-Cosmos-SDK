use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use tracing::{info, debug, warn};
use base64::{Engine as _, engine::general_purpose};

use crate::chains::{Chain, EventStream, ChainEvent, IbcPacket};
use crate::config::{ChainConfig, ChainSpecificConfig};

/// Cosmos SDK chain implementation
pub struct CosmosChain {
    chain_id: String,
    rpc_endpoint: String,
    ws_endpoint: Option<String>,
    address_prefix: String,
    gas_price: String,
    client: reqwest::Client,
}

impl CosmosChain {
    pub fn new(config: &ChainConfig) -> Result<Self> {
        if let ChainSpecificConfig::Cosmos { 
            address_prefix, 
            gas_price,
            .. 
        } = &config.config {
            Ok(Self {
                chain_id: config.chain_id.clone(),
                rpc_endpoint: config.rpc_endpoint.clone(),
                ws_endpoint: config.ws_endpoint.clone(),
                address_prefix: address_prefix.clone(),
                gas_price: gas_price.clone(),
                client: reqwest::Client::new(),
            })
        } else {
            anyhow::bail!("Invalid Cosmos chain configuration")
        }
    }

    /// Query Cosmos chain via RPC
    async fn rpc_query(&self, path: &str, params: Option<Value>) -> Result<Value> {
        let url = format!("{}/abci_query", self.rpc_endpoint);
        
        let query_params = if let Some(params) = params {
            format!("?{}", serde_json::to_string(&params)?)
        } else {
            String::new()
        };

        let full_url = format!("{}{}", url, query_params);
        debug!("Querying Cosmos RPC: {}", full_url);

        let response = self.client
            .get(&full_url)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Cosmos RPC request failed with status: {}", response.status());
        }

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            anyhow::bail!("Cosmos RPC error: {}", error);
        }

        Ok(response_json)
    }

    /// Submit transaction to Cosmos chain
    async fn broadcast_tx(&self, tx_bytes: Vec<u8>) -> Result<String> {
        let url = format!("{}/broadcast_tx_sync", self.rpc_endpoint);
        
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "broadcast_tx_sync",
            "params": {
                "tx": general_purpose::STANDARD.encode(&tx_bytes)
            }
        });

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            anyhow::bail!("Cosmos broadcast error: {}", error);
        }

        // Extract transaction hash
        if let Some(result) = response_json.get("result") {
            if let Some(hash) = result.get("hash") {
                return Ok(hash.as_str().unwrap_or("").to_string());
            }
        }

        anyhow::bail!("Could not extract transaction hash from response")
    }
}

#[async_trait]
impl Chain for CosmosChain {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    
    async fn get_height(&self) -> Result<u64> {
        let response = self.rpc_query("/status", None).await?;
        
        if let Some(result) = response.get("result") {
            if let Some(sync_info) = result.get("sync_info") {
                if let Some(height) = sync_info.get("latest_block_height") {
                    return Ok(height.as_str().unwrap_or("0").parse()?);
                }
            }
        }
        
        anyhow::bail!("Could not extract height from Cosmos status")
    }
    
    async fn get_header(&self, height: u64) -> Result<Value> {
        let response = self.rpc_query("/block", Some(serde_json::json!({
            "height": height.to_string()
        }))).await?;
        
        if let Some(result) = response.get("result") {
            if let Some(block) = result.get("block") {
                if let Some(header) = block.get("header") {
                    return Ok(header.clone());
                }
            }
        }
        
        anyhow::bail!("Could not extract header from Cosmos block")
    }
    
    async fn submit_tx(&self, tx: Vec<u8>) -> Result<String> {
        self.broadcast_tx(tx).await
    }
    
    async fn query(&self, path: &str, data: &[u8]) -> Result<Vec<u8>> {
        let query_data = general_purpose::STANDARD.encode(data);
        
        let response = self.rpc_query("/abci_query", Some(serde_json::json!({
            "path": path,
            "data": query_data,
            "prove": false
        }))).await?;
        
        if let Some(result) = response.get("result") {
            if let Some(response_data) = result.get("response") {
                if let Some(value) = response_data.get("value") {
                    if let Some(value_str) = value.as_str() {
                        return Ok(general_purpose::STANDARD.decode(value_str)?);
                    }
                }
            }
        }
        
        anyhow::bail!("Could not extract query result from Cosmos response")
    }
    
    async fn subscribe_events(&self) -> Result<Box<dyn EventStream>> {
        // TODO: Implement Cosmos WebSocket event subscription
        Ok(Box::new(CosmosEventStream::new()))
    }
    
    // IBC-specific operations for Cosmos chains
    
    async fn create_client(&self, client_state: Value, consensus_state: Value) -> Result<String> {
        info!("Creating IBC client on Cosmos chain");
        
        // TODO: Build MsgCreateClient transaction
        // This requires:
        // 1. Building the protobuf message
        // 2. Signing with the relayer's key
        // 3. Broadcasting to the network
        
        warn!("Cosmos client creation not yet implemented");
        Ok("mock_client_id".to_string())
    }
    
    async fn update_client(&self, client_id: &str, header: Value) -> Result<()> {
        info!("Updating IBC client {} on Cosmos chain", client_id);
        
        // TODO: Build MsgUpdateClient transaction
        warn!("Cosmos client update not yet implemented");
        Ok(())
    }
    
    async fn send_packet(&self, packet: IbcPacket) -> Result<u64> {
        info!("Sending IBC packet on Cosmos chain");
        
        // TODO: Build MsgTransfer or custom packet send transaction
        warn!("Cosmos packet send not yet implemented");
        Ok(packet.sequence)
    }
    
    async fn recv_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()> {
        info!("Receiving IBC packet on Cosmos chain");
        
        // TODO: Build MsgRecvPacket transaction
        warn!("Cosmos packet receive not yet implemented");
        Ok(())
    }
    
    async fn ack_packet(&self, packet: IbcPacket, ack: Vec<u8>, proof: Vec<u8>) -> Result<()> {
        info!("Acknowledging IBC packet on Cosmos chain");
        
        // TODO: Build MsgAcknowledgement transaction
        warn!("Cosmos packet acknowledgment not yet implemented");
        Ok(())
    }
    
    async fn timeout_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()> {
        info!("Timing out IBC packet on Cosmos chain");
        
        // TODO: Build MsgTimeout transaction
        warn!("Cosmos packet timeout not yet implemented");
        Ok(())
    }
}

/// Cosmos event stream implementation
pub struct CosmosEventStream {
    // TODO: Add WebSocket connection
}

impl CosmosEventStream {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventStream for CosmosEventStream {
    async fn next_event(&mut self) -> Result<Option<ChainEvent>> {
        // TODO: Implement Cosmos event streaming via WebSocket
        // This should:
        // 1. Connect to Tendermint WebSocket
        // 2. Subscribe to relevant IBC events
        // 3. Parse and return structured events
        
        warn!("Cosmos event streaming not yet implemented");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        Ok(None)
    }
}