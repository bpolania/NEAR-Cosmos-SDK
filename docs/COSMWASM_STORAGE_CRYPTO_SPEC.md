# CosmWasm Storage and Cryptographic Compatibility Specification

## Overview

This document provides detailed technical specifications for implementing CosmWasm-compatible storage patterns and cryptographic functions within the NEAR runtime environment, addressing the fundamental differences between the two ecosystems.

## Storage Compatibility Layer

### Core Storage Interface Implementation

```rust
use near_sdk::collections::{UnorderedMap, LookupMap, Vector};
use cosmwasm_std::{Storage, Order, Record, StdResult, StdError};
use std::collections::BTreeMap;

pub struct CosmWasmStorage {
    // Primary storage using NEAR's UnorderedMap for full iteration support
    data: UnorderedMap<Vec<u8>, Vec<u8>>,
    
    // Fast lookup cache for frequently accessed keys
    cache: LookupMap<Vec<u8>, Vec<u8>>,
    
    // Index support for secondary lookups
    indexes: UnorderedMap<String, UnorderedMap<Vec<u8>, Vec<u8>>>,
    
    // Range query optimization
    sorted_keys: Option<Vector<Vec<u8>>>,
    keys_dirty: bool,
}

impl Storage for CosmWasmStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // First check cache
        if let Some(value) = self.cache.get(&key.to_vec()) {
            return Some(value);
        }
        
        // Then check main storage
        let value = self.data.get(&key.to_vec());
        
        // Cache frequently accessed keys
        if let Some(ref v) = value {
            if self.should_cache(key) {
                // Note: In real implementation, need mutable reference
                // This would be handled in a separate caching method
            }
        }
        
        value
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        let key_vec = key.to_vec();
        let value_vec = value.to_vec();
        
        // Update main storage
        self.data.insert(&key_vec, &value_vec);
        
        // Update cache if key is cached
        if self.cache.contains_key(&key_vec) {
            self.cache.insert(&key_vec, &value_vec);
        }
        
        // Mark keys as dirty for range queries
        self.keys_dirty = true;
        
        // Update indexes if applicable
        self.update_indexes(&key_vec, &value_vec);
    }

    fn remove(&mut self, key: &[u8]) {
        let key_vec = key.to_vec();
        
        // Remove from main storage
        self.data.remove(&key_vec);
        
        // Remove from cache
        self.cache.remove(&key_vec);
        
        // Mark keys as dirty
        self.keys_dirty = true;
        
        // Remove from indexes
        self.remove_from_indexes(&key_vec);
    }
}
```

### Range Query Implementation

