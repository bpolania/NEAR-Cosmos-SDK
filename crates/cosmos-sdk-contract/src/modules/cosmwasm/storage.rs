use near_sdk::collections::{UnorderedMap, LookupMap, Vector};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use crate::modules::cosmwasm::types::{Storage, Order, Record, StdResult, StdError};
use std::cmp::Ordering;

/// CosmWasm-compatible storage implementation using NEAR collections
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CosmWasmStorage {
    /// Primary storage using NEAR's UnorderedMap for full iteration support
    data: UnorderedMap<Vec<u8>, Vec<u8>>,
    
    /// Fast lookup cache for frequently accessed keys
    cache: LookupMap<Vec<u8>, Vec<u8>>,
    
    /// Sorted keys for efficient range queries
    sorted_keys: Vector<Vec<u8>>,
    
    /// Flag to track if sorted keys need rebuilding
    keys_dirty: bool,
}

impl CosmWasmStorage {
    pub fn new() -> Self {
        Self {
            data: UnorderedMap::new(b"cw_data".to_vec()),
            cache: LookupMap::new(b"cw_cache".to_vec()),
            sorted_keys: Vector::new(b"cw_sorted".to_vec()),
            keys_dirty: false,
        }
    }
}

impl Default for CosmWasmStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl CosmWasmStorage {
    /// Check if a key should be cached based on access patterns
    fn should_cache(&self, _key: &[u8]) -> bool {
        // For now, cache keys shorter than 32 bytes
        // In production, would track access frequency
        _key.len() < 32
    }

    /// Ensure sorted keys are up to date for range queries
    fn ensure_sorted_keys(&mut self) {
        if self.keys_dirty {
            self.rebuild_sorted_keys();
            self.keys_dirty = false;
        }
    }

    /// Rebuild the sorted keys vector
    fn rebuild_sorted_keys(&mut self) {
        // Clear existing sorted keys
        self.sorted_keys.clear();
        
        // Collect all keys and sort them
        let mut keys: Vec<Vec<u8>> = self.data.keys_as_vector().to_vec();
        keys.sort();
        
        // Add sorted keys back to vector
        for key in keys {
            self.sorted_keys.push(&key);
        }
    }

    /// Find the index of a key in sorted keys (or where it would be inserted)
    fn find_key_index(&self, target: &[u8]) -> usize {
        let len = self.sorted_keys.len() as usize;
        let mut left = 0;
        let mut right = len;
        
        while left < right {
            let mid = left + (right - left) / 2;
            if let Some(key) = self.sorted_keys.get(mid as u64) {
                match key.as_slice().cmp(target) {
                    Ordering::Less => left = mid + 1,
                    Ordering::Greater => right = mid,
                    Ordering::Equal => return mid,
                }
            } else {
                break;
            }
        }
        
        left
    }

    /// Perform a range query over the storage
    pub fn range(
        &mut self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record>> + '_> {
        // Ensure sorted keys are up to date
        self.ensure_sorted_keys();
        
        let start_idx = match start {
            Some(key) => self.find_key_index(key),
            None => 0,
        };
        
        let end_idx = match end {
            Some(key) => self.find_key_index(key),
            None => self.sorted_keys.len() as usize,
        };
        
        match order {
            Order::Ascending => Box::new(RangeIterator::new_ascending(
                &self.data,
                &self.sorted_keys,
                start_idx,
                end_idx,
            )),
            Order::Descending => Box::new(RangeIterator::new_descending(
                &self.data,
                &self.sorted_keys,
                start_idx,
                end_idx,
            )),
        }
    }

    /// Get all keys with a specific prefix
    pub fn prefix_range(
        &mut self,
        prefix: &[u8],
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record>> + '_> {
        // Create end bound by incrementing the last byte of prefix
        let mut end_bound = prefix.to_vec();
        if let Some(last) = end_bound.last_mut() {
            *last = last.saturating_add(1);
        }
        
        self.range(Some(prefix), Some(&end_bound), order)
    }
}

