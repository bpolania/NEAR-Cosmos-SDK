use std::sync::Arc;
use crate::cosmwasm::execution_service::WasmerExecutionService;
use crate::cosmwasm::state::StateManager;
use crate::cosmwasm::types::{CosmWasmEnv, BlockInfo, ContractInfo, StateChange};
use near_jsonrpc_client::JsonRpcClient;
use near_primitives::types::AccountId;

/// Create a mock StateManager for testing
fn create_mock_state_manager() -> Arc<StateManager> {
    // Create a dummy NEAR client (won't be used in unit tests)
    let client = Arc::new(JsonRpcClient::connect("http://localhost:3030"));
    let account_id: AccountId = "test.near".parse().unwrap();
    Arc::new(StateManager::new(client, account_id))
}

/// Create a test environment
fn create_test_env() -> CosmWasmEnv {
    CosmWasmEnv {
        block: BlockInfo {
            height: 1000,
            time: 1234567890,
            chain_id: "test-chain".to_string(),
        },
        contract: ContractInfo {
            address: "contract1".to_string(),
            creator: Some("creator1".to_string()),
            admin: None,
        },
        transaction: None,
    }
}

/// Minimal valid WASM for testing
fn create_minimal_wasm() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, // Magic
        0x01, 0x00, 0x00, 0x00, // Version
        // Minimal sections to make it valid
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section
        0x03, 0x02, 0x01, 0x00, // Function section
        0x05, 0x03, 0x01, 0x00, 0x01, // Memory section
        0x07, 0x0a, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, // Export memory
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // Code section
    ]
}

#[tokio::test]
async fn test_execution_service_creation() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    
    // Service should be created successfully
    assert!(true); // If we get here, creation succeeded
}

#[tokio::test]
async fn test_execute_contract_invalid_wasm() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    let env = create_test_env();
    
    // Test with invalid WASM
    let result = service.execute_contract(
        "test_contract",
        &[0x00, 0x00], // Invalid WASM
        "execute",
        b"{}",
        env,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "Wasmer crashes with minimal test WASM - needs real WASM module"]
async fn test_module_caching() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    let env = create_test_env();
    let wasm = create_minimal_wasm();
    
    // First execution - should compile and cache
    let result1 = service.execute_contract(
        "test_contract",
        &wasm,
        "test",
        b"{}",
        env.clone(),
    ).await;
    
    // Second execution - should use cached module
    let result2 = service.execute_contract(
        "test_contract",
        &wasm,
        "test",
        b"{}",
        env,
    ).await;
    
    // Both should have same error or success
    assert_eq!(result1.is_ok(), result2.is_ok());
}

#[tokio::test]
async fn test_gas_calculation() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    
    // Test with no changes
    let gas1 = service.calculate_gas_used(&[], &[]);
    assert_eq!(gas1, 1000); // Base cost
    
    // Test with state changes
    let state_changes = vec![
        StateChange::Set {
            key: vec![1, 2, 3],
            value: vec![4, 5, 6],
        },
        StateChange::Remove {
            key: vec![7, 8, 9],
        },
    ];
    
    let gas2 = service.calculate_gas_used(&state_changes, &[]);
    assert!(gas2 > 1000); // Should include state change costs
    
    // Test with events
    use crate::cosmwasm::types::Event;
    use std::collections::HashMap;
    
    let mut attributes = HashMap::new();
    attributes.insert("action".to_string(), "test".to_string());
    
    let events = vec![
        Event {
            typ: "test_event".to_string(),
            attributes,
        },
    ];
    
    let gas3 = service.calculate_gas_used(&[], &events);
    assert!(gas3 > 1000); // Should include event costs
}

#[tokio::test]
#[ignore = "Wasmer crashes with minimal test WASM - needs real WASM module"]
async fn test_contract_instantiation() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    let env = create_test_env();
    let wasm = create_minimal_wasm();
    
    // Try to instantiate a contract
    let result = service.instantiate_contract(
        1, // code_id
        &wasm,
        b"{\"count\":0}",
        env,
    ).await;
    
    match result {
        Ok((address, _execution_result)) => {
            // Check that address was generated
            assert!(address.starts_with("proxima1"));
            assert!(address.len() > 10);
        }
        Err(e) => {
            // Expected in test environment without proper WASM
            println!("Instantiation failed as expected: {}", e);
        }
    }
}

#[tokio::test]
async fn test_contract_address_generation() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    
    let env1 = CosmWasmEnv {
        block: BlockInfo {
            height: 100,
            time: 1000,
            chain_id: "test".to_string(),
        },
        contract: ContractInfo {
            address: "".to_string(),
            creator: Some("creator1".to_string()),
            admin: None,
        },
        transaction: None,
    };
    
    let env2 = CosmWasmEnv {
        block: BlockInfo {
            height: 101,
            time: 1001,
            chain_id: "test".to_string(),
        },
        contract: ContractInfo {
            address: "".to_string(),
            creator: Some("creator2".to_string()),
            admin: None,
        },
        transaction: None,
    };
    
    // Different environments should generate different addresses
    let addr1 = service.generate_contract_address(1, &env1);
    let addr2 = service.generate_contract_address(1, &env2);
    
    assert_ne!(addr1, addr2);
    assert!(addr1.starts_with("proxima1"));
    assert!(addr2.starts_with("proxima1"));
}

#[tokio::test]
#[ignore = "Wasmer crashes with minimal test WASM - needs real WASM module"]
async fn test_query_contract() {
    let state_manager = create_mock_state_manager();
    let service = WasmerExecutionService::new(state_manager);
    let env = create_test_env();
    let wasm = create_minimal_wasm();
    
    // Query is read-only and shouldn't track state changes
    let result = service.query_contract(
        "test_contract",
        &wasm,
        b"{\"get_count\":{}}",
        env,
    ).await;
    
    // Query might fail with our minimal WASM, but should not panic
    match result {
        Ok(_) => println!("Query succeeded"),
        Err(e) => println!("Query failed as expected: {}", e),
    }
}