use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use sha2::{Digest, Sha256};
use near_sdk::env;

/// ICS-23 Merkle proof format implementation
/// 
/// This module implements the ICS-23 specification for generic Merkle proofs
/// used in IBC communication, with specific support for IAVL trees.

/// CommitmentProof is the generic proof format for ICS-23
/// This can contain either existence or non-existence proofs
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommitmentProof {
    pub proof: Option<ExistenceProof>,
    pub non_exist: Option<NonExistenceProof>,
    pub batch: Option<BatchProof>,
    pub compressed: Option<CompressedBatchProof>,
}

/// ExistenceProof proves that a key-value pair exists in the tree
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExistenceProof {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub leaf: LeafOp,
    pub path: Vec<InnerOp>,
}

/// NonExistenceProof proves that a key does not exist in the tree
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NonExistenceProof {
    pub key: Vec<u8>,
    pub left: Option<ExistenceProof>,  // Proof of key just left of `key`
    pub right: Option<ExistenceProof>, // Proof of key just right of `key`
}

/// BatchProof allows proving multiple keys at once
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BatchProof {
    pub entries: Vec<BatchEntry>,
}

/// CompressedBatchProof is a more efficient batch proof format
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompressedBatchProof {
    pub entries: Vec<CompressedBatchEntry>,
    pub lookup_inners: Vec<InnerOp>,
}

/// BatchEntry represents one entry in a batch proof
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BatchEntry {
    pub exist: Option<ExistenceProof>,
    pub non_exist: Option<NonExistenceProof>,
}

/// CompressedBatchEntry is a more efficient batch entry
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompressedBatchEntry {
    pub exist: Option<CompressedExistenceProof>,
    pub non_exist: Option<CompressedNonExistenceProof>,
}

/// CompressedExistenceProof uses indices into lookup table
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompressedExistenceProof {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub leaf: LeafOp,
    pub path: Vec<i32>, // Indices into lookup_inners
}

/// CompressedNonExistenceProof uses indices into lookup table
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CompressedNonExistenceProof {
    pub key: Vec<u8>,
    pub left: Option<CompressedExistenceProof>,
    pub right: Option<CompressedExistenceProof>,
}

/// LeafOp defines how to hash a leaf node
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LeafOp {
    pub hash: HashOp,
    pub prehash_key: HashOp,
    pub prehash_value: HashOp,
    pub length: LengthOp,
    pub prefix: Vec<u8>,
}

/// InnerOp defines how to hash an inner node
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InnerOp {
    pub hash: HashOp,
    pub prefix: Vec<u8>,
    pub suffix: Vec<u8>,
}

/// HashOp defines the hashing algorithm to use
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HashOp {
    NoHash,
    Sha256,
    Sha512,
    Keccak256,
    Ripemd160,
    Bitcoin,
}

/// LengthOp defines how to encode the length of data
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LengthOp {
    NoPrefix,
    VarProto,
    VarRlp,
    Fixed32Big,
    Fixed32Little,
    Fixed64Big,
    Fixed64Little,
    Require32Bytes,
    Require64Bytes,
}

/// ProofSpec defines how the proofs are constructed for a given tree
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProofSpec {
    pub leaf_spec: LeafOp,
    pub inner_spec: InnerSpec,
    pub max_depth: i32,
    pub min_depth: i32,
}

/// InnerSpec defines how inner nodes are structured
#[derive(BorshDeserialize, BorshSerialize, JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InnerSpec {
    pub child_order: Vec<i32>,
    pub child_size: i32,
    pub min_prefix_length: i32,
    pub max_prefix_length: i32,
    pub empty_child: Vec<u8>,
    pub hash: HashOp,
}

