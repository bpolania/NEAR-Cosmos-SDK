// Comprehensive unit tests for NearKey implementation
use ibc_relayer::keystore::near::NearKey;
use ibc_relayer::keystore::KeyError;
use near_crypto::KeyType;

#[cfg(test)]
mod near_key_tests {
    use super::*;

    const TEST_SECRET_KEY: &str = "ed25519:3D4YudUahN1Hls8rPkMhBsNEHNa4hqD4sCjxNKBLSanT65Zqd8SJRqWWr4HdqwrmK8VfuuZNxHwdKVMXJXKK4pAX";
    const TEST_ACCOUNT_ID: &str = "test.near";

    #[test]
    fn test_near_key_from_secret_key() {
        let result = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.account_id, TEST_ACCOUNT_ID);
        assert_eq!(key.secret_key, TEST_SECRET_KEY);
        assert_eq!(key.key_type, "ed25519");
        assert!(key.public_key.starts_with("ed25519:"));
    }

    #[test]
    fn test_near_key_from_secret_key_secp256k1() {
        let secp_secret = "secp256k1:test_secret_key";
        let result = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), secp_secret);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.key_type, "secp256k1");
        assert_eq!(key.secret_key, secp_secret);
    }

    #[test]
    fn test_near_key_from_secret_key_default_type() {
        let plain_secret = "test_secret_without_prefix";
        let result = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), plain_secret);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.key_type, "ed25519"); // Should default to ed25519
        assert_eq!(key.secret_key, plain_secret);
    }

    #[test]
    fn test_near_key_from_private_key_bytes_ed25519() {
        let private_key_bytes = vec![1; 32]; // Mock 32-byte key
        let result = NearKey::from_private_key_bytes(
            TEST_ACCOUNT_ID.to_string(),
            &private_key_bytes,
            KeyType::ED25519,
        );
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.account_id, TEST_ACCOUNT_ID);
        assert!(key.secret_key.starts_with("ed25519:"));
        assert!(key.secret_key.contains(&hex::encode(&private_key_bytes)));
    }

    #[test]
    fn test_near_key_from_private_key_bytes_secp256k1() {
        let private_key_bytes = vec![2; 32]; // Mock 32-byte key
        let result = NearKey::from_private_key_bytes(
            TEST_ACCOUNT_ID.to_string(),
            &private_key_bytes,
            KeyType::SECP256K1,
        );
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.account_id, TEST_ACCOUNT_ID);
        assert!(key.secret_key.starts_with("secp256k1:"));
        assert!(key.secret_key.contains(&hex::encode(&private_key_bytes)));
    }

    #[test]
    fn test_near_key_from_env_string_two_parts() {
        let env_str = format!("{}:{}", TEST_ACCOUNT_ID, TEST_SECRET_KEY);
        let result = NearKey::from_env_string(&env_str);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.account_id, TEST_ACCOUNT_ID);
        assert_eq!(key.secret_key, TEST_SECRET_KEY);
        assert_eq!(key.key_type, "ed25519");
    }

    #[test]
    fn test_near_key_from_env_string_three_parts() {
        let env_str = format!("{}:ed25519:test_key_data", TEST_ACCOUNT_ID);
        let result = NearKey::from_env_string(&env_str);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.account_id, TEST_ACCOUNT_ID);
        assert_eq!(key.secret_key, "ed25519:test_key_data");
        assert_eq!(key.key_type, "ed25519");
    }

    #[test]
    fn test_near_key_from_env_string_plain_key_gets_prefix() {
        let env_str = format!("{}:plain_key_without_prefix", TEST_ACCOUNT_ID);
        let result = NearKey::from_env_string(&env_str);
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.secret_key, "ed25519:plain_key_without_prefix");
    }

    #[test]
    fn test_near_key_from_env_string_invalid_format() {
        let invalid_formats = vec![
            "",
            "single_part",
            "too:many:parts:here:invalid",
        ];

        for invalid in invalid_formats {
            let result = NearKey::from_env_string(invalid);
            assert!(result.is_err(), "Should fail for: {}", invalid);
            
            if let Err(KeyError::InvalidFormat(msg)) = result {
                assert!(msg.contains("Expected format"));
            } else {
                panic!("Expected InvalidFormat error for: {}", invalid);
            }
        }
    }

    #[test]
    fn test_near_key_to_export_string() {
        let key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        let export_str = key.to_export_string();
        
        assert_eq!(export_str, format!("{}:{}", TEST_ACCOUNT_ID, TEST_SECRET_KEY));
        
        // Should be able to round-trip
        let imported_key = NearKey::from_env_string(&export_str).unwrap();
        assert_eq!(key.account_id, imported_key.account_id);
        assert_eq!(key.secret_key, imported_key.secret_key);
    }

    #[test]
    fn test_near_key_validation_success() {
        let key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        assert!(key.validate().is_ok());
    }

    #[test]
    fn test_near_key_validation_empty_account_id() {
        let mut key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        key.account_id = String::new();
        
        let result = key.validate();
        assert!(result.is_err());
        
        if let Err(KeyError::InvalidFormat(msg)) = result {
            assert!(msg.contains("Account ID cannot be empty"));
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_near_key_validation_empty_secret_key() {
        let mut key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        key.secret_key = String::new();
        
        let result = key.validate();
        assert!(result.is_err());
        
        if let Err(KeyError::InvalidFormat(msg)) = result {
            assert!(msg.contains("Secret key cannot be empty"));
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_near_key_get_secret_key_placeholder() {
        let key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        let result = key.get_secret_key();
        
        // Should return error as it's not implemented yet
        assert!(result.is_err());
        
        if let Err(KeyError::Crypto(msg)) = result {
            assert!(msg.contains("not yet implemented"));
        } else {
            panic!("Expected Crypto error");
        }
    }

    #[test]
    fn test_near_key_get_public_key_placeholder() {
        let key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        let result = key.get_public_key();
        
        // Should return error as it's not implemented yet
        assert!(result.is_err());
        
        if let Err(KeyError::Crypto(msg)) = result {
            assert!(msg.contains("not yet implemented"));
        } else {
            panic!("Expected Crypto error");
        }
    }

    #[test]
    fn test_near_key_create_access_key() {
        let key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        let result = key.create_access_key();
        
        assert!(result.is_ok());
        let access_key = result.unwrap();
        assert_eq!(access_key, key.public_key);
    }

    #[test]
    fn test_near_key_round_trip() {
        let original_key = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        
        // Export and re-import
        let export_str = original_key.to_export_string();
        let imported_key = NearKey::from_env_string(&export_str).unwrap();
        
        // Should be identical
        assert_eq!(original_key.account_id, imported_key.account_id);
        assert_eq!(original_key.secret_key, imported_key.secret_key);
        assert_eq!(original_key.key_type, imported_key.key_type);
    }

    #[test]
    fn test_near_key_different_account_ids() {
        let accounts = vec!["alice.near", "bob.testnet", "contract.alice.near"];
        
        for account in accounts {
            let key = NearKey::from_secret_key(account.to_string(), TEST_SECRET_KEY).unwrap();
            assert_eq!(key.account_id, account);
            assert!(key.validate().is_ok());
        }
    }

    #[test]
    fn test_near_key_deterministic() {
        // Same inputs should produce same results
        let key1 = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        let key2 = NearKey::from_secret_key(TEST_ACCOUNT_ID.to_string(), TEST_SECRET_KEY).unwrap();
        
        assert_eq!(key1.account_id, key2.account_id);
        assert_eq!(key1.secret_key, key2.secret_key);
        assert_eq!(key1.public_key, key2.public_key);
        assert_eq!(key1.key_type, key2.key_type);
    }
}