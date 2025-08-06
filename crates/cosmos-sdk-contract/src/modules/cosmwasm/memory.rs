use std::collections::HashMap;
use near_sdk::env;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

/// Memory manager that bridges CosmWasm's allocation model with NEAR's register system
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CosmWasmMemoryManager {
    /// Simulated memory regions indexed by "pointer" (actually an ID)
    regions: HashMap<usize, Vec<u8>>,
    
    /// Next available region ID
    next_id: usize,
    
    /// Pre-allocated buffers for common operations
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl CosmWasmMemoryManager {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            next_id: 1,
            read_buffer: Vec::with_capacity(1024),
            write_buffer: Vec::with_capacity(1024),
        }
    }
}

impl Default for CosmWasmMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CosmWasmMemoryManager {
    /// Allocate a new memory region and return its "pointer" (ID)
    pub fn allocate(&mut self, size: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        
        // Create a new region with the requested size
        self.regions.insert(id, vec![0; size]);
        
        // Log allocation for debugging
        env::log_str(&format!("Allocated {} bytes at region {}", size, id));
        
        id
    }
    
    /// Deallocate a memory region
    pub fn deallocate(&mut self, ptr: usize) {
        if self.regions.remove(&ptr).is_some() {
            env::log_str(&format!("Deallocated region {}", ptr));
        } else {
            env::log_str(&format!("Warning: Attempted to deallocate invalid region {}", ptr));
        }
    }
    
    /// Write data to a memory region
    pub fn write(&mut self, ptr: usize, offset: usize, data: &[u8]) -> Result<(), String> {
        match self.regions.get_mut(&ptr) {
            Some(region) => {
                let end = offset + data.len();
                
                if end > region.len() {
                    return Err(format!(
                        "Write out of bounds: offset {} + len {} > region size {}",
                        offset,
                        data.len(),
                        region.len()
                    ));
                }
                
                region[offset..end].copy_from_slice(data);
                Ok(())
            }
            None => Err(format!("Invalid memory region: {}", ptr)),
        }
    }
    
    /// Read data from a memory region
    pub fn read(&self, ptr: usize, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        match self.regions.get(&ptr) {
            Some(region) => {
                let end = offset + len;
                
                if end > region.len() {
                    return Err(format!(
                        "Read out of bounds: offset {} + len {} > region size {}",
                        offset,
                        len,
                        region.len()
                    ));
                }
                
                Ok(region[offset..end].to_vec())
            }
            None => Err(format!("Invalid memory region: {}", ptr)),
        }
    }
    
    /// Get the size of a memory region
    pub fn region_size(&self, ptr: usize) -> Option<usize> {
        self.regions.get(&ptr).map(|region| region.len())
    }
    
    /// Get a read buffer for temporary operations
    pub fn get_read_buffer(&mut self, size: usize) -> &mut Vec<u8> {
        self.read_buffer.clear();
        self.read_buffer.reserve(size);
        &mut self.read_buffer
    }
    
    /// Get a write buffer for temporary operations
    pub fn get_write_buffer(&mut self, size: usize) -> &mut Vec<u8> {
        self.write_buffer.clear();
        self.write_buffer.reserve(size);
        &mut self.write_buffer
    }
    
    /// Clear all allocated regions (useful for cleanup)
    pub fn clear(&mut self) {
        self.regions.clear();
        self.next_id = 1;
        self.read_buffer.clear();
        self.write_buffer.clear();
    }
    
    /// Get total memory usage
    pub fn total_memory_usage(&self) -> usize {
        self.regions.values().map(|region| region.len()).sum()
    }
    
    /// Get number of active regions
    pub fn active_regions(&self) -> usize {
        self.regions.len()
    }
}

/// Helper functions for CosmWasm compatibility
impl CosmWasmMemoryManager {
    /// Allocate and write data in one operation
    pub fn allocate_and_write(&mut self, data: &[u8]) -> Result<usize, String> {
        let ptr = self.allocate(data.len());
        self.write(ptr, 0, data)?;
        Ok(ptr)
    }
    
    /// Read and deallocate in one operation
    pub fn read_and_deallocate(&mut self, ptr: usize) -> Result<Vec<u8>, String> {
        let data = match self.regions.get(&ptr) {
            Some(region) => region.clone(),
            None => return Err(format!("Invalid memory region: {}", ptr)),
        };
        
        self.deallocate(ptr);
        Ok(data)
    }
    
