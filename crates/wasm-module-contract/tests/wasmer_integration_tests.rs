/// Integration tests for Wasmer WASM runtime
/// 
/// These tests verify the Wasmer integration infrastructure and fallback mechanisms

#[cfg(not(target_family = "wasm"))]
mod wasmer_tests {
    use wasm_module_contract::wasm_runtime::WasmRuntime;
    use serde_json::json;
    
    /// Helper to create a minimal valid WASM module
    fn create_minimal_wasm() -> Vec<u8> {
        // Minimal valid WASM module with just the header and an empty code section
        vec![
            0x00, 0x61, 0x73, 0x6d, // WASM magic number
            0x01, 0x00, 0x00, 0x00, // Version 1
            // Type section (empty)
            0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            // Function section (empty)
            0x03, 0x02, 0x01, 0x00,
            // Export section (empty)
            0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00,
            // Code section (empty function)
            0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
        ]
    }
    
    /// Helper to create mock CW20 instantiate message
    fn create_cw20_instantiate_msg() -> String {
        json!({
            "msg": {
                "name": "Test Token",
                "symbol": "TEST",
                "decimals": 6,
                "initial_balances": [
                    {
                        "address": "proxima1alice",
                        "amount": "1000000"
                    }
                ],
                "mint": {
                    "minter": "proxima1alice",
                    "cap": "10000000"
                }
            }
        }).to_string()
    }
    
    #[test]
    fn test_wasmer_runtime_creation() {
        let runtime = WasmRuntime::new(b"test_contract".to_vec());
        assert!(runtime.fallback_enabled);
    }
    
    #[test]
    fn test_wasmer_only_runtime_creation() {
        let runtime = WasmRuntime::new_wasmer_only(b"test_contract".to_vec());
        assert!(!runtime.fallback_enabled);
    }
    
    #[test]
    fn test_wasm_validation() {
        let runtime = WasmRuntime::new(b"test".to_vec());
        
        // Valid WASM should pass
        let valid_wasm = create_minimal_wasm();
        assert!(runtime.validate_wasm(&valid_wasm));
        
        // Invalid WASM should fail
        let invalid_wasm = b"not a wasm module";
        assert!(!runtime.validate_wasm(invalid_wasm));
        
        // Empty data should fail
        let empty = b"";
        assert!(!runtime.validate_wasm(empty));
        
        // Wrong magic number should fail
        let wrong_magic = b"\x00\x61\x73\x00\x01\x00\x00\x00";
        assert!(!runtime.validate_wasm(wrong_magic));
        
        // Wrong version should fail  
        let wrong_version = b"\x00\x61\x73\x6d\x02\x00\x00\x00";
        assert!(!runtime.validate_wasm(wrong_version));
    }
    
    #[test]
    fn test_wasmer_fallback_mechanism() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        let args = create_cw20_instantiate_msg();
        
        // With fallback enabled, should succeed even though Wasmer execution fails
        let result = runtime.execute_cosmwasm(&wasm, "instantiate", args.as_bytes());
        assert!(result.is_ok(), "Should fall back to pattern matching");
        