impl CommitmentProof {
    /// Verify a membership proof using ICS-23 format
    pub fn verify_membership(
        &self,
        spec: &ProofSpec,
        root: &[u8],
        key: &[u8],
        value: &[u8],
    ) -> bool {
        match &self.proof {
            Some(existence_proof) => {
                existence_proof.verify(spec, root, key, value)
            }
            None => {
                env::log_str("No existence proof provided for membership verification");
                false
            }
        }
    }

    /// Verify a non-membership proof using ICS-23 format
    pub fn verify_non_membership(
        &self,
        spec: &ProofSpec,
        root: &[u8],
        key: &[u8],
    ) -> bool {
        match &self.non_exist {
            Some(non_existence_proof) => {
                non_existence_proof.verify(spec, root, key)
            }
            None => {
                env::log_str("No non-existence proof provided for non-membership verification");
                false
            }
        }
    }
}

impl ExistenceProof {
    /// Verify an existence proof
    pub fn verify(&self, spec: &ProofSpec, root: &[u8], key: &[u8], value: &[u8]) -> bool {
        // Check that the key and value match
        if self.key != key || self.value != value {
            env::log_str("Key or value mismatch in existence proof");
            return false;
        }

        // Calculate the leaf hash
        let leaf_hash = match self.calculate_leaf_hash(&spec.leaf_spec) {
            Ok(hash) => hash,
            Err(e) => {
                env::log_str(&format!("Failed to calculate leaf hash: {}", e));
                return false;
            }
        };

        // Traverse the path from leaf to root
        let calculated_root = match self.calculate_root(&spec.inner_spec, &leaf_hash) {
            Ok(root_hash) => root_hash,
            Err(e) => {
                env::log_str(&format!("Failed to calculate root: {}", e));
                return false;
            }
        };

        // Compare with expected root
        calculated_root == root
    }

    /// Calculate the leaf hash according to the leaf specification
    fn calculate_leaf_hash(&self, leaf_spec: &LeafOp) -> Result<Vec<u8>, String> {
        let mut data = leaf_spec.prefix.clone();
        
        // Apply length prefix if specified
        match leaf_spec.length {
            LengthOp::VarProto => {
                data.extend_from_slice(&encode_varint(self.key.len()));
                data.extend_from_slice(&self.key);
                data.extend_from_slice(&encode_varint(self.value.len()));
                data.extend_from_slice(&self.value);
            }
            LengthOp::NoPrefix => {
                // Hash key if required
                let key_data = match leaf_spec.prehash_key {
                    HashOp::Sha256 => hash_sha256(&self.key),
                    HashOp::NoHash => self.key.clone(),
                    _ => return Err("Unsupported prehash_key operation".to_string()),
                };

                // Hash value if required
                let value_data = match leaf_spec.prehash_value {
                    HashOp::Sha256 => hash_sha256(&self.value),
                    HashOp::NoHash => self.value.clone(),
                    _ => return Err("Unsupported prehash_value operation".to_string()),
                };

                data.extend_from_slice(&key_data);
                data.extend_from_slice(&value_data);
            }
            _ => return Err("Unsupported length operation".to_string()),
        }

        // Apply final hash
        match leaf_spec.hash {
            HashOp::Sha256 => Ok(hash_sha256(&data)),
            HashOp::NoHash => Ok(data),
            _ => Err("Unsupported hash operation".to_string()),
        }
    }

    /// Calculate the root hash by traversing the inner path
    fn calculate_root(&self, _inner_spec: &InnerSpec, leaf_hash: &[u8]) -> Result<Vec<u8>, String> {
        let mut current_hash = leaf_hash.to_vec();

        for inner_op in &self.path {
            current_hash = self.apply_inner_op(inner_op, &current_hash)?;
        }

        Ok(current_hash)
    }

    /// Apply an inner operation to combine hashes
    fn apply_inner_op(&self, inner_op: &InnerOp, child_hash: &[u8]) -> Result<Vec<u8>, String> {
        let mut data = inner_op.prefix.clone();
        data.extend_from_slice(child_hash);
        data.extend_from_slice(&inner_op.suffix);

        match inner_op.hash {
            HashOp::Sha256 => Ok(hash_sha256(&data)),
            HashOp::NoHash => Ok(data),
            _ => Err("Unsupported inner hash operation".to_string()),
        }
    }
}

