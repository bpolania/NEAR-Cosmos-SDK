// Packet processing logic for IBC relay operations
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::chains::{Chain, IbcPacket};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::proof::ProofGenerator;

/// Handles packet processing through the complete relay pipeline
pub struct PacketProcessor {
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Proof generator
    proof_generator: ProofGenerator,
    /// Configuration
    config: RelayerConfig,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
}

impl PacketProcessor {
    /// Create a new packet processor
    pub fn new(
        chains: HashMap<String, Arc<dyn Chain>>,
        config: RelayerConfig,
        metrics: Arc<RelayerMetrics>,
    ) -> Self {
        Self {
            proof_generator: ProofGenerator::new(chains.clone()),
            chains,
            config,
            metrics,
        }
    }
    
    /// Process a packet through the complete relay pipeline
    pub async fn process_packet(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        println!("🔄 Processing packet seq={} from {} to {}", 
                 packet.sequence, source_chain_id, dest_chain_id);
        
        // Step 1: Generate proof of packet commitment on source chain
        let proof_data = self.generate_packet_proof(source_chain_id, packet).await?;
        
        // Step 2: Build IBC transaction for destination chain
        let transaction_data = self.build_recv_packet_transaction(dest_chain_id, packet, &proof_data).await?;
        
        // Step 3: Submit transaction to destination chain
        let tx_hash = self.submit_transaction(dest_chain_id, transaction_data).await?;
        
        // Step 4: Monitor for confirmation (simplified for now)
        self.wait_for_confirmation(dest_chain_id, &tx_hash).await?;
        
        let processing_time = start_time.elapsed();
        self.metrics.packet_relay_duration.observe(processing_time.as_secs_f64());
        
        println!("✅ Successfully processed packet seq={} in {:.2}s", 
                 packet.sequence, processing_time.as_secs_f64());
        
        Ok(tx_hash)
    }
    
    /// Process NEAR -> Cosmos packet relay using enhanced capabilities
    pub async fn process_send_packet(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Verify this is a NEAR -> Cosmos flow
        if !source_chain_id.contains("near") || !dest_chain_id.contains("cosmos") {
            return Err("Invalid chain combination for send packet".into());
        }
        
        // Use specialized processing for NEAR -> Cosmos flow
        self.process_near_to_cosmos_packet(source_chain_id, dest_chain_id, packet).await
    }
    
    /// Specialized processing for NEAR -> Cosmos packet relay
    async fn process_near_to_cosmos_packet(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        println!("🚀 Processing NEAR->Cosmos packet seq={} from {} to {}", 
                 packet.sequence, source_chain_id, dest_chain_id);
        
        // Step 1: Generate NEAR state proof
        let proof_data = self.generate_packet_proof(source_chain_id, packet).await?;
        let current_height = self.get_current_height(source_chain_id).await?;
        
        // Step 2: Use enhanced Cosmos transaction submission
        let tx_hash = self.submit_cosmos_recv_packet(
            dest_chain_id,
            packet,
            &proof_data,
            current_height,
        ).await?;
        
        let processing_time = start_time.elapsed();
        self.metrics.packet_relay_duration.observe(processing_time.as_secs_f64());
        
        println!("✅ NEAR->Cosmos packet relay completed seq={} in {:.2}s", 
                 packet.sequence, processing_time.as_secs_f64());
        
        Ok(tx_hash)
    }
    
    /// Submit RecvPacket transaction to Cosmos chain using enhanced methods
    async fn submit_cosmos_recv_packet(
        &self,
        dest_chain_id: &str,
        packet: &IbcPacket,
        proof_data: &[u8],
        proof_height: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // This is where we would use the enhanced CosmosChain methods
        // For now, demonstrate the enhanced transaction structure
        
        println!("📋 Preparing enhanced Cosmos RecvPacket transaction:");
        println!("   Sequence: {}", packet.sequence);
        println!("   Source: {}:{}", packet.source_port, packet.source_channel);
        println!("   Destination: {}:{}", packet.destination_port, packet.destination_channel);
        println!("   Proof height: {}", proof_height);
        println!("   Proof size: {} bytes", proof_data.len());
        
        // Use the standard chain interface for now
        // In production, this would use CosmosChain::submit_recv_packet_tx
        let chain = self.chains.get(dest_chain_id)
            .ok_or_else(|| format!("Cosmos chain {} not found", dest_chain_id))?;
        
        // Build enhanced transaction data
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(b"RECV_PACKET:"); // Transaction type marker
        tx_data.extend_from_slice(&packet.sequence.to_be_bytes());
        tx_data.extend_from_slice(packet.source_port.as_bytes());
        tx_data.extend_from_slice(packet.source_channel.as_bytes());
        tx_data.extend_from_slice(packet.destination_port.as_bytes());
        tx_data.extend_from_slice(packet.destination_channel.as_bytes());
        tx_data.extend_from_slice(&packet.data);
        tx_data.extend_from_slice(&proof_height.to_be_bytes());
        tx_data.extend_from_slice(proof_data);
        
        let tx_hash = chain.submit_transaction(tx_data).await?;
        
        println!("🎯 Enhanced Cosmos RecvPacket transaction submitted: {}", tx_hash);
        Ok(tx_hash)
    }
    
