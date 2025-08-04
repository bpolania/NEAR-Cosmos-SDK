use crate::types::cosmos_tx::{CosmosTx, SignDoc, SignerInfo, SignMode};
use near_sdk::serde::{Deserialize, Serialize};

/// Signature verification errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SignatureError {
    /// Invalid signature format
    InvalidSignature(String),
    /// Invalid public key format
    InvalidPublicKey(String),
    /// Signature verification failed
    VerificationFailed(String),
    /// Unsupported signing mode
    UnsupportedSignMode(String),
    /// Invalid signature length
    InvalidSignatureLength { expected: usize, actual: usize },
    /// Invalid public key length
    InvalidPublicKeyLength { expected: usize, actual: usize },
    /// Serialization error
    SerializationError(String),
    /// Hash computation error
    HashError(String),
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureError::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            SignatureError::InvalidPublicKey(msg) => write!(f, "Invalid public key: {}", msg),
            SignatureError::VerificationFailed(msg) => write!(f, "Signature verification failed: {}", msg),
            SignatureError::UnsupportedSignMode(mode) => write!(f, "Unsupported signing mode: {}", mode),
            SignatureError::InvalidSignatureLength { expected, actual } => {
                write!(f, "Invalid signature length: expected {}, got {}", expected, actual)
            }
            SignatureError::InvalidPublicKeyLength { expected, actual } => {
                write!(f, "Invalid public key length: expected {}, got {}", expected, actual)
            }
            SignatureError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            SignatureError::HashError(msg) => write!(f, "Hash computation error: {}", msg),
        }
    }
}

impl std::error::Error for SignatureError {}

/// Cosmos SDK public key types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CosmosPublicKey {
    /// secp256k1 public key (33 bytes compressed or 65 bytes uncompressed)
    Secp256k1(Vec<u8>),
    /// ed25519 public key (32 bytes)
    Ed25519(Vec<u8>),
    /// Multi-signature public key
    MultiSig {
        threshold: u32,
        public_keys: Vec<CosmosPublicKey>,
    },
}

impl CosmosPublicKey {
    /// Create a secp256k1 public key
    pub fn secp256k1(bytes: Vec<u8>) -> Result<Self, SignatureError> {
        if bytes.len() != 33 && bytes.len() != 65 {
            return Err(SignatureError::InvalidPublicKeyLength {
                expected: 33,
                actual: bytes.len(),
            });
        }
        Ok(CosmosPublicKey::Secp256k1(bytes))
    }

    /// Create an ed25519 public key
    pub fn ed25519(bytes: Vec<u8>) -> Result<Self, SignatureError> {
        if bytes.len() != 32 {
            return Err(SignatureError::InvalidPublicKeyLength {
                expected: 32,
                actual: bytes.len(),
            });
        }
        Ok(CosmosPublicKey::Ed25519(bytes))
    }

    /// Get the raw bytes of the public key
    pub fn bytes(&self) -> &[u8] {
        match self {
            CosmosPublicKey::Secp256k1(bytes) => bytes,
            CosmosPublicKey::Ed25519(bytes) => bytes,
            CosmosPublicKey::MultiSig { .. } => &[], // Multi-sig doesn't have single key bytes
        }
    }

    /// Get the Cosmos address (bech32 encoded)
    pub fn to_cosmos_address(&self, prefix: &str) -> Result<String, SignatureError> {
        let hash = self.address_hash()?;
        
        // Convert bytes to u5 array for bech32 encoding
        let u5_data: Vec<bech32::u5> = bech32::convert_bits(&hash, 8, 5, true)
            .map_err(|e| SignatureError::SerializationError(format!("bech32 conversion error: {:?}", e)))?
            .into_iter()
            .map(|b| bech32::u5::try_from_u8(b).unwrap())
            .collect();
        
        bech32::encode(prefix, u5_data, bech32::Variant::Bech32)
            .map_err(|e| SignatureError::SerializationError(e.to_string()))
    }