impl Storage for CosmWasmStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // First check cache
        if let Some(value) = self.cache.get(&key.to_vec()) {
            return Some(value);
        }
        
        // Then check main storage
        let value = self.data.get(&key.to_vec());
        
        // Cache the value if appropriate
        if let Some(ref v) = value {
            if self.should_cache(key) {
                // Note: In real implementation, we'd need a mutable reference
                // or use interior mutability for caching
            }
        }
        
        value
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        let key_vec = key.to_vec();
        let value_vec = value.to_vec();
        
        // Check if this is a new key
        let is_new = self.data.get(&key_vec).is_none();
        
        // Update main storage
        self.data.insert(&key_vec, &value_vec);
        
        // Update cache if key is cached or should be cached
        if self.cache.get(&key_vec).is_some() || self.should_cache(key) {
            self.cache.insert(&key_vec, &value_vec);
        }
        
        // Mark keys as dirty if this is a new key
        if is_new {
            self.keys_dirty = true;
        }
    }

    fn remove(&mut self, key: &[u8]) {
        let key_vec = key.to_vec();
        
        // Remove from main storage
        self.data.remove(&key_vec);
        
        // Remove from cache
        self.cache.remove(&key_vec);
        
        // Mark keys as dirty
        self.keys_dirty = true;
    }
}

/// Iterator for range queries
pub struct RangeIterator<'a> {
    storage: &'a UnorderedMap<Vec<u8>, Vec<u8>>,
    sorted_keys: &'a Vector<Vec<u8>>,
    current: u64,
    end: u64,
    reverse: bool,
}

impl<'a> RangeIterator<'a> {
    fn new_ascending(
        storage: &'a UnorderedMap<Vec<u8>, Vec<u8>>,
        sorted_keys: &'a Vector<Vec<u8>>,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            storage,
            sorted_keys,
            current: start as u64,
            end: end as u64,
            reverse: false,
        }
    }

    fn new_descending(
        storage: &'a UnorderedMap<Vec<u8>, Vec<u8>>,
        sorted_keys: &'a Vector<Vec<u8>>,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            storage,
            sorted_keys,
            current: if end > 0 { (end - 1) as u64 } else { 0 },
            end: if start > 0 { (start - 1) as u64 } else { 0 },
            reverse: true,
        }
    }
}

impl<'a> Iterator for RangeIterator<'a> {
    type Item = StdResult<Record>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.reverse {
            if self.current < self.end {
                return None;
            }
        } else {
            if self.current >= self.end {
                return None;
            }
        }
        
        // Get the key at current index
        let key = match self.sorted_keys.get(self.current) {
            Some(k) => k,
            None => return Some(Err(StdError::generic_err("Invalid key index"))),
        };
        
        // Get the value for this key
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

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;
    
    fn setup_context() {
        let context = VMContextBuilder::new()
            .current_account_id("contract.testnet".parse().unwrap())
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_basic_storage_operations() {
        setup_context();
        
        let mut storage = CosmWasmStorage::new();
        
        // Test set and get
        storage.set(b"key1", b"value1");
        assert_eq!(storage.get(b"key1"), Some(b"value1".to_vec()));
        
        // Test overwrite
        storage.set(b"key1", b"value2");
        assert_eq!(storage.get(b"key1"), Some(b"value2".to_vec()));
        
        // Test remove
        storage.remove(b"key1");
        assert_eq!(storage.get(b"key1"), None);
        
        // Test non-existent key
        assert_eq!(storage.get(b"nonexistent"), None);
    }
    
    #[test]
    fn test_range_queries() {
        setup_context();
        
        let mut storage = CosmWasmStorage::new();
        
        // Set up test data
        storage.set(b"key1", b"value1");
        storage.set(b"key3", b"value3");
        storage.set(b"key2", b"value2");
        storage.set(b"key4", b"value4");
        
        // Test ascending range query
        let results: Vec<_> = storage
            .range(Some(b"key1"), Some(b"key4"), Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, b"key1");
        assert_eq!(results[1].0, b"key2");
        assert_eq!(results[2].0, b"key3");
        
        // Test descending range query
        let results: Vec<_> = storage
            .range(Some(b"key2"), Some(b"key5"), Order::Descending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, b"key4");
        assert_eq!(results[1].0, b"key3");
        assert_eq!(results[2].0, b"key2");
    }
    
    #[test]
    fn test_prefix_range() {
        setup_context();
        
        let mut storage = CosmWasmStorage::new();
        
        // Set up test data with common prefixes
        storage.set(b"user:alice", b"data1");
        storage.set(b"user:bob", b"data2");
        storage.set(b"user:charlie", b"data3");
        storage.set(b"admin:root", b"data4");
        
        // Test prefix range query
        let results: Vec<_> = storage
            .prefix_range(b"user:", Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, b"user:alice");
        assert_eq!(results[1].0, b"user:bob");
        assert_eq!(results[2].0, b"user:charlie");
    }
}