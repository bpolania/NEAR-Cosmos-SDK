/// Integration tests for transaction decoding, signature verification, and end-to-end processing
use cosmos_sdk_contract::handler::{TxDecoder, CosmosTransactionHandler, TxProcessingConfig};
use cosmos_sdk_contract::crypto::{CosmosSignatureVerifier, CosmosPublicKey};
use cosmos_sdk_contract::types::cosmos_tx::{
    CosmosTx, TxBody, AuthInfo, Fee, SignerInfo, Coin, ModeInfo, SignMode, Any
};

fn create_test_decoder() -> TxDecoder {
    TxDecoder::new()
}

fn create_test_signature_verifier() -> CosmosSignatureVerifier {
    CosmosSignatureVerifier::new("test-chain".to_string())
}

fn create_complex_transaction() -> CosmosTx {
    // Create a transaction with multiple messages and complex structure
    let messages = vec![
        Any::new("/cosmos.bank.v1beta1.MsgSend", serde_json::to_vec(&serde_json::json!({
            "from_address": "cosmos1abc",
            "to_address": "cosmos1def",
            "amount": [{"denom": "uatom", "amount": "1000"}]
        })).unwrap()),
        Any::new("/cosmos.staking.v1beta1.MsgDelegate", serde_json::to_vec(&serde_json::json!({
            "delegator_address": "cosmos1abc",
            "validator_address": "cosmosvaloper1xyz",
            "amount": {"denom": "uatom", "amount": "5000"}
        })).unwrap()),
        Any::new("/cosmos.gov.v1beta1.MsgVote", serde_json::to_vec(&serde_json::json!({
            "voter": "cosmos1abc",
            "proposal_id": "1",
            "option": "VOTE_OPTION_YES"
        })).unwrap()),
    ];
    
    let mut body = TxBody::new(messages);
    body.memo = "Complex integration test transaction".to_string();
    body.timeout_height = 12345;
    
    let fee = Fee::new(
        vec![
            Coin::new("unear", "2000000"),
            Coin::new("near", "1")
        ],
        800000
    );
    
    let signer_info = SignerInfo {
        public_key: Some(Any::new(
            "/cosmos.crypto.secp256k1.PubKey",
            vec![2; 33] // Compressed public key
        )),
        mode_info: ModeInfo {
            mode: SignMode::Direct,
            multi: None,
        },
        sequence: 42,
    };
    
    let auth_info = AuthInfo::new(vec![signer_info], fee);
    let signatures = vec![vec![1; 64]]; // Mock signature
    
    CosmosTx::new(body, auth_info, signatures)
}

#[cfg(test)]
mod transaction_processing_tests {
    use super::*;

    #[test]
    fn test_transaction_decoder_creation_and_basic_operations() {
        let decoder = create_test_decoder();
        
        // Test supported message types
        assert!(decoder.is_message_type_supported("/cosmos.bank.v1beta1.MsgSend"));
        assert!(decoder.is_message_type_supported("/cosmos.staking.v1beta1.MsgDelegate"));
        assert!(decoder.is_message_type_supported("/cosmos.gov.v1beta1.MsgVote"));
        assert!(!decoder.is_message_type_supported("/unsupported.module.MsgUnsupported"));
    }

    #[test]
    fn test_complex_transaction_decoding() {
        let decoder = create_test_decoder();
        let tx = create_complex_transaction();
        
        // Serialize the transaction
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        
        // Decode the transaction
        let decoded_tx = decoder.decode_cosmos_tx(tx_bytes).unwrap();
        
        // Verify decoded transaction matches original
        assert_eq!(decoded_tx.body.messages.len(), 3);
        assert_eq!(decoded_tx.body.memo, "Complex integration test transaction");
        assert_eq!(decoded_tx.body.timeout_height, 12345);
        assert_eq!(decoded_tx.auth_info.fee.gas_limit, 800000);
        assert_eq!(decoded_tx.auth_info.fee.amount.len(), 2);
        assert_eq!(decoded_tx.auth_info.signer_infos[0].sequence, 42);
    }

    #[test]
    fn test_message_extraction_and_validation() {
        let decoder = create_test_decoder();
        let tx = create_complex_transaction();
        
        // Extract messages
        let messages = decoder.extract_messages(&tx).unwrap();
        
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        assert_eq!(messages[1].type_url, "/cosmos.staking.v1beta1.MsgDelegate");
        assert_eq!(messages[2].type_url, "/cosmos.gov.v1beta1.MsgVote");
        
        // Verify message data is preserved
        for message in &messages {
            assert!(!message.raw_data.is_empty(), "Message should have data");
        }
    }