    /// Compute the address hash (ripemd160(sha256(pubkey)))
    fn address_hash(&self) -> Result<[u8; 20], SignatureError> {
        use sha2::{Digest, Sha256};
        use ripemd::{Ripemd160};

        match self {
            CosmosPublicKey::Secp256k1(bytes) => {
                let sha256_hash = Sha256::digest(bytes);
                let ripemd_hash = Ripemd160::digest(sha256_hash);
                Ok(ripemd_hash.into())
            }
            CosmosPublicKey::Ed25519(bytes) => {
                let sha256_hash = Sha256::digest(bytes);
                let ripemd_hash = Ripemd160::digest(sha256_hash);
                Ok(ripemd_hash.into())
            }
            CosmosPublicKey::MultiSig { .. } => {
                Err(SignatureError::InvalidPublicKey("Cannot compute address for multi-sig key directly".to_string()))
            }
        }
    }
}

/// Cosmos SDK signature verifier
pub struct CosmosSignatureVerifier {
    /// Chain ID for signature verification
    pub chain_id: String,
}

impl CosmosSignatureVerifier {
    /// Create a new signature verifier
    pub fn new(chain_id: String) -> Self {
        Self { chain_id }
    }

    /// Verify all signatures in a transaction
    pub fn verify_signatures(&self, tx: &CosmosTx, account_numbers: &[u64]) -> Result<Vec<CosmosPublicKey>, SignatureError> {
        if tx.signatures.len() != tx.auth_info.signer_infos.len() {
            return Err(SignatureError::VerificationFailed(
                format!("Signature count mismatch: {} signatures for {} signers",
                    tx.signatures.len(), tx.auth_info.signer_infos.len())
            ));
        }

        if account_numbers.len() != tx.auth_info.signer_infos.len() {
            return Err(SignatureError::VerificationFailed(
                format!("Account number count mismatch: {} account numbers for {} signers",
                    account_numbers.len(), tx.auth_info.signer_infos.len())
            ));
        }

        let mut recovered_keys = Vec::new();

        for (i, (signature, signer_info)) in tx.signatures.iter().zip(tx.auth_info.signer_infos.iter()).enumerate() {
            let sign_doc = self.create_sign_doc(tx, account_numbers[i])?;
            let public_key = self.verify_single_signature(signature, &sign_doc, signer_info)?;
            recovered_keys.push(public_key);
        }

        Ok(recovered_keys)
    }

    /// Verify a single signature
    pub fn verify_single_signature(
        &self,
        signature: &[u8],
        sign_doc: &SignDoc,
        signer_info: &SignerInfo,
    ) -> Result<CosmosPublicKey, SignatureError> {
        match signer_info.mode_info.mode {
            SignMode::Direct => self.verify_direct_signature(signature, sign_doc, signer_info),
            SignMode::Textual => {
                Err(SignatureError::UnsupportedSignMode("Textual signing mode not yet supported".to_string()))
            }
            SignMode::LegacyAminoJson => {
                Err(SignatureError::UnsupportedSignMode("Legacy Amino JSON signing mode deprecated".to_string()))
            }
        }
    }

    /// Verify a direct mode signature
    fn verify_direct_signature(
        &self,
        signature: &[u8],
        sign_doc: &SignDoc,
        signer_info: &SignerInfo,
    ) -> Result<CosmosPublicKey, SignatureError> {
        // Get the signing bytes
        let signing_bytes = sign_doc.signing_bytes();

        // Hash the signing bytes
        let message_hash = self.hash_message(&signing_bytes)?;

        // If public key is provided, verify against it
        if let Some(pub_key_any) = &signer_info.public_key {
            self.verify_with_known_pubkey(signature, &message_hash, pub_key_any)
        } else {
            // Recover public key from signature (for secp256k1)
            self.recover_public_key(signature, &message_hash)
        }
    }

    /// Verify signature with a known public key
    fn verify_with_known_pubkey(
        &self,
        signature: &[u8],
        message_hash: &[u8],
        _pub_key_any: &crate::types::cosmos_tx::Any,
    ) -> Result<CosmosPublicKey, SignatureError> {
        // TODO: Decode the public key from Any type and verify
        // For now, we'll implement public key recovery approach
        self.recover_public_key(signature, message_hash)
    }

