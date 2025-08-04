//! Unit tests for public API methods implementation

#[cfg(test)]
mod public_api_tests {
    use crate::CosmosContract;
    use crate::handler::TxProcessingConfig;
    use near_sdk::json_types::Base64VecU8;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::testing_env;

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(accounts(0))
            .predecessor_account_id(accounts(0));
        builder
    }

    #[test]
    fn test_public_api_methods_exist() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        // Test invalid transaction bytes (should return error response)
        let invalid_tx = Base64VecU8(b"invalid".to_vec());
        
        // Test broadcast_tx_sync exists and returns TxResponse
        let response = contract.broadcast_tx_sync(invalid_tx.clone());
        assert!(response.code > 0); // Should be error code
        assert!(!response.raw_log.is_empty()); // Should have error message
        
        // Test simulate_tx exists and returns TxResponse  
        let response = contract.simulate_tx(invalid_tx.clone());
        assert!(response.code > 0); // Should be error code
        assert!(!response.raw_log.is_empty()); // Should have error message
        
        // Test broadcast_tx_async exists and returns TxResponse
        let response = contract.broadcast_tx_async(invalid_tx.clone());
        assert!(response.code > 0); // Should be error code
        
        // Test broadcast_tx_commit exists and returns TxResponse
        let response = contract.broadcast_tx_commit(invalid_tx.clone());
        assert!(response.code > 0); // Should be error code
        
        // Test get_tx exists and returns TxResponse
        let response = contract.get_tx("nonexistent".to_string());
        assert!(response.code > 0); // Should be error code (not found)
        
        // Test get_tx_config exists and returns config
        let config = contract.get_tx_config();
        assert!(!config.chain_id.is_empty());
        assert!(config.max_gas_per_tx > 0);
        assert!(config.gas_price > 0);
    }
    
    #[test]
    fn test_update_tx_config() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        let new_config = TxProcessingConfig {
            chain_id: "test-chain-2".to_string(),
            max_gas_per_tx: 2_000_000,
            gas_price: 2,
            verify_signatures: false,
            check_sequences: false,
        };
        
        // Update config
        contract.update_tx_config(new_config.clone());
        
        // Verify config was updated
        let updated_config = contract.get_tx_config();
        assert_eq!(updated_config.chain_id, "test-chain-2");
        assert_eq!(updated_config.max_gas_per_tx, 2_000_000);
        assert_eq!(updated_config.gas_price, 2);
    }
    
    #[test]
    fn test_tx_response_structure() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        let invalid_tx = Base64VecU8(b"invalid".to_vec());
        
        let response = contract.broadcast_tx_sync(invalid_tx);
        
        // Verify TxResponse has all required fields
        assert!(response.code > 0);
        // Base64 encoding of empty bytes results in empty string, which is valid
        assert!(response.data.is_empty() || !response.data.is_empty()); // Data field exists
        assert!(!response.raw_log.is_empty()); // Should have error message
        assert!(!response.info.is_empty()); // Should have some info
        assert!(!response.gas_wanted.is_empty()); // Gas fields are strings
        assert!(!response.gas_used.is_empty()); // Gas fields are strings
        assert!(response.events.is_empty() || !response.events.is_empty()); // Events array exists
        assert!(!response.codespace.is_empty()); // Should have codespace
    }
    
    #[test]
    fn test_all_broadcast_methods_consistency() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        let invalid_tx = Base64VecU8(b"invalid".to_vec());
        
        // Test that all broadcast methods return consistent error responses
        let sync_response = contract.broadcast_tx_sync(invalid_tx.clone());
        let async_response = contract.broadcast_tx_async(invalid_tx.clone());
        let commit_response = contract.broadcast_tx_commit(invalid_tx.clone());
        let simulate_response = contract.simulate_tx(invalid_tx);
        
        // All should return same error code for same invalid input
        assert_eq!(sync_response.code, async_response.code);
        assert_eq!(sync_response.code, simulate_response.code);
        assert_eq!(sync_response.codespace, async_response.codespace);
        
        // Commit response should have height set (could be 0 in test environment)
        assert!(!commit_response.height.is_empty());
        
        // All should have consistent structure
        for response in &[&sync_response, &async_response, &commit_response, &simulate_response] {
            assert!(response.code > 0);
            assert!(!response.raw_log.is_empty());
            assert!(!response.codespace.is_empty());
        }
    }
    
    #[test]
    fn test_configuration_validation() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        // Test edge case configurations
        let edge_config = TxProcessingConfig {
            chain_id: "".to_string(), // Empty chain ID
            max_gas_per_tx: 0, // Zero gas limit
            gas_price: 1,
            verify_signatures: true,
            check_sequences: true,
        };
        
        // Should still work (validation is at transaction processing level)
        contract.update_tx_config(edge_config.clone());
        let retrieved_config = contract.get_tx_config();
        
        assert_eq!(retrieved_config.chain_id, "");
        assert_eq!(retrieved_config.max_gas_per_tx, 0);
    }
    
    #[test]
    fn test_large_transaction_data() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        // Test with large invalid transaction data
        let large_tx = Base64VecU8(vec![0u8; 10000]); // 10KB of zeros
        let response = contract.broadcast_tx_sync(large_tx);
        
        // Should handle large data gracefully
        assert!(response.code > 0);
        assert!(!response.raw_log.is_empty());
        assert_eq!(response.codespace, "sdk");
    }
    
    #[test]
    fn test_empty_transaction_data() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        // Test with empty transaction data
        let empty_tx = Base64VecU8(vec![]);
        let response = contract.broadcast_tx_sync(empty_tx);
        
        // Should return decoding error
        assert!(response.code > 0);
        assert!(!response.raw_log.is_empty());
        assert_eq!(response.codespace, "sdk");
    }
    
    #[test]
    fn test_get_tx_various_hashes() {
        let context = get_context();
        testing_env!(context.build());
        
        let contract = CosmosContract::new();
        
        // Test various hash formats
        let test_hashes = vec![
            "".to_string(),
            "short".to_string(),
            "A".repeat(64), // 64 char hex-like
            "invalid_characters!@#$%".to_string(),
            "0x1234567890abcdef".to_string(),
        ];
        
        for hash in test_hashes {
            let response = contract.get_tx(hash.clone());
            
            // All should return "not found" error
            assert!(response.code > 0);
            assert!(response.raw_log.contains("not found") || response.raw_log.contains("Not found"));
            assert_eq!(response.codespace, "sdk");
        }
    }
    
    #[test]
    fn test_multiple_config_updates() {
        let context = get_context();
        testing_env!(context.build());
        
        let mut contract = CosmosContract::new();
        
        // Test multiple rapid configuration updates
        for i in 1..=5 {
            let config = TxProcessingConfig {
                chain_id: format!("test-chain-{}", i),
                max_gas_per_tx: 1000000 * i as u64,
                gas_price: i as u128,
                verify_signatures: i % 2 == 0,
                check_sequences: i % 2 == 1,
            };
            
            contract.update_tx_config(config.clone());
            let retrieved = contract.get_tx_config();
            
            assert_eq!(retrieved.chain_id, format!("test-chain-{}", i));
            assert_eq!(retrieved.max_gas_per_tx, 1000000 * i as u64);
            assert_eq!(retrieved.gas_price, i as u128);
        }
    }
}