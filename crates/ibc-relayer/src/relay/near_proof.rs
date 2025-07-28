// NEAR-specific state proof generation for IBC packets
use std::sync::Arc;
use near_jsonrpc_client::JsonRpcClient;
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    types::{AccountId, BlockHeight, BlockReference},
    views::QueryRequest,
    hash::CryptoHash,
};
use sha2::{Sha256, Digest};

use crate::chains::{Chain, near_simple::NearChain};

/// NEAR-specific proof generator that creates real blockchain state proofs
pub struct NearProofGenerator {
    chain_id: String,
    contract_id: AccountId,
    rpc_client: JsonRpcClient,
}

impl NearProofGenerator {
    /// Create a new NEAR proof generator
    pub fn new(
        chain_id: String,
        contract_id: AccountId,
        rpc_client: JsonRpcClient,
    ) -> Self {
        Self {
            chain_id,
            contract_id,
            rpc_client,
        }
    }
    
    /// Generate real state proof for packet commitment
    pub async fn generate_packet_commitment_proof(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
        block_height: Option<BlockHeight>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Generating NEAR state proof for packet commitment seq={}", sequence);
        
        // Step 1: Get the storage key for the packet commitment
        let storage_key = self.packet_commitment_storage_key(port_id, channel_id, sequence);
        
        // Step 2: Query NEAR state with proof
        let proof = self.get_state_proof_with_data(&storage_key, block_height).await?;
        
        // Step 3: Format as IBC-compatible proof
        let ibc_proof = self.format_as_ibc_proof(&proof, &storage_key, "packet_commitment").await?;
        
        println!("✅ Generated NEAR state proof ({} bytes)", ibc_proof.len());
        Ok(ibc_proof)
    }
    
    /// Generate real state proof for packet acknowledgment
    pub async fn generate_acknowledgment_proof(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
        block_height: Option<BlockHeight>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("🎯 Generating NEAR state proof for acknowledgment seq={}", sequence);
        
        let storage_key = self.packet_acknowledgment_storage_key(port_id, channel_id, sequence);
        let proof = self.get_state_proof_with_data(&storage_key, block_height).await?;
        let ibc_proof = self.format_as_ibc_proof(&proof, &storage_key, "packet_acknowledgment").await?;
        
        println!("✅ Generated NEAR acknowledgment proof ({} bytes)", ibc_proof.len());
        Ok(ibc_proof)
    }
    
    /// Generate real timeout proof (non-existence)
    pub async fn generate_timeout_proof(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
        block_height: Option<BlockHeight>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("⏰ Generating NEAR timeout proof (non-existence) seq={}", sequence);
        
        // For timeout, we need to prove the packet receipt does NOT exist
        let _storage_key = self.packet_receipt_storage_key(port_id, channel_id, sequence);
        
        // Get the next sequence to prove this packet wasn't received
        let next_seq_key = self.next_sequence_recv_storage_key(port_id, channel_id);
        let next_seq_proof = self.get_state_proof_with_data(&next_seq_key, block_height).await?;
        
        let ibc_proof = self.format_as_ibc_proof(&next_seq_proof, &next_seq_key, "timeout_proof").await?;
        
        println!("✅ Generated NEAR timeout proof ({} bytes)", ibc_proof.len());
        Ok(ibc_proof)
    }
    
    /// Get state proof from NEAR blockchain with actual data
    async fn get_state_proof_with_data(
        &self,
        storage_key: &str,
        block_height: Option<BlockHeight>,
    ) -> Result<NearStateProof, Box<dyn std::error::Error + Send + Sync>> {
        let block_ref = match block_height {
            Some(height) => BlockReference::BlockId(near_primitives::types::BlockId::Height(height)),
            None => BlockReference::latest(),
        };
        
        // Query with proof
        let request = near_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: block_ref,
            request: QueryRequest::ViewState {
                account_id: self.contract_id.clone(),
                prefix: storage_key.as_bytes().to_vec().into(),
                include_proof: true,
            },
        };
        
        let response = self.rpc_client.call(request).await
            .map_err(|e| format!("NEAR state query failed: {}", e))?;
        