    /// Recover public key from signature (secp256k1 only)
    fn recover_public_key(&self, signature: &[u8], message_hash: &[u8]) -> Result<CosmosPublicKey, SignatureError> {
        use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

        // Cosmos signatures are 65 bytes: 64 bytes signature + 1 byte recovery ID
        if signature.len() != 65 {
            return Err(SignatureError::InvalidSignatureLength {
                expected: 65,
                actual: signature.len(),
            });
        }

        // Split signature and recovery ID
        let (sig_bytes, recovery_id_bytes) = signature.split_at(64);
        let recovery_id = recovery_id_bytes[0];

        // Create signature and recovery ID
        let signature = Signature::from_slice(sig_bytes)
            .map_err(|e| SignatureError::InvalidSignature(e.to_string()))?;

        let recovery_id = RecoveryId::try_from(recovery_id)
            .map_err(|e| SignatureError::InvalidSignature(e.to_string()))?;

        // Recover the public key
        let verifying_key = VerifyingKey::recover_from_msg(message_hash, &signature, recovery_id)
            .map_err(|e| SignatureError::VerificationFailed(e.to_string()))?;

        // Convert to compressed bytes
        let public_key_bytes = verifying_key.to_encoded_point(true).as_bytes().to_vec();

        CosmosPublicKey::secp256k1(public_key_bytes)
    }

    /// Create signing document for verification
    pub fn create_sign_doc(&self, tx: &CosmosTx, account_number: u64) -> Result<SignDoc, SignatureError> {
        tx.get_sign_doc(&self.chain_id, account_number)
            .map_err(|e| SignatureError::SerializationError(e.to_string()))
    }

    /// Hash the message for signing (SHA256)
    fn hash_message(&self, message: &[u8]) -> Result<Vec<u8>, SignatureError> {
        use sha2::{Digest, Sha256};
        Ok(Sha256::digest(message).to_vec())
    }

    /// Verify a multi-signature
    pub fn verify_multisig(
        &self,
        signatures: &[Vec<u8>],
        sign_doc: &SignDoc,
        multisig_info: &crate::types::cosmos_tx::MultiSignatureInfo,
    ) -> Result<(), SignatureError> {
        // Count valid signatures
        let mut valid_signatures = 0;

        for (i, mode_info) in multisig_info.mode_infos.iter().enumerate() {
            if i >= signatures.len() {
                break;
            }

            // Check if this signature slot is filled (using bitmap)
            if !self.is_signature_present(&multisig_info.bitarray, i) {
                continue;
            }

            // Create a temporary signer info for verification
            let temp_signer_info = SignerInfo {
                public_key: None, // Will recover from signature
                mode_info: mode_info.clone(),
                sequence: 0, // Not used in verification
            };

            if self.verify_single_signature(&signatures[i], sign_doc, &temp_signer_info).is_ok() {
                valid_signatures += 1;
            }
        }

        // Check if we have enough valid signatures
        if valid_signatures < multisig_info.bitarray.extra_bits_stored {
            return Err(SignatureError::VerificationFailed(
                format!("Insufficient valid signatures: got {}, need {}", 
                    valid_signatures, multisig_info.bitarray.extra_bits_stored)
            ));
        }

        Ok(())
    }

    /// Check if a signature is present in the bitmap
    fn is_signature_present(&self, bitarray: &crate::types::cosmos_tx::CompactBitArray, index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = index % 8;

        if byte_index >= bitarray.elems.len() {
            return false;
        }

        (bitarray.elems[byte_index] & (1 << bit_index)) != 0
    }
}

/// Signature builder for creating signatures
pub struct SignatureBuilder {
    verifier: CosmosSignatureVerifier,
}

impl SignatureBuilder {
    /// Create a new signature builder
    pub fn new(chain_id: String) -> Self {
        Self {
            verifier: CosmosSignatureVerifier::new(chain_id),
        }
    }

    /// Create a signing document for a transaction
    pub fn create_sign_doc(&self, tx: &CosmosTx, account_number: u64) -> Result<SignDoc, SignatureError> {
        self.verifier.create_sign_doc(tx, account_number)
    }

