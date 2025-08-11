/// Integration Tests for Standalone WASM Module Contract
/// 
/// Tests the x/wasm module functionality as a separate deployed contract
/// that integrates with the router contract.

use anyhow::Result;
use near_workspaces::{Account, Contract, Worker};
use serde_json::json;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

const ROUTER_WASM: &str = "../cosmos-sdk-contract/target/near/cosmos_sdk_contract.wasm";
const WASM_MODULE_WASM: &str = "./target/near/wasm_module_contract.wasm";

/// Deploy the router contract to local sandbox
async fn deploy_router(worker: &Worker<near_workspaces::network::Sandbox>) -> Result<Contract> {
    let wasm = std::fs::read(ROUTER_WASM)
        .map_err(|_| anyhow::anyhow!("Failed to read router WASM. Run 'cargo near build' in cosmos-sdk-contract first"))?;
    
    let contract = worker.dev_deploy(&wasm).await?;

    // Initialize the router
    let init_result = contract
        .call("new")
        .max_gas()
        .transact()
        .await?;
    
    if !init_result.is_success() {
        return Err(anyhow::anyhow!("Router initialization failed"));
    }
    println!("✅ Router contract deployed: {}", contract.id());

    Ok(contract)
}

/// Deploy the wasm module contract to local sandbox
async fn deploy_wasm_module(
    worker: &Worker<near_workspaces::network::Sandbox>,
    router_id: &str,
) -> Result<Contract> {
    let wasm = std::fs::read(WASM_MODULE_WASM)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM module. Run 'cargo near build' first"))?;
    
    let contract = worker.dev_deploy(&wasm).await?;

    // Initialize with router reference
    let init_result = contract
        .call("new")
        .args_json(json!({
            "owner": contract.id(),
            "router_contract": router_id
        }))
        .max_gas()
        .transact()
        .await?;
    
    if !init_result.is_success() {
        return Err(anyhow::anyhow!("WASM module initialization failed"));
    }
    println!("✅ WASM module deployed: {}", contract.id());

    Ok(contract)
}

/// Register wasm module with router
async fn register_module_with_router(
    router: &Contract,
    wasm_module_id: &str,
    caller: &Account,
) -> Result<()> {
    let result = caller
        .call(router.id(), "register_module")
        .args_json(json!({
            "module_type": "wasm",
            "contract_id": wasm_module_id,
            "version": "0.1.0"
        }))
        .max_gas()
        .transact()
        .await?;
    
    if !result.is_success() {
        return Err(anyhow::anyhow!("Failed to register wasm module with router"));
    }
    println!("✅ WASM module registered with router");
    Ok(())
}

/// Create a test account
async fn create_test_account(worker: &Worker<near_workspaces::network::Sandbox>, name: &str) -> Result<Account> {
    let account = worker
        .create_tla(name.parse()?, near_workspaces::types::SecretKey::from_random(near_workspaces::types::KeyType::ED25519))
        .await?
        .result;
    Ok(account)
}

// =============================================================================
// Tests
// =============================================================================

