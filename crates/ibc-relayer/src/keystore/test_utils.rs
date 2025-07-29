// Test utilities for keystore functionality
// Provides mock implementations and helpers for testing

use super::{KeyEntry, KeyManager, KeyManagerConfig};
use super::cosmos::CosmosKey;
use super::near::NearKey;
use std::collections::HashMap;
use tempfile::TempDir;

/// Create a test keystore configuration with temporary directory
pub fn create_test_config() -> (KeyManagerConfig, TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = KeyManagerConfig {
        keystore_dir: temp_dir.path().to_path_buf(),
        allow_env_keys: true,
        env_prefix: "TEST_KEY_".to_string(),
        kdf_iterations: 1000, // Lower for testing
    };
    (config, temp_dir)
}

/// Create a test CosmosKey with valid structure
pub fn create_test_cosmos_key() -> CosmosKey {
    CosmosKey {
        address: "cosmos1test123".to_string(),
        private_key: vec![1; 32], // 32 bytes for secp256k1
        public_key: vec![2; 33],  // 33 bytes compressed
        key_type: "secp256k1".to_string(),
    }
}

/// Create a test NearKey with valid structure
pub fn create_test_near_key() -> NearKey {
    NearKey {
        account_id: "test.near".to_string(),
        secret_key: "ed25519:test_secret_key".to_string(),
        public_key: "ed25519:test_public_key".to_string(),
        key_type: "ed25519".to_string(),
    }
}

/// Mock KeyManager that doesn't use encryption for testing
pub struct MockKeyManager {
    pub keys: HashMap<String, KeyEntry>,
    pub config: KeyManagerConfig,
}

impl MockKeyManager {
    pub fn new(config: KeyManagerConfig) -> Self {
        Self {
            keys: HashMap::new(),
            config,
        }
    }

    pub async fn store_key(&mut self, chain_id: &str, key: KeyEntry, _password: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.keys.insert(chain_id.to_string(), key);
        Ok(())
    }

    pub async fn load_key(&mut self, chain_id: &str) -> Result<KeyEntry, Box<dyn std::error::Error + Send + Sync>> {
        self.keys.get(chain_id)
            .cloned()
            .ok_or_else(|| format!("Key not found for chain: {}", chain_id).into())
    }

    pub async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.keys.keys().cloned().collect())
    }

    pub fn get_address(&self, chain_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match self.keys.get(chain_id) {
            Some(KeyEntry::Cosmos(key)) => Ok(key.address.clone()),
            Some(KeyEntry::Near(key)) => Ok(key.account_id.clone()),
            None => Err(format!("Key not found for chain: {}", chain_id).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_config() {
        let (config, _temp_dir) = create_test_config();
        assert!(config.keystore_dir.exists() || config.keystore_dir.parent().map_or(false, |p| p.exists()));
        assert_eq!(config.env_prefix, "TEST_KEY_");
        assert_eq!(config.kdf_iterations, 1000);
    }

    #[test]
    fn test_create_test_cosmos_key() {
        let key = create_test_cosmos_key();
        assert_eq!(key.address, "cosmos1test123");
        assert_eq!(key.private_key.len(), 32);
        assert_eq!(key.public_key.len(), 33);
        assert_eq!(key.key_type, "secp256k1");
    }

    #[test]
    fn test_create_test_near_key() {
        let key = create_test_near_key();
        assert_eq!(key.account_id, "test.near");
        assert!(key.secret_key.starts_with("ed25519:"));
        assert_eq!(key.key_type, "ed25519");
    }

    #[tokio::test]
    async fn test_mock_key_manager() {
        let (config, _temp_dir) = create_test_config();
        let mut mock_manager = MockKeyManager::new(config);

        // Test storing and retrieving keys
        let cosmos_key = create_test_cosmos_key();
        let key_entry = KeyEntry::Cosmos(cosmos_key.clone());

        mock_manager.store_key("test-chain", key_entry, "password").await.unwrap();

        // Test retrieval
        let retrieved = mock_manager.load_key("test-chain").await.unwrap();
        match retrieved {
            KeyEntry::Cosmos(key) => {
                assert_eq!(key.address, cosmos_key.address);
            }
            _ => panic!("Expected Cosmos key"),
        }

        // Test address retrieval
        let address = mock_manager.get_address("test-chain").unwrap();
        assert_eq!(address, cosmos_key.address);

        // Test key listing
        let keys = mock_manager.list_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"test-chain".to_string()));
    }
}