/// Comprehensive Unit Tests for x/wasm Module
/// 
/// This module provides complete test coverage for all x/wasm module functionality
/// including types, storage operations, access control, and contract lifecycle.

#[cfg(test)]
mod tests {
    use super::super::*;
    use near_sdk::{AccountId, env, test_utils::VMContextBuilder, testing_env};

    // Test helper to set up testing environment
    fn setup_test_env() {
        let context = VMContextBuilder::new()
            .current_account_id("contract.testnet".parse().unwrap())
            .predecessor_account_id("alice.testnet".parse().unwrap())
            .block_height(1000)
            .build();
        testing_env!(context);
    }

    // Test helper to create mock WASM bytecode
    fn mock_wasm_code(name: &str) -> Vec<u8> {
        format!("mock_wasm_bytecode_{}", name).into_bytes()
    }

    // Test helper to create test account ID
    fn test_account(name: &str) -> AccountId {
        format!("{}.testnet", name).parse().unwrap()
    }

    #[cfg(test)]
    mod types_tests {
        use super::*;

        #[test]
        fn test_code_info_creation() {
            setup_test_env();
            
            let creator = test_account("creator");
            let code_hash = env::sha256(b"test_code");
            
            let code_info = CodeInfo {
                code_id: 1,
                creator: creator.to_string(),
                code_hash: code_hash.clone(),
                source: "https://github.com/example/contract".to_string(),
                builder: "cosmwasm/rust-optimizer:0.12.0".to_string(),
                instantiate_permission: AccessType::Everybody,
            };
            
            assert_eq!(code_info.code_id, 1);
            assert_eq!(code_info.creator, creator);
            assert_eq!(code_info.code_hash, code_hash);
            assert_eq!(code_info.source, "https://github.com/example/contract");
            assert_eq!(code_info.builder, "cosmwasm/rust-optimizer:0.12.0");
            assert!(matches!(code_info.instantiate_permission, AccessType::Everybody));
        }

        #[test]
        fn test_contract_info_creation() {
            setup_test_env();
            
            let address: ContractAddress = "contract.1.1".parse().unwrap();
            let creator = test_account("creator");
            let admin = Some(test_account("admin"));
            
            let contract_info = ContractInfo {
                address: address.clone(),
                code_id: 1,
                creator: creator.to_string(),
                admin: admin.as_ref().map(|a| a.to_string()),
                label: "Test Contract".to_string(),
                created: 1000,
                ibc_port_id: None,
                extension: None,
            };
            
            assert_eq!(contract_info.address, address);
            assert_eq!(contract_info.code_id, 1);
            assert_eq!(contract_info.creator, creator);
            assert_eq!(contract_info.admin, admin.as_ref().map(|a| a.to_string()));
            assert_eq!(contract_info.label, "Test Contract");
            assert_eq!(contract_info.created, 1000);
            assert!(contract_info.ibc_port_id.is_none());
            assert!(contract_info.extension.is_none());
        }

        #[test]
        fn test_access_config_conversion() {
            let module = WasmModule::new();
            
            // Test Everybody
            let everybody = module.convert_access_config(Some(AccessConfig::Everybody {}));
            assert!(matches!(everybody, AccessType::Everybody));
            
            // Test Nobody
            let nobody = module.convert_access_config(Some(AccessConfig::Nobody {}));
            assert!(matches!(nobody, AccessType::Nobody));
            
            // Test OnlyAddress
            let only_addr = module.convert_access_config(Some(AccessConfig::OnlyAddress {
                address: "alice.testnet".to_string()
            }));
            if let AccessType::OnlyAddress(addr) = only_addr {
                assert_eq!(addr.as_str(), "alice.testnet");
            } else {
                panic!("Expected OnlyAddress variant");
            }
            
            // Test AnyOfAddresses
            let any_of = module.convert_access_config(Some(AccessConfig::AnyOfAddresses {
                addresses: vec!["alice.testnet".to_string(), "bob.testnet".to_string()]
            }));
            if let AccessType::AnyOfAddresses(addrs) = any_of {
                assert_eq!(addrs.len(), 2);
                assert!(addrs.contains(&"alice.testnet".parse().unwrap()));
                assert!(addrs.contains(&"bob.testnet".parse().unwrap()));
            } else {
                panic!("Expected AnyOfAddresses variant");
            }
        }