#[tokio::test]
async fn test_wasm_module_deployment_and_health() -> Result<()> {
    println!("🧪 Testing WASM Module Deployment and Health Check");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    
    // Register module with router
    register_module_with_router(&router, wasm_module.id().as_str(), &router.as_account()).await?;
    
    // Check wasm module health
    let health_result = wasm_module
        .view("health_check")
        .await?;
    
    let health: serde_json::Value = health_result.json()?;
    assert_eq!(health["status"], "healthy");
    assert_eq!(health["module"], "x/wasm");
    println!("✅ Health check response: {:?}", health);
    
    // Check module metadata
    let metadata_result = wasm_module
        .view("get_metadata")
        .await?;
    
    let metadata: serde_json::Value = metadata_result.json()?;
    assert_eq!(metadata["name"], "CosmWasm x/wasm Module");
    println!("✅ Metadata: {:?}", metadata);
    
    // Verify router has the module registered
    let modules_result = router
        .view("get_modules")
        .await?;
    
    let modules: serde_json::Value = modules_result.json()?;
    assert!(modules.as_object().unwrap().contains_key("wasm"));
    println!("✅ Router has wasm module registered");
    
    println!("🎉 Deployment and health test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_code_storage() -> Result<()> {
    println!("🧪 Testing WASM Module Code Storage");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    
    // Use the wasm module account itself as it's the owner
    let admin = wasm_module.as_account();
    
    // Store WASM code
    // Using valid WASM magic number and minimal valid structure
    let mock_wasm_code = vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Minimal valid WASM module
    ];
    
    // Convert to Base64 for NEAR SDK's Base64VecU8 type
    let wasm_base64 = BASE64.encode(&mock_wasm_code);
    
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": wasm_base64,
            "source": "https://github.com/example/test-contract",
            "builder": "cosmwasm/rust-optimizer:0.12.0",
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    if !store_result.is_success() {
        println!("Store code failed!");
        println!("Logs: {:?}", store_result.logs());
        println!("Failures: {:?}", store_result.failures());
        panic!("Store code failed");
    }
    
    let response: serde_json::Value = store_result.json()?;
    let code_id = response["code_id"].as_u64().unwrap();
    assert_eq!(code_id, 1, "First code should have ID 1");
    assert!(response["checksum"].is_string(), "Should return checksum");
    println!("✅ Stored WASM code with ID: {} and checksum: {}", code_id, response["checksum"]);
    
    // Verify code info
    let code_info_result = wasm_module
        .view("get_code_info")
        .args_json(json!({
            "code_id": code_id
        }))
        .await?;
    
    let code_info: Option<serde_json::Value> = code_info_result.json()?;
    assert!(code_info.is_some());
    let info = code_info.unwrap();
    assert_eq!(info["code_id"], code_id);
    assert_eq!(info["creator"], admin.id().to_string());
    println!("✅ Retrieved code info: {:?}", info);
    
    println!("🎉 Code storage test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_instantiation() -> Result<()> {
    println!("🧪 Testing WASM Module Contract Instantiation");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let admin = wasm_module.as_account();
    
    // First store code
    let mock_wasm_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let wasm_base64 = BASE64.encode(&mock_wasm_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": wasm_base64,
            "source": "test",
            "builder": "test",
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let store_response: serde_json::Value = store_result.json()?;
    let code_id = store_response["code_id"].as_u64().unwrap();
    
    // Instantiate contract
    let init_msg = json!({
        "count": 0,
        "owner": admin.id()
    });
    
    let instantiate_result = admin
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": code_id,
            "msg": serde_json::to_string(&init_msg)?,
            "funds": null,
            "label": "Test Counter Contract",
            "admin": admin.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(instantiate_result.is_success(), "Instantiation failed: {:?}", instantiate_result.logs());
    
    let inst_response: serde_json::Value = instantiate_result.json()?;
    let contract_addr = inst_response["address"].as_str().unwrap();
    assert!(contract_addr.starts_with("contract1."));
    println!("✅ Instantiated contract at: {}", contract_addr);
    
    // Verify contract info
    let contract_info_result = wasm_module
        .view("get_contract_info")
        .args_json(json!({
            "contract_addr": contract_addr
        }))
        .await?;
    
    let contract_info: Option<serde_json::Value> = contract_info_result.json()?;
    assert!(contract_info.is_some());
    let info = contract_info.unwrap();
    assert_eq!(info["address"], contract_addr);
    assert_eq!(info["code_id"], code_id);
    assert_eq!(info["label"], "Test Counter Contract");
    println!("✅ Retrieved contract info: {:?}", info);
    
    println!("🎉 Contract instantiation test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_execution_and_query() -> Result<()> {
    println!("🧪 Testing WASM Module Execution and Query");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let admin = wasm_module.as_account();
    
    // Store and instantiate
    let mock_wasm_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let wasm_base64 = BASE64.encode(&mock_wasm_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": wasm_base64,
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let code_id = store_result.json::<serde_json::Value>()?["code_id"].as_u64().unwrap();
    
    let instantiate_result = admin
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": code_id,
            "msg": "{\"count\": 0}",
            "label": "Execution Test Contract",
            "admin": admin.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    let contract_addr = instantiate_result.json::<serde_json::Value>()?["address"]
        .as_str()
        .unwrap()
        .to_string();
    
    // Execute contract
    let execute_msg = json!({
        "increment": {}
    });
    
    let execute_result = admin
        .call(wasm_module.id(), "execute")
        .args_json(json!({
            "contract_addr": contract_addr,
            "msg": serde_json::to_string(&execute_msg)?,
            "funds": null
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(execute_result.is_success());
    let exec_response: serde_json::Value = execute_result.json()?;
    assert!(exec_response["data"].is_string());
    assert!(!exec_response["events"].as_array().unwrap().is_empty());
    println!("✅ Executed contract: {:?}", exec_response);
    
    // Query contract
    let query_msg = json!({
        "get_count": {}
    });
    
    let query_result = wasm_module
        .view("query")
        .args_json(json!({
            "contract_addr": contract_addr,
            "msg": serde_json::to_string(&query_msg)?
        }))
        .await?;
    
    let query_response: String = query_result.json()?;
    assert!(query_response.contains("query_result"));
    println!("✅ Query response: {}", query_response);
    
    println!("🎉 Execution and query test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_permissions() -> Result<()> {
    println!("🧪 Testing WASM Module Permissions");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let admin = wasm_module.as_account();
    let user1 = create_test_account(&worker, "user1").await?;
    let user2 = create_test_account(&worker, "user2").await?;
    
    // Test 1: OnlyAddress permission
    println!("Testing OnlyAddress permission...");
    let restricted_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let restricted_base64 = BASE64.encode(&restricted_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": restricted_base64,
            "instantiate_permission": {
                "only_address": {
                    "address": user1.id().to_string()
                }
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let restricted_code_id = store_result.json::<serde_json::Value>()?["code_id"].as_u64().unwrap();
    
    // user1 should succeed
    let user1_result = user1
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": restricted_code_id,
            "msg": "{}",
            "label": "User1 Contract"
        }))
        .max_gas()
        .transact()
        .await?;
    
    if !user1_result.is_success() {
        println!("User1 instantiate failed!");
        println!("Logs: {:?}", user1_result.logs());
        println!("Failures: {:?}", user1_result.failures());
    }
    assert!(user1_result.is_success());
    println!("✅ User1 can instantiate restricted code");
    
    // user2 should fail
    let user2_result = user2
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": restricted_code_id,
            "msg": "{}",
            "label": "Should Fail"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!user2_result.is_success());
    println!("✅ User2 cannot instantiate restricted code");
    
    // Test 2: Nobody permission
    println!("Testing Nobody permission...");
    let nobody_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let nobody_base64 = BASE64.encode(&nobody_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": nobody_base64,
            "instantiate_permission": {
                "nobody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let nobody_code_id = store_result.json::<serde_json::Value>()?["code_id"].as_u64().unwrap();
    
    // Even admin should fail
    let admin_result = admin
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": nobody_code_id,
            "msg": "{}",
            "label": "Should Fail"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!admin_result.is_success());
    println!("✅ Nobody can instantiate 'nobody' permission code");
    
    // Test 3: AnyOfAddresses permission
    println!("Testing AnyOfAddresses permission...");
    let any_of_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let any_of_base64 = BASE64.encode(&any_of_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": any_of_base64,
            "instantiate_permission": {
                "any_of_addresses": {
                    "addresses": [user1.id().to_string(), user2.id().to_string()]
                }
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let any_of_code_id = store_result.json::<serde_json::Value>()?["code_id"].as_u64().unwrap();
    
    // Both user1 and user2 should succeed
    let user1_result = user1
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": "{}",
            "label": "User1 AnyOf"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(user1_result.is_success());
    
    let user2_result = user2
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": "{}",
            "label": "User2 AnyOf"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(user2_result.is_success());
    println!("✅ Both allowed users can instantiate");
    
    // Admin should fail
    let admin_result = admin
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": "{}",
            "label": "Should Fail"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!admin_result.is_success());
    println!("✅ Non-allowed user cannot instantiate");
    
    println!("🎉 Permissions test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_listing_and_pagination() -> Result<()> {
    println!("🧪 Testing WASM Module Listing and Pagination");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let admin = wasm_module.as_account();
    
    // Store multiple codes
    let mut code_ids = Vec::new();
    for i in 0..5 {
        let code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, i];
        let code_base64 = BASE64.encode(&code);
        let store_result = admin
            .call(wasm_module.id(), "store_code")
            .args_json(json!({
                "wasm_byte_code": code_base64,
                "source": format!("test-{}", i),
                "instantiate_permission": {
                    "everybody": {}
                }
            }))
            .max_gas()
            .transact()
            .await?;
        
        let code_id = store_result.json::<serde_json::Value>()?["code_id"].as_u64().unwrap();
        code_ids.push(code_id);
    }
    println!("✅ Stored {} codes", code_ids.len());
    
    // List all codes
    let all_codes_result = wasm_module
        .view("list_codes")
        .args_json(json!({
            "limit": 10,
            "start_after": null
        }))
        .await?;
    
    let all_codes: Vec<serde_json::Value> = all_codes_result.json()?;
    assert_eq!(all_codes.len(), 5);
    println!("✅ Listed all {} codes", all_codes.len());
    
    // Test pagination
    let page1_result = wasm_module
        .view("list_codes")
        .args_json(json!({
            "limit": 2,
            "start_after": null
        }))
        .await?;
    
    let page1: Vec<serde_json::Value> = page1_result.json()?;
    assert_eq!(page1.len(), 2);
    
    let last_code_id = page1.last().unwrap()["code_id"].as_u64().unwrap();
    
    let page2_result = wasm_module
        .view("list_codes")
        .args_json(json!({
            "limit": 2,
            "start_after": last_code_id
        }))
        .await?;
    
    let page2: Vec<serde_json::Value> = page2_result.json()?;
    assert_eq!(page2.len(), 2);
    println!("✅ Pagination works correctly");
    
    // Instantiate some contracts
    let mut contract_addrs = Vec::new();
    for i in 0..3 {
        let instantiate_result = admin
            .call(wasm_module.id(), "instantiate")
            .args_json(json!({
                "code_id": code_ids[0],
                "msg": format!("{{\"id\": {}}}", i),
                "label": format!("Contract {}", i)
            }))
            .max_gas()
            .transact()
            .await?;
        
        let addr = instantiate_result.json::<serde_json::Value>()?["address"]
            .as_str()
            .unwrap()
            .to_string();
        contract_addrs.push(addr);
    }
    println!("✅ Instantiated {} contracts", contract_addrs.len());
    
    // List contracts
    let contracts_result = wasm_module
        .view("list_contracts")
        .args_json(json!({
            "limit": 10,
            "start_after": null
        }))
        .await?;
    
    let contracts: Vec<serde_json::Value> = contracts_result.json()?;
    assert_eq!(contracts.len(), 3);
    println!("✅ Listed all {} contracts", contracts.len());
    
    println!("🎉 Listing and pagination test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_admin_functions() -> Result<()> {
    println!("🧪 Testing WASM Module Admin Functions");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let new_owner = create_test_account(&worker, "newowner").await?;
    
    // As the contract owner (itself initially)
    let owner = wasm_module.as_account();
    
    // Test updating max code size
    let update_size_result = owner
        .call(wasm_module.id(), "update_max_code_size")
        .args_json(json!({
            "new_size": 5_000_000  // 5MB
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(update_size_result.is_success());
    println!("✅ Updated max code size");
    
    // Verify in health check
    let health_result = wasm_module
        .view("health_check")
        .await?;
    
    let health: serde_json::Value = health_result.json()?;
    assert_eq!(health["max_code_size"], 5_000_000);
    println!("✅ Max code size updated to 5MB");
    
    // Test transfer ownership
    let transfer_result = owner
        .call(wasm_module.id(), "transfer_ownership")
        .args_json(json!({
            "new_owner": new_owner.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(transfer_result.is_success());
    println!("✅ Ownership transferred to {}", new_owner.id());
    
    // Verify new owner in health check
    let health_result = wasm_module
        .view("health_check")
        .await?;
    
    let health: serde_json::Value = health_result.json()?;
    assert_eq!(health["owner"], new_owner.id().to_string());
    println!("✅ New owner confirmed");
    
    // Old owner should not be able to update settings
    let failed_update = owner
        .call(wasm_module.id(), "update_max_code_size")
        .args_json(json!({
            "new_size": 1_000_000
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!failed_update.is_success());
    println!("✅ Old owner cannot update settings");
    
    // New owner should be able to update
    let new_owner_update = new_owner
        .call(wasm_module.id(), "update_max_code_size")
        .args_json(json!({
            "new_size": 4_000_000
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(new_owner_update.is_success());
    println!("✅ New owner can update settings");
    
    println!("🎉 Admin functions test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_wasm_module_error_scenarios() -> Result<()> {
    println!("🧪 Testing WASM Module Error Scenarios");
    
    let worker = near_workspaces::sandbox().await?;
    let router = deploy_router(&worker).await?;
    let wasm_module = deploy_wasm_module(&worker, router.id().as_str()).await?;
    let admin = wasm_module.as_account();
    
    // Test 1: Invalid WASM magic number
    println!("Testing invalid WASM magic number...");
    let invalid_wasm = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let invalid_base64 = BASE64.encode(&invalid_wasm);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": invalid_base64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!store_result.is_success());
    println!("✅ Invalid WASM rejected");
    
    // Test 2: Code too small
    println!("Testing code too small...");
    let tiny_code = vec![0x00];
    let tiny_base64 = BASE64.encode(&tiny_code);
    let store_result = admin
        .call(wasm_module.id(), "store_code")
        .args_json(json!({
            "wasm_byte_code": tiny_base64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!store_result.is_success());
    println!("✅ Too small code rejected");
    
    // Test 3: Instantiate non-existent code
    println!("Testing instantiation of non-existent code...");
    let instantiate_result = admin
        .call(wasm_module.id(), "instantiate")
        .args_json(json!({
            "code_id": 999,
            "msg": "{}",
            "label": "Should Fail"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!instantiate_result.is_success());
    println!("✅ Non-existent code instantiation rejected");
    
    // Test 4: Query non-existent contract
    println!("Testing query of non-existent contract...");
    let query_result = wasm_module
        .view("query")
        .args_json(json!({
            "contract_addr": "nonexistent.contract",
            "msg": "{}"
        }))
        .await;
    
    assert!(query_result.is_err());
    println!("✅ Non-existent contract query rejected");
    
    // Test 5: Execute on non-existent contract
    println!("Testing execution on non-existent contract...");
    let execute_result = admin
        .call(wasm_module.id(), "execute")
        .args_json(json!({
            "contract_addr": "nonexistent.contract",
            "msg": "{}",
            "funds": null
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!execute_result.is_success());
    println!("✅ Non-existent contract execution rejected");
    
    // Test 6: Get info for non-existent code
    let code_info_result = wasm_module
        .view("get_code_info")
        .args_json(json!({
            "code_id": 999
        }))
        .await?;
    
    let code_info: Option<serde_json::Value> = code_info_result.json()?;
    assert!(code_info.is_none());
    println!("✅ Non-existent code info returns None");
    
    // Test 7: Get info for non-existent contract
    let contract_info_result = wasm_module
        .view("get_contract_info")
        .args_json(json!({
            "contract_addr": "nonexistent.contract"
        }))
        .await?;
    
    let contract_info: Option<serde_json::Value> = contract_info_result.json()?;
    assert!(contract_info.is_none());
    println!("✅ Non-existent contract info returns None");
    
    println!("🎉 Error scenarios test completed successfully!");
    Ok(())
}