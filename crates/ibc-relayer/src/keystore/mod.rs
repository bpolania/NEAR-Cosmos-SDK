// Secure key management for IBC relayer
// Supports encrypted keystores, environment variables, and secure key handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod cosmos;
pub mod near;
pub mod storage;

#[cfg(test)]
pub mod test_utils;

// Re-export key types
pub use cosmos::CosmosKey;
pub use near::NearKey;
pub use storage::{KeyStorage, EncryptedKeystore, KeystoreError};

/// Errors that can occur during key management operations
#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Key not found: {0}")]
    NotFound(String),
    
    #[error("Invalid key format: {0}")]
    InvalidFormat(String),
    
    #[error("Keystore error: {0}")]
    Keystore(#[from] KeystoreError),
    
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Unified key manager for both NEAR and Cosmos chains
pub struct KeyManager {
    /// Storage backend for encrypted keys
    storage: Box<dyn KeyStorage>,
    /// Cache of loaded keys (chain_id -> key)
    key_cache: HashMap<String, KeyEntry>,
    /// Configuration
    config: KeyManagerConfig,
}

/// Configuration for the key manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagerConfig {
    /// Directory for keystore files
    pub keystore_dir: PathBuf,
    /// Whether to allow environment variable keys
    pub allow_env_keys: bool,
    /// Environment variable prefix (e.g., "RELAYER_KEY_")
    pub env_prefix: String,
    /// Default key derivation iterations for encryption
    pub kdf_iterations: u32,
}

impl Default for KeyManagerConfig {
    fn default() -> Self {
        Self {
            keystore_dir: PathBuf::from("~/.relayer/keys"),
            allow_env_keys: true,
            env_prefix: "RELAYER_KEY_".to_string(),
            kdf_iterations: 10_000,
        }
    }
}

/// A key entry in the manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyEntry {
    /// NEAR account key
    Near(NearKey),
    /// Cosmos account key
    Cosmos(CosmosKey),
}

impl KeyManager {
    /// Create a new key manager with the given configuration
    pub fn new(config: KeyManagerConfig) -> Result<Self, KeyError> {
        // Expand home directory
        let keystore_dir = shellexpand::tilde(&config.keystore_dir.to_string_lossy()).to_string();
        let keystore_dir = PathBuf::from(keystore_dir);
        
        // Create keystore directory if it doesn't exist
        std::fs::create_dir_all(&keystore_dir)?;
        
        // Create encrypted keystore backend
        let storage = Box::new(EncryptedKeystore::new(keystore_dir)?);
        
        Ok(Self {
            storage,
            key_cache: HashMap::new(),
            config,
        })
    }
    
    /// Load a key for the given chain ID
    pub async fn load_key(&mut self, chain_id: &str) -> Result<KeyEntry, KeyError> {
        // Check cache first
        if let Some(key) = self.key_cache.get(chain_id) {
            return Ok(key.clone());
        }
        
        // Try environment variable if allowed
        if self.config.allow_env_keys {
            if let Ok(key) = self.load_from_env(chain_id).await {
                self.key_cache.insert(chain_id.to_string(), key.clone());
                return Ok(key);
            }
        }
        
        // Try loading from keystore
        let key = self.storage.load_key(chain_id).await?;
        self.key_cache.insert(chain_id.to_string(), key.clone());
        Ok(key)
    }
    
    /// Store a key for the given chain ID with encryption
    pub async fn store_key(
        &mut self,
        chain_id: &str,
        key: KeyEntry,
        password: &str,
    ) -> Result<(), KeyError> {
        // Store in keystore
        self.storage.store_key(chain_id, &key, password).await?;
        
        // Update cache
        self.key_cache.insert(chain_id.to_string(), key);
        
        Ok(())
    }
    
    /// Remove a key from storage
    pub async fn remove_key(&mut self, chain_id: &str) -> Result<(), KeyError> {
        // Remove from storage
        self.storage.remove_key(chain_id).await?;
        
        // Remove from cache
        self.key_cache.remove(chain_id);
        
        Ok(())
    }
    
    /// List all available keys
    pub async fn list_keys(&self) -> Result<Vec<String>, KeyError> {
        self.storage.list_keys().await
    }
    
    /// Load a key from environment variable
    async fn load_from_env(&self, chain_id: &str) -> Result<KeyEntry, KeyError> {
        let env_var = format!("{}{}", self.config.env_prefix, chain_id.to_uppercase());
        
        let key_data = std::env::var(&env_var)
            .map_err(|_| KeyError::EnvVarNotFound(env_var.clone()))?;
        
        // Determine key type based on chain ID
        if chain_id.contains("near") {
            Ok(KeyEntry::Near(NearKey::from_env_string(&key_data)?))
        } else if chain_id.contains("cosmos") || chain_id.contains("hub") {
            Ok(KeyEntry::Cosmos(CosmosKey::from_env_string(&key_data)?))
        } else {
            Err(KeyError::InvalidFormat(
                "Cannot determine key type from chain ID".to_string()
            ))
        }
    }
    
    /// Export a key as a string (for display or backup)
    pub fn export_key(&self, chain_id: &str) -> Result<String, KeyError> {
        let key = self.key_cache.get(chain_id)
            .ok_or_else(|| KeyError::NotFound(chain_id.to_string()))?;
        
        match key {
            KeyEntry::Near(near_key) => Ok(near_key.to_export_string()),
            KeyEntry::Cosmos(cosmos_key) => Ok(cosmos_key.to_export_string()),
        }
    }
    
    /// Get the public key/address for a chain
    pub fn get_address(&self, chain_id: &str) -> Result<String, KeyError> {
        let key = self.key_cache.get(chain_id)
            .ok_or_else(|| KeyError::NotFound(chain_id.to_string()))?;
        
        match key {
            KeyEntry::Near(near_key) => Ok(near_key.account_id.clone()),
            KeyEntry::Cosmos(cosmos_key) => Ok(cosmos_key.address.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_key_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let config = KeyManagerConfig {
            keystore_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = KeyManager::new(config).unwrap();
        assert!(manager.key_cache.is_empty());
    }
    
    #[tokio::test]
    async fn test_key_storage_and_retrieval() {
        let temp_dir = tempdir().unwrap();
        let config = KeyManagerConfig {
            keystore_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let mut manager = KeyManager::new(config).unwrap();
        
        // Create a test Cosmos key
        let cosmos_key = CosmosKey {
            address: "cosmos1test".to_string(),
            private_key: vec![1, 2, 3, 4], // Mock key
            public_key: vec![5, 6, 7, 8],  // Mock key
            key_type: "secp256k1".to_string(),
        };
        
        let key_entry = KeyEntry::Cosmos(cosmos_key);
        
        // Store the key
        manager.store_key("cosmoshub-testnet", key_entry, "test_password").await.unwrap();
        
        // List keys
        let keys = manager.list_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"cosmoshub-testnet".to_string()));
    }
}