    #[test]
    fn test_transaction_validation_comprehensive() {
        let tx = create_complex_transaction();
        
        // Test basic validation
        let result = tx.validate();
        assert!(result.is_ok(), "Complex transaction should be valid");
        
        // Test invalid transaction - mismatched signatures
        let mut invalid_tx = tx.clone();
        invalid_tx.signatures.push(vec![2; 64]); // Extra signature
        
        let result = invalid_tx.validate();
        assert!(result.is_err(), "Transaction with extra signature should be invalid");
        
        // Test invalid transaction - empty type URL
        let mut invalid_tx2 = tx.clone();
        invalid_tx2.body.messages[0].type_url = String::new();
        
        let result = invalid_tx2.validate();
        assert!(result.is_err(), "Transaction with empty type URL should be invalid");
    }

    #[test]
    fn test_signature_verifier_creation_and_sign_doc() {
        let verifier = create_test_signature_verifier();
        assert_eq!(verifier.chain_id, "test-chain");
        
        let tx = create_complex_transaction();
        
        // Create sign document
        let sign_doc = verifier.create_sign_doc(&tx, 123).unwrap();
        
        assert_eq!(sign_doc.chain_id, "test-chain");
        assert_eq!(sign_doc.account_number, 123);
        assert!(!sign_doc.body_bytes.is_empty(), "Sign doc should have body bytes");
        assert!(!sign_doc.auth_info_bytes.is_empty(), "Sign doc should have auth info bytes");
        
        // Test signing bytes generation
        let signing_bytes = sign_doc.signing_bytes();
        assert!(!signing_bytes.is_empty(), "Should generate signing bytes");
    }

    #[test]
    fn test_transaction_hash_generation() {
        let tx1 = create_complex_transaction();
        let tx2 = create_complex_transaction();
        
        let hash1 = tx1.hash();
        let hash2 = tx2.hash();
        
        // Identical transactions should have identical hashes
        assert_eq!(hash1, hash2, "Identical transactions should have same hash");
        assert_eq!(hash1.len(), 64, "Hash should be 64 characters (SHA256 hex)");
        
        // Different transactions should have different hashes
        let mut tx3 = tx1.clone();
        tx3.body.memo = "Different memo".to_string();
        let hash3 = tx3.hash();
        
        assert_ne!(hash1, hash3, "Different transactions should have different hashes");
    }

