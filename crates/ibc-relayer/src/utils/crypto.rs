// Cryptographic utilities

use anyhow::Result;
use sha2::{Sha256, Digest};

/// Calculate SHA256 hash
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Verify Ed25519 signature
pub fn verify_ed25519_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<bool> {
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};
    
    let public_key = VerifyingKey::from_bytes(public_key)?;
    let signature = Signature::from_bytes(signature);
    
    Ok(public_key.verify(message, &signature).is_ok())
}

/// Generate Merkle root from leaves
pub fn calculate_merkle_root(leaves: &[Vec<u8>]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0; 32];
    }
    
    if leaves.len() == 1 {
        return sha256(&leaves[0]);
    }
    
    // Simple binary Merkle tree implementation
    let mut level = leaves.iter().map(|leaf| sha256(leaf)).collect::<Vec<_>>();
    
    while level.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in level.chunks(2) {
            let hash = if chunk.len() == 2 {
                let mut combined = Vec::new();
                combined.extend_from_slice(&chunk[0]);
                combined.extend_from_slice(&chunk[1]);
                sha256(&combined)
            } else {
                chunk[0]
            };
            next_level.push(hash);
        }
        
        level = next_level;
    }
    
    level[0]
}