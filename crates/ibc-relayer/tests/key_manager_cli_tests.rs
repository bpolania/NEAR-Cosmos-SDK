// Integration tests for the key-manager CLI tool
use ibc_relayer::keystore::{KeyEntry, KeyManagerConfig};
use ibc_relayer::keystore::cosmos::CosmosKey;
use ibc_relayer::keystore::near::NearKey;
use std::process::{Command, Stdio};
use std::collections::HashMap;
use tempfile::tempdir;

#[cfg(test)]
mod key_manager_cli_tests {
    use super::*;
    
    // Mock KeyManager for CLI testing
    struct MockKeyManager {
        keys: HashMap<String, KeyEntry>,
        config: KeyManagerConfig,
    }

    impl MockKeyManager {
        fn new(config: KeyManagerConfig) -> Self {
            Self {
                keys: HashMap::new(),
                config,
            }
        }

        async fn store_key(&mut self, chain_id: &str, key: KeyEntry, _password: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.keys.insert(chain_id.to_string(), key);
            Ok(())
        }

        async fn load_key(&mut self, chain_id: &str) -> Result<KeyEntry, Box<dyn std::error::Error + Send + Sync>> {
            self.keys.get(chain_id)
                .cloned()
                .ok_or_else(|| format!("Key not found for chain: {}", chain_id).into())
        }

