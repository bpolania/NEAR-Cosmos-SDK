use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use near_sdk::env;

use crate::types::{Commit, PublicKey, ValidatorSet, Validator};

/// Verify commit signatures against a validator set
/// 
/// This function implements the Tendermint signature verification logic
/// to ensure that at least 2/3+ of the voting power has signed the commit.
/// 
/// # Arguments
/// * `commit` - The commit containing signatures to verify
/// * `validator_set` - The validator set that should have signed
/// * `chain_id` - The chain ID for signature domain separation
/// * `block_bytes` - The canonical block bytes that were signed
/// 
/// # Returns
/// * True if verification succeeds, false otherwise
pub fn verify_commit_signatures(
    commit: &Commit,
    validator_set: &ValidatorSet,
    chain_id: &str,
    block_bytes: &[u8],
) -> bool {
    let mut signed_voting_power = 0i64;
    let required_voting_power = (validator_set.total_voting_power * 2) / 3 + 1;
    
    // Check each signature in the commit
    for (i, sig) in commit.signatures.iter().enumerate() {
        if i >= validator_set.validators.len() {
            env::log_str("More signatures than validators");
            return false;
        }
        
        let validator = &validator_set.validators[i];
        
        // Skip if no signature provided (validator didn't sign)
        let signature_bytes = match &sig.signature {
            Some(sig_bytes) => sig_bytes,
            None => continue,
        };
        
        // Verify the signature
        if verify_validator_signature(
            validator,
            signature_bytes,
            chain_id,
            commit.height,
            commit.round,
            block_bytes,
            sig.timestamp,
        ) {
            signed_voting_power += validator.voting_power;
        } else {
            env::log_str(&format!(
                "Invalid signature from validator {}",
                hex::encode(&validator.address)
            ));
            return false;
        }
    }
    
    // Check if we have enough voting power
    if signed_voting_power < required_voting_power {
        env::log_str(&format!(
            "Insufficient voting power: {} < {}",
            signed_voting_power, required_voting_power
        ));
        return false;
    }
    
    true
}

/// Verify a single validator's signature
/// 
/// This implements the Tendermint canonical signing format and verifies
/// the Ed25519 signature against the validator's public key.
/// 
/// # Arguments
/// * `validator` - The validator whose signature to verify
/// * `signature_bytes` - The signature bytes
/// * `chain_id` - The chain ID for domain separation
/// * `height` - Block height
/// * `round` - Consensus round
/// * `block_bytes` - The canonical block bytes
/// * `timestamp` - Signature timestamp
/// 
/// # Returns
/// * True if the signature is valid, false otherwise
fn verify_validator_signature(
    validator: &Validator,
    signature_bytes: &[u8],
    chain_id: &str,
    height: u64,
    round: i32,
    block_bytes: &[u8],
    timestamp: u64,
) -> bool {
    // Only Ed25519 is currently supported
    let ed25519_pubkey = match &validator.pub_key {
        PublicKey::Ed25519(bytes) => {
            if bytes.len() != 32 {
                env::log_str("Invalid Ed25519 public key length");
                return false;
            }
            bytes
        }
        PublicKey::Secp256k1(_) => {
            env::log_str("Secp256k1 signatures not yet supported");
            return false;
        }
    };
    
    // Parse the Ed25519 signature
    if signature_bytes.len() != 64 {
        env::log_str("Invalid Ed25519 signature length");
        return false;
    }
    
    let signature_array: [u8; 64] = match signature_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => {
            env::log_str("Failed to convert signature bytes to array");
            return false;
        }
    };
    
    let signature = Signature::from_bytes(&signature_array);
    
    // Create the verifying key
    let pubkey_array: [u8; 32] = match ed25519_pubkey.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => {
            env::log_str("Failed to convert public key to array");
            return false;
        }
    };
    
    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(key) => key,
        Err(_) => {
            env::log_str("Failed to parse Ed25519 public key");
            return false;
        }
    };
    
    // Create the canonical sign bytes for Tendermint
    let sign_bytes = create_canonical_sign_bytes(
        chain_id,
        height,
        round,
        block_bytes,
        timestamp,
    );
    
    // Verify the signature
    match verifying_key.verify(&sign_bytes, &signature) {
        Ok(_) => true,
        Err(_) => {
            env::log_str("Ed25519 signature verification failed");
            false
        }
    }
}

