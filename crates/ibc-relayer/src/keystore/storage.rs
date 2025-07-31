// Encrypted key storage implementation
use super::{KeyEntry, KeyError, CosmosKey, NearKey};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::{RngCore, Rng};

/// Errors specific to keystore operations
#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Decryption error: {0}")]
    Decryption(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Corrupted keystore file")]
    CorruptedFile,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Trait for key storage backends
#[async_trait]
pub trait KeyStorage: Send + Sync {
    /// Load a key by chain ID
    async fn load_key(&self, chain_id: &str) -> Result<KeyEntry, KeyError>;
    
    /// Store a key with encryption
    async fn store_key(&self, chain_id: &str, key: &KeyEntry, password: &str) -> Result<(), KeyError>;
    
    /// Remove a key
    async fn remove_key(&self, chain_id: &str) -> Result<(), KeyError>;
    
    /// List all available keys
    async fn list_keys(&self) -> Result<Vec<String>, KeyError>;
}

/// Encrypted keystore file format
#[derive(Debug, Serialize, Deserialize)]
struct KeystoreFile {
    /// Version of the keystore format
    version: u32,
    /// Chain ID this key belongs to
    chain_id: String,
    /// Key type ("cosmos" or "near")
    key_type: String,
    /// Encrypted key data
    ciphertext: Vec<u8>,
    /// Nonce used for encryption
    nonce: Vec<u8>,
    /// Salt for key derivation
    salt: String,
    /// KDF parameters
    kdf: KdfParams,
}

/// Key derivation function parameters
#[derive(Debug, Serialize, Deserialize)]
struct KdfParams {
    /// Number of iterations
    iterations: u32,
    /// Memory cost
    memory: u32,
    /// Parallelism
    parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            iterations: 3,      // Argon2 iterations
            memory: 65536,      // 64 MB
            parallelism: 4,     // 4 threads
        }
    }
}

/// Encrypted keystore implementation
pub struct EncryptedKeystore {
    /// Directory for keystore files
    keystore_dir: PathBuf,
}

impl EncryptedKeystore {
    /// Create a new encrypted keystore
    pub fn new<P: AsRef<Path>>(keystore_dir: P) -> Result<Self, KeystoreError> {
        let keystore_dir = keystore_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&keystore_dir)?;
        
        Ok(Self { keystore_dir })
    }
    
    /// Get the path for a keystore file
    fn get_keystore_path(&self, chain_id: &str) -> PathBuf {
        self.keystore_dir.join(format!("{}.json", chain_id))
    }
    
    /// Derive encryption key from password
    fn derive_key(password: &str, salt: &str) -> Result<[u8; 32], KeystoreError> {
        let argon2 = Argon2::default();
        
        // Parse the salt string (it's in SaltString format, not PasswordHash format)
        let salt_bytes = SaltString::from_b64(salt)
            .map_err(|e| KeystoreError::Encryption(format!("Invalid salt: {}", e)))?;
        
        // Derive key using Argon2
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt_bytes)
            .map_err(|e| KeystoreError::Encryption(format!("Key derivation failed: {}", e)))?;
        
        // Extract the hash output as our key
        let hash_bytes = password_hash.hash.unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
        
        Ok(key)
    }
    
    /// Encrypt key data
    fn encrypt_key(key_data: &[u8], password: &str) -> Result<KeystoreFile, KeystoreError> {
        // Generate salt
        let salt = SaltString::generate(&mut OsRng);
        
        // Derive encryption key
        let key = Self::derive_key(password, salt.as_str())?;
        let key = Key::<Aes256Gcm>::from_slice(&key);
        
        // Generate nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let cipher = Aes256Gcm::new(key);
        let ciphertext = cipher
            .encrypt(nonce, key_data)
            .map_err(|e| KeystoreError::Encryption(format!("Encryption failed: {}", e)))?;
        
        Ok(KeystoreFile {
            version: 1,
            chain_id: String::new(), // Will be set by caller
            key_type: String::new(), // Will be set by caller
            ciphertext,
            nonce: nonce_bytes.to_vec(),
            salt: salt.to_string(),
            kdf: KdfParams::default(),
        })
    }
    
    /// Decrypt key data
    fn decrypt_key(keystore: &KeystoreFile, password: &str) -> Result<Vec<u8>, KeystoreError> {
        // Derive decryption key
        let key = Self::derive_key(password, &keystore.salt)?;
        let key = Key::<Aes256Gcm>::from_slice(&key);
        
        // Verify nonce length
        if keystore.nonce.len() != 12 {
            return Err(KeystoreError::CorruptedFile);
        }
        let nonce = Nonce::from_slice(&keystore.nonce);
        
        // Decrypt
        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher
            .decrypt(nonce, keystore.ciphertext.as_ref())
            .map_err(|_| KeystoreError::InvalidPassword)?;
        
        Ok(plaintext)
    }
}