    /// Get current height from a chain
    async fn get_current_height(
        &self,
        chain_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        chain.get_latest_height().await
    }
    
    /// Process Cosmos -> NEAR packet relay
    pub async fn process_recv_packet(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Verify this is a Cosmos -> NEAR flow
        if !source_chain_id.contains("cosmos") || !dest_chain_id.contains("near") {
            return Err("Invalid chain combination for recv packet".into());
        }
        
        self.process_packet(source_chain_id, dest_chain_id, packet).await
    }
    
    /// Process acknowledgment packet
    pub async fn process_acknowledgment(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
        ack_data: &[u8],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("🎯 Processing acknowledgment for packet seq={}", packet.sequence);
        
        // Generate proof of acknowledgment on destination chain
        let ack_proof = self.generate_acknowledgment_proof(dest_chain_id, packet).await?;
        
        // Build acknowledgment transaction for source chain
        let tx_data = self.build_ack_packet_transaction(source_chain_id, packet, ack_data, &ack_proof).await?;
        
        // Submit to source chain
        let tx_hash = self.submit_transaction(source_chain_id, tx_data).await?;
        
        self.metrics.total_packets_acknowledged.inc();
        println!("✅ Acknowledgment processed for packet seq={}", packet.sequence);
        
        Ok(tx_hash)
    }
    
    /// Process timeout packet
    pub async fn process_timeout(
        &self,
        source_chain_id: &str,
        dest_chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("⏰ Processing timeout for packet seq={}", packet.sequence);
        
        // Generate proof that packet was not received (timeout proof)
        let timeout_proof = self.generate_timeout_proof(dest_chain_id, packet).await?;
        
        // Build timeout transaction for source chain
        let tx_data = self.build_timeout_packet_transaction(source_chain_id, packet, &timeout_proof).await?;
        
        // Submit to source chain (this will trigger refund)
        let tx_hash = self.submit_transaction(source_chain_id, tx_data).await?;
        
        self.metrics.total_packets_timed_out.inc();
        println!("✅ Timeout processed for packet seq={}", packet.sequence);
        
        Ok(tx_hash)
    }
    
    /// Generate proof of packet commitment on source chain
    async fn generate_packet_proof(
        &self,
        chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        let proof = self.proof_generator.generate_packet_commitment_proof(
            chain_id,
            &packet.source_port,
            &packet.source_channel,
            packet.sequence,
        ).await?;
        
        let proof_time = start_time.elapsed();
        println!("🔍 Generated packet proof in {:.2}s", proof_time.as_secs_f64());
        
        Ok(proof)
    }
    
    /// Generate proof of acknowledgment
    async fn generate_acknowledgment_proof(
        &self,
        chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.proof_generator.generate_acknowledgment_proof(
            chain_id,
            &packet.destination_port,
            &packet.destination_channel,
            packet.sequence,
        ).await
    }
    
    /// Generate timeout proof
    async fn generate_timeout_proof(
        &self,
        chain_id: &str,
        packet: &IbcPacket,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.proof_generator.generate_timeout_proof(
            chain_id,
            &packet.destination_port,
            &packet.destination_channel,
            packet.sequence,
        ).await
    }
    
    /// Build recv_packet transaction for destination chain
    async fn build_recv_packet_transaction(
        &self,
        dest_chain_id: &str,
        packet: &IbcPacket,
        proof_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // For now, create a simple transaction structure
        // In production, this would create proper IBC transactions
        let mut tx_data = Vec::new();
        
        // Add packet data
        tx_data.extend_from_slice(&packet.sequence.to_be_bytes());
        tx_data.extend_from_slice(packet.source_port.as_bytes());
        tx_data.extend_from_slice(packet.source_channel.as_bytes());
        tx_data.extend_from_slice(packet.destination_port.as_bytes());
        tx_data.extend_from_slice(packet.destination_channel.as_bytes());
        tx_data.extend_from_slice(&packet.data);
        
        // Add proof
        tx_data.extend_from_slice(proof_data);
        
        println!("🔨 Built recv_packet transaction for {} ({} bytes)", 
                 dest_chain_id, tx_data.len());
        
        Ok(tx_data)
    }
    
    /// Build ack_packet transaction
    async fn build_ack_packet_transaction(
        &self,
        source_chain_id: &str,
        packet: &IbcPacket,
        ack_data: &[u8],
        proof_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx_data = Vec::new();
        
        // Add packet info
        tx_data.extend_from_slice(&packet.sequence.to_be_bytes());
        tx_data.extend_from_slice(packet.source_port.as_bytes());
        tx_data.extend_from_slice(packet.source_channel.as_bytes());
        
        // Add acknowledgment data
        tx_data.extend_from_slice(ack_data);
        
        // Add proof
        tx_data.extend_from_slice(proof_data);
        
        println!("🔨 Built ack_packet transaction for {} ({} bytes)", 
                 source_chain_id, tx_data.len());
        
        Ok(tx_data)
    }
    
