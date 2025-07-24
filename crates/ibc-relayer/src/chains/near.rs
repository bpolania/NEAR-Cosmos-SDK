use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, debug, warn};
use base64::{Engine as _, engine::general_purpose};

use crate::chains::{Chain, EventStream, ChainEvent, IbcPacket};
use crate::config::{ChainConfig, ChainSpecificConfig};

/// NEAR chain implementation that interfaces with our Cosmos SDK contract
pub struct NearChain {
    chain_id: String,
    rpc_endpoint: String,
    contract_id: String,
    signer_account_id: String,
    network_id: String,
    client: reqwest::Client,
}

impl NearChain {
    pub fn new(config: &ChainConfig) -> Result<Self> {
        if let ChainSpecificConfig::Near { 
            contract_id, 
            signer_account_id, 
            network_id,
            .. 
        } = &config.config {
            Ok(Self {
                chain_id: config.chain_id.clone(),
                rpc_endpoint: config.rpc_endpoint.clone(),
                contract_id: contract_id.clone(),
                signer_account_id: signer_account_id.clone(),
                network_id: network_id.clone(),
                client: reqwest::Client::new(),
            })
        } else {
            anyhow::bail!("Invalid NEAR chain configuration")
        }
    }

    /// Call a view function on our Cosmos SDK contract
    async fn call_view_function(&self, method_name: &str, args: Value) -> Result<Value> {
        debug!("Calling view function: {} with args: {}", method_name, args);
        
        let args_base64 = if args.is_null() {
            String::new()
        } else {
            general_purpose::STANDARD.encode(args.to_string())
        };

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "query",
            "params": {
                "request_type": "call_function",
                "finality": "final",
                "account_id": self.contract_id,
                "method_name": method_name,
                "args_base64": args_base64
            }
        });

        let response = self.client
            .post(&self.rpc_endpoint)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("RPC request failed with status: {}", response.status());
        }

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        Ok(response_json)
    }

    /// Parse RPC result as string
    fn parse_rpc_result_as_string(response: &Value) -> Result<String> {
        if let Some(result) = response.get("result") {
            if let Some(result_data) = result.get("result") {
                if let Some(result_array) = result_data.as_array() {
                    let result_bytes: Vec<u8> = result_array.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                    return Ok(String::from_utf8(result_bytes)?);
                }
            }
        }
        anyhow::bail!("Could not parse RPC result")
    }

    /// Call a change function on our Cosmos SDK contract
    async fn call_change_function(&self, method_name: &str, args: Value) -> Result<String> {
        info!("Calling change function: {} with args: {}", method_name, args);
        
        // TODO: Implement transaction signing and submission
        // This requires:
        // 1. Building a NEAR transaction
        // 2. Signing with the relayer's private key
        // 3. Submitting to the network
        // 4. Waiting for confirmation
        
        warn!("Change function calls not yet implemented");
        Ok("mock_tx_hash".to_string())
    }
}

#[async_trait]
impl Chain for NearChain {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    
    async fn get_height(&self) -> Result<u64> {
        let response = self.call_view_function("get_block_height", json!({})).await?;
        let height_str = Self::parse_rpc_result_as_string(&response)?;
        Ok(height_str.parse()?)
    }
    