    /// Resize a memory region
    pub fn resize(&mut self, ptr: usize, new_size: usize) -> Result<(), String> {
        match self.regions.get_mut(&ptr) {
            Some(region) => {
                region.resize(new_size, 0);
                Ok(())
            }
            None => Err(format!("Invalid memory region: {}", ptr)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocate_deallocate() {
        let mut manager = CosmWasmMemoryManager::new();
        
        // Test allocation
        let ptr1 = manager.allocate(100);
        let ptr2 = manager.allocate(200);
        
        assert_ne!(ptr1, ptr2);
        assert_eq!(manager.region_size(ptr1), Some(100));
        assert_eq!(manager.region_size(ptr2), Some(200));
        assert_eq!(manager.active_regions(), 2);
        
        // Test deallocation
        manager.deallocate(ptr1);
        assert_eq!(manager.region_size(ptr1), None);
        assert_eq!(manager.active_regions(), 1);
    }
    
    #[test]
    fn test_read_write() {
        let mut manager = CosmWasmMemoryManager::new();
        
        let ptr = manager.allocate(100);
        let data = b"Hello, CosmWasm!";
        
        // Test write
        assert!(manager.write(ptr, 0, data).is_ok());
        
        // Test read
        let read_data = manager.read(ptr, 0, data.len()).unwrap();
        assert_eq!(read_data, data);
        
        // Test partial read
        let partial = manager.read(ptr, 7, 8).unwrap();
        assert_eq!(partial, b"CosmWasm");
        
        // Test out of bounds write
        assert!(manager.write(ptr, 95, b"Too long").is_err());
        
        // Test out of bounds read
        assert!(manager.read(ptr, 0, 101).is_err());
    }
    
    #[test]
    fn test_memory_helpers() {
        let mut manager = CosmWasmMemoryManager::new();
        
        // Test allocate_and_write
        let data = b"Test data";
        let ptr = manager.allocate_and_write(data).unwrap();
        
        let read_data = manager.read(ptr, 0, data.len()).unwrap();
        assert_eq!(read_data, data);
        
        // Test read_and_deallocate
        let final_data = manager.read_and_deallocate(ptr).unwrap();
        assert_eq!(final_data, data);
        assert_eq!(manager.region_size(ptr), None);
    }
    
    #[test]
    fn test_resize() {
        let mut manager = CosmWasmMemoryManager::new();
        
        let ptr = manager.allocate(50);
        let data = b"Initial data";
        manager.write(ptr, 0, data).unwrap();
        
        // Resize larger
        assert!(manager.resize(ptr, 100).is_ok());
        assert_eq!(manager.region_size(ptr), Some(100));
        
        // Original data should still be there
        let read_data = manager.read(ptr, 0, data.len()).unwrap();
        assert_eq!(read_data, data);
        
        // New space should be zeroed
        let new_space = manager.read(ptr, 50, 50).unwrap();
        assert_eq!(new_space, vec![0; 50]);
        
        // Resize smaller
        assert!(manager.resize(ptr, 30).is_ok());
        assert_eq!(manager.region_size(ptr), Some(30));
    }
    
    #[test]
    fn test_buffers() {
        let mut manager = CosmWasmMemoryManager::new();
        
        // Test read buffer
        let read_buf = manager.get_read_buffer(100);
        assert!(read_buf.capacity() >= 100);
        assert_eq!(read_buf.len(), 0);
        
        // Test write buffer
        let write_buf = manager.get_write_buffer(200);
        assert!(write_buf.capacity() >= 200);
        assert_eq!(write_buf.len(), 0);
    }
    
    #[test]
    fn test_memory_usage() {
        let mut manager = CosmWasmMemoryManager::new();
        
        let _ptr1 = manager.allocate(100);
        let ptr2 = manager.allocate(200);
        let _ptr3 = manager.allocate(300);
        
        assert_eq!(manager.total_memory_usage(), 600);
        assert_eq!(manager.active_regions(), 3);
        
        manager.deallocate(ptr2);
        assert_eq!(manager.total_memory_usage(), 400);
        assert_eq!(manager.active_regions(), 2);
        
        manager.clear();
        assert_eq!(manager.total_memory_usage(), 0);
        assert_eq!(manager.active_regions(), 0);
    }
}