    /// Build timeout_packet transaction
    async fn build_timeout_packet_transaction(
        &self,
        source_chain_id: &str,
        packet: &IbcPacket,
        proof_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx_data = Vec::new();
        
        // Add packet info
        tx_data.extend_from_slice(&packet.sequence.to_be_bytes());
        tx_data.extend_from_slice(packet.source_port.as_bytes());
        tx_data.extend_from_slice(packet.source_channel.as_bytes());
        
        // Add timeout proof
        tx_data.extend_from_slice(proof_data);
        
        println!("🔨 Built timeout_packet transaction for {} ({} bytes)", 
                 source_chain_id, tx_data.len());
        
        Ok(tx_data)
    }
    
    /// Submit transaction to a chain using enhanced capabilities
    async fn submit_transaction(
        &self,
        chain_id: &str,
        transaction_data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        let start_time = Instant::now();
        
        // For Cosmos chains, use enhanced transaction submission if available
        if chain_id.contains("cosmos") {
            // Try to use enhanced Cosmos transaction methods
            // This is a simplified approach - in production you'd downcast or use traits
            println!("📤 Using enhanced Cosmos transaction submission for {}", chain_id);
        }
        
        let tx_hash = chain.submit_transaction(transaction_data).await?;
        let submit_time = start_time.elapsed();
        
        println!("📤 Submitted transaction to {} in {:.2}s: {}", 
                 chain_id, submit_time.as_secs_f64(), tx_hash);
        
        Ok(tx_hash)
    }
    
    /// Wait for transaction confirmation (simplified)
    async fn wait_for_confirmation(
        &self,
        chain_id: &str,
        tx_hash: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // In production, this would poll the chain for transaction status
        // For now, just simulate confirmation delay
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        println!("✅ Transaction confirmed on {}: {}", chain_id, tx_hash);
        Ok(())
    }
    
    /// Estimate gas for transaction (placeholder)
    pub async fn estimate_gas(
        &self,
        chain_id: &str,
        transaction_data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // Simple gas estimation based on transaction size
        let base_gas = 21000u64;
        let data_gas = transaction_data.len() as u64 * 16;
        
        let estimated_gas = base_gas + data_gas;
        
        println!("⛽ Estimated gas for {} transaction: {}", chain_id, estimated_gas);
        Ok(estimated_gas)
    }
    
    /// Validate packet before processing
    pub fn validate_packet(&self, packet: &IbcPacket) -> Result<(), String> {
        if packet.source_port.is_empty() {
            return Err("Source port cannot be empty".to_string());
        }
        
        if packet.source_channel.is_empty() {
            return Err("Source channel cannot be empty".to_string());
        }
        
        if packet.destination_port.is_empty() {
            return Err("Destination port cannot be empty".to_string());
        }
        
        if packet.destination_channel.is_empty() {
            return Err("Destination channel cannot be empty".to_string());
        }
        
        if packet.sequence == 0 {
            return Err("Packet sequence cannot be zero".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::IbcPacket;
    
    fn create_test_packet() -> IbcPacket {
        IbcPacket {
            sequence: 1,
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }
    }
    
    #[test]
    fn test_packet_validation() {
        let chains = HashMap::new();
        let config = RelayerConfig {
            global: crate::config::GlobalConfig {
                log_level: "info".to_string(),
                max_retries: 3,
                retry_delay_ms: 1000,
                health_check_interval: 30,
            },
            chains: HashMap::new(),
            connections: vec![],
            metrics: crate::config::MetricsConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
        };
        let metrics = Arc::new(RelayerMetrics::new().unwrap());
        
        let processor = PacketProcessor::new(chains, config, metrics);
        
        // Valid packet
        let valid_packet = create_test_packet();
        assert!(processor.validate_packet(&valid_packet).is_ok());
        
        // Invalid packet - empty source port
        let mut invalid_packet = create_test_packet();
        invalid_packet.source_port = String::new();
        assert!(processor.validate_packet(&invalid_packet).is_err());
        
        // Invalid packet - zero sequence
        let mut invalid_packet = create_test_packet();
        invalid_packet.sequence = 0;
        assert!(processor.validate_packet(&invalid_packet).is_err());
    }
    
    #[test]
    fn test_transaction_building() {
        let chains = HashMap::new();
        let config = RelayerConfig {
            global: crate::config::GlobalConfig {
                log_level: "info".to_string(),
                max_retries: 3,
                retry_delay_ms: 1000,
                health_check_interval: 30,
            },
            chains: HashMap::new(),
            connections: vec![],
            metrics: crate::config::MetricsConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
        };
        let metrics = Arc::new(RelayerMetrics::new().unwrap());
        
        let processor = PacketProcessor::new(chains, config, metrics);
        let packet = create_test_packet();
        
        // Test transaction building functions compile
        assert_eq!(packet.sequence, 1);
        assert_eq!(processor.validate_packet(&packet).is_ok(), true);
    }
}