/// Tests for relayer integration with NEAR contract
use wasm_module_contract::{
    ExecutionResultInput, StateChangeInput, StateOperation, ExecutionEvent,
    WasmModuleContract, ExecuteResponse,
};
use near_sdk::json_types::Base64VecU8;
use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;

fn setup_contract() -> (WasmModuleContract, String) {
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id(accounts(1))
        .predecessor_account_id(accounts(1));
    testing_env!(context.build());
    
    let mut contract = WasmModuleContract::new(Some(accounts(1)), None);
    
    // Store some test code - use valid WASM magic number
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d]; // \0asm
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1
    // Add minimal sections to make it valid
    wasm.extend_from_slice(&[0x00]); // empty module
    
    contract.store_code(
        Base64VecU8::from(wasm),
        None,
        None,
        None,
        None,
    );
    
    // Instantiate a test contract
    let response = contract.instantiate(
        1,
        "{}".to_string(),
        None,
        "test_contract".to_string(),
        None, // admin
        None,
    );
    
    println!("Contract instantiated at: {}", response.address);
    
    (contract, response.address)
}

#[test]
fn test_apply_execution_result_from_owner() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Create execution result
    let execution_result = ExecutionResultInput {
        data: Some(vec![1, 2, 3, 4]),
        state_changes: vec![
            StateChangeInput {
                key: vec![0, 1],
                value: Some(vec![10, 20, 30]),
                operation: StateOperation::Set,
            },
            StateChangeInput {
                key: vec![2, 3],
                value: None,
                operation: StateOperation::Remove,
            },
        ],
        events: vec![
            ExecutionEvent {
                event_type: "custom_event".to_string(),
                attributes: vec![
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ],
            },
        ],
        gas_used: 1000000,
    };
    
    // Apply execution result
    let response = contract.apply_execution_result(
        contract_addr,
        execution_result,
    );
    
    // Verify response
    assert!(response.data.is_some());
    assert_eq!(response.data.unwrap(), "01020304"); // hex encoded
    
    // Check events
    assert!(response.events.len() >= 2);
    assert_eq!(response.events[0].r#type, "wasm");
    
    // Find gas_used attribute
    let gas_attr = response.events[0].attributes.iter()
        .find(|a| a.key == "gas_used");
    assert!(gas_attr.is_some());
    assert_eq!(gas_attr.unwrap().value, "1000000");
}

#[test]
fn test_apply_execution_result_from_relayer() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Change context to relayer account
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id("relayer.near".parse().unwrap())
        .predecessor_account_id("relayer.near".parse().unwrap());
    testing_env!(context.build());
    
    let execution_result = ExecutionResultInput {
        data: None,
        state_changes: vec![
            StateChangeInput {
                key: vec![1, 2, 3],
                value: Some(vec![4, 5, 6]),
                operation: StateOperation::Set,
            },
        ],
        events: vec![],
        gas_used: 500000,
    };
    
    // Should succeed from relayer account
    let response = contract.apply_execution_result(
        contract_addr.clone(),
        execution_result,
    );
    
    assert!(response.data.is_none());
    assert!(!response.events.is_empty());
}

#[test]
#[should_panic(expected = "Unauthorized: caller is not an authorized relayer")]
fn test_apply_execution_result_unauthorized() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Change context to unauthorized account
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id(accounts(2))
        .predecessor_account_id(accounts(2));
    testing_env!(context.build());
    
    let execution_result = ExecutionResultInput {
        data: None,
        state_changes: vec![],
        events: vec![],
        gas_used: 0,
    };
    
    // Should panic for unauthorized account
    contract.apply_execution_result(
        contract_addr.clone(),
        execution_result,
    );
}

#[test]
fn test_get_contract_state() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Apply some state changes
    let execution_result = ExecutionResultInput {
        data: None,
        state_changes: vec![
            StateChangeInput {
                key: vec![1, 2, 3],
                value: Some(vec![10, 20, 30]),
                operation: StateOperation::Set,
            },
            StateChangeInput {
                key: vec![4, 5, 6],
                value: Some(vec![40, 50, 60]),
                operation: StateOperation::Set,
            },
        ],
        events: vec![],
        gas_used: 0,
    };
    
    contract.apply_execution_result(
        contract_addr.clone(),
        execution_result,
    );
    
    // Read state back
    let value1 = contract.get_contract_state(
        contract_addr.clone(),
        vec![1, 2, 3],
    );
    assert_eq!(value1, Some(vec![10, 20, 30]));
    
    let value2 = contract.get_contract_state(
        contract_addr.clone(),
        vec![4, 5, 6],
    );
    assert_eq!(value2, Some(vec![40, 50, 60]));
    
    // Non-existent key
    let value3 = contract.get_contract_state(
        contract_addr.clone(),
        vec![7, 8, 9],
    );
    assert_eq!(value3, None);
}

#[test]
fn test_get_contract_state_batch() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Apply some state changes
    let execution_result = ExecutionResultInput {
        data: None,
        state_changes: vec![
            StateChangeInput {
                key: vec![1],
                value: Some(vec![10]),
                operation: StateOperation::Set,
            },
            StateChangeInput {
                key: vec![2],
                value: Some(vec![20]),
                operation: StateOperation::Set,
            },
            StateChangeInput {
                key: vec![3],
                value: Some(vec![30]),
                operation: StateOperation::Set,
            },
        ],
        events: vec![],
        gas_used: 0,
    };
    
    contract.apply_execution_result(
        contract_addr.clone(),
        execution_result,
    );
    
    // Batch read
    let values = contract.get_contract_state_batch(
        contract_addr.clone(),
        vec![vec![1], vec![2], vec![3], vec![4]],
    );
    
    assert_eq!(values.len(), 4);
    assert_eq!(values[0], Some(vec![10]));
    assert_eq!(values[1], Some(vec![20]));
    assert_eq!(values[2], Some(vec![30]));
    assert_eq!(values[3], None); // Non-existent
}

#[test]
fn test_state_remove_operation() {
    let (mut contract, contract_addr) = setup_contract();
    
    // Set then remove
    let execution_result1 = ExecutionResultInput {
        data: None,
        state_changes: vec![
            StateChangeInput {
                key: vec![1, 2, 3],
                value: Some(vec![10, 20, 30]),
                operation: StateOperation::Set,
            },
        ],
        events: vec![],
        gas_used: 0,
    };
    
    contract.apply_execution_result(
        contract_addr.clone(),
        execution_result1,
    );
    
    // Verify it was set
    let value = contract.get_contract_state(
        contract_addr.clone(),
        vec![1, 2, 3],
    );
    assert_eq!(value, Some(vec![10, 20, 30]));
    
    // Now remove it
    let execution_result2 = ExecutionResultInput {
        data: None,
        state_changes: vec![
            StateChangeInput {
                key: vec![1, 2, 3],
                value: None,
                operation: StateOperation::Remove,
            },
        ],
        events: vec![],
        gas_used: 0,
    };
    
    contract.apply_execution_result(
        contract_addr.clone(),
        execution_result2,
    );
    
    // Verify it was removed
    let value = contract.get_contract_state(
        contract_addr.clone(),
        vec![1, 2, 3],
    );
    assert_eq!(value, None);
}