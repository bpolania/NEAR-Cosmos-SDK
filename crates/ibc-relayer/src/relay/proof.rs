// Proof generation and validation for IBC packets
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::chains::Chain;

/// Generates and validates proofs for IBC packet relay
pub struct ProofGenerator {
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Proof cache for optimization
    proof_cache: std::sync::RwLock<HashMap<String, CachedProof>>,
}

/// Cached proof data with expiration
#[derive(Debug, Clone)]
struct CachedProof {
    data: Vec<u8>,
    generated_at: Instant,
    expires_at: Instant,
}

impl ProofGenerator {
    /// Create a new proof generator
    pub fn new(chains: HashMap<String, Arc<dyn Chain>>) -> Self {
        Self {
            chains,
            proof_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }
    
    /// Generate proof of packet commitment on source chain
    pub async fn generate_packet_commitment_proof(
        &self,
        chain_id: &str,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("commitment:{}:{}:{}:{}", chain_id, port_id, channel_id, sequence);
        
        // Check cache first
        if let Some(cached_proof) = self.get_cached_proof(&cache_key) {
            println!("📁 Using cached packet commitment proof");
            return Ok(cached_proof);
        }
        
        let start_time = Instant::now();
        
        // Get chain implementation
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        // Query packet commitment from chain
        let commitment = chain.query_packet_commitment(port_id, channel_id, sequence).await?
            .ok_or_else(|| format!("Packet commitment not found for seq {}", sequence))?;
        
        // Generate proof (for now, mock implementation)
        let proof = self.generate_mock_proof(chain_id, "packet_commitment", &commitment).await?;
        
        // Cache the proof
        self.cache_proof(cache_key, proof.clone(), Duration::from_secs(300));
        
        let proof_time = start_time.elapsed();
        println!("🔍 Generated packet commitment proof in {:.2}s ({} bytes)", 
                 proof_time.as_secs_f64(), proof.len());
        
        Ok(proof)
    }
    
    /// Generate proof of packet acknowledgment
    pub async fn generate_acknowledgment_proof(
        &self,
        chain_id: &str,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("ack:{}:{}:{}:{}", chain_id, port_id, channel_id, sequence);
        
        // Check cache first
        if let Some(cached_proof) = self.get_cached_proof(&cache_key) {
            println!("📁 Using cached acknowledgment proof");
            return Ok(cached_proof);
        }
        
        let start_time = Instant::now();
        
        // Get chain implementation
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        // Query acknowledgment from chain
        let ack_data = chain.query_packet_acknowledgment(port_id, channel_id, sequence).await?
            .ok_or_else(|| format!("Packet acknowledgment not found for seq {}", sequence))?;
        
        // Generate proof
        let proof = self.generate_mock_proof(chain_id, "packet_acknowledgment", &ack_data).await?;
        
        // Cache the proof
        self.cache_proof(cache_key, proof.clone(), Duration::from_secs(300));
        
        let proof_time = start_time.elapsed();
        println!("🎯 Generated acknowledgment proof in {:.2}s ({} bytes)", 
                 proof_time.as_secs_f64(), proof.len());
        
        Ok(proof)
    }
    
    /// Generate timeout proof (proving packet was not received)
    pub async fn generate_timeout_proof(
        &self,
        chain_id: &str,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("timeout:{}:{}:{}:{}", chain_id, port_id, channel_id, sequence);
        
        // Check cache first
        if let Some(cached_proof) = self.get_cached_proof(&cache_key) {
            println!("📁 Using cached timeout proof");
            return Ok(cached_proof);
        }
        
        let start_time = Instant::now();
        
        // Get chain implementation
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        // Query next sequence receive to prove packet was not received
        let next_seq = chain.query_next_sequence_recv(port_id, channel_id).await?;
        
        if next_seq > sequence {
            return Err(format!("Packet {} was already received (next_seq: {})", sequence, next_seq).into());
        }
        
        // Generate timeout proof
        let seq_bytes = next_seq.to_be_bytes();
        let proof = self.generate_mock_proof(chain_id, "timeout", &seq_bytes).await?;
        
        // Cache the proof
        self.cache_proof(cache_key, proof.clone(), Duration::from_secs(60));
        
        let proof_time = start_time.elapsed();
        println!("⏰ Generated timeout proof in {:.2}s ({} bytes)", 
                 proof_time.as_secs_f64(), proof.len());
        
        Ok(proof)
    }
    
    /// Generate receipt proof (for unordered channels)
    pub async fn generate_receipt_proof(
        &self,
        chain_id: &str,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("receipt:{}:{}:{}:{}", chain_id, port_id, channel_id, sequence);
        
        // Check cache first
        if let Some(cached_proof) = self.get_cached_proof(&cache_key) {
            println!("📁 Using cached receipt proof");
            return Ok(cached_proof);
        }
        
        let start_time = Instant::now();
        
        // Get chain implementation
        let chain = self.chains.get(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        
        // Query packet receipt
        let has_receipt = chain.query_packet_receipt(port_id, channel_id, sequence).await?;
        
        if !has_receipt {
            return Err(format!("Packet receipt not found for seq {}", sequence).into());
        }
        
        // Generate receipt proof
        let receipt_data = [1u8]; // Simple receipt marker
        let proof = self.generate_mock_proof(chain_id, "packet_receipt", &receipt_data).await?;
        
        // Cache the proof
        self.cache_proof(cache_key, proof.clone(), Duration::from_secs(300));
        
        let proof_time = start_time.elapsed();
        println!("📨 Generated receipt proof in {:.2}s ({} bytes)", 
                 proof_time.as_secs_f64(), proof.len());
        
        Ok(proof)
    }
    
    /// Validate a proof against chain state
    pub async fn validate_proof(
        &self,
        chain_id: &str,
        proof_data: &[u8],
        expected_value: &[u8],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // For mock implementation, just check if proof contains expected value
        let proof_str = String::from_utf8_lossy(proof_data);
        let expected_str = String::from_utf8_lossy(expected_value);
        
        let is_valid = proof_str.contains(expected_str.as_ref());
        
        println!("🔍 Proof validation for {}: {}", chain_id, 
                 if is_valid { "✅ VALID" } else { "❌ INVALID" });
        
        Ok(is_valid)
    }
    
    /// Generate a mock proof for development/testing
    async fn generate_mock_proof(
        &self,
        chain_id: &str,
        proof_type: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate proof generation delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Create a mock proof structure
        let mut proof = Vec::new();
        
        // Add metadata
        proof.extend_from_slice(b"IBC_PROOF_V1:");
        proof.extend_from_slice(chain_id.as_bytes());
        proof.extend_from_slice(b":");
        proof.extend_from_slice(proof_type.as_bytes());
        proof.extend_from_slice(b":");
        
        // Add timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        proof.extend_from_slice(&timestamp.to_be_bytes());
        
        // Add data hash (mock)
        let data_hash = self.simple_hash(data);
        proof.extend_from_slice(&data_hash);
        
        // Add original data
        proof.extend_from_slice(data);
        
        Ok(proof)
    }
    
    /// Simple hash function for mock proofs
    fn simple_hash(&self, data: &[u8]) -> [u8; 8] {
        let mut hash = [0u8; 8];
        for (i, &byte) in data.iter().enumerate() {
            hash[i % 8] ^= byte;
        }
        hash
    }
    
    /// Cache a proof with expiration
    fn cache_proof(&self, key: String, proof: Vec<u8>, ttl: Duration) {
        let now = Instant::now();
        let cached_proof = CachedProof {
            data: proof,
            generated_at: now,
            expires_at: now + ttl,
        };
        
        if let Ok(mut cache) = self.proof_cache.write() {
            cache.insert(key, cached_proof);
        }
    }
    
    /// Get cached proof if valid
    fn get_cached_proof(&self, key: &str) -> Option<Vec<u8>> {
        if let Ok(cache) = self.proof_cache.read() {
            if let Some(cached) = cache.get(key) {
                if Instant::now() < cached.expires_at {
                    return Some(cached.data.clone());
                }
            }
        }
        None
    }
    
    /// Clean expired proofs from cache
    pub fn cleanup_cache(&self) {
        if let Ok(mut cache) = self.proof_cache.write() {
            let now = Instant::now();
            cache.retain(|_, proof| now < proof.expires_at);
        }
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.proof_cache.read() {
            let total = cache.len();
            let expired = cache.values()
                .filter(|proof| Instant::now() >= proof.expires_at)
                .count();
            (total, expired)
        } else {
            (0, 0)
        }
    }
    
    /// Generate batch proofs for multiple packets (optimization)
    pub async fn generate_batch_commitment_proofs(
        &self,
        chain_id: &str,
        port_id: &str,
        channel_id: &str,
        sequences: &[u64],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut proofs = Vec::new();
        
        println!("📦 Generating batch proofs for {} packets on {}", sequences.len(), chain_id);
        
        for &sequence in sequences {
            let proof = self.generate_packet_commitment_proof(chain_id, port_id, channel_id, sequence).await?;
            proofs.push(proof);
        }
        
        println!("✅ Generated {} batch proofs", proofs.len());
        Ok(proofs)
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proof_generator_creation() {
        let chains = HashMap::new();
        let generator = ProofGenerator::new(chains);
        
        // Test cache operations
        let (total, expired) = generator.get_cache_stats();
        assert_eq!(total, 0);
        assert_eq!(expired, 0);
    }
    
    #[test]
    fn test_proof_caching() {
        let generator = ProofGenerator::default();
        
        // Cache a proof
        generator.cache_proof(
            "test_key".to_string(),
            vec![1, 2, 3, 4],
            Duration::from_secs(60),
        );
        
        // Retrieve cached proof
        let cached = generator.get_cached_proof("test_key");
        assert_eq!(cached, Some(vec![1, 2, 3, 4]));
        
        // Try non-existent key
        let not_found = generator.get_cached_proof("nonexistent");
        assert_eq!(not_found, None);
    }
    
    #[test]
    fn test_simple_hash() {
        let generator = ProofGenerator::default();
        
        let data1 = b"hello world";
        let data2 = b"hello world";
        let data3 = b"different data";
        
        let hash1 = generator.simple_hash(data1);
        let hash2 = generator.simple_hash(data2);
        let hash3 = generator.simple_hash(data3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
    
    #[tokio::test]
    async fn test_mock_proof_generation() {
        let generator = ProofGenerator::default();
        
        let proof = generator.generate_mock_proof(
            "test-chain",
            "packet_commitment",
            b"test data",
        ).await.unwrap();
        
        assert!(!proof.is_empty());
        assert!(proof.starts_with(b"IBC_PROOF_V1:"));
        
        // Proof should contain chain ID and proof type
        let proof_str = String::from_utf8_lossy(&proof);
        assert!(proof_str.contains("test-chain"));
        assert!(proof_str.contains("packet_commitment"));
    }
}