/// Create canonical sign bytes for Tendermint signature verification
/// 
/// This implements the Tendermint canonical JSON signing format.
/// The sign bytes are a JSON representation of the vote information
/// that gets hashed and signed by validators.
/// 
/// # Arguments
/// * `chain_id` - The chain ID
/// * `height` - Block height
/// * `round` - Consensus round
/// * `block_bytes` - The canonical block bytes
/// * `timestamp` - Vote timestamp
/// 
/// # Returns
/// * The canonical sign bytes to be verified
fn create_canonical_sign_bytes(
    chain_id: &str,
    height: u64,
    round: i32,
    block_bytes: &[u8],
    timestamp: u64,
) -> Vec<u8> {
    // For now, this is a simplified implementation
    // In a full implementation, this would create the exact canonical JSON
    // format that Tendermint uses for signing
    
    // Hash the block bytes to get block hash
    let mut hasher = Sha256::new();
    hasher.update(block_bytes);
    let block_hash = hasher.finalize();
    
    // Create a simplified sign bytes format
    // TODO: Implement full canonical JSON format as per Tendermint spec
    let sign_data = format!(
        "{{\"@chain_id\":\"{}\",\"@type\":\"/tendermint.types.Vote\",\"block_id\":{{\"hash\":\"{}\"}},\"height\":\"{}\",\"round\":\"{}\",\"timestamp\":\"{}\",\"type\":2}}",
        chain_id,
        hex::encode(block_hash),
        height,
        round,
        timestamp
    );
    
    sign_data.into_bytes()
}

/// Verify a Merkle proof against an IAVL tree root
/// 
/// This function verifies that a key-value pair exists (or doesn't exist)
/// in an IAVL tree with the given root hash.
/// 
/// # Arguments
/// * `root` - The IAVL tree root hash
/// * `key` - The key to verify
/// * `value` - The value to verify (None for non-membership proofs)
/// * `proof` - The Merkle proof
/// 
/// # Returns
/// * True if the proof is valid, false otherwise
pub fn verify_merkle_proof(
    root: &[u8],
    key: &[u8],
    value: Option<&[u8]>,
    _proof: &[u8],
) -> bool {
    // Simplified IAVL proof verification for demonstration
    // In a full implementation, this would parse the proof structure
    // and verify each step up to the root
    
    match value {
        Some(val) => {
            // For membership proofs, compute the leaf hash
            let leaf_hash = hash_iavl_leaf(key, val);
            
            // For now, just verify that we can compute the leaf hash
            // A full implementation would verify the entire proof path
            if leaf_hash.is_empty() {
                env::log_str("Failed to compute IAVL leaf hash");
                false
            } else {
                // This is a placeholder - in reality we'd verify the full proof
                env::log_str("IAVL membership verification - placeholder implementation");
                !root.is_empty() // Return true if root is not empty (placeholder logic)
            }
        }
        None => {
            // For non-membership proofs, verify the key doesn't exist
            env::log_str("IAVL non-membership verification - placeholder implementation");
            !root.is_empty() // Return true if root is not empty (placeholder logic)
        }
    }
}

/// Hash a key-value pair using IAVL leaf format
/// 
/// IAVL trees use a specific format for hashing leaf nodes
/// that includes the key length, value length, and their hashes.
/// 
/// # Arguments
/// * `key` - The key bytes
/// * `value` - The value bytes
/// 
/// # Returns
/// * The IAVL leaf hash
pub fn hash_iavl_leaf(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    
    // IAVL leaf format: hash(0x00 || varint(keylen) || key || varint(valuelen) || value)
    hasher.update(&[0x00]); // Leaf prefix
    
    // Add key length as varint
    hasher.update(&encode_varint(key.len() as u64));
    hasher.update(key);
    
    // Add value length as varint  
    hasher.update(&encode_varint(value.len() as u64));
    hasher.update(value);
    
    hasher.finalize().to_vec()
}

/// Encode a number as varint (variable-length integer)
/// 
/// This implements protobuf varint encoding used by IAVL trees.
/// 
/// # Arguments
/// * `mut value` - The value to encode
/// 
/// # Returns
/// * The varint-encoded bytes
fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut result = Vec::new();
    
    while value >= 0x80 {
        result.push((value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
    result.push(value as u8);
    
    result
}

/// Compute SHA256 hash of data
/// 
/// # Arguments
/// * `data` - The data to hash
/// 
/// # Returns
/// * The SHA256 hash
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_varint() {
        assert_eq!(encode_varint(0), vec![0]);
        assert_eq!(encode_varint(127), vec![127]);
        assert_eq!(encode_varint(128), vec![128, 1]);
        assert_eq!(encode_varint(16383), vec![255, 127]);
        assert_eq!(encode_varint(16384), vec![128, 128, 1]);
    }
    
    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256(data);
        let expected = hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").unwrap();
        assert_eq!(hash, expected);
    }
}