    /// Get the bytes that need to be signed
    pub fn get_signing_bytes(&self, tx: &CosmosTx, account_number: u64) -> Result<Vec<u8>, SignatureError> {
        let sign_doc = self.create_sign_doc(tx, account_number)?;
        Ok(sign_doc.signing_bytes())
    }

    /// Get the message hash for signing
    pub fn get_message_hash(&self, tx: &CosmosTx, account_number: u64) -> Result<Vec<u8>, SignatureError> {
        let signing_bytes = self.get_signing_bytes(tx, account_number)?;
        self.verifier.hash_message(&signing_bytes)
    }

    /// Verify a signature without recovering the public key
    pub fn verify_signature_only(
        &self,
        signature: &[u8],
        tx: &CosmosTx,
        account_number: u64,
        public_key: &CosmosPublicKey,
    ) -> Result<bool, SignatureError> {
        let message_hash = self.get_message_hash(tx, account_number)?;
        
        match public_key {
            CosmosPublicKey::Secp256k1(pub_key_bytes) => {
                self.verify_secp256k1_signature(signature, &message_hash, pub_key_bytes)
            }
            CosmosPublicKey::Ed25519(pub_key_bytes) => {
                self.verify_ed25519_signature(signature, &message_hash, pub_key_bytes)
            }
            CosmosPublicKey::MultiSig { .. } => {
                Err(SignatureError::UnsupportedSignMode("Direct multi-sig verification not supported".to_string()))
            }
        }
    }

    /// Verify secp256k1 signature with known public key
    fn verify_secp256k1_signature(&self, signature: &[u8], message_hash: &[u8], public_key: &[u8]) -> Result<bool, SignatureError> {
        use k256::ecdsa::{Signature, VerifyingKey};
        use k256::ecdsa::signature::Verifier;

        if signature.len() != 65 {
            return Err(SignatureError::InvalidSignatureLength {
                expected: 65,
                actual: signature.len(),
            });
        }

        // Extract the signature part (first 64 bytes)
        let sig_bytes = &signature[..64];
        
        let signature = Signature::from_slice(sig_bytes)
            .map_err(|e| SignatureError::InvalidSignature(e.to_string()))?;

        let verifying_key = VerifyingKey::from_sec1_bytes(public_key)
            .map_err(|e| SignatureError::InvalidPublicKey(e.to_string()))?;

        Ok(verifying_key.verify(message_hash, &signature).is_ok())
    }