```rust
impl CosmWasmStorage {
    pub fn range<'a>(
        &'a mut self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record>> + 'a> {
        // Ensure sorted keys are up to date
        self.ensure_sorted_keys();
        
        match order {
            Order::Ascending => self.range_ascending(start, end),
            Order::Descending => self.range_descending(start, end),
        }
    }
    
    fn ensure_sorted_keys(&mut self) {
        if self.keys_dirty || self.sorted_keys.is_none() {
            self.rebuild_sorted_keys();
            self.keys_dirty = false;
        }
    }
    
    fn rebuild_sorted_keys(&mut self) {
        let mut keys: Vec<Vec<u8>> = self.data.keys_as_vector().to_vec();
        keys.sort();
        
        if let Some(ref mut sorted_keys) = self.sorted_keys {
            sorted_keys.clear();
            for key in keys {
                sorted_keys.push(&key);
            }
        } else {
            let mut new_sorted_keys = Vector::new(b"sorted_keys");
            for key in keys {
                new_sorted_keys.push(&key);
            }
            self.sorted_keys = Some(new_sorted_keys);
        }
    }
    
    fn range_ascending<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
    ) -> Box<dyn Iterator<Item = StdResult<Record>> + 'a> {
        let sorted_keys = self.sorted_keys.as_ref().unwrap();
        
        let start_idx = match start {
            Some(key) => self.find_key_index(key, true),
            None => 0,
        };
        
        let end_idx = match end {
            Some(key) => self.find_key_index(key, false),
            None => sorted_keys.len(),
        };
        
        Box::new(RangeIterator::new(
            &self.data,
            sorted_keys,
            start_idx,
            end_idx,
            false, // ascending
        ))
    }
    
    fn range_descending<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
    ) -> Box<dyn Iterator<Item = StdResult<Record>> + 'a> {
        // Similar to ascending but reversed bounds and iterator direction
        // Implementation details...
        todo!("Implement descending range iterator")
    }
    
    fn find_key_index(&self, target: &[u8], inclusive: bool) -> usize {
        let sorted_keys = self.sorted_keys.as_ref().unwrap();
        
        // Binary search for the key position
        let mut left = 0;
        let mut right = sorted_keys.len();
        
        while left < right {
            let mid = left + (right - left) / 2;
            let key = sorted_keys.get(mid).unwrap();
            
            match key.cmp(&target.to_vec()) {
                std::cmp::Ordering::Less => left = mid + 1,
                std::cmp::Ordering::Greater => right = mid,
                std::cmp::Ordering::Equal => {
                    return if inclusive { mid } else { mid + 1 };
                }
            }
        }
        
        left
    }
}

// Range iterator implementation
pub struct RangeIterator<'a> {
    storage: &'a UnorderedMap<Vec<u8>, Vec<u8>>,
    sorted_keys: &'a Vector<Vec<u8>>,
    current: usize,
    end: usize,
    reverse: bool,
}

impl<'a> RangeIterator<'a> {
    fn new(
        storage: &'a UnorderedMap<Vec<u8>, Vec<u8>>,
        sorted_keys: &'a Vector<Vec<u8>>,
        start: usize,
        end: usize,
        reverse: bool,
    ) -> Self {
        Self {
            storage,
            sorted_keys,
            current: if reverse { end.saturating_sub(1) } else { start },
            end,
            reverse,
        }
    }
}

impl<'a> Iterator for RangeIterator<'a> {
    type Item = StdResult<Record>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.reverse {
            if self.current == 0 || self.current < self.end {
                return None;
            }
        } else {
            if self.current >= self.end {
                return None;
            }
        }
        
        let key = match self.sorted_keys.get(self.current) {
            Some(k) => k,
            None => return Some(Err(StdError::generic_err("Invalid key index"))),
        };
        
        let value = match self.storage.get(&key) {
            Some(v) => v,
            None => return Some(Err(StdError::generic_err("Value not found for key"))),
        };
        
        // Advance iterator
        if self.reverse {
            self.current = self.current.saturating_sub(1);
        } else {
            self.current += 1;
        }
        
        Some(Ok((key, value)))
    }
}
```

### Advanced Storage Patterns

#### cw-storage-plus Compatibility

