/// Host Functions for CosmWasm Compatibility
/// 
/// This module provides the host functions that CosmWasm contracts expect
/// when executing in the VM. These functions bridge CosmWasm's expectations
/// with NEAR's runtime environment.

use near_sdk::env;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use bech32::{self, ToBase32, Variant};

/// The standard CosmWasm bech32 prefix (we use "proxima" for this chain)
pub const BECH32_PREFIX: &str = "proxima";

/// Result type for host functions
pub type HostResult<T> = Result<T, HostError>;

/// Errors that can occur in host functions
#[derive(Debug)]
pub enum HostError {
    InvalidAddress(String),
    StorageError(String),
    SerializationError(String),
    CryptoError(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            HostError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            HostError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            HostError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
        }
    }
}

/// Storage operations - These map directly to NEAR's storage
pub mod storage {
    use super::*;
    
    /// Read a value from storage
    pub fn db_read(key: &[u8]) -> Option<Vec<u8>> {
        env::storage_read(key)
    }
    
    /// Write a value to storage
    pub fn db_write(key: &[u8], value: &[u8]) {
        env::storage_write(key, value);
    }
    
    /// Remove a key from storage
    pub fn db_remove(key: &[u8]) {
        env::storage_remove(key);
    }
    
    /// Scan storage keys with a prefix
    pub fn db_scan(prefix: &[u8], start: Option<&[u8]>, limit: u32) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        let mut count = 0;
        
        // In NEAR, we'd need to iterate through storage with the prefix
        // This is a simplified implementation
        for i in 0..limit {
            let mut key = prefix.to_vec();
            if let Some(start_key) = start {
                key.extend_from_slice(start_key);
            }
            key.push(i as u8);
            
            if let Some(value) = env::storage_read(&key) {
                results.push((key, value));
                count += 1;
                if count >= limit {
                    break;
                }
            }
        }
        
        results
    }
    
    /// Get the next key in lexicographic order
    pub fn db_next(key: &[u8]) -> Option<Vec<u8>> {
        // This would require iteration in NEAR
        // Simplified: just append a byte
        let mut next_key = key.to_vec();
        next_key.push(0);
        if env::storage_has_key(&next_key) {
            Some(next_key)
        } else {
            None
        }
    }
}

/// Address operations - Convert between NEAR and Cosmos addresses
pub mod address {
    use super::*;
    
    /// Canonicalize a human-readable address to its canonical form
    /// Accepts both NEAR accounts and Cosmos bech32 addresses
    pub fn canonicalize_address(human: &str) -> HostResult<Vec<u8>> {
        if human.starts_with(BECH32_PREFIX) {
            // It's a Cosmos address, decode it
            let (hrp, data, _variant) = bech32::decode(human)
                .map_err(|e| HostError::InvalidAddress(format!("Invalid bech32: {}", e)))?;
            
            if hrp != BECH32_PREFIX {
                return Err(HostError::InvalidAddress(format!("Wrong prefix: {}", hrp)));
            }
            
            let canonical = bech32::convert_bits(&data, 5, 8, false)
                .map_err(|e| HostError::InvalidAddress(format!("Bit conversion failed: {}", e)))?;
            
            Ok(canonical)
        } else {
            // It's a NEAR account, convert to canonical form
            // Use SHA256 hash of the account ID, take first 20 bytes
            let mut hasher = Sha256::new();
            hasher.update(human.as_bytes());
            let hash = hasher.finalize();
            Ok(hash[..20].to_vec())
        }
    }
    
    /// Humanize a canonical address to human-readable form
    pub fn humanize_address(canonical: &[u8]) -> HostResult<String> {
        if canonical.len() != 20 {
            return Err(HostError::InvalidAddress(
                format!("Invalid canonical address length: {}", canonical.len())
            ));
        }
        
        let encoded = bech32::encode(BECH32_PREFIX, canonical.to_base32(), Variant::Bech32)
            .map_err(|e| HostError::InvalidAddress(format!("Bech32 encoding failed: {}", e)))?;
        
        Ok(encoded)
    }
    
    /// Validate an address format
    pub fn addr_validate(human: &str) -> HostResult<String> {
        // Validate and normalize the address
        let canonical = canonicalize_address(human)?;
        humanize_address(&canonical)
    }
}

/// Cryptographic operations
pub mod crypto {
    use super::*;
    use near_sdk::env;
    
    /// Verify a secp256k1 signature
    pub fn secp256k1_verify(
        _message_hash: &[u8; 32],
        signature: &[u8],
        public_key: &[u8],
    ) -> HostResult<bool> {
        // NEAR doesn't have built-in secp256k1 verification
        // We'd need to implement this or use a library
        // For now, return a placeholder
        
        // In production, use a library like:
        // use secp256k1::{Secp256k1, Message, Signature, PublicKey};
        
        if signature.len() != 64 || public_key.len() != 33 {
            return Ok(false);
        }
        
        // Placeholder - in production, implement actual verification
        env::log_str("Warning: secp256k1_verify not fully implemented");
        Ok(true)
    }
    