#[async_trait]
impl KeyStorage for EncryptedKeystore {
    async fn load_key(&self, chain_id: &str) -> Result<KeyEntry, KeyError> {
        let path = self.get_keystore_path(chain_id);
        
        // Read keystore file
        let data = tokio::fs::read_to_string(&path)
            .await
            .map_err(|_| KeyError::NotFound(chain_id.to_string()))?;
        
        let keystore: KeystoreFile = serde_json::from_str(&data)
            .map_err(|e| KeyError::Keystore(KeystoreError::Serialization(e.to_string())))?;
        
        // For loading, we need the password - this would typically come from a prompt
        // For now, return an error indicating password is needed
        Err(KeyError::Keystore(KeystoreError::InvalidPassword))
    }
    
    async fn store_key(&self, chain_id: &str, key: &KeyEntry, password: &str) -> Result<(), KeyError> {
        // Serialize key entry
        let key_data = serde_json::to_vec(key)
            .map_err(|e| KeyError::Serialization(e.to_string()))?;
        
        // Encrypt
        let mut keystore = Self::encrypt_key(&key_data, password)?;
        keystore.chain_id = chain_id.to_string();
        keystore.key_type = match key {
            KeyEntry::Near(_) => "near",
            KeyEntry::Cosmos(_) => "cosmos",
        }.to_string();
        
        // Write to file
        let path = self.get_keystore_path(chain_id);
        let data = serde_json::to_string_pretty(&keystore)
            .map_err(|e| KeyError::Serialization(e.to_string()))?;
        
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| KeyError::Io(e))?;
        
        Ok(())
    }
    
    async fn remove_key(&self, chain_id: &str) -> Result<(), KeyError> {
        let path = self.get_keystore_path(chain_id);
        tokio::fs::remove_file(&path)
            .await
            .map_err(|_| KeyError::NotFound(chain_id.to_string()))?;
        Ok(())
    }
    
    async fn list_keys(&self) -> Result<Vec<String>, KeyError> {
        let mut keys = Vec::new();
        
        let mut entries = tokio::fs::read_dir(&self.keystore_dir)
            .await
            .map_err(|e| KeyError::Io(e))?;
        
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    keys.push(name.trim_end_matches(".json").to_string());
                }
            }
        }
        
        Ok(keys)
    }
}

/// In-memory storage for testing
pub struct MemoryKeyStorage {
    keys: std::sync::Mutex<std::collections::HashMap<String, (KeyEntry, String)>>,
}

impl MemoryKeyStorage {
    pub fn new() -> Self {
        Self {
            keys: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl KeyStorage for MemoryKeyStorage {
    async fn load_key(&self, chain_id: &str) -> Result<KeyEntry, KeyError> {
        let keys = self.keys.lock().unwrap();
        keys.get(chain_id)
            .map(|(key, _)| key.clone())
            .ok_or_else(|| KeyError::NotFound(chain_id.to_string()))
    }
    
    async fn store_key(&self, chain_id: &str, key: &KeyEntry, password: &str) -> Result<(), KeyError> {
        let mut keys = self.keys.lock().unwrap();
        keys.insert(chain_id.to_string(), (key.clone(), password.to_string()));
        Ok(())
    }
    
    async fn remove_key(&self, chain_id: &str) -> Result<(), KeyError> {
        let mut keys = self.keys.lock().unwrap();
        keys.remove(chain_id)
            .ok_or_else(|| KeyError::NotFound(chain_id.to_string()))?;
        Ok(())
    }
    
    async fn list_keys(&self) -> Result<Vec<String>, KeyError> {
        let keys = self.keys.lock().unwrap();
        Ok(keys.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_encrypted_keystore() {
        let temp_dir = tempdir().unwrap();
        let keystore = EncryptedKeystore::new(temp_dir.path()).unwrap();
        
        // Create a test key
        let cosmos_key = CosmosKey {
            address: "cosmos1test".to_string(),
            private_key: vec![1, 2, 3, 4],
            public_key: vec![5, 6, 7, 8],
            key_type: "secp256k1".to_string(),
        };
        let key_entry = KeyEntry::Cosmos(cosmos_key);
        
        // Store the key
        keystore.store_key("testnet", &key_entry, "test_password").await.unwrap();
        
        // List keys
        let keys = keystore.list_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "testnet");
        
        // Remove key
        keystore.remove_key("testnet").await.unwrap();
        let keys = keystore.list_keys().await.unwrap();
        assert_eq!(keys.len(), 0);
    }
    
    #[test]
    fn test_encryption_decryption() {
        let data = b"test key data";
        let password = "secure_password";
        
        // Encrypt
        let mut keystore_file = EncryptedKeystore::encrypt_key(data, password).unwrap();
        keystore_file.chain_id = "test".to_string();
        keystore_file.key_type = "test".to_string();
        
        // Decrypt
        let decrypted = EncryptedKeystore::decrypt_key(&keystore_file, password).unwrap();
        assert_eq!(decrypted, data);
        
        // Wrong password should fail
        let result = EncryptedKeystore::decrypt_key(&keystore_file, "wrong_password");
        assert!(result.is_err());
    }
}