    #[test]
    fn test_transaction_builder_integration() {
        let decoder = create_test_decoder();
        
        // Build transaction manually since tx_builder doesn't exist yet
        let msg1 = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1, 2, 3]);
        let msg2 = Any::new("/cosmos.staking.v1beta1.MsgDelegate", vec![4, 5, 6]);
        
        let mut body = TxBody::new(vec![msg1, msg2]);
        body.memo = "Builder pattern test".to_string();
        
        let fee = Fee::new(vec![Coin::new("unear", "1500000")], 500000);
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 42,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![0u8; 64]];
        
        let tx = CosmosTx::new(body, auth_info, signatures);
        
        assert_eq!(tx.body.messages.len(), 2);
        assert_eq!(tx.body.memo, "Builder pattern test");
        assert_eq!(tx.auth_info.fee.gas_limit, 500000);
        assert_eq!(tx.auth_info.fee.amount[0].denom, "unear");
        assert_eq!(tx.auth_info.signer_infos[0].sequence, 42);
    }

    #[test]
    fn test_comprehensive_transaction_processing_pipeline() {
        let mut handler = CosmosTransactionHandler::new(TxProcessingConfig {
            chain_id: "integration-test".to_string(),
            max_gas_per_tx: 1_000_000,
            gas_price: 1,
            verify_signatures: false, // Disable for testing
            check_sequences: false,
        });
        
        let tx = create_complex_transaction();
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        
        // Test transaction simulation
        let sim_result = handler.simulate_transaction(tx_bytes.clone());
        assert!(sim_result.is_ok(), "Transaction simulation should succeed");
        
        let sim_response = sim_result.unwrap();
        assert_eq!(sim_response.code, 0, "Simulation should indicate success");
        assert!(sim_response.raw_log.contains("SIMULATION"), "Should be marked as simulation");
        assert_eq!(sim_response.logs.len(), 3, "Should have logs for all 3 messages");
    }

    #[test]
    fn test_error_handling_in_transaction_processing() {
        let decoder = create_test_decoder();
        
        // Test invalid JSON
        let invalid_json = b"{ invalid json }";
        let result = decoder.decode_cosmos_tx(invalid_json.to_vec());
        assert!(result.is_err(), "Invalid JSON should cause decode error");
        
        // Test empty transaction bytes
        let result = decoder.decode_cosmos_tx(vec![]);
        assert!(result.is_err(), "Empty bytes should cause decode error");
        
        // Test transaction with unsupported message
        let mut tx = create_complex_transaction();
        tx.body.messages.push(Any::new("/unsupported.module.MsgUnsupported", vec![1, 2, 3]));
        
        let handler = CosmosTransactionHandler::new(TxProcessingConfig::default());
        let result = handler.validate_transaction(&tx);
        assert!(result.is_err(), "Transaction with unsupported message should fail validation");
    }

    #[test]
    fn test_public_key_handling_and_address_derivation() {
        // Test public key creation and validation
        let secp256k1_key = CosmosPublicKey::Secp256k1(vec![
            2, 121, 190, 102, 126, 249, 220, 187, 172, 85, 160, 98, 149, 206, 135, 11,
            7, 2, 155, 252, 219, 45, 206, 40, 217, 89, 242, 129, 91, 22, 248, 23, 152
        ]);
        
        // Verify key data
        match &secp256k1_key {
            CosmosPublicKey::Secp256k1(data) => {
                assert_eq!(data.len(), 33, "Secp256k1 compressed key should be 33 bytes");
                assert_eq!(data[0], 2, "Should be compressed key starting with 02");
            }
            _ => panic!("Expected secp256k1 key"),
        }
        
        // Test ed25519 public key
        let ed25519_key = CosmosPublicKey::Ed25519(vec![
            215, 90, 152, 1, 130, 177, 10, 183, 213, 75, 254, 211, 201, 100, 7, 58,
            14, 225, 114, 243, 218, 166, 35, 37, 175, 2, 26, 104, 247, 7, 81, 26
        ]);
        
        // Verify key data
        match &ed25519_key {
            CosmosPublicKey::Ed25519(data) => {
                assert_eq!(data.len(), 32, "Ed25519 key should be 32 bytes");
            }
            _ => panic!("Expected ed25519 key"),
        }
        
        // Test that different key types are different
        assert_ne!(secp256k1_key, ed25519_key, "Different key types should not be equal");
    }

    #[test]
    fn test_sign_mode_support() {
        // Test Direct sign mode
        let signer_direct = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Direct,
                multi: None,
            },
            sequence: 1,
        };
        
        assert_eq!(signer_direct.mode_info.mode, SignMode::Direct);
        
        // Test Textual sign mode
        let signer_textual = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::Textual,
                multi: None,
            },
            sequence: 1,
        };
        
        assert_eq!(signer_textual.mode_info.mode, SignMode::Textual);
        
        // Test Legacy Amino sign mode
        let signer_amino = SignerInfo {
            public_key: None,
            mode_info: ModeInfo {
                mode: SignMode::LegacyAminoJson,
                multi: None,
            },
            sequence: 1,
        };
        
        assert_eq!(signer_amino.mode_info.mode, SignMode::LegacyAminoJson);
    }

    #[test]
    fn test_fee_structure_validation() {
        // Test valid fee structure
        let valid_fee = Fee::new(
            vec![Coin::new("unear", "1000000")],
            200000
        );
        assert!(valid_fee.validate().is_ok(), "Valid fee should pass validation");
        
        // Test fee with zero gas limit
        let zero_gas_fee = Fee::new(
            vec![Coin::new("unear", "1000000")],
            0
        );
        assert!(zero_gas_fee.validate().is_err(), "Zero gas limit should fail validation");
        
        // Test fee with invalid coin
        let invalid_coin_fee = Fee::new(
            vec![Coin::new("", "1000000")], // Empty denomination
            200000
        );
        assert!(invalid_coin_fee.validate().is_err(), "Invalid coin should fail validation");
    }

    #[test]
    fn test_transaction_timeout_handling() {
        let mut tx = create_complex_transaction();
        
        // Test transaction without timeout
        tx.body.timeout_height = 0;
        assert!(tx.validate().is_ok(), "Transaction without timeout should be valid");
        
        // Test transaction with timeout
        tx.body.timeout_height = 100000;
        assert!(tx.validate().is_ok(), "Transaction with timeout should be valid");
        
        // Verify timeout is preserved in encoding/decoding
        let decoder = create_test_decoder();
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        let decoded_tx = decoder.decode_cosmos_tx(tx_bytes).unwrap();
        assert_eq!(decoded_tx.body.timeout_height, 100000, "Timeout should be preserved");
    }

    #[test]
    fn test_multiple_coin_fee_handling() {
        let multi_coin_fee = Fee::new(
            vec![
                Coin::new("unear", "1000000"),
                Coin::new("near", "1"),
                Coin::new("custom", "500")
            ],
            300000
        );
        
        assert!(multi_coin_fee.validate().is_ok(), "Multi-coin fee should be valid");
        assert_eq!(multi_coin_fee.amount.len(), 3, "Should have 3 different coin types");
        
        // Test that all coins have valid amounts
        for coin in &multi_coin_fee.amount {
            assert!(coin.validate().is_ok(), "Each coin should be valid");
            assert!(!coin.denom.is_empty(), "Denomination should not be empty");
            assert!(!coin.amount.is_empty(), "Amount should not be empty");
        }
    }
}