    /// Verify ed25519 signature with known public key
    fn verify_ed25519_signature(&self, signature: &[u8], message_hash: &[u8], public_key: &[u8]) -> Result<bool, SignatureError> {
        use ed25519_dalek::{VerifyingKey, Signature, Verifier};

        if signature.len() != 64 {
            return Err(SignatureError::InvalidSignatureLength {
                expected: 64,
                actual: signature.len(),
            });
        }

        let signature = Signature::from_slice(signature)
            .map_err(|e| SignatureError::InvalidSignature(e.to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(public_key.try_into().map_err(|_| {
            SignatureError::InvalidPublicKey("Invalid ed25519 public key length".to_string())
        })?)
        .map_err(|e| SignatureError::InvalidPublicKey(e.to_string()))?;

        Ok(verifying_key.verify(message_hash, &signature).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::cosmos_tx::{TxBody, AuthInfo, Fee, Coin, SignerInfo, ModeInfo, Any};

    fn create_test_transaction() -> CosmosTx {
        let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let body = TxBody::new(vec![msg]);
        let fee = Fee::new(vec![Coin::new("uatom", "1000")], 200000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![0u8; 65]]; // Dummy signature

        CosmosTx::new(body, auth_info, signatures)
    }

    #[test]
    fn test_cosmos_public_key_creation() {
        // Test secp256k1 key
        let secp256k1_bytes = vec![0u8; 33];
        let secp256k1_key = CosmosPublicKey::secp256k1(secp256k1_bytes.clone()).unwrap();
        assert_eq!(secp256k1_key.bytes(), &secp256k1_bytes);

        // Test ed25519 key
        let ed25519_bytes = vec![0u8; 32];
        let ed25519_key = CosmosPublicKey::ed25519(ed25519_bytes.clone()).unwrap();
        assert_eq!(ed25519_key.bytes(), &ed25519_bytes);

        // Test invalid lengths
        assert!(CosmosPublicKey::secp256k1(vec![0u8; 32]).is_err());
        assert!(CosmosPublicKey::ed25519(vec![0u8; 31]).is_err());
    }

    #[test]
    fn test_signature_verifier_creation() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        assert_eq!(verifier.chain_id, "test-chain");
    }

    #[test]
    fn test_create_sign_doc() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let tx = create_test_transaction();
        
        let sign_doc = verifier.create_sign_doc(&tx, 42).unwrap();
        assert_eq!(sign_doc.chain_id, "test-chain");
        assert_eq!(sign_doc.account_number, 42);
        assert!(!sign_doc.body_bytes.is_empty());
        assert!(!sign_doc.auth_info_bytes.is_empty());
    }

    #[test]
    fn test_hash_message() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let message = b"test message";
        
        let hash = verifier.hash_message(message).unwrap();
        assert_eq!(hash.len(), 32); // SHA256 produces 32-byte hash
    }

    #[test]
    fn test_signature_builder() {
        let builder = SignatureBuilder::new("test-chain".to_string());
        let tx = create_test_transaction();
        
        let signing_bytes = builder.get_signing_bytes(&tx, 42).unwrap();
        assert!(!signing_bytes.is_empty());
        
        let message_hash = builder.get_message_hash(&tx, 42).unwrap();
        assert_eq!(message_hash.len(), 32);
    }

    #[test]
    fn test_invalid_signature_lengths() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let short_signature = vec![0u8; 32]; // Too short
        let message_hash = vec![0u8; 32];
        
        let result = verifier.recover_public_key(&short_signature, &message_hash);
        assert!(matches!(result, Err(SignatureError::InvalidSignatureLength { .. })));
    }

    #[test]
    fn test_signature_count_mismatch() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let mut tx = create_test_transaction();
        tx.signatures.clear(); // Remove signatures but keep signer info
        
        let result = verifier.verify_signatures(&tx, &[42]);
        assert!(matches!(result, Err(SignatureError::VerificationFailed(_))));
    }

    #[test]
    fn test_account_number_mismatch() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let tx = create_test_transaction();
        
        // Provide wrong number of account numbers
        let result = verifier.verify_signatures(&tx, &[]);
        assert!(matches!(result, Err(SignatureError::VerificationFailed(_))));
    }

    #[test]
    fn test_unsupported_sign_modes() {
        let verifier = CosmosSignatureVerifier::new("test-chain".to_string());
        let signature = vec![0u8; 65];
        let sign_doc = SignDoc {
            body_bytes: vec![],
            auth_info_bytes: vec![],
            chain_id: "test".to_string(),
            account_number: 0,
        };

        // Test textual mode
        let textual_signer = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Textual,
                multi: None,
            },
            sequence: 1,
        };
        let result = verifier.verify_single_signature(&signature, &sign_doc, &textual_signer);
        assert!(matches!(result, Err(SignatureError::UnsupportedSignMode(_))));

        // Test legacy amino mode
        let amino_signer = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::LegacyAminoJson,
                multi: None,
            },
            sequence: 1,
        };
        let result = verifier.verify_single_signature(&signature, &sign_doc, &amino_signer);
        assert!(matches!(result, Err(SignatureError::UnsupportedSignMode(_))));
    }

    #[test]
    fn test_cosmos_address_generation() {
        // Test with known secp256k1 key (this would normally be from a real key)
        let pub_key_bytes = vec![
            0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
        ];
        let public_key = CosmosPublicKey::secp256k1(pub_key_bytes).unwrap();
        
        let address = public_key.to_cosmos_address("cosmos").unwrap();
        assert!(address.starts_with("cosmos"));
        assert!(address.len() > 10); // Reasonable address length
    }
}