    async fn get_header(&self, height: u64) -> Result<Value> {
        // Get NEAR block header at specific height
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "block",
            "params": {
                "block_id": height
            }
        });

        let response = self.client
            .post(&self.rpc_endpoint)
            .json(&request_body)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        Ok(response_json)
    }
    
    async fn submit_tx(&self, tx: Vec<u8>) -> Result<String> {
        // TODO: Submit signed transaction to NEAR
        warn!("Transaction submission not yet implemented");
        Ok("mock_tx_hash".to_string())
    }
    
    async fn query(&self, path: &str, data: &[u8]) -> Result<Vec<u8>> {
        // Query our contract state based on path
        debug!("Querying path: {}", path);
        
        // Map IBC query paths to our contract functions
        let result = match path {
            "/ibc.core.client.v1.Query/ClientState" => {
                let args = json!({"client_id": String::from_utf8_lossy(data)});
                self.call_view_function("ibc_get_client_state", args).await?
            }
            "/ibc.core.connection.v1.Query/Connection" => {
                let args = json!({"connection_id": String::from_utf8_lossy(data)});
                self.call_view_function("ibc_get_connection", args).await?
            }
            "/ibc.core.channel.v1.Query/Channel" => {
                // Parse port_id and channel_id from data
                let args = json!({"port_id": "transfer", "channel_id": String::from_utf8_lossy(data)});
                self.call_view_function("ibc_get_channel", args).await?
            }
            _ => {
                warn!("Unknown query path: {}", path);
                json!({})
            }
        };

        Ok(result.to_string().into_bytes())
    }
    
    async fn subscribe_events(&self) -> Result<Box<dyn EventStream>> {
        // TODO: Implement NEAR event subscription
        // This could use WebSocket or polling
        Ok(Box::new(NearEventStream::new()))
    }
    
    // IBC-specific operations using our contract
    
    async fn create_client(&self, client_state: Value, consensus_state: Value) -> Result<String> {
        let args = json!({
            "client_state": client_state,
            "consensus_state": consensus_state
        });
        
        self.call_change_function("ibc_create_client", args).await
    }
    
    async fn update_client(&self, client_id: &str, header: Value) -> Result<()> {
        let args = json!({
            "client_id": client_id,
            "header": header
        });
        
        self.call_change_function("ibc_update_client", args).await?;
        Ok(())
    }
    
    async fn send_packet(&self, packet: IbcPacket) -> Result<u64> {
        let args = json!({
            "source_port": packet.source_port,
            "source_channel": packet.source_channel,
            "destination_port": packet.destination_port,
            "destination_channel": packet.destination_channel,
            "data": general_purpose::STANDARD.encode(&packet.data),
            "timeout_height": packet.timeout_height,
            "timeout_timestamp": packet.timeout_timestamp
        });
        
        let tx_hash = self.call_change_function("ibc_send_packet", args).await?;
        info!("Sent packet via tx: {}", tx_hash);
        
        // Return sequence number (mock for now)
        Ok(packet.sequence)
    }
    
    async fn recv_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()> {
        let args = json!({
            "packet": {
                "sequence": packet.sequence,
                "source_port": packet.source_port,
                "source_channel": packet.source_channel,
                "destination_port": packet.destination_port,
                "destination_channel": packet.destination_channel,
                "data": general_purpose::STANDARD.encode(&packet.data),
                "timeout_height": packet.timeout_height,
                "timeout_timestamp": packet.timeout_timestamp
            },
            "proof": general_purpose::STANDARD.encode(&proof)
        });
        
        self.call_change_function("ibc_recv_packet", args).await?;
        Ok(())
    }
    
    async fn ack_packet(&self, packet: IbcPacket, ack: Vec<u8>, proof: Vec<u8>) -> Result<()> {
        let args = json!({
            "packet": {
                "sequence": packet.sequence,
                "source_port": packet.source_port,
                "source_channel": packet.source_channel,
                "destination_port": packet.destination_port,
                "destination_channel": packet.destination_channel,
                "data": general_purpose::STANDARD.encode(&packet.data),
                "timeout_height": packet.timeout_height,
                "timeout_timestamp": packet.timeout_timestamp
            },
            "acknowledgement": general_purpose::STANDARD.encode(&ack),
            "proof": general_purpose::STANDARD.encode(&proof)
        });
        
        self.call_change_function("ibc_acknowledge_packet", args).await?;
        Ok(())
    }
    
    async fn timeout_packet(&self, packet: IbcPacket, proof: Vec<u8>) -> Result<()> {
        let args = json!({
            "packet": {
                "sequence": packet.sequence,
                "source_port": packet.source_port,
                "source_channel": packet.source_channel,
                "destination_port": packet.destination_port,
                "destination_channel": packet.destination_channel,
                "data": general_purpose::STANDARD.encode(&packet.data),
                "timeout_height": packet.timeout_height,
                "timeout_timestamp": packet.timeout_timestamp
            },
            "proof": general_purpose::STANDARD.encode(&proof)
        });
        
        self.call_change_function("ibc_timeout_packet", args).await?;
        Ok(())
    }
}

/// NEAR event stream implementation
pub struct NearEventStream {
    // TODO: Add WebSocket or polling mechanism
}

impl NearEventStream {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventStream for NearEventStream {
    async fn next_event(&mut self) -> Result<Option<ChainEvent>> {
        // TODO: Implement event streaming
        // This could:
        // 1. Poll NEAR RPC for new blocks
        // 2. Parse transaction logs for IBC events
        // 3. Return structured events
        
        warn!("Event streaming not yet implemented");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        Ok(None)
    }
}