```rust
use cosmwasm_std::{StdResult, StdError};
use serde::{Serialize, Deserialize};

// Item compatibility
pub struct Item<T> {
    key: &'static [u8],
    phantom: std::marker::PhantomData<T>,
}

impl<T> Item<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub const fn new(key: &'static str) -> Self {
        Self {
            key: key.as_bytes(),
            phantom: std::marker::PhantomData,
        }
    }
    
    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        let bytes = serde_json::to_vec(data)
            .map_err(|e| StdError::generic_err(format!("Serialization error: {}", e)))?;
        storage.set(self.key, &bytes);
        Ok(())
    }
    
    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        let bytes = storage.get(self.key)
            .ok_or_else(|| StdError::not_found("Item not found"))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| StdError::generic_err(format!("Deserialization error: {}", e)))
    }
    
    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        match storage.get(self.key) {
            Some(bytes) => {
                let data = serde_json::from_slice(&bytes)
                    .map_err(|e| StdError::generic_err(format!("Deserialization error: {}", e)))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }
}

// Map compatibility
pub struct Map<'a, K, T> {
    namespace: &'static [u8],
    phantom_k: std::marker::PhantomData<&'a K>,
    phantom_t: std::marker::PhantomData<T>,
}

impl<'a, K, T> Map<'a, K, T>
where
    K: ?Sized,
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub const fn new(namespace: &'static str) -> Self {
        Self {
            namespace: namespace.as_bytes(),
            phantom_k: std::marker::PhantomData,
            phantom_t: std::marker::PhantomData,
        }
    }
    
    fn key(&self, k: &K) -> Vec<u8>
    where
        K: ?Sized + Serialize,
    {
        let mut key = self.namespace.to_vec();
        key.extend_from_slice(b"::");
        let k_bytes = serde_json::to_vec(k).unwrap(); // In production, handle error properly
        key.extend_from_slice(&k_bytes);
        key
    }
    
    pub fn save(&self, storage: &mut dyn Storage, key: &K, data: &T) -> StdResult<()>
    where
        K: ?Sized + Serialize,
    {
        let storage_key = self.key(key);
        let bytes = serde_json::to_vec(data)
            .map_err(|e| StdError::generic_err(format!("Serialization error: {}", e)))?;
        storage.set(&storage_key, &bytes);
        Ok(())
    }
    
    pub fn load(&self, storage: &dyn Storage, key: &K) -> StdResult<T>
    where
        K: ?Sized + Serialize,
    {
        let storage_key = self.key(key);
        let bytes = storage.get(&storage_key)
            .ok_or_else(|| StdError::not_found("Key not found"))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| StdError::generic_err(format!("Deserialization error: {}", e)))
    }
    
    pub fn range<'b>(
        &self,
        storage: &'b mut dyn Storage,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(Vec<u8>, T)>> + 'b> {
        // Create prefix bounds
        let prefix = self.namespace;
        let actual_start = match start {
            Some(s) => {
                let mut key = prefix.to_vec();
                key.extend_from_slice(b"::");
                key.extend_from_slice(s);
                Some(key)
            }
            None => {
                let mut key = prefix.to_vec();
                key.extend_from_slice(b"::");
                Some(key)
            }
        };
        
        let actual_end = match end {
            Some(e) => {
                let mut key = prefix.to_vec();
                key.extend_from_slice(b"::");
                key.extend_from_slice(e);
                Some(key)
            }
            None => {
                let mut key = prefix.to_vec();
                key.extend_from_slice(b"::~"); // Ensure we get all keys with this prefix
                Some(key)
            }
        };
        
        if let Ok(storage_mut) = storage.downcast_mut::<CosmWasmStorage>() {
            let iter = storage_mut.range(
                actual_start.as_deref(),
                actual_end.as_deref(),
                order,
            );
            
            Box::new(iter.filter_map(move |result| {
                match result {
                    Ok((key, value)) => {
                        // Remove namespace prefix from key
                        if key.starts_with(prefix) && key.len() > prefix.len() + 2 {
                            let trimmed_key = key[prefix.len() + 2..].to_vec();
                            match serde_json::from_slice::<T>(&value) {
                                Ok(data) => Some(Ok((trimmed_key, data))),
                                Err(e) => Some(Err(StdError::generic_err(format!("Deserialization error: {}", e)))),
                            }
                        } else {
                            None // Skip keys that don't match our prefix
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }))
        } else {
            Box::new(std::iter::empty())
        }
    }
}
```

## Cryptographic Compatibility Layer

### Core Cryptographic Interface