    /// Verify a secp256k1 signature with recovery
    pub fn secp256k1_recover_pubkey(
        _message_hash: &[u8; 32],
        signature: &[u8],
        _recovery_id: u8,
    ) -> HostResult<Vec<u8>> {
        // Recover public key from signature
        // This requires secp256k1 library
        
        if signature.len() != 64 {
            return Err(HostError::CryptoError("Invalid signature length".to_string()));
        }
        
        // Placeholder - return a dummy public key
        env::log_str("Warning: secp256k1_recover_pubkey not fully implemented");
        Ok(vec![0u8; 33])
    }
    
    /// Verify an ed25519 signature (NEAR has this built-in!)
    pub fn ed25519_verify(
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> HostResult<bool> {
        if signature.len() != 64 || public_key.len() != 32 {
            return Ok(false);
        }
        
        // Convert slices to fixed-size arrays for NEAR's ed25519_verify
        let mut sig_array = [0u8; 64];
        let mut pk_array = [0u8; 32];
        sig_array.copy_from_slice(signature);
        pk_array.copy_from_slice(public_key);
        
        // NEAR provides this natively
        Ok(env::ed25519_verify(&sig_array, message, &pk_array))
    }
    
    /// Batch verify ed25519 signatures
    pub fn ed25519_batch_verify(
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> HostResult<bool> {
        if messages.len() != signatures.len() || messages.len() != public_keys.len() {
            return Ok(false);
        }
        
        for i in 0..messages.len() {
            if !ed25519_verify(messages[i], signatures[i], public_keys[i])? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

/// Query operations for cross-contract calls
pub mod query {
    use super::*;
    // use near_sdk::Promise; // For future cross-contract calls
    
    /// Query raw storage from another contract
    pub fn query_raw(contract_addr: &str, key: &[u8]) -> HostResult<Vec<u8>> {
        // In NEAR, we'd need to make a cross-contract call
        // This is async in NEAR, but CosmWasm expects sync
        // We'll need to handle this with promises
        
        env::log_str(&format!("query_raw: {} with key {:?}", contract_addr, key));
        
        // Placeholder - in production, use Promise for cross-contract call
        Ok(vec![])
    }
    
    /// Query smart contract with a message
    pub fn query_smart(contract_addr: &str, msg: &[u8]) -> HostResult<Vec<u8>> {
        env::log_str(&format!("query_smart: {} with msg {:?}", contract_addr, msg));
        
        // Placeholder - in production, use Promise for cross-contract call
        Ok(vec![])
    }
}

/// Abort function for contract panics
pub fn abort(msg: &str) -> ! {
    env::panic_str(msg)
}

/// Gas metering (NEAR handles this automatically)
pub mod gas {
    use super::*;
    
    /// Get remaining gas
    pub fn gas_remaining() -> u64 {
        // NEAR SDK 5.x uses Gas type, convert to u64
        let prepaid = env::prepaid_gas().as_gas();
        let used = env::used_gas().as_gas();
        prepaid.saturating_sub(used)
    }
    
    /// Check if we have enough gas
    pub fn check_gas(required: u64) -> bool {
        gas_remaining() >= required
    }
}

/// CosmWasm environment info
pub mod env_info {
    use super::*;
    
    #[derive(Serialize, Deserialize)]
    pub struct Env {
        pub block: BlockInfo,
        pub transaction: Option<TransactionInfo>,
        pub contract: ContractInfo,
    }
    
    #[derive(Serialize, Deserialize)]
    pub struct BlockInfo {
        pub height: u64,
        pub time: u64,
        pub chain_id: String,
    }
    
    #[derive(Serialize, Deserialize)]
    pub struct TransactionInfo {
        pub index: u32,
    }
    
    #[derive(Serialize, Deserialize)]
    pub struct ContractInfo {
        pub address: String,
    }
    
    /// Get the current environment
    pub fn get_env(contract_addr: &str) -> Env {
        Env {
            block: BlockInfo {
                height: env::block_height(),
                time: env::block_timestamp(),
                chain_id: "near-testnet".to_string(),
            },
            transaction: None,
            contract: ContractInfo {
                address: contract_addr.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_conversion() {
        // Test NEAR to canonical
        let near_account = "alice.near";
        let canonical = address::canonicalize_address(near_account).unwrap();
        assert_eq!(canonical.len(), 20);
        
        // Test canonical to human
        let human = address::humanize_address(&canonical).unwrap();
        assert!(human.starts_with(BECH32_PREFIX));
    }
    
    #[test]
    fn test_bech32_roundtrip() {
        let canonical = vec![1u8; 20];
        let human = address::humanize_address(&canonical).unwrap();
        let canonical2 = address::canonicalize_address(&human).unwrap();
        assert_eq!(canonical, canonical2);
    }
}