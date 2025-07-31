// Comprehensive unit tests for CosmosKey implementation
use ibc_relayer::keystore::cosmos::CosmosKey;
use ibc_relayer::keystore::KeyError;

#[cfg(test)]
mod cosmos_key_tests {
    use super::*;

    const TEST_PRIVATE_KEY: &str = "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
    
    #[test]
    fn test_cosmos_key_creation_from_private_key() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        
        let result = CosmosKey::from_private_key(private_key_bytes.clone(), "cosmos");
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.private_key, private_key_bytes);
        assert_eq!(key.private_key.len(), 32);
        assert_eq!(key.public_key.len(), 33);
        assert_eq!(key.key_type, "secp256k1");
        assert!(key.address.starts_with("cosmos"));
    }

    #[test]
    fn test_cosmos_key_invalid_private_key_length() {
        // Test with wrong length private key
        let short_key = vec![1, 2, 3]; // Too short
        let result = CosmosKey::from_private_key(short_key, "cosmos");
        assert!(result.is_err());
        
        if let Err(KeyError::InvalidFormat(msg)) = result {
            assert!(msg.contains("32 bytes"));
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_cosmos_key_from_env_string_with_address() {
        let env_str = format!("cosmos1test:{}", TEST_PRIVATE_KEY);
        let result = CosmosKey::from_env_string(&env_str);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.address, "cosmos1test");
        assert_eq!(key.private_key, hex::decode(TEST_PRIVATE_KEY).unwrap());
    }

    #[test]
    fn test_cosmos_key_from_env_string_private_key_only() {
        let result = CosmosKey::from_env_string(TEST_PRIVATE_KEY);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert!(key.address.starts_with("cosmos"));
        assert_eq!(key.private_key, hex::decode(TEST_PRIVATE_KEY).unwrap());
    }

    #[test]
    fn test_cosmos_key_from_env_string_invalid_format() {
        let invalid_formats = vec![
            "",
            "a:b:c:d", // Too many parts
            "cosmos1test:invalidhex",
            "cosmos1test:short",
        ];

        for invalid in invalid_formats {
            let result = CosmosKey::from_env_string(invalid);
            assert!(result.is_err(), "Should fail for: {}", invalid);
        }
    }

    #[test]
    fn test_cosmos_key_hex_methods() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        let key = CosmosKey::from_private_key(private_key_bytes, "cosmos").unwrap();
        
        assert_eq!(key.private_key_hex(), TEST_PRIVATE_KEY);
        assert!(!key.public_key_hex().is_empty());
        assert_eq!(key.public_key_hex().len(), 66); // 33 bytes * 2 hex chars
    }

    #[test]
    fn test_cosmos_key_export_string() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        let key = CosmosKey::from_private_key(private_key_bytes, "cosmos").unwrap();
        
        let export_str = key.to_export_string();
        assert!(export_str.contains(&key.address));
        assert!(export_str.contains(TEST_PRIVATE_KEY));
        
        // Should be in format "address:private_key"
        let parts: Vec<&str> = export_str.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], key.address);
        assert_eq!(parts[1], TEST_PRIVATE_KEY);
    }

    #[test]
    fn test_cosmos_key_validation_success() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        let key = CosmosKey::from_private_key(private_key_bytes, "cosmos").unwrap();
        
        assert!(key.validate().is_ok());
    }

    #[test]
    fn test_cosmos_key_validation_failure() {
        // Create an invalid key manually
        let mut key = CosmosKey {
            address: "cosmos1test".to_string(),
            private_key: vec![1; 31], // Wrong length
            public_key: vec![2; 33],
            key_type: "secp256k1".to_string(),
        };
        
        // Should fail due to wrong private key length
        assert!(key.validate().is_err());
        
        // Fix length but make keys mismatched
        key.private_key = vec![1; 32];
        assert!(key.validate().is_err()); // Should fail due to key mismatch
    }

    #[test]
    fn test_cosmos_key_different_prefixes() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        
        let cosmos_key = CosmosKey::from_private_key(private_key_bytes.clone(), "cosmos").unwrap();
        let osmosis_key = CosmosKey::from_private_key(private_key_bytes.clone(), "osmo").unwrap();
        
        assert!(cosmos_key.address.starts_with("cosmos"));
        assert!(osmosis_key.address.starts_with("osmo"));
        
        // Same private key should produce same public key
        assert_eq!(cosmos_key.public_key, osmosis_key.public_key);
        assert_eq!(cosmos_key.private_key, osmosis_key.private_key);
    }

    #[test]
    fn test_cosmos_key_deterministic() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        
        // Create the same key twice
        let key1 = CosmosKey::from_private_key(private_key_bytes.clone(), "cosmos").unwrap();
        let key2 = CosmosKey::from_private_key(private_key_bytes.clone(), "cosmos").unwrap();
        
        // Should be identical
        assert_eq!(key1.address, key2.address);
        assert_eq!(key1.private_key, key2.private_key);
        assert_eq!(key1.public_key, key2.public_key);
    }

    #[test]
    fn test_cosmos_key_public_key_derivation() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        let key = CosmosKey::from_private_key(private_key_bytes, "cosmos").unwrap();
        
        // Public key should be 33 bytes (compressed secp256k1)
        assert_eq!(key.public_key.len(), 33);
        
        // First byte should be 0x02 or 0x03 (compressed public key format)
        let first_byte = key.public_key[0];
        assert!(first_byte == 0x02 || first_byte == 0x03);
    }

    #[test]
    fn test_cosmos_key_round_trip() {
        let private_key_bytes = hex::decode(TEST_PRIVATE_KEY).unwrap();
        let original_key = CosmosKey::from_private_key(private_key_bytes, "cosmos").unwrap();
        
        // Export and re-import
        let export_str = original_key.to_export_string();
        let imported_key = CosmosKey::from_env_string(&export_str).unwrap();
        
        // Should be identical
        assert_eq!(original_key.address, imported_key.address);
        assert_eq!(original_key.private_key, imported_key.private_key);
        assert_eq!(original_key.public_key, imported_key.public_key);
    }
}