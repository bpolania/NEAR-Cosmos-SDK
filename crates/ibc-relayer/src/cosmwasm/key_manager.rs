/// Key Manager for CosmWasm Relayer
/// 
/// This module handles secure key management for the relayer service.
/// In production, keys should be stored in a secure key management system
/// like AWS KMS, HashiCorp Vault, or hardware security modules.

use near_crypto::{SecretKey, PublicKey};
use near_primitives::types::AccountId;
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

/// Manages private keys for the relayer
pub struct RelayerKeyManager {
    /// The account ID of the relayer
    pub account_id: AccountId,
    /// The public key of the relayer
    pub public_key: PublicKey,
    /// The secret key (should be encrypted in production)
    secret_key: SecretKey,
}

impl RelayerKeyManager {
    /// Load key from a file (for development/testing)
    pub fn from_file<P: AsRef<Path>>(
        account_id: AccountId,
        key_file: P,
    ) -> Result<Self> {
        let key_data = fs::read_to_string(key_file)
            .context("Failed to read key file")?;
        
        let secret_key: SecretKey = key_data.trim().parse()
            .context("Failed to parse secret key")?;
        
        let public_key = secret_key.public_key();
        
        Ok(Self {
            account_id,
            public_key,
            secret_key,
        })
    }
    
    /// Load key from environment variable
    pub fn from_env(
        account_id: AccountId,
        env_var: &str,
    ) -> Result<Self> {
        let key_data = std::env::var(env_var)
            .context(format!("Failed to read key from env var {}", env_var))?;
        
        let secret_key: SecretKey = key_data.parse()
            .context("Failed to parse secret key from env")?;
        
        let public_key = secret_key.public_key();
        
        Ok(Self {
            account_id,
            public_key,
            secret_key,
        })
    }
    
    /// Create from raw key string (least secure, for testing only)
    pub fn from_string(
        account_id: AccountId,
        key_string: &str,
    ) -> Result<Self> {
        let secret_key: SecretKey = key_string.parse()
            .context("Failed to parse secret key")?;
        
        let public_key = secret_key.public_key();
        
        Ok(Self {
            account_id,
            public_key,
            secret_key,
        })
    }
    
    /// Get the secret key (use with caution)
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }
    
    /// Clone the secret key (use with extreme caution)
    pub fn clone_secret_key(&self) -> SecretKey {
        self.secret_key.clone()
    }
}

/// Configuration for key management
#[derive(Debug, Clone)]
pub enum KeySource {
    /// Load from file
    File(String),
    /// Load from environment variable
    Environment(String),
    /// Raw key string (testing only)
    Raw(String),
}

impl KeySource {
    /// Load the key from the configured source
    pub fn load(&self, account_id: AccountId) -> Result<RelayerKeyManager> {
        match self {
            KeySource::File(path) => RelayerKeyManager::from_file(account_id, path),
            KeySource::Environment(var) => RelayerKeyManager::from_env(account_id, var),
            KeySource::Raw(key) => RelayerKeyManager::from_string(account_id, key),
        }
    }
}