        // Verify the response contains expected CW20 instantiation data
        let response_bytes = result.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_bytes).unwrap();
        
        assert_eq!(response["attributes"][1]["key"], "contract_type");
        assert_eq!(response["attributes"][1]["value"], "cw20");
    }
    
    #[test]
    fn test_wasmer_only_mode_fails_without_implementation() {
        let mut runtime = WasmRuntime::new_wasmer_only(b"test".to_vec());
        let wasm = create_minimal_wasm();
        let args = create_cw20_instantiate_msg();
        
        // Without fallback, should fail since Wasmer isn't fully implemented
        let result = runtime.execute_cosmwasm(&wasm, "instantiate", args.as_bytes());
        assert!(result.is_err(), "Should fail without fallback");
        assert!(result.unwrap_err().contains("Wasmer execution failed"));
    }
    
    #[test]
    fn test_execute_with_pattern_matching() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        
        // First instantiate the CW20 token with initial balances
        let instantiate_msg = create_cw20_instantiate_msg();
        let inst_result = runtime.execute_cosmwasm(&wasm, "instantiate", instantiate_msg.as_bytes());
        assert!(inst_result.is_ok(), "Instantiation failed: {:?}", inst_result.err());
        
        // Now test CW20 transfer execution
        let transfer_msg = json!({
            "msg": {
                "transfer": {
                    "recipient": "proxima1bob",
                    "amount": "100"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "execute", transfer_msg.as_bytes());
        assert!(result.is_ok(), "Execution failed: {:?}", result.err());
        
        let response_bytes = result.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(response["attributes"][0]["value"], "transfer");
    }
    
    #[test]
    fn test_query_with_pattern_matching() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        
        // Test balance query
        let query_msg = json!({
            "msg": {
                "balance": {
                    "address": "proxima1alice"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "query", query_msg.as_bytes());
        assert!(result.is_ok());
        
        let response_bytes = result.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(response["balance"], "1000000");
    }
    
    #[test]
    fn test_migrate_entry_point() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        let migrate_msg = json!({"msg": {}}).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "migrate", migrate_msg.as_bytes());
        assert!(result.is_ok());
        
        let response_bytes = result.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(response["attributes"][0]["value"], "migrate");
    }
    
    #[test]
    fn test_unknown_entry_point() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        let args = json!({"msg": {}}).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "unknown", args.as_bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown entry point"));
    }
    
    #[test]
    fn test_invalid_wasm_rejection() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let invalid_wasm = b"not wasm";
        let args = json!({"msg": {}}).to_string();
        
        let result = runtime.execute_cosmwasm(invalid_wasm, "instantiate", args.as_bytes());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid WASM code");
    }
    
    #[test]
    fn test_contract_type_detection() {
        let runtime = WasmRuntime::new(b"test".to_vec());
        
        // CW20 token detection
        let cw20_msg = json!({
            "name": "Token",
            "symbol": "TKN", 
            "decimals": 6
        });
        assert_eq!(runtime.detect_contract_type(&cw20_msg), "cw20");
        
        // CW721 NFT detection
        let cw721_msg = json!({
            "name": "NFT Collection",
            "symbol": "NFT",
            "minter": "proxima1alice"
        });
        assert_eq!(runtime.detect_contract_type(&cw721_msg), "cw721");
        
        // CW1 multisig detection
        let cw1_msg = json!({
            "admins": ["proxima1alice", "proxima1bob"]
        });
        assert_eq!(runtime.detect_contract_type(&cw1_msg), "cw1");
        
        // Unknown contract type
        let unknown_msg = json!({
            "random": "data"
        });
        assert_eq!(runtime.detect_contract_type(&unknown_msg), "unknown");
    }
    
    #[test]
    fn test_cw20_operations() {
        let mut runtime = WasmRuntime::new(b"cw20_token".to_vec());
        let wasm = create_minimal_wasm();
        
        // Test mint operation
        let mint_msg = json!({
            "msg": {
                "mint": {
                    "recipient": "proxima1bob",
                    "amount": "5000"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "execute", mint_msg.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert_eq!(response["attributes"][0]["value"], "mint");
        
        // Test burn operation
        let burn_msg = json!({
            "msg": {
                "burn": {
                    "amount": "1000"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "execute", burn_msg.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert_eq!(response["attributes"][0]["value"], "burn");
        
        // Test increase allowance
        let increase_msg = json!({
            "msg": {
                "increase_allowance": {
                    "spender": "proxima1charlie",
                    "amount": "2000"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "execute", increase_msg.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert_eq!(response["attributes"][0]["value"], "increase_allowance");
    }
    
    #[test]
    fn test_cw721_mint() {
        let mut runtime = WasmRuntime::new(b"cw721_nft".to_vec());
        let wasm = create_minimal_wasm();
        
        let mint_nft_msg = json!({
            "msg": {
                "mint": {
                    "token_id": "NFT001",
                    "owner": "proxima1alice",
                    "token_uri": "https://example.com/nft/001"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "execute", mint_nft_msg.as_bytes());
        assert!(result.is_ok());
        
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert_eq!(response["attributes"][0]["value"], "mint");
        assert_eq!(response["attributes"][1]["value"], "NFT001");
    }
    
    #[test]
    fn test_query_operations() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        
        // Test token info query
        let token_info_query = json!({
            "msg": {
                "token_info": {}
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "query", token_info_query.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert!(response["token_info"]["name"].is_string());
        
        // Test minter query
        let minter_query = json!({
            "msg": {
                "minter": {}
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "query", minter_query.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert!(response["minter"]["minter"].is_string());
        
        // Test allowance query
        let allowance_query = json!({
            "msg": {
                "allowance": {
                    "owner": "proxima1alice",
                    "spender": "proxima1bob"
                }
            }
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "query", allowance_query.as_bytes());
        assert!(result.is_ok());
        let response: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
        assert!(response["allowance"].is_string());
    }
    
    #[test]
    fn test_invalid_json_args() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        
        let invalid_json = b"not json {]";
        let result = runtime.execute_cosmwasm(&wasm, "instantiate", invalid_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }
    
    #[test]
    fn test_missing_msg_field() {
        let mut runtime = WasmRuntime::new(b"test".to_vec());
        let wasm = create_minimal_wasm();
        
        let no_msg = json!({
            "data": "something"
        }).to_string();
        
        let result = runtime.execute_cosmwasm(&wasm, "instantiate", no_msg.as_bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing 'msg'"));
    }
}

/// Tests that should work in both WASM and non-WASM environments
#[cfg(test)]
mod cross_platform_tests {
    use wasm_module_contract::wasm_runtime::WasmRuntime;
    
    #[test]
    fn test_runtime_creation_always_works() {
        let runtime = WasmRuntime::new(b"test".to_vec());
        // Runtime should always be creatable
        assert_eq!(runtime.host_functions.storage_prefix, b"test");
    }
    
    #[test]
    fn test_wasm_validation_logic() {
        let runtime = WasmRuntime::new(b"test".to_vec());
        
        // These tests work regardless of Wasmer availability
        let valid = b"\0asm\x01\x00\x00\x00rest_of_wasm";
        assert!(runtime.validate_wasm(valid));
        
        let invalid = b"invalid";
        assert!(!runtime.validate_wasm(invalid));
    }
}