        #[test]
        fn test_instantiate_response() {
            let response = InstantiateResponse {
                address: "contract.1.1".to_string(),
                data: Some(b"response_data".to_vec()),
                events: vec![],
            };
            
            assert_eq!(response.address, "contract.1.1");
            assert_eq!(response.data, Some(b"response_data".to_vec()));
        }

        #[test]
        fn test_execute_response() {
            let response = ExecuteResponse {
                data: Some(b"execute_result".to_vec()),
                events: vec![],
            };
            
            assert_eq!(response.data, Some(b"execute_result".to_vec()));
        }

        #[test]
        fn test_coin_structure() {
            let coin = Coin {
                denom: "uatom".to_string(),
                amount: "1000000".to_string(),
            };
            
            assert_eq!(coin.denom, "uatom");
            assert_eq!(coin.amount, "1000000");
        }
    }

    #[cfg(test)]
    mod storage_tests {
        use super::*;

        #[test]
        fn test_wasm_module_initialization() {
            let module = WasmModule::new();
            
            assert_eq!(module.get_next_code_id(), 1);
            
            // Test that storage maps are properly initialized (they should be empty)
            assert!(module.get_code_info(1).is_none());
            assert!(module.get_contract_info(&"test.testnet".parse().unwrap()).is_none());
        }

        #[test]
        fn test_store_code_basic() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            let code = mock_wasm_code("test");
            
            let result = module.store_code(
                &creator,
                code.clone(),
                Some("https://github.com/test".to_string()),
                Some("cosmwasm/optimizer:0.12.0".to_string()),
                Some(AccessConfig::Everybody {})
            );
            
            assert!(result.is_ok());
            let code_id = result.unwrap();
            assert_eq!(code_id, 1);
            assert_eq!(module.get_next_code_id(), 2);
            
            // Verify code info was stored
            let code_info = module.get_code_info(code_id).unwrap();
            assert_eq!(code_info.code_id, code_id);
            assert_eq!(code_info.creator, creator);
            assert_eq!(code_info.source, "https://github.com/test");
            assert_eq!(code_info.builder, "cosmwasm/optimizer:0.12.0");
        }

        #[test]
        fn test_store_code_size_limit() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Create code larger than 3MB limit
            let large_code = vec![0u8; 3_000_001];
            
