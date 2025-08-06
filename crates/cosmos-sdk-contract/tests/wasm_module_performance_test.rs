/// CosmWasm Module Performance Tests
/// 
/// Tests the performance characteristics and scalability of the x/wasm module
/// with bulk operations, concurrent access, and large datasets.

use anyhow::Result;
use near_workspaces::{Account, Contract, Worker};
use serde_json::json;
use std::time::{Duration, Instant};

const WASM_FILEPATH: &str = "./target/near/cosmos_sdk_near.wasm";

/// Deploy the Cosmos SDK contract to local sandbox
async fn deploy_cosmos_contract(worker: &Worker<near_workspaces::network::Sandbox>) -> Result<Contract> {
    let wasm = std::fs::read(WASM_FILEPATH)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM file. Run 'cargo near build' first"))?;
    
    let contract = worker.dev_deploy(&wasm).await?;

    // Initialize the main contract
    contract
        .call("new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok(contract)
}

/// Create a test account
async fn create_test_account(worker: &Worker<near_workspaces::network::Sandbox>, name: &str) -> Result<Account> {
    let account = worker
        .create_tla(name.parse()?, near_workspaces::types::SecretKey::from_random(near_workspaces::types::KeyType::ED25519))
        .await?
        .result;
    Ok(account)
}

/// Measure execution time of an async operation
async fn measure_time<F, Fut, T>(operation: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = operation().await;
    let duration = start.elapsed();
    (result, duration)
}

/// Test bulk code storage performance
#[tokio::test]
async fn test_bulk_code_storage_performance() -> Result<()> {
    println!("📊 Testing Bulk Code Storage Performance");
    
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // Test different batch sizes
    let batch_sizes = vec![5, 10, 20];
    
    for batch_size in batch_sizes {
        println!("Testing batch size: {}", batch_size);
        
        let (results, total_time) = measure_time(|| async {
            let mut results = Vec::new();
            let batch_start = Instant::now();
            
            for i in 0..batch_size {
                let code = format!("performance_test_code_{}_batch_{}", i, batch_size).into_bytes();
                
                let (store_result, store_time) = measure_time(|| {
                    admin
                        .call(contract.id(), "wasm_store_code")
                        .args_json(json!({
                            "wasm_byte_code": code,
                            "source": format!("https://github.com/perf/contract{}", i),
                            "builder": "cosmwasm/rust-optimizer:0.12.0",
                            "instantiate_permission": {
                                "everybody": {}
                            }
                        }))
                        .max_gas()
                        .transact()
                }).await;
                
                let result = store_result?;
                assert!(result.is_success());
                
                let code_id: u64 = result.json()?;
                results.push((code_id, store_time));
            }
            
            Ok::<_, anyhow::Error>((results, batch_start.elapsed()))
        }).await;
        
        let (store_results, batch_time) = results?;
        
        println!("Batch {} completed:", batch_size);
        println!("  Total time: {:?}", total_time);
        println!("  Batch processing time: {:?}", batch_time);
        println!("  Average per operation: {:?}", total_time / batch_size as u32);
        println!("  Operations per second: {:.2}", batch_size as f64 / total_time.as_secs_f64());
        
        // Verify all codes were stored successfully
        assert_eq!(store_results.len(), batch_size);
        
        // Check that times are reasonable (less than 10 seconds per operation)
        for (code_id, store_time) in &store_results {
            assert!(*code_id > 0);
            assert!(store_time.as_secs() < 10, "Code storage time {} seconds is too slow", store_time.as_secs());
        }
        
        println!("✅ Batch size {} performance test passed\n", batch_size);
    }
    
    println!("🎉 Bulk code storage performance test completed successfully!");
    Ok(())
}

/// Test bulk contract instantiation performance
#[tokio::test]
async fn test_bulk_instantiation_performance() -> Result<()> {
    println!("🏗️ Testing Bulk Contract Instantiation Performance");
    
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // First, store some codes to use for instantiation
    let num_codes = 3;
    let mut code_ids = Vec::new();
    
    println!("Setting up {} codes for instantiation tests...", num_codes);
    for i in 0..num_codes {
        let code = format!("instantiation_perf_code_{}", i).into_bytes();
        let store_result = admin
            .call(contract.id(), "wasm_store_code")
            .args_json(json!({
                "wasm_byte_code": code,
                "source": format!("https://github.com/perf/inst{}", i),
                "builder": "cosmwasm/rust-optimizer:0.12.0",
                "instantiate_permission": {
                    "everybody": {}
                }
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(store_result.is_success());
        let code_id: u64 = store_result.json()?;
        code_ids.push(code_id);
    }
    
    println!("✅ Setup completed with {} codes", code_ids.len());
    
    // Test different instantiation batch sizes
    let instantiation_batches = vec![5, 10, 15];
    
    for batch_size in instantiation_batches {
        println!("Testing instantiation batch size: {}", batch_size);
        
        let (results, total_time) = measure_time(|| async {
            let mut results = Vec::new();
            
            for i in 0..batch_size {
                let code_id = code_ids[i % code_ids.len()]; // Cycle through available codes
                
                let (instantiate_result, instantiate_time) = measure_time(|| {
                    let msg = serde_json::to_vec(&json!({
                        "batch": batch_size,
                        "instance": i,
                        "timestamp": chrono::Utc::now().timestamp()
                    })).unwrap_or_default();
                    
                    admin
                        .call(contract.id(), "wasm_instantiate")
                        .args_json(json!({
                            "code_id": code_id,
                            "msg": msg,
                            "funds": [],
                            "label": format!("Perf Contract Batch {} Instance {}", batch_size, i),
                            "admin": admin.id()
                        }))
                        .max_gas()
                        .transact()
                }).await;
                
                let result = instantiate_result?;
                assert!(result.is_success());
                
                let response: serde_json::Value = result.json()?;
                let contract_address = response["address"].as_str().unwrap().to_string();
                results.push((contract_address, instantiate_time));
            }
            
            Ok::<_, anyhow::Error>(results)
        }).await;
        
        let instantiate_results = results?;
        
        println!("Instantiation batch {} completed:", batch_size);
        println!("  Total time: {:?}", total_time);
        println!("  Average per instantiation: {:?}", total_time / batch_size as u32);
        println!("  Instantiations per second: {:.2}", batch_size as f64 / total_time.as_secs_f64());
        
        // Verify all contracts were instantiated successfully
        assert_eq!(instantiate_results.len(), batch_size);
        
        // Check that times are reasonable (less than 15 seconds per operation)
        let mut total_instantiate_time = Duration::new(0, 0);
        for (address, instantiate_time) in &instantiate_results {
            assert!(!address.is_empty());
            assert!(instantiate_time.as_secs() < 15, "Contract instantiation time {} seconds is too slow", instantiate_time.as_secs());
            total_instantiate_time += *instantiate_time;
        }
        
        println!("  Total instantiation time: {:?}", total_instantiate_time);
        println!("✅ Instantiation batch size {} performance test passed\n", batch_size);
    }
    
    println!("🎉 Bulk contract instantiation performance test completed successfully!");
    Ok(())
}

/// Test query performance with large datasets
#[tokio::test]
async fn test_query_performance_large_datasets() -> Result<()> {
    println!("🔍 Testing Query Performance with Large Datasets");
    
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // Create a substantial dataset
    let num_codes = 25;
    let contracts_per_code = 4;
    let total_contracts = num_codes * contracts_per_code;
    
    println!("Setting up dataset: {} codes, {} contracts per code = {} total contracts", 
             num_codes, contracts_per_code, total_contracts);
    
    let (setup_results, setup_time) = measure_time(|| async {
        let mut code_ids = Vec::new();
        let mut contract_addresses = Vec::new();
        
        // Store codes
        for i in 0..num_codes {
            let code = format!("query_perf_code_{}", i).into_bytes();
            let store_result = admin
                .call(contract.id(), "wasm_store_code")
                .args_json(json!({
                    "wasm_byte_code": code,
                    "source": format!("https://github.com/query-perf/code{}", i),
                    "builder": "cosmwasm/rust-optimizer:0.12.0",
                    "instantiate_permission": {
                        "everybody": {}
                    }
                }))
                .max_gas()
                .transact()
                .await?;
            
            assert!(store_result.is_success());
            let code_id: u64 = store_result.json()?;
            code_ids.push(code_id);
            
            // Instantiate contracts for this code
            for j in 0..contracts_per_code {
                let instantiate_result = admin
                    .call(contract.id(), "wasm_instantiate")
                    .args_json(json!({
                        "code_id": code_id,
                        "msg": serde_json::to_vec(&json!({
                            "code_index": i,
                            "contract_index": j
                        }))?,
                        "funds": [],
                        "label": format!("Query Perf Contract {}-{}", i, j),
                        "admin": admin.id()
                    }))
                    .max_gas()
                    .transact()
                    .await?;
                
                assert!(instantiate_result.is_success());
                let response: serde_json::Value = instantiate_result.json()?;
                let address = response["address"].as_str().unwrap().to_string();
                contract_addresses.push(address);
            }
        }
        
        Ok::<_, anyhow::Error>((code_ids, contract_addresses))
    }).await;
    
    let (code_ids, contract_addresses) = setup_results?;
    println!("✅ Dataset setup completed in {:?}", setup_time);
    println!("   Stored {} codes and instantiated {} contracts", code_ids.len(), contract_addresses.len());
    
    // Test listing all codes performance
    println!("Testing list_codes performance...");
    let (list_codes_result, list_codes_time) = measure_time(|| async {
        contract
            .view("wasm_list_codes")
            .args_json(json!({
                "start_after": null,
                "limit": 100
            }))
            .await
    }).await;
    
    let all_codes: Vec<serde_json::Value> = list_codes_result?.json()?;
    println!("  Listed {} codes in {:?}", all_codes.len(), list_codes_time);
    assert_eq!(all_codes.len(), num_codes);
    assert!(list_codes_time.as_secs() < 5, "List codes query too slow: {:?}", list_codes_time);
    
    // Test paginated listing performance
    println!("Testing paginated listing performance...");
    let page_size = 10;
    let expected_pages = (num_codes + page_size - 1) / page_size; // Ceiling division
    
    let (paginated_results, paginated_time) = measure_time(|| async {
        let mut all_paginated_codes = Vec::new();
        let mut start_after = None;
        let mut page_count = 0;
        
        loop {
            let page_result = contract
                .view("wasm_list_codes")
                .args_json(json!({
                    "start_after": start_after,
                    "limit": page_size
                }))
                .await?;
            
            let page_codes: Vec<serde_json::Value> = page_result.json()?;
            
            if page_codes.is_empty() {
                break;
            }
            
            // Set start_after to the last code_id of current page
            if let Some(last_code) = page_codes.last() {
                start_after = Some(last_code["code_id"].as_u64().unwrap());
            }
            
            all_paginated_codes.extend(page_codes);
            page_count += 1;
            
            // Safety check to prevent infinite loops
            if page_count > expected_pages + 2 {
                break;
            }
        }
        
        Ok::<_, anyhow::Error>((all_paginated_codes, page_count))
    }).await;
    
    let (paginated_codes, page_count) = paginated_results?;
    println!("  Paginated through {} codes in {} pages in {:?}", 
             paginated_codes.len(), page_count, paginated_time);
    assert!(paginated_codes.len() >= num_codes);
    assert!(paginated_time.as_secs() < 10, "Paginated listing too slow: {:?}", paginated_time);
    
    // Test contracts-by-code queries for different codes
    println!("Testing contracts-by-code queries...");
    let mut total_query_time = Duration::new(0, 0);
    
    for (i, &code_id) in code_ids.iter().take(5).enumerate() { // Test first 5 codes
        let (contracts_result, query_time) = measure_time(|| async {
            contract
                .view("wasm_list_contracts_by_code")
                .args_json(json!({
                    "code_id": code_id,
                    "start_after": null,
                    "limit": 10
                }))
                .await
        }).await;
        
        let contracts: Vec<serde_json::Value> = contracts_result?.json()?;
        total_query_time += query_time;
        
        println!("  Code {} has {} contracts (queried in {:?})", 
                 code_id, contracts.len(), query_time);
        assert_eq!(contracts.len(), contracts_per_code);
        assert!(query_time.as_secs() < 3, "Individual contract query too slow: {:?}", query_time);
    }
    
    println!("  Total query time for 5 codes: {:?}", total_query_time);
    println!("  Average query time: {:?}", total_query_time / 5);
    
    // Test individual code info queries
    println!("Testing individual code info queries...");
    let (code_info_results, code_info_time) = measure_time(|| async {
        let mut code_infos = Vec::new();
        
        for &code_id in code_ids.iter().take(10) { // Test first 10 codes
            let info_result = contract
                .view("wasm_code_info")
                .args_json(json!({
                    "code_id": code_id
                }))
                .await?;
            
            let info: Option<serde_json::Value> = info_result.json()?;
            code_infos.push(info);
        }
        
        Ok::<_, anyhow::Error>(code_infos)
    }).await;
    
    let code_infos = code_info_results?;
    println!("  Retrieved {} code infos in {:?}", code_infos.len(), code_info_time);
    println!("  Average per code info: {:?}", code_info_time / code_infos.len() as u32);
    
    // Verify all code infos were retrieved
    for info in &code_infos {
        assert!(info.is_some());
    }
    
    assert!(code_info_time.as_secs() < 5, "Code info queries too slow: {:?}", code_info_time);
    
    println!("🎉 Query performance test completed successfully!");
    println!("📈 Performance Summary:");
    println!("  - Dataset: {} codes, {} contracts", num_codes, total_contracts);
    println!("  - Setup time: {:?}", setup_time);
    println!("  - List all codes: {:?}", list_codes_time);
    println!("  - Paginated listing: {:?}", paginated_time);
    println!("  - Contract queries (avg): {:?}", total_query_time / 5);
    println!("  - Code info queries (avg): {:?}", code_info_time / code_infos.len() as u32);
    
    Ok(())
}

/// Test concurrent access patterns (simulated)
#[tokio::test]
async fn test_concurrent_access_simulation() -> Result<()> {
    println!("🔄 Testing Concurrent Access Patterns");
    
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    
    // Create multiple accounts to simulate concurrent users
    let num_users = 5;
    let mut users = Vec::new();
    
    for i in 0..num_users {
        let user = create_test_account(&worker, &format!("user{}", i)).await?;
        users.push(user);
    }
    
    println!("Created {} concurrent users", users.len());
    
    // Each user stores their own code
    println!("Testing concurrent code storage...");
    let (concurrent_storage_results, concurrent_storage_time) = measure_time(|| async {
        let mut handles = Vec::new();
        
        for (i, user) in users.iter().enumerate() {
            let user = user.clone();
            let contract_id = contract.id().clone();
            
            let handle = tokio::spawn(async move {
                let code = format!("concurrent_code_user_{}", i).into_bytes();
                
                let store_result = user
                    .call(&contract_id, "wasm_store_code")
                    .args_json(json!({
                        "wasm_byte_code": code,
                        "source": format!("https://github.com/concurrent/user{}", i),
                        "builder": "cosmwasm/rust-optimizer:0.12.0",
                        "instantiate_permission": {
                            "everybody": {}
                        }
                    }))
                    .max_gas()
                    .transact()
                    .await;
                
                match store_result {
                    Ok(result) => {
                        if result.is_success() {
                            Ok(result.json::<u64>().unwrap_or(0))
                        } else {
                            Err(anyhow::anyhow!("Store failed for user {}", i))
                        }
                    }
                    Err(e) => Err(e.into())
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all concurrent operations to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => return Err(anyhow::anyhow!("Concurrent task failed: {}", e)),
            }
        }
        
        Ok::<_, anyhow::Error>(results)
    }).await;
    
    let storage_results = concurrent_storage_results?;
    println!("  Concurrent storage completed in {:?}", concurrent_storage_time);
    println!("  All {} users stored code successfully", storage_results.len());
    
    // Verify all operations succeeded and returned unique code IDs
    assert_eq!(storage_results.len(), num_users);
    for result in &storage_results {
        assert!(result.is_ok());
        let code_id = result.as_ref().unwrap();
        assert!(*code_id > 0);
    }
    
    // Check that all code IDs are unique
    let mut code_ids: Vec<u64> = storage_results.iter().map(|r| *r.as_ref().unwrap()).collect();
    code_ids.sort();
    code_ids.dedup();
    assert_eq!(code_ids.len(), num_users, "Some code IDs were not unique");
    
    println!("✅ Concurrent code storage test passed");
    
    // Test concurrent contract instantiation
    println!("Testing concurrent contract instantiation...");
    let first_code_id = code_ids[0];
    
    let (concurrent_instantiation_results, concurrent_instantiation_time) = measure_time(|| async {
        let mut handles = Vec::new();
        
        for (i, user) in users.iter().enumerate() {
            let user = user.clone();
            let contract_id = contract.id().clone();
            
            let handle = tokio::spawn(async move {
                let instantiate_result = user
                    .call(&contract_id, "wasm_instantiate")
                    .args_json(json!({
                        "code_id": first_code_id,
                        "msg": serde_json::to_vec(&json!({
                            "user_id": i,
                            "concurrent_test": true
                        })).unwrap(),
                        "funds": [],
                        "label": format!("Concurrent Contract User {}", i),
                        "admin": user.id()
                    }))
                    .max_gas()
                    .transact()
                    .await;
                
                match instantiate_result {
                    Ok(result) => {
                        if result.is_success() {
                            let response: serde_json::Value = result.json().unwrap();
                            Ok(response["address"].as_str().unwrap().to_string())
                        } else {
                            Err(anyhow::anyhow!("Instantiate failed for user {}", i))
                        }
                    }
                    Err(e) => Err(e.into())
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all concurrent instantiations
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => return Err(anyhow::anyhow!("Concurrent instantiation task failed: {}", e)),
            }
        }
        
        Ok::<_, anyhow::Error>(results)
    }).await;
    
    let instantiation_results = concurrent_instantiation_results?;
    println!("  Concurrent instantiation completed in {:?}", concurrent_instantiation_time);
    println!("  All {} users instantiated contracts successfully", instantiation_results.len());
    
    // Verify all instantiations succeeded and returned unique addresses
    assert_eq!(instantiation_results.len(), num_users);
    for result in &instantiation_results {
        assert!(result.is_ok());
        let address = result.as_ref().unwrap();
        assert!(!address.is_empty());
    }
    
    // Check that all contract addresses are unique
    let mut addresses: Vec<String> = instantiation_results.iter().map(|r| r.as_ref().unwrap().clone()).collect();
    addresses.sort();
    addresses.dedup();
    assert_eq!(addresses.len(), num_users, "Some contract addresses were not unique");
    
    println!("✅ Concurrent contract instantiation test passed");
    
    println!("🎉 Concurrent access simulation completed successfully!");
    println!("📊 Concurrent Performance Summary:");
    println!("  - Users: {}", num_users);
    println!("  - Concurrent storage time: {:?}", concurrent_storage_time);
    println!("  - Concurrent instantiation time: {:?}", concurrent_instantiation_time);
    println!("  - Average storage time per user: {:?}", concurrent_storage_time / num_users as u32);
    println!("  - Average instantiation time per user: {:?}", concurrent_instantiation_time / num_users as u32);
    
    Ok(())
}