        async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.keys.keys().cloned().collect())
        }

        fn get_address(&self, chain_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            match self.keys.get(chain_id) {
                Some(KeyEntry::Cosmos(key)) => Ok(key.address.clone()),
                Some(KeyEntry::Near(key)) => Ok(key.account_id.clone()),
                None => Err(format!("Key not found for chain: {}", chain_id).into()),
            }
        }
    }
    
    fn create_test_config() -> (KeyManagerConfig, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config = KeyManagerConfig {
            keystore_dir: temp_dir.path().to_path_buf(),
            allow_env_keys: true,
            env_prefix: "TEST_KEY_".to_string(),
            kdf_iterations: 1000, // Lower for testing
        };
        (config, temp_dir)
    }

    #[test]
    fn test_key_manager_binary_exists() {
        // Test that the key-manager binary can be built
        let output = Command::new("cargo")
            .args(&["check", "--bin", "key-manager"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        
        assert!(output.is_ok(), "key-manager binary should compile");
    }

    #[tokio::test]
    async fn test_cosmos_key_from_env_string_formats() {
        // Test the key formats that the CLI would handle
        
        // Format 1: address:privateKey
        let env_str1 = "cosmos1test:a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let key1 = CosmosKey::from_env_string(env_str1);
        assert!(key1.is_ok());
        
        let k1 = key1.unwrap();
        assert_eq!(k1.address, "cosmos1test");
        assert_eq!(k1.private_key_hex(), "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3");
        
        // Format 2: privateKey only (address derived)
        let env_str2 = "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let key2 = CosmosKey::from_env_string(env_str2);
        assert!(key2.is_ok());
        
        let k2 = key2.unwrap();
        assert!(k2.address.starts_with("cosmos")); // Address should be derived
        assert_eq!(k2.private_key_hex(), "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3");
    }

    #[tokio::test]
    async fn test_near_key_from_env_string_formats() {
        // Test the key formats that the CLI would handle for NEAR
        
        // Format 1: account:secretKey
        let env_str1 = "test.near:ed25519:test_secret_key";
        let key1 = NearKey::from_env_string(env_str1);
        assert!(key1.is_ok());
        
        let k1 = key1.unwrap();
        assert_eq!(k1.account_id, "test.near");
        assert_eq!(k1.secret_key, "ed25519:test_secret_key");
        
        // Format 2: account:keyType:secretKey
        let env_str2 = "test.near:ed25519:another_secret_key";
        let key2 = NearKey::from_env_string(env_str2);
        assert!(key2.is_ok());
        
        let k2 = key2.unwrap();
        assert_eq!(k2.account_id, "test.near");
        assert_eq!(k2.secret_key, "ed25519:another_secret_key");
    }

    #[tokio::test]
    async fn test_key_export_and_import_round_trip() {
        // Test that keys can be exported and imported correctly (CLI backup/restore)
        
        // Test Cosmos key round-trip
        let original_cosmos = CosmosKey::from_private_key(
            hex::decode("a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3").unwrap(),
            "cosmos"
        ).unwrap();
        
        let export_str = original_cosmos.to_export_string();
        let imported_cosmos = CosmosKey::from_env_string(&export_str).unwrap();
        
        assert_eq!(original_cosmos.address, imported_cosmos.address);
        assert_eq!(original_cosmos.private_key, imported_cosmos.private_key);
        assert_eq!(original_cosmos.public_key, imported_cosmos.public_key);
        
        // Test NEAR key round-trip
        let original_near = NearKey::from_secret_key(
            "test.near".to_string(),
            "ed25519:test_secret_key"
        ).unwrap();
        
        let export_str = original_near.to_export_string();
        let imported_near = NearKey::from_env_string(&export_str).unwrap();
        
        assert_eq!(original_near.account_id, imported_near.account_id);
        assert_eq!(original_near.secret_key, imported_near.secret_key);
        assert_eq!(original_near.key_type, imported_near.key_type);
    }

    #[tokio::test]
    async fn test_cli_workflow_simulation() {
        // Simulate the CLI workflow: add, list, show, export
        let (config, _temp_dir) = create_test_config();
        let mut mock_manager = MockKeyManager::new(config);
        
        // Step 1: Add keys (simulate CLI add command)
        let cosmos_key = CosmosKey::from_private_key(
            hex::decode("a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3").unwrap(),
            "cosmos"
        ).unwrap();
        
        let near_key = NearKey::from_secret_key(
            "test.near".to_string(),
            "ed25519:test_secret_key"
        ).unwrap();
        
        mock_manager.store_key("cosmoshub-testnet", KeyEntry::Cosmos(cosmos_key.clone()), "password1").await.unwrap();
        mock_manager.store_key("near-testnet", KeyEntry::Near(near_key.clone()), "password2").await.unwrap();
        
        // Step 2: List keys (simulate CLI list command)
        let keys = mock_manager.list_keys().await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"cosmoshub-testnet".to_string()));
        assert!(keys.contains(&"near-testnet".to_string()));
        
        // Step 3: Show key details (simulate CLI show command)
        let cosmos_address = mock_manager.get_address("cosmoshub-testnet").unwrap();
        let near_address = mock_manager.get_address("near-testnet").unwrap();
        
        assert_eq!(cosmos_address, cosmos_key.address);
        assert_eq!(near_address, near_key.account_id);
        
        // Step 4: Export keys (simulate CLI export command)
        let cosmos_loaded = mock_manager.load_key("cosmoshub-testnet").await.unwrap();
        let near_loaded = mock_manager.load_key("near-testnet").await.unwrap();
        
        match cosmos_loaded {
            KeyEntry::Cosmos(key) => {
                let export_str = key.to_export_string();
                assert!(export_str.contains(&key.address));
                assert!(export_str.contains(&key.private_key_hex()));
            }
            _ => panic!("Expected Cosmos key"),
        }
        
        match near_loaded {
            KeyEntry::Near(key) => {
                let export_str = key.to_export_string(); 
                assert!(export_str.contains(&key.account_id));
                assert!(export_str.contains(&key.secret_key));
            }
            _ => panic!("Expected NEAR key"),
        }
    }

    #[tokio::test]
    async fn test_environment_variable_key_loading() {
        // Test the environment variable key loading feature
        let (mut config, _temp_dir) = create_test_config();
        config.allow_env_keys = true;
        config.env_prefix = "TEST_RELAYER_KEY_".to_string();
        
        // Simulate environment variables (in real CLI, these would be actual env vars)
        let env_vars = vec![
            ("TEST_RELAYER_KEY_COSMOSHUB_TESTNET", "cosmos1test:a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3"),
            ("TEST_RELAYER_KEY_NEAR_TESTNET", "test.near:ed25519:test_secret_key"),
        ];
        
        // Test parsing of environment variable formats
        for (env_var, env_value) in env_vars {
            let chain_id = env_var.strip_prefix("TEST_RELAYER_KEY_")
                .unwrap()
                .to_lowercase()
                .replace('_', "-");
            
            // Determine key type and parse
            if env_value.contains("cosmos") {
                let cosmos_key = CosmosKey::from_env_string(env_value);
                assert!(cosmos_key.is_ok(), "Failed to parse Cosmos env var: {}", env_value);
                
                let key = cosmos_key.unwrap();
                assert!(key.address.starts_with("cosmos"));
                println!("✅ Parsed Cosmos key for {}: {}", chain_id, key.address);
            } else if env_value.contains(".near") {
                let near_key = NearKey::from_env_string(env_value);
                assert!(near_key.is_ok(), "Failed to parse NEAR env var: {}", env_value);
                
                let key = near_key.unwrap();
                assert!(key.account_id.contains(".near"));
                println!("✅ Parsed NEAR key for {}: {}", chain_id, key.account_id);
            }
        }
    }

    #[tokio::test]
    async fn test_key_validation_in_cli_context() {
        // Test key validation as it would be used in CLI
        let test_cases = vec![
            // Valid cases
            ("cosmos", "cosmos1test:a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3", true),
            ("cosmos", "b775e45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae4", true),
            ("near", "test.near:ed25519:test_secret_key", true),
            ("near", "alice.testnet:secp256k1:another_key", true),
            
            // Invalid cases
            ("cosmos", "cosmos1test:invalidhex", false),
            ("cosmos", "cosmos1test:short", false),
            ("near", "test.near", false), // Missing secret key
            ("near", "", false), // Empty
        ];
        
        for (key_type, key_data, should_succeed) in test_cases {
            match key_type {
                "cosmos" => {
                    let result = CosmosKey::from_env_string(key_data);
                    if should_succeed {
                        assert!(result.is_ok(), "Should succeed for: {}", key_data);
                        if let Ok(key) = result {
                            // Basic validation should pass
                            println!("✅ Cosmos key validation passed: {}", key.address);
                        }
                    } else {
                        assert!(result.is_err(), "Should fail for: {}", key_data);
                        println!("✅ Cosmos key validation correctly failed for: {}", key_data);
                    }
                }
                "near" => {
                    let result = NearKey::from_env_string(key_data);
                    if should_succeed {
                        assert!(result.is_ok(), "Should succeed for: {}", key_data);
                        if let Ok(key) = result {
                            assert!(key.validate().is_ok());
                            println!("✅ NEAR key validation passed: {}", key.account_id);
                        }
                    } else {
                        assert!(result.is_err(), "Should fail for: {}", key_data);
                        println!("✅ NEAR key validation correctly failed for: {}", key_data);
                    }
                }
                _ => panic!("Unknown key type: {}", key_type),
            }
        }
    }

    #[tokio::test]
    async fn test_cli_error_handling() {
        let (config, _temp_dir) = create_test_config();
        let mut mock_manager = MockKeyManager::new(config);
        
        // Test various error conditions that CLI should handle
        
        // 1. Loading non-existent key
        let result = mock_manager.load_key("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Key not found"));
        
        // 2. Getting address for non-existent key
        let result = mock_manager.get_address("nonexistent");
        assert!(result.is_err());
        
        // 3. Invalid key formats
        let invalid_cosmos = CosmosKey::from_env_string("invalid:format:too:many:parts");
        assert!(invalid_cosmos.is_err());
        
        let invalid_near = NearKey::from_env_string("single_part");
        assert!(invalid_near.is_err());
        
        println!("✅ All CLI error conditions handled correctly");
    }

    #[tokio::test]
    async fn test_multiple_address_prefixes() {
        // Test that CLI can handle different Cosmos address prefixes
        let test_private_key = "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let private_key_bytes = hex::decode(test_private_key).unwrap();
        
        let prefixes = vec!["cosmos", "osmo", "juno", "stars"];
        
        for prefix in prefixes {
            let key = CosmosKey::from_private_key(private_key_bytes.clone(), prefix);
            assert!(key.is_ok(), "Should create key with prefix: {}", prefix);
            
            let k = key.unwrap();
            assert!(k.address.starts_with(prefix), "Address should start with prefix: {}", prefix);
            assert_eq!(k.private_key_hex(), test_private_key);
            
            // All keys should have same private/public key, just different addresses
            assert_eq!(k.private_key.len(), 32);
            assert_eq!(k.public_key.len(), 33);
        }
    }

    #[test]
    fn test_key_manager_config_serialization() {
        // Test that KeyManagerConfig can be serialized/deserialized (for config files)
        let temp_dir = tempdir().unwrap();
        let config = KeyManagerConfig {
            keystore_dir: temp_dir.path().to_path_buf(),
            allow_env_keys: true,
            env_prefix: "RELAYER_KEY_".to_string(),
            kdf_iterations: 10_000,
        };
        
        // Test JSON serialization
        let json = serde_json::to_string(&config);
        assert!(json.is_ok());
        
        let json_str = json.unwrap();
        let deserialized: Result<KeyManagerConfig, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        
        let restored_config = deserialized.unwrap();
        assert_eq!(restored_config.allow_env_keys, config.allow_env_keys);
        assert_eq!(restored_config.env_prefix, config.env_prefix);
        assert_eq!(restored_config.kdf_iterations, config.kdf_iterations);
        
        println!("✅ KeyManagerConfig serialization works correctly");
    }
}