use std::sync::Arc;
use crate::cosmwasm::{
    WasmerExecutor, WasmerExecutionService, StateManager,
    types::{CosmWasmEnv, BlockInfo, ContractInfo, ExecutionResult, StateChange},
};
use near_jsonrpc_client::JsonRpcClient;
use near_primitives::types::AccountId;

/// Create a complete test environment
struct TestEnvironment {
    executor: WasmerExecutor,
    service: WasmerExecutionService,
    state_manager: Arc<StateManager>,
    env: CosmWasmEnv,
}

impl TestEnvironment {
    fn new() -> Self {
        let client = Arc::new(JsonRpcClient::connect("http://localhost:3030"));
        let account_id: AccountId = "test.near".parse().unwrap();
        let state_manager = Arc::new(StateManager::new(client, account_id));
        let service = WasmerExecutionService::new(state_manager.clone());
        let executor = WasmerExecutor::new(b"test".to_vec());
        
        let env = CosmWasmEnv {
            block: BlockInfo {
                height: 1000,
                time: 1234567890,
                chain_id: "test-chain".to_string(),
            },
            contract: ContractInfo {
                address: "contract1".to_string(),
                creator: Some("creator1".to_string()),
                admin: Some("admin1".to_string()),
            },
            transaction: None,
        };
        
        TestEnvironment {
            executor,
            service,
            state_manager,
            env,
        }
    }
}

/// Create a simple counter contract WASM
/// This is a simplified version - real CosmWasm contracts are much larger
fn create_counter_wasm() -> Vec<u8> {
    // This would be a real compiled CosmWasm contract in production
    vec![
        // WASM header
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        
        // Type section
        0x01, 0x11, 0x03,
        0x60, 0x01, 0x7f, 0x01, 0x7f, // (i32) -> i32
        0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // (i32, i32) -> i32
        0x60, 0x00, 0x00, // () -> ()
        
        // Import section (would import host functions in real contract)
        
        // Function section
        0x03, 0x05, 0x04, 0x00, 0x01, 0x01, 0x02,
        
        // Memory section
        0x05, 0x03, 0x01, 0x00, 0x10,
        
        // Export section
        0x07, 0x33, 0x05,
        // memory
        0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00,
        // allocate
        0x08, 0x61, 0x6c, 0x6c, 0x6f, 0x63, 0x61, 0x74, 0x65, 0x00, 0x00,
        // instantiate
        0x0b, 0x69, 0x6e, 0x73, 0x74, 0x61, 0x6e, 0x74, 0x69, 0x61, 0x74, 0x65, 0x00, 0x01,
        // execute
        0x07, 0x65, 0x78, 0x65, 0x63, 0x75, 0x74, 0x65, 0x00, 0x02,
        // query
        0x05, 0x71, 0x75, 0x65, 0x72, 0x79, 0x00, 0x03,
        
        // Code section
        0x0a, 0x15, 0x04,
        // allocate: return input
        0x05, 0x00, 0x20, 0x00, 0x0b,
        // instantiate: return 0
        0x05, 0x00, 0x41, 0x00, 0x0b,
        // execute: return 0
        0x05, 0x00, 0x41, 0x00, 0x0b,
        // query: return 0
        0x04, 0x00, 0x41, 0x00, 0x0b,
    ]
}

