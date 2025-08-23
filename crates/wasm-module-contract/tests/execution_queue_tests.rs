/// Tests for execution queue and relayer query features
use wasm_module_contract::WasmModuleContract;
use wasm_module_contract::execution_queue::{ExecutionRequest, ExecutionStatus};
use near_sdk::json_types::Base64VecU8;
use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::{testing_env, env};

fn setup_contract_with_code() -> (WasmModuleContract, u64, String) {
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id(accounts(1))
        .predecessor_account_id(accounts(1));
    testing_env!(context.build());
    
    let mut contract = WasmModuleContract::new(Some(accounts(1)), None);
    
    // Store some test code
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d]; // \0asm
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1
    wasm.extend_from_slice(&[0x00]); // empty module
    
    let store_response = contract.store_code(
        Base64VecU8::from(wasm.clone()),
        Some("test".to_string()),
        None,
        None,
        None,
    );
    
    // Instantiate a test contract
    let instantiate_response = contract.instantiate(
        store_response.code_id,
        "{}".to_string(),
        None,
        "test_contract".to_string(),
        None,
        None,
    );
    
    (contract, store_response.code_id, instantiate_response.address)
}

#[test]
fn test_execute_queues_request() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Test executing a contract (should queue the request)
    let response = contract.execute(
        contract_addr.clone(),
        r#"{"increment": {}}"#.to_string(),
        None,
        None,
    );
    
    // Should return queued response
    assert!(response.data.unwrap().contains("Request queued:"));
    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].r#type, "wasm");
    
    let attributes = &response.events[0].attributes;
    assert_eq!(attributes[0].key, "_contract_address");
    assert_eq!(attributes[0].value, contract_addr);
    assert_eq!(attributes[1].key, "action");
    assert_eq!(attributes[1].value, "execute_queued");
    assert_eq!(attributes[2].key, "request_id");
    assert_eq!(attributes[2].value, "exec_1");
}

#[test]
fn test_get_pending_executions() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Initially no pending executions
    let pending = contract.get_pending_executions(None);
    assert_eq!(pending.len(), 0);
    
    // Execute a contract to create a pending request
    contract.execute(
        contract_addr.clone(),
        r#"{"increment": {}}"#.to_string(),
        None,
        None,
    );
    
    // Now should have one pending execution
    let pending = contract.get_pending_executions(None);
    assert_eq!(pending.len(), 1);
    
    let request = &pending[0];
    assert_eq!(request.contract_address, contract_addr);
    assert_eq!(request.entry_point, "execute");
    assert_eq!(request.status, ExecutionStatus::Pending);
    assert_eq!(request.request_id, "exec_1");
}

#[test]
fn test_get_pending_executions_with_height_filter() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Execute at current height
    contract.execute(
        contract_addr.clone(),
        r#"{"increment": {}}"#.to_string(),
        None,
        None,
    );
    
    let current_height = env::block_height();
    
    // Query with height filter - should return the request
    let pending = contract.get_pending_executions(Some(current_height.saturating_sub(1)));
    assert_eq!(pending.len(), 1);
    
    // Query with height filter that excludes the request
    let pending = contract.get_pending_executions(Some(current_height + 1));
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_get_code_by_id() {
    let (contract, code_id, _contract_addr) = setup_contract_with_code();
    
    // Should be able to get the stored code
    let code = contract.get_code(code_id);
    assert!(code.is_some());
    
    let wasm_bytes = code.unwrap();
    assert!(wasm_bytes.len() > 0);
    assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]); // WASM magic number
    
    // Should return None for non-existent code
    let missing_code = contract.get_code(999);
    assert!(missing_code.is_none());
}

#[test]
fn test_update_execution_status_as_relayer() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Execute to create a pending request
    let response = contract.execute(
        contract_addr.clone(),
        r#"{"increment": {}}"#.to_string(),
        None,
        None,
    );
    
    // Extract request_id from response
    let request_id = response.events[0].attributes
        .iter()
        .find(|attr| attr.key == "request_id")
        .unwrap()
        .value
        .clone();
    
    // Update status as relayer (using owner for this test)
    contract.update_execution_status(
        request_id.clone(),
        ExecutionStatus::Processing,
    );
    
    // Verify status was updated
    let pending = contract.get_pending_executions(None);
    assert_eq!(pending.len(), 0); // No longer pending
}

#[test]
#[should_panic(expected = "Unauthorized: caller is not an authorized relayer")]
fn test_update_execution_status_unauthorized() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Execute to create a pending request
    let response = contract.execute(
        contract_addr.clone(),
        r#"{"increment": {}}"#.to_string(),
        None,
        None,
    );
    
    let request_id = response.events[0].attributes
        .iter()
        .find(|attr| attr.key == "request_id")
        .unwrap()
        .value
        .clone();
    
    // Change context to unauthorized user
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id(accounts(2)) // Different account
        .predecessor_account_id(accounts(2));
    testing_env!(context.build());
    
    // Should fail - not authorized
    contract.update_execution_status(
        request_id,
        ExecutionStatus::Processing,
    );
}

#[test]
fn test_multiple_pending_executions() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Execute multiple times to create multiple pending requests
    for i in 0..5 {
        contract.execute(
            contract_addr.clone(),
            format!(r#"{{"increment": {}}}"#, i),
            None,
            None,
        );
    }
    
    // Should have 5 pending executions
    let pending = contract.get_pending_executions(None);
    assert_eq!(pending.len(), 5);
    
    // Each should be unique
    let mut request_ids: Vec<String> = pending.iter().map(|r| r.request_id.clone()).collect();
    request_ids.sort();
    request_ids.dedup();
    assert_eq!(request_ids.len(), 5); // All unique
}

#[test]
fn test_execution_queue_pagination() {
    let (mut contract, _code_id, contract_addr) = setup_contract_with_code();
    
    // Execute 15 times (more than the 10 limit)
    for i in 0..15 {
        contract.execute(
            contract_addr.clone(),
            format!(r#"{{"increment": {}}}"#, i),
            None,
            None,
        );
    }
    
    // Should only return 10 (pagination limit)
    let pending = contract.get_pending_executions(None);
    assert_eq!(pending.len(), 10);
}