```rust
use cosmwasm_std::{
    Secp256k1VerifyError, Ed25519VerifyError, RecoverPubkeyError,
    HashFunction, StdError, StdResult,
};
use k256::ecdsa::{Signature, VerifyingKey, RecoveryId};
use k256::ecdsa::signature::Verifier;
use sha2::{Sha256, Digest};

pub struct CosmWasmCrypto;

impl CosmWasmCrypto {
    /// Verify secp256k1 signature (not natively available in NEAR)
    pub fn secp256k1_verify(
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Secp256k1VerifyError> {
        // Validate input lengths
        if message_hash.len() != 32 {
            return Err(Secp256k1VerifyError::InvalidHashFormat);
        }
        
        if signature.len() != 64 {
            return Err(Secp256k1VerifyError::InvalidSignatureFormat);
        }
        
        // Parse signature
        let signature = Signature::from_slice(signature)
            .map_err(|_| Secp256k1VerifyError::InvalidSignatureFormat)?;
        
        // Parse public key - handle both compressed (33 bytes) and uncompressed (65 bytes)
        let verifying_key = if public_key.len() == 33 {
            // Compressed format
            VerifyingKey::from_sec1_bytes(public_key)
                .map_err(|_| Secp256k1VerifyError::InvalidPublicKeyFormat)?
        } else if public_key.len() == 65 {
            // Uncompressed format
            VerifyingKey::from_sec1_bytes(public_key)
                .map_err(|_| Secp256k1VerifyError::InvalidPublicKeyFormat)?
        } else {
            return Err(Secp256k1VerifyError::InvalidPublicKeyFormat);
        };
        
        // Perform verification
        Ok(verifying_key.verify(message_hash, &signature).is_ok())
    }
    
    /// Recover public key from secp256k1 signature
    pub fn secp256k1_recover_pubkey(
        message_hash: &[u8],
        signature: &[u8],
        recovery_id: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        // Validate inputs
        if message_hash.len() != 32 {
            return Err(RecoverPubkeyError::InvalidHashFormat);
        }
        
        if signature.len() != 64 {
            return Err(RecoverPubkeyError::InvalidSignatureFormat);
        }
        
        if recovery_id > 3 {
            return Err(RecoverPubkeyError::InvalidRecoveryId);
        }
        
        // Parse signature and recovery ID
        let signature = Signature::from_slice(signature)
            .map_err(|_| RecoverPubkeyError::InvalidSignatureFormat)?;
        
        let recovery_id = RecoveryId::try_from(recovery_id)
            .map_err(|_| RecoverPubkeyError::InvalidRecoveryId)?;
        
        // Recover public key
        let verifying_key = VerifyingKey::recover_from_msg(message_hash, &signature, recovery_id)
            .map_err(|_| RecoverPubkeyError::RecoveryFailed)?;
        
        // Return compressed public key (33 bytes)
        Ok(verifying_key.to_encoded_point(true).as_bytes().to_vec())
    }
    
    /// Verify Ed25519 signature (use NEAR's native implementation)
    pub fn ed25519_verify(
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Ed25519VerifyError> {
        // Validate input lengths
        if signature.len() != 64 {
            return Err(Ed25519VerifyError::InvalidSignatureFormat);
        }
        
        if public_key.len() != 32 {
            return Err(Ed25519VerifyError::InvalidPublicKeyFormat);
        }
        
        // Use NEAR's native Ed25519 verification
        Ok(near_sdk::env::ed25519_verify(signature, message, public_key))
    }
    
    /// Batch verify Ed25519 signatures (not natively supported in NEAR)
    pub fn ed25519_batch_verify(
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, Ed25519VerifyError> {
        // Validate input lengths match
        if messages.len() != signatures.len() || messages.len() != public_keys.len() {
            return Err(Ed25519VerifyError::BatchLengthMismatch);
        }
        
        // For now, implement as individual verifications
        // In production, could optimize with dedicated batch verification library
        for i in 0..messages.len() {
            if !self.ed25519_verify(messages[i], signatures[i], public_keys[i])? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Hash functions compatibility
    pub fn hash(hash_function: HashFunction, data: &[u8]) -> Vec<u8> {
        match hash_function {
            HashFunction::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            HashFunction::Keccak256 => {
                // Use NEAR's native keccak256
                near_sdk::env::keccak256(data).to_vec()
            }
            HashFunction::Blake2b => {
                // Implement using blake2 crate if needed
                todo!("Implement Blake2b hashing")
            }
            HashFunction::Blake3 => {
                // Implement using blake3 crate if needed
                todo!("Implement Blake3 hashing")
            }
        }
    }
}

// Error type definitions to match CosmWasm
#[derive(Debug)]
pub enum Secp256k1VerifyError {
    InvalidHashFormat,
    InvalidSignatureFormat,
    InvalidPublicKeyFormat,
    VerificationFailed,
}

#[derive(Debug)]
pub enum Ed25519VerifyError {
    InvalidSignatureFormat,
    InvalidPublicKeyFormat,
    InvalidMessageFormat,
    BatchLengthMismatch,
    VerificationFailed,
}

#[derive(Debug)]
pub enum RecoverPubkeyError {
    InvalidHashFormat,
    InvalidSignatureFormat,
    InvalidRecoveryId,
    RecoveryFailed,
}
```

### Performance Optimization Strategies

