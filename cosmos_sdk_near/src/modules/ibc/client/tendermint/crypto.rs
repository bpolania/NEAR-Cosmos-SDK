use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use near_sdk::{env, serde_json};
use chrono::{Utc, TimeZone, Datelike, Timelike};
use near_sdk::borsh::BorshDeserialize;

use super::types::{Commit, PublicKey, ValidatorSet, Validator};
use super::ics23::{CommitmentProof, get_iavl_spec};

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
pub fn create_canonical_sign_bytes(
    chain_id: &str,
    height: u64,
    round: i32,
    block_bytes: &[u8],
    timestamp: u64,
) -> Vec<u8> {
    // Implement Tendermint canonical JSON format for signing
    // This follows the exact format used by Tendermint for vote signing
    
    // Hash the block bytes to get block hash
    let mut hasher = Sha256::new();
    hasher.update(block_bytes);
    let block_hash = hasher.finalize();
    
    // Create canonical JSON format as per Tendermint specification
    // https://github.com/tendermint/tendermint/blob/main/types/canonical.go
    let canonical_vote = serde_json::json!({
        "@chain_id": chain_id,
        "@type": "/tendermint.types.CanonicalVote",
        "block_id": {
            "hash": hex::encode(block_hash).to_uppercase(),
            "parts": {
                "hash": "",
                "total": 0
            }
        },
        "height": height.to_string(),
        "round": round.to_string(), 
        "timestamp": format_canonical_time(timestamp),
        "type": 2 // PREVOTE_TYPE = 1, PRECOMMIT_TYPE = 2
    });
    
    let sign_data = canonical_vote.to_string();
    
    sign_data.into_bytes()
}

/// Format timestamp in Tendermint canonical time format
/// Tendermint uses RFC3339 format with nanosecond precision
fn format_canonical_time(timestamp: u64) -> String {
    // Convert Unix timestamp to RFC3339 format
    // Tendermint expects: "2006-01-02T15:04:05.000000000Z"
    let datetime = Utc.timestamp_opt(timestamp as i64, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap());
    
    // Format with nanosecond precision as required by Tendermint
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
        datetime.year(),
        datetime.month(),
        datetime.day(),
        datetime.hour(),
        datetime.minute(), 
        datetime.second(),
        datetime.nanosecond()
    )
}