impl NonExistenceProof {
    /// Verify a non-existence proof
    pub fn verify(&self, spec: &ProofSpec, root: &[u8], key: &[u8]) -> bool {
        // Check that the key matches
        if self.key != key {
            env::log_str("Key mismatch in non-existence proof");
            return false;
        }

        // Verify left neighbor if present
        let left_valid = match &self.left {
            Some(left_proof) => {
                if left_proof.key.as_slice() >= key {
                    env::log_str("Left neighbor key is not less than target key");
                    return false;
                }
                left_proof.verify(spec, root, &left_proof.key, &left_proof.value)
            }
            None => true,
        };

        // Verify right neighbor if present
        let right_valid = match &self.right {
            Some(right_proof) => {
                if right_proof.key.as_slice() <= key {
                    env::log_str("Right neighbor key is not greater than target key");
                    return false;
                }
                right_proof.verify(spec, root, &right_proof.key, &right_proof.value)
            }
            None => true,
        };

        left_valid && right_valid
    }
}

/// IAVL-specific proof specification
pub fn get_iavl_spec() -> ProofSpec {
    ProofSpec {
        leaf_spec: LeafOp {
            hash: HashOp::Sha256,
            prehash_key: HashOp::NoHash,
            prehash_value: HashOp::Sha256,
            length: LengthOp::VarProto,
            prefix: vec![0], // IAVL leaf prefix
        },
        inner_spec: InnerSpec {
            child_order: vec![0, 1], // Binary tree
            child_size: 32,          // SHA-256 hash size
            min_prefix_length: 4,    // IAVL inner node prefix
            max_prefix_length: 12,   // IAVL inner node prefix + height + size
            empty_child: vec![],
            hash: HashOp::Sha256,
        },
        max_depth: 256,
        min_depth: 0,
    }
}

/// Tendermint-specific proof specification
#[allow(dead_code)]
pub fn get_tendermint_spec() -> ProofSpec {
    ProofSpec {
        leaf_spec: LeafOp {
            hash: HashOp::Sha256,
            prehash_key: HashOp::NoHash,
            prehash_value: HashOp::Sha256,
            length: LengthOp::VarProto,
            prefix: vec![],
        },
        inner_spec: InnerSpec {
            child_order: vec![0, 1],
            child_size: 32,
            min_prefix_length: 1,
            max_prefix_length: 1,
            empty_child: vec![],
            hash: HashOp::Sha256,
        },
        max_depth: 256,
        min_depth: 0,
    }
}

// Helper functions

/// Hash data using SHA-256
fn hash_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Encode an integer using varint encoding (protobuf style)
fn encode_varint(value: usize) -> Vec<u8> {
    let mut result = Vec::new();
    let mut val = value;
    
    while val >= 0x80 {
        result.push((val & 0x7F) as u8 | 0x80);
        val >>= 7;
    }
    result.push(val as u8);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_varint() {
        assert_eq!(encode_varint(0), vec![0]);
        assert_eq!(encode_varint(127), vec![127]);
        assert_eq!(encode_varint(128), vec![128, 1]);
        assert_eq!(encode_varint(300), vec![172, 2]);
    }

    #[test]
    fn test_hash_sha256() {
        let data = b"hello world";
        let hash = hash_sha256(data);
        assert_eq!(hash.len(), 32);
        
        // Known SHA-256 of "hello world"
        let expected = hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").unwrap();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_iavl_spec_creation() {
        let spec = get_iavl_spec();
        assert_eq!(spec.leaf_spec.hash, HashOp::Sha256);
        assert_eq!(spec.leaf_spec.prefix, vec![0]);
        assert_eq!(spec.inner_spec.child_order, vec![0, 1]);
    }
}