// Performance and edge case tests
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_large_transaction_processing() {
        let decoder = create_test_decoder();
        
        // Create transaction with many messages
        let mut messages = Vec::new();
        for i in 0..50 {
            messages.push(Any::new(
                "/cosmos.bank.v1beta1.MsgSend",
                serde_json::to_vec(&serde_json::json!({
                    "from_address": format!("cosmos1sender{}", i),
                    "to_address": format!("cosmos1recipient{}", i),
                    "amount": [{"denom": "unear", "amount": format!("{}", 1000 * (i + 1))}]
                })).unwrap()
            ));
        }
        
        let body = TxBody::new(messages);
        let fee = Fee::new(vec![Coin::new("unear", "50000000")], 2000000); // Large fee for large tx
        let signer_info = SignerInfo {
            public_key: None,
            mode_info: ModeInfo { mode: SignMode::Direct, multi: None },
            sequence: 1,
        };
        let auth_info = AuthInfo::new(vec![signer_info], fee);
        let signatures = vec![vec![0u8; 64]];
        
        let large_tx = CosmosTx::new(body, auth_info, signatures);
        
        // Test validation
        assert!(large_tx.validate().is_ok(), "Large transaction should be valid");
        
        // Test decoding
        let tx_bytes = serde_json::to_vec(&large_tx).unwrap();
        let decoded_tx = decoder.decode_cosmos_tx(tx_bytes).unwrap();
        assert_eq!(decoded_tx.body.messages.len(), 50, "Should preserve all messages");
    }

    #[test]
    fn test_empty_and_minimal_transactions() {
        // Test minimal valid transaction
        let minimal_tx = CosmosTx::new(
            TxBody::new(vec![Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1])]),
            AuthInfo::new(
                vec![SignerInfo {
                    public_key: None,
                    mode_info: ModeInfo { mode: SignMode::Direct, multi: None },
                    sequence: 0,
                }],
                Fee::new(vec![Coin::new("unear", "1")], 1)
            ),
            vec![vec![0]]
        );
        
        assert!(minimal_tx.validate().is_ok(), "Minimal transaction should be valid");
        
        // Test transaction with empty memo
        let mut tx_empty_memo = minimal_tx.clone();
        tx_empty_memo.body.memo = String::new();
        assert!(tx_empty_memo.validate().is_ok(), "Empty memo should be valid");
        
        // Test transaction with very long memo
        let mut tx_long_memo = minimal_tx.clone();
        tx_long_memo.body.memo = "a".repeat(1000);
        assert!(tx_long_memo.validate().is_ok(), "Long memo should be valid");
    }

    #[test]
    fn test_special_characters_in_transaction_data() {
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>? Unicode: 🚀💰🔥";
        
        let mut tx = create_complex_transaction();
        tx.body.memo = special_chars.to_string();
        
        // Test that special characters don't break validation
        assert!(tx.validate().is_ok(), "Special characters should not break validation");
        
        // Test encoding/decoding preserves special characters
        let decoder = create_test_decoder();
        let tx_bytes = serde_json::to_vec(&tx).unwrap();
        let decoded_tx = decoder.decode_cosmos_tx(tx_bytes).unwrap();
        assert_eq!(decoded_tx.body.memo, special_chars, "Special characters should be preserved");
    }

    #[test]
    fn test_boundary_values() {
        // Test maximum gas limit
        let max_gas_tx = CosmosTx::new(
            TxBody::new(vec![Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1])]),
            AuthInfo::new(
                vec![SignerInfo {
                    public_key: None,
                    mode_info: ModeInfo { mode: SignMode::Direct, multi: None },
                    sequence: u64::MAX, // Maximum sequence
                }],
                Fee::new(vec![Coin::new("unear", &u64::MAX.to_string())], u64::MAX) // Maximum values
            ),
            vec![vec![0]]
        );
        
        assert!(max_gas_tx.validate().is_ok(), "Maximum values should be valid");
        
        // Test zero sequence (valid for first transaction)
        let zero_seq_tx = CosmosTx::new(
            TxBody::new(vec![Any::new("/cosmos.bank.v1beta1.MsgSend", vec![1])]),
            AuthInfo::new(
                vec![SignerInfo {
                    public_key: None,
                    mode_info: ModeInfo { mode: SignMode::Direct, multi: None },
                    sequence: 0,
                }],
                Fee::new(vec![Coin::new("unear", "1")], 1)
            ),
            vec![vec![0]]
        );
        
        assert!(zero_seq_tx.validate().is_ok(), "Zero sequence should be valid");
    }
}