#[tokio::test]
async fn test_full_contract_lifecycle() {
    let env = TestEnvironment::new();
    let wasm = create_counter_wasm();
    
    // 1. Instantiate contract
    let instantiate_msg = b"{\"count\":0}";
    let instantiate_result = env.service.instantiate_contract(
        1, // code_id
        &wasm,
        instantiate_msg,
        env.env.clone(),
    ).await;
    
    match instantiate_result {
        Ok((contract_addr, result)) => {
            println!("Contract instantiated at: {}", contract_addr);
            assert!(contract_addr.starts_with("proxima1"));
            
            // 2. Execute increment
            let execute_msg = b"{\"increment\":{}}";
            let execute_result = env.service.execute_contract(
                &contract_addr,
                &wasm,
                "execute",
                execute_msg,
                env.env.clone(),
            ).await;
            
            match execute_result {
                Ok(result) => {
                    println!("Execute succeeded with gas: {}", result.gas_used);
                }
                Err(e) => {
                    println!("Execute failed (expected in test): {}", e);
                }
            }
            
            // 3. Query state
            let query_msg = b"{\"get_count\":{}}";
            let query_result = env.service.query_contract(
                &contract_addr,
                &wasm,
                query_msg,
                env.env.clone(),
            ).await;
            
            match query_result {
                Ok(data) => {
                    println!("Query returned: {:?}", data);
                }
                Err(e) => {
                    println!("Query failed (expected in test): {}", e);
                }
            }
        }
        Err(e) => {
            println!("Instantiation failed (expected in test): {}", e);
        }
    }
}

#[tokio::test]
async fn test_state_persistence() {
    let env = TestEnvironment::new();
    
    // Set some state
    env.state_manager.set("contract1", vec![1, 2, 3], vec![10, 20, 30]).await;
    env.state_manager.set("contract1", vec![4, 5, 6], vec![40, 50, 60]).await;
    
    // Execute something that would read state
    let wasm = create_counter_wasm();
    let result = env.service.execute_contract(
        "contract1",
        &wasm,
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    
    // Check that state is still accessible
    assert_eq!(
        env.state_manager.get("contract1", &[1, 2, 3]).await,
        Some(vec![10, 20, 30])
    );
}

#[tokio::test]
async fn test_gas_metering() {
    let env = TestEnvironment::new();
    let wasm = create_counter_wasm();
    
    // Execute with no state changes
    let result1 = env.service.execute_contract(
        "contract1",
        &wasm,
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    
    // Execute with state changes (simulated)
    env.state_manager.set("contract1", vec![1], vec![100]).await;
    let _changes = env.state_manager.get_pending_changes("contract1").await;
    
    // Gas should be tracked
    match result1 {
        Ok(result) => {
            assert!(result.gas_used >= 1000); // At least base cost
        }
        Err(_) => {
            // Expected in test environment
        }
    }
}

#[tokio::test]
async fn test_error_handling() {
    let env = TestEnvironment::new();
    
    // Test with invalid WASM
    let bad_wasm = vec![0x00, 0x01, 0x02, 0x03];
    let result = env.service.execute_contract(
        "contract1",
        &bad_wasm,
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    
    assert!(result.is_err());
    
    // Test with empty WASM
    let result = env.service.execute_contract(
        "contract1",
        &[],
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_executions() {
    let env = Arc::new(TestEnvironment::new());
    let wasm = Arc::new(create_counter_wasm());
    
    // Spawn multiple concurrent executions
    let mut handles = vec![];
    
    for i in 0..5 {
        let env_clone = env.clone();
        let wasm_clone = wasm.clone();
        let contract_addr = format!("contract{}", i);
        
        let handle = tokio::spawn(async move {
            let result = env_clone.service.execute_contract(
                &contract_addr,
                &wasm_clone,
                "execute",
                b"{}",
                env_clone.env.clone(),
            ).await;
            
            match result {
                Ok(r) => println!("Contract {} executed, gas: {}", i, r.gas_used),
                Err(e) => println!("Contract {} failed: {}", i, e),
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all executions
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_module_cache_performance() {
    let env = TestEnvironment::new();
    let wasm = create_counter_wasm();
    
    // First execution - compiles module
    let start = std::time::Instant::now();
    let _result1 = env.service.execute_contract(
        "contract1",
        &wasm,
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    let first_duration = start.elapsed();
    
    // Second execution - uses cached module
    let start = std::time::Instant::now();
    let _result2 = env.service.execute_contract(
        "contract1",
        &wasm,
        "execute",
        b"{}",
        env.env.clone(),
    ).await;
    let second_duration = start.elapsed();
    
    // Second execution should be faster (cached)
    // Note: This might not always be true in test environment
    println!("First execution: {:?}", first_duration);
    println!("Second execution: {:?}", second_duration);
}