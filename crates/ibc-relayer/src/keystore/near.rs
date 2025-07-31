// NEAR key management implementation
use super::KeyError;
use near_crypto::{PublicKey, SecretKey, KeyType};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// NEAR account key with ed25519 cryptography
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearKey {
    /// NEAR account ID (e.g., account.near)
    pub account_id: String,
    /// Secret key in NEAR format
    pub secret_key: String,
    /// Public key in NEAR format
    pub public_key: String,
    /// Key type (usually "ed25519")
    pub key_type: String,
}

impl NearKey {
    /// Create a new NEAR key from secret key string
    pub fn from_secret_key(account_id: String, secret_key_str: &str) -> Result<Self, KeyError> {
        // For now, store the raw key string and derive type from format
        let key_type = if secret_key_str.starts_with("ed25519:") {
            "ed25519"
        } else if secret_key_str.starts_with("secp256k1:") {
            "secp256k1"  
        } else {
            "ed25519" // Default
        }.to_string();
        
        // For now, we'll store the keys as strings and parse them when needed
        // This avoids the immediate parsing issues with NEAR crypto
        Ok(Self {
            account_id,
            secret_key: secret_key_str.to_string(),
            public_key: format!("{}:placeholder_public", key_type), // Placeholder
            key_type,
        })
    }
    
    /// Create from private key bytes (placeholder implementation)
    pub fn from_private_key_bytes(
        account_id: String,
        private_key: &[u8],
        key_type: KeyType,
    ) -> Result<Self, KeyError> {
        // For now, create a placeholder key
        let key_type_str = match key_type {
            KeyType::ED25519 => "ed25519",
            KeyType::SECP256K1 => "secp256k1",
        };
        
        let secret_key = format!("{}:{}", key_type_str, hex::encode(private_key));
        Self::from_secret_key(account_id, &secret_key)
    }
    
    /// Create from environment variable string format
    /// Format: "account.near:ed25519:base58key" or "account.near:base58key"
    pub fn from_env_string(env_str: &str) -> Result<Self, KeyError> {
        let parts: Vec<&str> = env_str.split(':').collect();
        
        match parts.len() {
            2 => {
                // Format: account:secretKey
                let account_id = parts[0].to_string();
                let secret_key_str = parts[1];
                
                // Try to parse with ed25519: prefix
                let full_key = if secret_key_str.starts_with("ed25519:") || 
                               secret_key_str.starts_with("secp256k1:") {
                    secret_key_str.to_string()
                } else {
                    format!("ed25519:{}", secret_key_str)
                };
                
                Self::from_secret_key(account_id, &full_key)
            }
            3 => {
                // Format: account:keyType:secretKey
                let account_id = parts[0].to_string();
                let key_type = parts[1];
                let secret_key_data = parts[2];
                
                let full_key = format!("{}:{}", key_type, secret_key_data);
                Self::from_secret_key(account_id, &full_key)
            }
            _ => Err(KeyError::InvalidFormat(
                "Expected format: 'account:secretKey' or 'account:keyType:secretKey'".to_string()
            ))
        }
    }
    
    /// Export key as string (for display/backup)
    pub fn to_export_string(&self) -> String {
        format!("{}:{}", self.account_id, self.secret_key)
    }
    
    /// Get the parsed secret key
    pub fn get_secret_key(&self) -> Result<SecretKey, KeyError> {
        // For now, return an error - actual parsing will be implemented later
        Err(KeyError::Crypto("NEAR key parsing not yet implemented".to_string()))
    }
    
    /// Get the parsed public key
    pub fn get_public_key(&self) -> Result<PublicKey, KeyError> {
        // For now, return an error - actual parsing will be implemented later
        Err(KeyError::Crypto("NEAR key parsing not yet implemented".to_string()))
    }
    
    /// Validate the key structure
    pub fn validate(&self) -> Result<(), KeyError> {
        // Basic validation for now
        if self.account_id.is_empty() {
            return Err(KeyError::InvalidFormat(
                "Account ID cannot be empty".to_string()
            ));
        }
        
        if self.secret_key.is_empty() {
            return Err(KeyError::InvalidFormat(
                "Secret key cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Create access key for NEAR transactions (placeholder)
    pub fn create_access_key(&self) -> Result<String, KeyError> {
        // For now, return the public key string
        Ok(self.public_key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_near_key_from_secret_key() {
        let secret_key_str = "ed25519:5JueXZhECnDeY6cDT3gKmFKKJt8k9J7EN4kBx6tB3kP6y6g4yKUXG5Kv7t2gUBqGPgUnfunPZfz8QMwGxdPVUAmj";
        let key = NearKey::from_secret_key("test.near".to_string(), secret_key_str).unwrap();
        
        assert_eq!(key.account_id, "test.near");
        assert_eq!(key.key_type, "ed25519");
        assert!(!key.public_key.is_empty());
    }
    
    #[test]
    fn test_near_key_from_env_string() {
        // Test format: account:secretKey
        let env_str = "test.near:ed25519:5JueXZhECnDeY6cDT3gKmFKKJt8k9J7EN4kBx6tB3kP6y6g4yKUXG5Kv7t2gUBqGPgUnfunPZfz8QMwGxdPVUAmj";
        let key = NearKey::from_env_string(env_str).unwrap();
        assert_eq!(key.account_id, "test.near");
        assert_eq!(key.key_type, "ed25519");
        
        // Test format: account:keyType:secretKey
        let env_str = "test.near:ed25519:5JueXZhECnDeY6cDT3gKmFKKJt8k9J7EN4kBx6tB3kP6y6g4yKUXG5Kv7t2gUBqGPgUnfunPZfz8QMwGxdPVUAmj";
        let key = NearKey::from_env_string(env_str).unwrap();
        assert_eq!(key.account_id, "test.near");
        assert_eq!(key.key_type, "ed25519");
    }
    
    #[test]
    fn test_near_key_validation() {
        let secret_key_str = "ed25519:5JueXZhECnDeY6cDT3gKmFKKJt8k9J7EN4kBx6tB3kP6y6g4yKUXG5Kv7t2gUBqGPgUnfunPZfz8QMwGxdPVUAmj";
        let key = NearKey::from_secret_key("test.near".to_string(), secret_key_str).unwrap();
        
        // Should pass validation
        assert!(key.validate().is_ok());
        
        // Test with empty account ID
        let mut invalid_key = key.clone();
        invalid_key.account_id = String::new();
        assert!(invalid_key.validate().is_err());
    }
}