        match response.kind {
            QueryResponseKind::ViewState(view_state) => {
                let block_height = response.block_height;
                let block_hash = response.block_hash;
                
                Ok(NearStateProof {
                    block_height,
                    block_hash,
                    account_id: self.contract_id.clone(),
                    storage_key: storage_key.to_string(),
                    storage_proof: {
                        let mut combined_proof = Vec::new();
                        for chunk in view_state.proof {
                            combined_proof.extend_from_slice(&chunk);
                        }
                        combined_proof
                    },
                    values: view_state.values,
                })
            }
            _ => Err("Unexpected response type for state proof".into()),
        }
    }
    
    /// Format NEAR state proof as IBC-compatible proof
    async fn format_as_ibc_proof(
        &self,
        near_proof: &NearStateProof,
        storage_key: &str,
        proof_type: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Create IBC-compatible proof structure
        let mut ibc_proof = Vec::new();
        
        // Header: IBC proof format version
        ibc_proof.extend_from_slice(b"IBC_NEAR_PROOF_V1:");
        
        // Chain and contract information
        ibc_proof.extend_from_slice(self.chain_id.as_bytes());
        ibc_proof.push(b':');
        ibc_proof.extend_from_slice(self.contract_id.as_str().as_bytes());
        ibc_proof.push(b':');
        
        // Block information
        ibc_proof.extend_from_slice(&near_proof.block_height.to_be_bytes());
        ibc_proof.push(b':');
        ibc_proof.extend_from_slice(near_proof.block_hash.as_ref());
        ibc_proof.push(b':');
        
        // Proof type and storage key
        ibc_proof.extend_from_slice(proof_type.as_bytes());
        ibc_proof.push(b':');
        ibc_proof.extend_from_slice(storage_key.as_bytes());
        ibc_proof.push(b':');
        
        // Storage values (if any)
        if !near_proof.values.is_empty() {
            let values_json = serde_json::to_vec(&near_proof.values)
                .map_err(|e| format!("Failed to serialize storage values: {}", e))?;
            ibc_proof.extend_from_slice(&values_json);
        }
        ibc_proof.push(b':');
        
        // NEAR merkle proof data
        ibc_proof.extend_from_slice(&near_proof.storage_proof);
        
        // Add integrity hash
        let mut hasher = Sha256::new();
        hasher.update(&ibc_proof);
        let hash = hasher.finalize();
        ibc_proof.push(b':');
        ibc_proof.extend_from_slice(&hash);
        
        Ok(ibc_proof)
    }
    
    /// Generate storage key for packet commitment (following IBC spec)
    fn packet_commitment_storage_key(&self, port_id: &str, channel_id: &str, sequence: u64) -> String {
        format!("commitments/ports/{}/channels/{}/sequences/{}", port_id, channel_id, sequence)
    }
    
    /// Generate storage key for packet acknowledgment
    fn packet_acknowledgment_storage_key(&self, port_id: &str, channel_id: &str, sequence: u64) -> String {
        format!("acks/ports/{}/channels/{}/sequences/{}", port_id, channel_id, sequence)
    }
    
    /// Generate storage key for packet receipt
    fn packet_receipt_storage_key(&self, port_id: &str, channel_id: &str, sequence: u64) -> String {
        format!("receipts/ports/{}/channels/{}/sequences/{}", port_id, channel_id, sequence)
    }
    
    /// Generate storage key for next sequence receive
    fn next_sequence_recv_storage_key(&self, port_id: &str, channel_id: &str) -> String {
        format!("nextSequenceRecv/ports/{}/channels/{}", port_id, channel_id)
    }
    
    /// Validate proof integrity
    pub fn validate_proof(&self, proof_data: &[u8]) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if proof_data.len() < 32 {
            return Ok(false);
        }
        
        // Extract the integrity hash from the end
        let (proof_body, hash_bytes) = proof_data.split_at(proof_data.len() - 33); // 32 bytes hash + 1 separator
        
        if hash_bytes[0] != b':' {
            return Ok(false);
        }
        
        let provided_hash = &hash_bytes[1..];
        
        // Recalculate hash
        let mut hasher = Sha256::new();
        hasher.update(proof_body);
        let calculated_hash = hasher.finalize();
        
        Ok(provided_hash == calculated_hash.as_slice())
    }
}

/// NEAR state proof structure
#[derive(Debug, Clone)]
pub struct NearStateProof {
    pub block_height: BlockHeight,
    pub block_hash: CryptoHash,
    pub account_id: AccountId,
    pub storage_key: String,
    pub storage_proof: Vec<u8>,
    pub values: Vec<near_primitives::views::StateItem>,
}

/// Extract NEAR chain for proof generation
pub fn extract_near_chain(_chain: &Arc<dyn Chain>) -> Option<&NearChain> {
    // This is a bit hacky - in production we'd use a proper trait cast
    // For now, we'll implement this when we integrate with the proof generator
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_key_generation() {
        let proof_gen = NearProofGenerator::new(
            "near-testnet".to_string(),
            "test.testnet".parse().unwrap(),
            JsonRpcClient::connect("https://rpc.testnet.near.org"),
        );
        
        assert_eq!(
            proof_gen.packet_commitment_storage_key("transfer", "channel-0", 1),
            "commitments/ports/transfer/channels/channel-0/sequences/1"
        );
        
        assert_eq!(
            proof_gen.packet_acknowledgment_storage_key("transfer", "channel-0", 1),
            "acks/ports/transfer/channels/channel-0/sequences/1"
        );
        
        assert_eq!(
            proof_gen.next_sequence_recv_storage_key("transfer", "channel-0"),
            "nextSequenceRecv/ports/transfer/channels/channel-0"
        );
    }
    
    #[test]
    fn test_proof_validation() {
        let proof_gen = NearProofGenerator::new(
            "near-testnet".to_string(),
            "test.testnet".parse().unwrap(),
            JsonRpcClient::connect("https://rpc.testnet.near.org"),
        );
        
        // Test with invalid proof
        let invalid_proof = b"invalid proof data";
        assert!(!proof_gen.validate_proof(invalid_proof).unwrap());
        
        // Test with empty proof
        let empty_proof = b"";
        assert!(!proof_gen.validate_proof(empty_proof).unwrap());
    }
}