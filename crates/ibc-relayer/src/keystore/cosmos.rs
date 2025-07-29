// Cosmos key management implementation
use super::KeyError;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Cosmos account key with secp256k1 cryptography
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosKey {
    /// Bech32 address (e.g., cosmos1...)
    pub address: String,
    /// Private key bytes (32 bytes for secp256k1)
    pub private_key: Vec<u8>,
    /// Public key bytes (33 bytes compressed)
    pub public_key: Vec<u8>,
    /// Key type (always "secp256k1" for Cosmos)
    pub key_type: String,
}

impl CosmosKey {
    /// Create a new Cosmos key from private key bytes
    pub fn from_private_key(private_key: Vec<u8>, address_prefix: &str) -> Result<Self, KeyError> {
        if private_key.len() != 32 {
            return Err(KeyError::InvalidFormat(
                "Private key must be 32 bytes".to_string()
            ));
        }
        
        // Derive public key
        let public_key = Self::derive_public_key(&private_key)?;
        
        // Derive address
        let address = Self::derive_address(&public_key, address_prefix)?;
        
        Ok(Self {
            address,
            private_key,
            public_key,
            key_type: "secp256k1".to_string(),
        })
    }
    
    /// Create from environment variable string format
    /// Format: "cosmos1...:hexPrivateKey" or just "hexPrivateKey"
    pub fn from_env_string(env_str: &str) -> Result<Self, KeyError> {
        let parts: Vec<&str> = env_str.split(':').collect();
        
        let (address, hex_key) = if parts.len() == 2 {
            // Format: address:privateKey
            (Some(parts[0].to_string()), parts[1])
        } else if parts.len() == 1 {
            // Format: just privateKey (derive address)
            (None, parts[0])
        } else {
            return Err(KeyError::InvalidFormat(
                "Expected format: 'address:privateKey' or 'privateKey'".to_string()
            ));
        };
        
        // Decode hex private key
        let private_key = hex::decode(hex_key)
            .map_err(|e| KeyError::InvalidFormat(format!("Invalid hex key: {}", e)))?;
        
        if private_key.len() != 32 {
            return Err(KeyError::InvalidFormat(
                "Private key must be 32 bytes".to_string()
            ));
        }
        
        // Derive public key
        let public_key = Self::derive_public_key(&private_key)?;
        
        // Use provided address or derive it
        let final_address = if let Some(addr) = address {
            addr
        } else {
            // Default to cosmos prefix if not provided
            Self::derive_address(&public_key, "cosmos")?
        };
        
        Ok(Self {
            address: final_address,
            private_key,
            public_key,
            key_type: "secp256k1".to_string(),
        })
    }
    
    /// Export key as string (for display/backup)
    pub fn to_export_string(&self) -> String {
        format!("{}:{}", self.address, hex::encode(&self.private_key))
    }
    
    /// Get private key as hex string
    pub fn private_key_hex(&self) -> String {
        hex::encode(&self.private_key)
    }
    
    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(&self.public_key)
    }
    
    /// Derive public key from private key using secp256k1
    fn derive_public_key(private_key: &[u8]) -> Result<Vec<u8>, KeyError> {
        let secp = secp256k1::Secp256k1::new();
        let secret_key = secp256k1::SecretKey::from_slice(private_key)
            .map_err(|e| KeyError::Crypto(format!("Invalid private key: {}", e)))?;
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        
        // Return compressed public key (33 bytes)
        Ok(public_key.serialize().to_vec())
    }
    
    /// Derive Cosmos address from public key (simplified version)
    fn derive_address(public_key: &[u8], prefix: &str) -> Result<String, KeyError> {
        // Hash the public key with SHA256
        let sha_hash = Sha256::digest(public_key);
        
        // Take first 20 bytes for the address
        let addr_bytes = &sha_hash[..20];
        
        // For now, create a simplified bech32-style address with proper prefix format
        // In production, this would use proper RIPEMD160 and bech32 encoding
        let hex_addr = hex::encode(addr_bytes);
        Ok(format!("{}1{}", prefix, &hex_addr[..38])) // cosmos1... format
    }
    
    /// Validate the key structure
    pub fn validate(&self) -> Result<(), KeyError> {
        // Check private key length
        if self.private_key.len() != 32 {
            return Err(KeyError::InvalidFormat(
                "Private key must be 32 bytes".to_string()
            ));
        }
        
        // Check public key length
        if self.public_key.len() != 33 {
            return Err(KeyError::InvalidFormat(
                "Public key must be 33 bytes (compressed)".to_string()
            ));
        }
        
        // Verify public key derivation matches
        let derived_pubkey = Self::derive_public_key(&self.private_key)?;
        if derived_pubkey != self.public_key {
            return Err(KeyError::InvalidFormat(
                "Public key does not match private key".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosmos_key_from_private_key() {
        let private_key = hex::decode(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        ).unwrap();
        
        let key = CosmosKey::from_private_key(private_key.clone(), "cosmos").unwrap();
        
        assert_eq!(key.private_key, private_key);
        assert_eq!(key.public_key.len(), 33);
        assert!(key.address.starts_with("cosmos1"));
        assert_eq!(key.key_type, "secp256k1");
    }
    
    #[test]
    fn test_cosmos_key_from_env_string() {
        // Test with address:privateKey format
        let env_str = "cosmos1test:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let key = CosmosKey::from_env_string(env_str).unwrap();
        assert_eq!(key.address, "cosmos1test");
        
        // Test with just privateKey format
        let env_str = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let key = CosmosKey::from_env_string(env_str).unwrap();
        assert!(key.address.starts_with("cosmos1"));
    }
    
    #[test]
    fn test_cosmos_key_validation() {
        let mut key = CosmosKey {
            address: "cosmos1test".to_string(),
            private_key: vec![1; 32],
            public_key: vec![2; 33],
            key_type: "secp256k1".to_string(),
        };
        
        // Should fail validation due to mismatched keys
        assert!(key.validate().is_err());
        
        // Create valid key
        let private_key = hex::decode(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        ).unwrap();
        key = CosmosKey::from_private_key(private_key, "cosmos").unwrap();
        
        // Should pass validation
        assert!(key.validate().is_ok());
    }
}