            let result = module.store_code(
                &creator,
                large_code,
                None,
                None,
                None
            );
            
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Code size exceeds maximum allowed");
        }

        #[test]
        fn test_multiple_code_storage() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store multiple codes
            let code1 = mock_wasm_code("contract1");
            let code2 = mock_wasm_code("contract2");
            let code3 = mock_wasm_code("contract3");
            
            let id1 = module.store_code(&creator, code1, None, None, None).unwrap();
            let id2 = module.store_code(&creator, code2, None, None, None).unwrap();
            let id3 = module.store_code(&creator, code3, None, None, None).unwrap();
            
            assert_eq!(id1, 1);
            assert_eq!(id2, 2);
            assert_eq!(id3, 3);
            assert_eq!(module.get_next_code_id(), 4);
            
            // Verify all codes can be retrieved
            assert!(module.get_code_info(id1).is_some());
            assert!(module.get_code_info(id2).is_some());
            assert!(module.get_code_info(id3).is_some());
        }

        #[test]
        fn test_list_codes() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store several codes
            for i in 0..5 {
                let code = mock_wasm_code(&format!("contract{}", i));
                module.store_code(&creator, code, None, None, None).unwrap();
            }
            
            // Test listing all codes
            let all_codes = module.list_codes(None, None);
            assert_eq!(all_codes.len(), 5);
            
            // Test pagination
            let limited_codes = module.list_codes(None, Some(3));
            assert_eq!(limited_codes.len(), 3);
            
            // Test start_after
            let codes_after_2 = module.list_codes(Some(2), None);
            assert_eq!(codes_after_2.len(), 3); // codes 3, 4, 5
            assert_eq!(codes_after_2[0].code_id, 3);
        }

        #[test]
        fn test_contract_instantiation() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            let admin = test_account("admin");
            
            // First store code
            let code = mock_wasm_code("test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            // Instantiate contract
            let result = module.instantiate_contract(
                &creator,
                code_id,
                b"init_msg".to_vec(),
                vec![],
                "Test Contract".to_string(),
                Some(admin.clone())
            );
            
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.address.is_empty());
            
            // Verify contract info was stored
            let contract_address: ContractAddress = response.address.parse().unwrap();
            let contract_info = module.get_contract_info(&contract_address).unwrap();
            assert_eq!(contract_info.code_id, code_id);
            assert_eq!(contract_info.creator, creator);
            assert_eq!(contract_info.admin, Some(admin.to_string()));
            assert_eq!(contract_info.label, "Test Contract");
        }

        #[test]
        fn test_instantiate_nonexistent_code() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            let result = module.instantiate_contract(
                &creator,
                999, // Non-existent code ID
                b"init_msg".to_vec(),
                vec![],
                "Test Contract".to_string(),
                None
            );
            
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Code ID 999 not found");
        }

        #[test]
        fn test_contract_indexing() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store code
            let code = mock_wasm_code("test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            // Instantiate multiple contracts from same code
            let mut contract_addresses = Vec::new();
            for i in 0..3 {
                let response = module.instantiate_contract(
                    &creator,
                    code_id,
                    format!("init_msg_{}", i).into_bytes(),
                    vec![],
                    format!("Contract {}", i),
                    None
                ).unwrap();
                contract_addresses.push(response.address);
            }
            
            // Test listing contracts by code
            let contracts = module.list_contracts_by_code(code_id, None, None);
            assert_eq!(contracts.len(), 3);
            
            // Verify contract addresses match
            for (i, contract) in contracts.iter().enumerate() {
                assert_eq!(contract.address.to_string(), contract_addresses[i]);
                assert_eq!(contract.code_id, code_id);
            }
        }

        #[test]
        fn test_list_contracts_pagination() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store code and instantiate multiple contracts
            let code = mock_wasm_code("test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            for i in 0..5 {
                module.instantiate_contract(
                    &creator,
                    code_id,
                    format!("init_{}", i).into_bytes(),
                    vec![],
                    format!("Contract {}", i),
                    None
                ).unwrap();
            }
            
            // Test pagination
            let limited = module.list_contracts_by_code(code_id, None, Some(3));
            assert_eq!(limited.len(), 3);
            
            // Test start_after (using first contract address)
            let all_contracts = module.list_contracts_by_code(code_id, None, None);
            let first_address = all_contracts[0].address.to_string();
            let after_first = module.list_contracts_by_code(code_id, Some(first_address), None);
            assert_eq!(after_first.len(), 4); // Remaining contracts after first
        }
    }

    #[cfg(test)]
    mod access_control_tests {
        use super::*;

        #[test]
        fn test_everybody_permission() {
            let module = WasmModule::new();
            let permission = AccessType::Everybody;
            let sender = test_account("anyone");
            
            assert!(module.can_instantiate(&permission, &sender));
        }

        #[test]
        fn test_nobody_permission() {
            let module = WasmModule::new();
            let permission = AccessType::Nobody;
            let sender = test_account("anyone");
            
            assert!(!module.can_instantiate(&permission, &sender));
        }

        #[test]
        fn test_only_address_permission() {
            let module = WasmModule::new();
            let allowed_account = test_account("allowed");
            let permission = AccessType::OnlyAddress(allowed_account.to_string());
            
            // Test allowed account
            assert!(module.can_instantiate(&permission, &allowed_account));
            
            // Test different account
            let other_account = test_account("other");
            assert!(!module.can_instantiate(&permission, &other_account));
        }

        #[test]
        fn test_any_of_addresses_permission() {
            let module = WasmModule::new();
            let allowed1 = test_account("allowed1");
            let allowed2 = test_account("allowed2");
            let permission = AccessType::AnyOfAddresses(vec![allowed1.to_string(), allowed2.to_string()]);
            
            // Test first allowed account
            assert!(module.can_instantiate(&permission, &allowed1));
            
            // Test second allowed account
            assert!(module.can_instantiate(&permission, &allowed2));
            
            // Test disallowed account
            let other_account = test_account("other");
            assert!(!module.can_instantiate(&permission, &other_account));
        }

        #[test]
        fn test_instantiate_with_permissions() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            let allowed_user = test_account("allowed");
            let other_user = test_account("other");
            
            // Store code with OnlyAddress permission
            let code = mock_wasm_code("restricted");
            let code_id = module.store_code(
                &creator,
                code,
                None,
                None,
                Some(AccessConfig::OnlyAddress {
                    address: allowed_user.to_string()
                })
            ).unwrap();
            
            // Test allowed user can instantiate
            let result = module.instantiate_contract(
                &allowed_user,
                code_id,
                b"init".to_vec(),
                vec![],
                "Allowed Contract".to_string(),
                None
            );
            assert!(result.is_ok());
            
            // Test other user cannot instantiate
            let result = module.instantiate_contract(
                &other_user,
                code_id,
                b"init".to_vec(),
                vec![],
                "Forbidden Contract".to_string(),
                None
            );
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Unauthorized to instantiate this code");
        }

        #[test]
        fn test_complex_permission_scenarios() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Test with multiple allowed addresses
            let user1 = test_account("user1");
            let user2 = test_account("user2");
            let user3 = test_account("user3");
            
            let code = mock_wasm_code("multi_user");
            let code_id = module.store_code(
                &creator,
                code,
                None,
                None,
                Some(AccessConfig::AnyOfAddresses {
                    addresses: vec![user1.to_string(), user2.to_string()]
                })
            ).unwrap();
            
            // user1 should be able to instantiate
            let result1 = module.instantiate_contract(&user1, code_id, b"init1".to_vec(), vec![], "Contract1".to_string(), None);
            assert!(result1.is_ok());
            
            // user2 should be able to instantiate
            let result2 = module.instantiate_contract(&user2, code_id, b"init2".to_vec(), vec![], "Contract2".to_string(), None);
            assert!(result2.is_ok());
            
            // user3 should not be able to instantiate
            let result3 = module.instantiate_contract(&user3, code_id, b"init3".to_vec(), vec![], "Contract3".to_string(), None);
            assert!(result3.is_err());
        }
    }

    #[cfg(test)]
    mod lifecycle_tests {
        use super::*;

        #[test]
        fn test_complete_contract_lifecycle() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // 1. Store code
            let code = mock_wasm_code("lifecycle_test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            // 2. Instantiate contract
            let response = module.instantiate_contract(
                &creator,
                code_id,
                b"init_msg".to_vec(),
                vec![],
                "Lifecycle Contract".to_string(),
                Some(creator.clone())
            ).unwrap();
            
            let contract_address: ContractAddress = response.address.parse().unwrap();
            
            // 3. Execute contract
            let execute_result = module.execute_contract(
                &creator,
                &contract_address,
                b"execute_msg".to_vec(),
                vec![]
            );
            assert!(execute_result.is_ok());
            
            // 4. Query contract
            let query_result = module.query_contract(
                &contract_address,
                b"query_msg".to_vec()
            );
            assert!(query_result.is_ok());
        }

        #[test]
        fn test_contract_address_generation() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store code
            let code = mock_wasm_code("address_test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            // Instantiate multiple contracts from same code
            let mut addresses = Vec::new();
            for i in 0..3 {
                let response = module.instantiate_contract(
                    &creator,
                    code_id,
                    format!("init_{}", i).into_bytes(),
                    vec![],
                    format!("Contract {}", i),
                    None
                ).unwrap();
                addresses.push(response.address);
            }
            
            // Verify addresses are unique
            for i in 0..addresses.len() {
                for j in i+1..addresses.len() {
                    assert_ne!(addresses[i], addresses[j], "Contract addresses should be unique");
                }
            }
            
            // Verify address format follows expected pattern
            for (i, address) in addresses.iter().enumerate() {
                assert!(address.starts_with(&format!("contract.{}", code_id)));
                // Address should contain instance number
                assert!(address.contains(&(i + 1).to_string()));
            }
        }

        #[test]
        fn test_next_instance_id_calculation() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store code
            let code = mock_wasm_code("instance_test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            // Initially should be 1 (no contracts yet)
            assert_eq!(module.get_next_instance_id(code_id), 1);
            
            // After first instantiation should be 2
            module.instantiate_contract(&creator, code_id, b"init1".to_vec(), vec![], "C1".to_string(), None).unwrap();
            assert_eq!(module.get_next_instance_id(code_id), 2);
            
            // After second instantiation should be 3
            module.instantiate_contract(&creator, code_id, b"init2".to_vec(), vec![], "C2".to_string(), None).unwrap();
            assert_eq!(module.get_next_instance_id(code_id), 3);
        }

        #[test]
        fn test_execute_nonexistent_contract() {
            setup_test_env();
            let mut module = WasmModule::new();
            let sender = test_account("sender");
            let fake_address: ContractAddress = "nonexistent.contract".parse().unwrap();
            
            let result = module.execute_contract(
                &sender,
                &fake_address,
                b"execute_msg".to_vec(),
                vec![]
            );
            
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Contract nonexistent.contract not found");
        }

        #[test]
        fn test_query_nonexistent_contract() {
            setup_test_env();
            let module = WasmModule::new();
            let fake_address: ContractAddress = "nonexistent.contract".parse().unwrap();
            
            let result = module.query_contract(
                &fake_address,
                b"query_msg".to_vec()
            );
            
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Contract nonexistent.contract not found");
        }

        #[test]
        fn test_contract_state_isolation() {
            setup_test_env();
            let mut module = WasmModule::new();
            let creator = test_account("creator");
            
            // Store code and instantiate two contracts
            let code = mock_wasm_code("isolation_test");
            let code_id = module.store_code(&creator, code, None, None, None).unwrap();
            
            let contract1 = module.instantiate_contract(&creator, code_id, b"init1".to_vec(), vec![], "Contract1".to_string(), None).unwrap();
            let contract2 = module.instantiate_contract(&creator, code_id, b"init2".to_vec(), vec![], "Contract2".to_string(), None).unwrap();
            
            // Verify contracts have different addresses
            assert_ne!(contract1.address, contract2.address);
            
            // Verify both contracts exist in storage
            let addr1: ContractAddress = contract1.address.parse().unwrap();
            let addr2: ContractAddress = contract2.address.parse().unwrap();
            
            let info1 = module.get_contract_info(&addr1).unwrap();
            let info2 = module.get_contract_info(&addr2).unwrap();
            
            assert_eq!(info1.label, "Contract1");
            assert_eq!(info2.label, "Contract2");
            assert_eq!(info1.code_id, code_id);
            assert_eq!(info2.code_id, code_id);
        }
    }

    #[cfg(test)]
    mod helper_tests {
        use super::*;

        #[test]
        fn test_convert_access_config_edge_cases() {
            let module = WasmModule::new();
            
            // Test None case
            let none_result = module.convert_access_config(None);
            assert!(matches!(none_result, AccessType::Everybody));
            
            // Test edge case address handling (NEAR allows flexible account IDs)
            let edge_case_addr = module.convert_access_config(Some(AccessConfig::OnlyAddress {
                address: "invalid-account-id".to_string()
            }));
            // NEAR SDK allows this as a valid account ID
            if let AccessType::OnlyAddress(addr) = edge_case_addr {
                assert_eq!(addr.as_str(), "invalid-account-id");
            } else {
                panic!("Expected OnlyAddress variant");
            }
            
            // Test empty addresses array
            let empty_addrs = module.convert_access_config(Some(AccessConfig::AnyOfAddresses {
                addresses: vec![]
            }));
            if let AccessType::AnyOfAddresses(addrs) = empty_addrs {
                assert!(addrs.is_empty());
            } else {
                panic!("Expected AnyOfAddresses variant");
            }
            
            // Test that all addresses are passed through without validation
            let mixed_addrs = module.convert_access_config(Some(AccessConfig::AnyOfAddresses {
                addresses: vec![
                    "valid.testnet".to_string(),
                    "INVALID..ACCOUNT".to_string(), // Double dots are invalid in NEAR
                    "also-valid.testnet".to_string()
                ]
            }));
            if let AccessType::AnyOfAddresses(addrs) = mixed_addrs {
                assert_eq!(addrs.len(), 3); // All addresses are included without validation
                assert!(addrs.contains(&"valid.testnet".to_string()));
                assert!(addrs.contains(&"INVALID..ACCOUNT".to_string()));
                assert!(addrs.contains(&"also-valid.testnet".to_string()));
            } else {
                panic!("Expected AnyOfAddresses variant");
            }
        }

        #[test]
        fn test_can_instantiate_edge_cases() {
            let module = WasmModule::new();
            let sender = test_account("sender");
            
            // Test empty AnyOfAddresses
            let empty_permission = AccessType::AnyOfAddresses(vec![]);
            assert!(!module.can_instantiate(&empty_permission, &sender));
            
            // Test sender matches one of many addresses
            let many_addresses = AccessType::AnyOfAddresses(vec![
                test_account("user1").to_string(),
                test_account("user2").to_string(),
                sender.to_string(),
                test_account("user3").to_string(),
            ]);
            assert!(module.can_instantiate(&many_addresses, &sender));
        }

        #[test]
        fn test_get_next_instance_id_edge_cases() {
            let module = WasmModule::new();
            
            // Test with non-existent code ID
            assert_eq!(module.get_next_instance_id(999), 1);
            
            // Test with code ID that has no contracts
            assert_eq!(module.get_next_instance_id(1), 1);
        }
    }
}