/// Verify a Merkle proof against an IAVL tree root using ICS-23 format
/// 
/// This function verifies that a key-value pair exists (or doesn't exist)
/// in an IAVL tree with the given root hash using the ICS-23 proof specification.
/// 
/// # Arguments
/// * `root` - The IAVL tree root hash
/// * `key` - The key to verify
/// * `value` - The value to verify (None for non-membership proofs)
/// * `proof_bytes` - The ICS-23 formatted proof bytes
/// 
/// # Returns
/// * True if the proof is valid, false otherwise
pub fn verify_merkle_proof(
    root: &[u8],
    key: &[u8],
    value: Option<&[u8]>,
    proof_bytes: &[u8],
) -> bool {
    // Parse the ICS-23 proof from bytes
    let proof = match parse_ics23_proof(proof_bytes) {
        Ok(p) => p,
        Err(e) => {
            env::log_str(&format!("Failed to parse ICS-23 proof: {}", e));
            return false;
        }
    };

    // Get the appropriate proof specification for IAVL/Tendermint
    let spec = get_iavl_spec();
    
    match value {
        Some(val) => {
            // For membership proofs
            proof.verify_membership(&spec, root, key, val)
        }
        None => {
            // For non-membership proofs
            proof.verify_non_membership(&spec, root, key)
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Verify Ed25519 signature
/// 
/// # Arguments
/// * `pubkey_bytes` - The Ed25519 public key bytes (32 bytes)
/// * `message` - The message that was signed
/// * `signature` - The signature bytes (64 bytes)
/// 
/// # Returns
/// * `true` if signature is valid, `false` otherwise
pub fn verify_ed25519_signature(pubkey_bytes: &[u8], message: &[u8], signature: &[u8]) -> bool {
    // Verify input lengths
    if pubkey_bytes.len() != 32 || signature.len() != 64 {
        return false;
    }
    
    // Convert to fixed-size arrays
    let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(array) => array,
        Err(_) => return false,
    };
    
    let sig_array: [u8; 64] = match signature.try_into() {
        Ok(array) => array,
        Err(_) => return false,
    };
    
    // Convert to Ed25519 types
    let pubkey = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(key) => key,
        Err(_) => return false,
    };
    
    let sig = Signature::from_bytes(&sig_array);
    
    // Verify the signature
    pubkey.verify(message, &sig).is_ok()
}

/// Parse ICS-23 proof from bytes
/// 
/// This function deserializes the ICS-23 proof structure from bytes.
/// The proof can be in Borsh format for NEAR compatibility or JSON for
/// cross-chain compatibility.
/// 
/// # Arguments
/// * `proof_bytes` - The serialized proof bytes
/// 
/// # Returns
/// * The deserialized CommitmentProof or an error
pub fn parse_ics23_proof(proof_bytes: &[u8]) -> Result<CommitmentProof, String> {
    // First try Borsh deserialization (NEAR format)
    if let Ok(proof) = CommitmentProof::try_from_slice(proof_bytes) {
        return Ok(proof);
    }
    
    // Fall back to JSON deserialization (cross-chain format)
    match serde_json::from_slice::<CommitmentProof>(proof_bytes) {
        Ok(proof) => Ok(proof),
        Err(e) => Err(format!("Failed to parse proof as JSON: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::ibc::client::tendermint::ics23::*;
    use serde_json::json;
    
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
    
    #[test]
    fn test_parse_ics23_proof_json() {
        // Test JSON deserialization of ICS-23 proof
        let proof_json = json!({
            "proof": {
                "key": [1, 2, 3, 4],
                "value": [5, 6, 7, 8],
                "leaf": {
                    "hash": "Sha256",
                    "prehash_key": "NoHash",
                    "prehash_value": "Sha256", 
                    "length": "VarProto",
                    "prefix": [0]
                },
                "path": []
            },
            "non_exist": null,
            "batch": null,
            "compressed": null
        });
        
        let proof_bytes = serde_json::to_vec(&proof_json).unwrap();
        let parsed_proof = parse_ics23_proof(&proof_bytes).unwrap();
        
        assert!(parsed_proof.proof.is_some());
        assert!(parsed_proof.non_exist.is_none());
        
        let existence_proof = parsed_proof.proof.unwrap();
        assert_eq!(existence_proof.key, vec![1, 2, 3, 4]);
        assert_eq!(existence_proof.value, vec![5, 6, 7, 8]);
        assert_eq!(existence_proof.leaf.hash, HashOp::Sha256);
    }
    
    #[test]
    fn test_verify_merkle_proof_with_invalid_proof() {
        // Test that invalid proof bytes are handled gracefully
        let root = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let key = b"test_key";
        let value = b"test_value";
        let invalid_proof = b"invalid_proof_bytes";
        
        let result = verify_merkle_proof(&root, key, Some(value), invalid_proof);
        assert!(!result); // Should return false for invalid proof
    }
    
    #[test]
    fn test_iavl_spec_generation() {
        let spec = get_iavl_spec();
        
        // Verify IAVL-specific settings
        assert_eq!(spec.leaf_spec.hash, HashOp::Sha256);
        assert_eq!(spec.leaf_spec.prehash_value, HashOp::Sha256);
        assert_eq!(spec.leaf_spec.length, LengthOp::VarProto);
        assert_eq!(spec.leaf_spec.prefix, vec![0]); // IAVL leaf prefix
        
        assert_eq!(spec.inner_spec.child_order, vec![0, 1]); // Binary tree
        assert_eq!(spec.inner_spec.child_size, 32); // SHA-256 hash size
        assert_eq!(spec.inner_spec.hash, HashOp::Sha256);
    }
}