```rust
use std::collections::LRU;
use once_cell::sync::Lazy;

// Global signature verification cache
static SIGNATURE_CACHE: Lazy<std::sync::Mutex<LruCache<(Vec<u8>, Vec<u8>), bool>>> = 
    Lazy::new(|| std::sync::Mutex::new(LruCache::new(1000)));

impl CosmWasmCrypto {
    /// Cached secp256k1 verification to avoid redundant computations
    pub fn secp256k1_verify_cached(
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Secp256k1VerifyError> {
        // Create cache key from signature and public key
        let cache_key = (signature.to_vec(), public_key.to_vec());
        
        // Check cache first
        if let Ok(mut cache) = SIGNATURE_CACHE.lock() {
            if let Some(&result) = cache.get(&cache_key) {
                return Ok(result);
            }
        }
        
        // Perform verification
        let result = Self::secp256k1_verify(message_hash, signature, public_key)?;
        
        // Cache the result
        if let Ok(mut cache) = SIGNATURE_CACHE.lock() {
            cache.put(cache_key, result);
        }
        
        Ok(result)
    }
    
    /// Gas estimation for cryptographic operations
    pub fn estimate_verify_gas(signature_type: &str, batch_size: usize) -> u64 {
        match signature_type {
            "secp256k1" => 100_000 * batch_size as u64, // ~100k gas per secp256k1 verification
            "ed25519" => 50_000 * batch_size as u64,    // ~50k gas per ed25519 verification (native)
            _ => 200_000 * batch_size as u64,           // Conservative estimate for unknown types
        }
    }
}
```

### Integration with Proxima Modules

```rust
// Integration point for cosmwasm crypto with Proxima
use crate::modules::auth::accounts::AccountManager;

impl CosmWasmCrypto {
    /// Verify transaction signature in Proxima context
    pub fn verify_transaction_signature(
        &self,
        account_manager: &AccountManager,
        tx_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
        signature_scheme: &str,
    ) -> StdResult<bool> {
        match signature_scheme {
            "secp256k1" => {
                self.secp256k1_verify(tx_hash, signature, public_key)
                    .map_err(|e| StdError::generic_err(format!("secp256k1 verification failed: {:?}", e)))
            }
            "ed25519" => {
                self.ed25519_verify(tx_hash, signature, public_key)
                    .map_err(|e| StdError::generic_err(format!("ed25519 verification failed: {:?}", e)))
            }
            _ => Err(StdError::generic_err(format!("Unsupported signature scheme: {}", signature_scheme))),
        }
    }
    
    /// Generate address from public key (CosmWasm compatibility)
    pub fn pubkey_to_address(&self, public_key: &[u8], prefix: &str) -> StdResult<String> {
        // Implement Cosmos-style address derivation
        let hash = self.hash(HashFunction::Sha256, public_key);
        let ripemd_hash = self.ripemd160(&hash);
        
        // Encode with bech32
        self.encode_bech32(prefix, &ripemd_hash)
    }
    
    fn ripemd160(&self, data: &[u8]) -> Vec<u8> {
        // Implement RIPEMD160 hashing
        // This might require external crate since NEAR doesn't have native RIPEMD160
        todo!("Implement RIPEMD160 hashing")
    }
    
    fn encode_bech32(&self, prefix: &str, data: &[u8]) -> StdResult<String> {
        // Implement bech32 encoding for Cosmos addresses
        // This would use the bech32 crate
        todo!("Implement bech32 encoding")
    }
}
```

## Testing and Validation Strategy

### Unit Testing Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};
    
    fn get_context() -> VMContext {
        VMContextBuilder::new()
            .current_account_id("contract.testnet".parse().unwrap())
            .build()
    }
    
    #[test]
    fn test_storage_basic_operations() {
        testing_env!(get_context());
        
        let mut storage = CosmWasmStorage::new();
        
        // Test set and get
        storage.set(b"key1", b"value1");
        assert_eq!(storage.get(b"key1"), Some(b"value1".to_vec()));
        
        // Test remove
        storage.remove(b"key1");
        assert_eq!(storage.get(b"key1"), None);
    }
    
    #[test]
    fn test_storage_range_queries() {
        testing_env!(get_context());
        
        let mut storage = CosmWasmStorage::new();
        
        // Set up test data
        storage.set(b"key1", b"value1");
        storage.set(b"key2", b"value2");
        storage.set(b"key3", b"value3");
        
        // Test range query
        let results: Vec<_> = storage
            .range(Some(b"key1"), Some(b"key3"), Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, b"key1");
        assert_eq!(results[1].0, b"key2");
    }
    
    #[test]
    fn test_secp256k1_verification() {
        let crypto = CosmWasmCrypto;
        
        // Test vectors from known good signature
        let message_hash = hex::decode("5d41402abc4b2a76b9719d911017c592").unwrap();
        let signature = hex::decode("...").unwrap(); // 64-byte signature
        let public_key = hex::decode("...").unwrap(); // 33-byte compressed pubkey
        
        let result = crypto.secp256k1_verify(&message_hash, &signature, &public_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
    
    #[test]
    fn test_ed25519_verification() {
        let crypto = CosmWasmCrypto;
        
        // Test vectors for Ed25519
        let message = b"test message";
        let signature = hex::decode("...").unwrap(); // 64-byte signature
        let public_key = hex::decode("...").unwrap(); // 32-byte pubkey
        
        let result = crypto.ed25519_verify(message, &signature, &public_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
```

### Performance Benchmarks

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn benchmark_storage_operations() {
        let mut storage = CosmWasmStorage::new();
        
        // Benchmark writes
        let start = Instant::now();
        for i in 0..1000 {
            storage.set(&format!("key{}", i).as_bytes(), &format!("value{}", i).as_bytes());
        }
        let write_duration = start.elapsed();
        println!("1000 writes took: {:?}", write_duration);
        
        // Benchmark reads
        let start = Instant::now();
        for i in 0..1000 {
            storage.get(&format!("key{}", i).as_bytes());
        }
        let read_duration = start.elapsed();
        println!("1000 reads took: {:?}", read_duration);
        
        // Benchmark range queries
        let start = Instant::now();
        let _results: Vec<_> = storage
            .range(None, None, Order::Ascending)
            .take(100)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        let range_duration = start.elapsed();
        println!("Range query (100 items) took: {:?}", range_duration);
    }
    
    #[test]
    fn benchmark_crypto_operations() {
        let crypto = CosmWasmCrypto;
        
        // Setup test data
        let message_hash = vec![0u8; 32];
        let signature = vec![0u8; 64];
        let public_key = vec![0u8; 33];
        
        // Benchmark secp256k1 verification
        let start = Instant::now();
        for _ in 0..100 {
            let _ = crypto.secp256k1_verify(&message_hash, &signature, &public_key);
        }
        let secp256k1_duration = start.elapsed();
        println!("100 secp256k1 verifications took: {:?}", secp256k1_duration);
        
        // Benchmark ed25519 verification
        let message = vec![0u8; 100];
        let ed25519_signature = vec![0u8; 64];
        let ed25519_pubkey = vec![0u8; 32];
        
        let start = Instant::now();
        for _ in 0..100 {
            let _ = crypto.ed25519_verify(&message, &ed25519_signature, &ed25519_pubkey);
        }
        let ed25519_duration = start.elapsed();
        println!("100 ed25519 verifications took: {:?}", ed25519_duration);
    }
}
```

## Implementation Roadmap

### Phase 1: Basic Storage Layer (Week 1)
- [ ] Implement basic Storage trait for NEAR collections
- [ ] Create CosmWasmStorage struct with UnorderedMap backend
- [ ] Implement get/set/remove operations
- [ ] Add basic caching mechanism
- [ ] Unit tests for basic operations

### Phase 2: Range Query Implementation (Week 2)
- [ ] Implement sorted key management
- [ ] Create range iterator with bounds support
- [ ] Add ascending/descending order support
- [ ] Optimize performance for large datasets
- [ ] Integration tests for range queries

### Phase 3: cw-storage-plus Compatibility (Week 3)
- [ ] Implement Item<T> wrapper
- [ ] Implement Map<K, T> wrapper
- [ ] Add serialization/deserialization support
- [ ] Create namespace-based key management
- [ ] Compatibility tests with existing patterns

### Phase 4: Cryptographic Implementation (Week 4)
- [ ] Implement secp256k1 verification using k256 crate
- [ ] Implement secp256k1 public key recovery
- [ ] Wrap NEAR's ed25519 verification
- [ ] Add signature caching for performance
- [ ] Comprehensive crypto testing

### Phase 5: Performance Optimization (Week 5)
- [ ] Profile storage operations and optimize bottlenecks
- [ ] Implement efficient range query caching
- [ ] Optimize cryptographic operations
- [ ] Add gas estimation functions
- [ ] Performance benchmarking suite

### Phase 6: Integration and Testing (Week 6)
- [ ] Integrate with existing Proxima modules
- [ ] End-to-end testing with real CosmWasm contracts
- [ ] Security audit of compatibility layer
- [ ] Documentation and examples
- [ ] Migration guides for developers

This specification provides the detailed technical foundation needed to implement robust CosmWasm compatibility within the NEAR ecosystem, addressing the core challenges of storage patterns and cryptographic operations while maintaining performance and security standards.