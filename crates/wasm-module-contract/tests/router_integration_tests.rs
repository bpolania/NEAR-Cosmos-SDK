/// Router Integration Tests for WASM Module
/// 
/// Tests the x/wasm module functionality when called through the router contract.
/// This simulates how the module would be used in production where all calls
/// go through the router's cross-contract call mechanism.

use anyhow::Result;
use near_workspaces::{Account, Contract, Worker};
use serde_json::json;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};

// Response types from the contracts
#[derive(Serialize, Deserialize, Debug)]
pub struct StoreCodeResponse {
    pub code_id: u64,
    pub checksum: String,  // Hex string in JSON
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstantiateResponse {
    pub address: String,
    pub data: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExecuteResponse {
    pub data: Option<String>,
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

const ROUTER_WASM: &str = "../cosmos-sdk-contract/target/near/cosmos_sdk_contract.wasm";
const WASM_MODULE_WASM: &str = "./target/near/wasm_module_contract.wasm";

/// Full deployment setup with router and wasm module
async fn setup_full_deployment(
    worker: &Worker<near_workspaces::network::Sandbox>
) -> Result<(Contract, Contract)> {
    // Deploy router
    let router_wasm = std::fs::read(ROUTER_WASM)
        .map_err(|_| anyhow::anyhow!("Failed to read router WASM"))?;
    let router = worker.dev_deploy(&router_wasm).await?;
    
    router.call("new").max_gas().transact().await?;
    println!("✅ Router deployed: {}", router.id());
    
    // Deploy wasm module
    let wasm_module_wasm = std::fs::read(WASM_MODULE_WASM)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM module"))?;
    let wasm_module = worker.dev_deploy(&wasm_module_wasm).await?;
    
    wasm_module
        .call("new")
        .args_json(json!({
            "owner": wasm_module.id(),
            "router_contract": router.id()
        }))
        .max_gas()
        .transact()
        .await?;
    println!("✅ WASM module deployed: {}", wasm_module.id());
    
    // Register wasm module with router
    router.as_account()
        .call(router.id(), "register_module")
        .args_json(json!({
            "module_type": "wasm",
            "contract_id": wasm_module.id(),
            "version": "0.1.0"
        }))
        .max_gas()
        .transact()
        .await?;
    println!("✅ WASM module registered with router");
    
    Ok((router, wasm_module))
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
// Tests calling WASM module through router
// =============================================================================

#[tokio::test]
async fn test_router_wasm_store_code() -> Result<()> {
    println!("🧪 Testing WASM Code Storage Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, wasm_module) = setup_full_deployment(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // Call wasm_store_code through router
    let mock_wasm_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    
    let store_result = admin
        .call(router.id(), "wasm_store_code")
        .args_json(json!({
            "wasm_byte_code": mock_wasm_code,
            "source": "https://github.com/example/test",
            "builder": "cosmwasm/rust-optimizer:0.12.0",
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(store_result.is_success(), "Store code through router failed: {:?}", store_result.logs());
    
    let response: StoreCodeResponse = store_result.json()?;
    let code_id = response.code_id;
    assert_eq!(code_id, 1);
    println!("✅ Stored code through router with ID: {}", code_id);
    
    // Verify code was actually stored in wasm module
    let code_info_result = wasm_module
        .view("get_code_info")
        .args_json(json!({
            "code_id": code_id
        }))
        .await?;
    
    let code_info: Option<serde_json::Value> = code_info_result.json()?;
    assert!(code_info.is_some());
    println!("✅ Code verified in wasm module: {:?}", code_info);
    
    // Note: Can't verify through router's view function since cross-contract 
    // calls require transactions, not views. The router can only forward
    // mutable calls, not view calls.
    
    println!("🎉 Router code storage test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_router_wasm_instantiate() -> Result<()> {
    println!("🧪 Testing Contract Instantiation Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, wasm_module) = setup_full_deployment(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // First store code through router
    let mock_wasm_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let store_result = admin
        .call(router.id(), "wasm_store_code")
        .args_json(json!({
            "wasm_byte_code": mock_wasm_code,
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let code_id: u64 = store_result.json()?;
    
    // Instantiate through router
    let init_msg = json!({
        "name": "Test Token",
        "symbol": "TEST",
        "decimals": 6,
        "initial_balances": [{
            "address": admin.id(),
            "amount": "1000000"
        }]
    });
    
    let instantiate_result = admin
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": code_id,
            "msg": serde_json::to_vec(&init_msg)?,
            "funds": [],
            "label": "Test Token Contract",
            "admin": admin.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(instantiate_result.is_success(), "Instantiate through router failed: {:?}", instantiate_result.logs());
    
    let response: serde_json::Value = instantiate_result.json()?;
    let contract_addr = response["address"].as_str().unwrap();
    assert!(contract_addr.starts_with("contract"));
    println!("✅ Instantiated contract through router at: {}", contract_addr);
    
    // Verify contract in wasm module
    let contract_info = wasm_module
        .view("get_contract_info")
        .args_json(json!({
            "contract_addr": contract_addr
        }))
        .await?;
    
    let info: Option<serde_json::Value> = contract_info.json()?;
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info["code_id"], code_id);
    assert_eq!(info["label"], "Test Token Contract");
    println!("✅ Contract verified in wasm module");
    
    // Also verify through router
    let router_contract_info = router
        .view("wasm_contract_info")
        .args_json(json!({
            "address": contract_addr
        }))
        .await?;
    
    let router_info: Option<serde_json::Value> = router_contract_info.json()?;
    assert!(router_info.is_some());
    println!("✅ Contract info accessible through router");
    
    println!("🎉 Router instantiation test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_router_wasm_execute() -> Result<()> {
    println!("🧪 Testing Contract Execution Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, _wasm_module) = setup_full_deployment(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    let user = create_test_account(&worker, "user").await?;
    
    // Store and instantiate through router
    let mock_wasm_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let store_result = admin
        .call(router.id(), "wasm_store_code")
        .args_json(json!({
            "wasm_byte_code": mock_wasm_code,
            "instantiate_permission": {
                "everybody": {}
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let code_id: u64 = store_result.json()?;
    
    let instantiate_result = admin
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": code_id,
            "msg": serde_json::to_vec(&json!({
                "name": "Test Token",
                "initial_balance": "1000000"
            }))?,
            "funds": [],
            "label": "Execution Test",
            "admin": admin.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    let response: serde_json::Value = instantiate_result.json()?;
    let contract_addr = response["address"].as_str().unwrap();
    
    // Execute through router
    let execute_msg = json!({
        "transfer": {
            "recipient": user.id(),
            "amount": "100000"
        }
    });
    
    let execute_result = admin
        .call(router.id(), "wasm_execute")
        .args_json(json!({
            "contract_addr": contract_addr,
            "msg": serde_json::to_vec(&execute_msg)?,
            "funds": []
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(execute_result.is_success(), "Execute through router failed: {:?}", execute_result.logs());
    println!("✅ Executed contract through router");
    
    // Query through router
    let query_msg = json!({
        "balance": {
            "address": user.id()
        }
    });
    
    let query_result = router
        .view("wasm_smart_query")
        .args_json(json!({
            "contract_addr": contract_addr,
            "msg": serde_json::to_vec(&query_msg)?
        }))
        .await;
    
    // Query returns mock data but should not error
    assert!(query_result.is_ok());
    println!("✅ Query through router succeeded");
    
    println!("🎉 Router execution test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_router_wasm_listing() -> Result<()> {
    println!("🧪 Testing Listing Functions Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, _wasm_module) = setup_full_deployment(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    
    // Store multiple codes through router
    let mut code_ids = Vec::new();
    for i in 0..3 {
        let code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, i];
        let store_result = admin
            .call(router.id(), "wasm_store_code")
            .args_json(json!({
                "wasm_byte_code": code,
                "source": format!("test-{}", i),
                "instantiate_permission": {
                    "everybody": {}
                }
            }))
            .max_gas()
            .transact()
            .await?;
        
        let code_id: u64 = store_result.json()?;
        code_ids.push(code_id);
    }
    println!("✅ Stored {} codes through router", code_ids.len());
    
    // List codes through router
    let list_codes_result = router
        .view("wasm_list_codes")
        .args_json(json!({
            "start_after": null,
            "limit": 10
        }))
        .await?;
    
    let codes: Vec<serde_json::Value> = list_codes_result.json()?;
    assert_eq!(codes.len(), 3);
    println!("✅ Listed {} codes through router", codes.len());
    
    // Instantiate contracts through router
    let mut contract_addrs = Vec::new();
    for (i, code_id) in code_ids.iter().enumerate() {
        let instantiate_result = admin
            .call(router.id(), "wasm_instantiate")
            .args_json(json!({
                "code_id": code_id,
                "msg": serde_json::to_vec(&json!({"id": i}))?,
                "funds": [],
                "label": format!("Contract {}", i),
                "admin": admin.id()
            }))
            .max_gas()
            .transact()
            .await?;
        
        let response: serde_json::Value = instantiate_result.json()?;
        let addr = response["address"].as_str().unwrap().to_string();
        contract_addrs.push(addr);
    }
    println!("✅ Instantiated {} contracts through router", contract_addrs.len());
    
    // List contracts by code through router
    let list_by_code_result = router
        .view("wasm_list_contracts_by_code")
        .args_json(json!({
            "code_id": code_ids[0],
            "start_after": null,
            "limit": 10
        }))
        .await?;
    
    let contracts: Vec<String> = list_by_code_result.json()?;
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0], contract_addrs[0]);
    println!("✅ Listed contracts by code through router");
    
    println!("🎉 Router listing test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_router_wasm_permissions() -> Result<()> {
    println!("🧪 Testing Permissions Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, _wasm_module) = setup_full_deployment(&worker).await?;
    let admin = create_test_account(&worker, "admin").await?;
    let user1 = create_test_account(&worker, "user1").await?;
    let user2 = create_test_account(&worker, "user2").await?;
    
    // Store code with restricted permission through router
    let restricted_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let store_result = admin
        .call(router.id(), "wasm_store_code")
        .args_json(json!({
            "wasm_byte_code": restricted_code,
            "instantiate_permission": {
                "only_address": {
                    "address": user1.id().to_string()
                }
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let restricted_code_id: u64 = store_result.json()?;
    println!("✅ Stored restricted code with ID: {}", restricted_code_id);
    
    // user1 should succeed instantiating through router
    let user1_result = user1
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": restricted_code_id,
            "msg": serde_json::to_vec(&json!({}))?,
            "funds": [],
            "label": "User1 Contract",
            "admin": user1.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(user1_result.is_success());
    println!("✅ User1 can instantiate restricted code through router");
    
    // user2 should fail instantiating through router
    let user2_result = user2
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": restricted_code_id,
            "msg": serde_json::to_vec(&json!({}))?,
            "funds": [],
            "label": "Should Fail",
            "admin": user2.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!user2_result.is_success());
    println!("✅ User2 cannot instantiate restricted code through router");
    
    // Store code with AnyOfAddresses permission
    let any_of_code = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x01];
    let store_result = admin
        .call(router.id(), "wasm_store_code")
        .args_json(json!({
            "wasm_byte_code": any_of_code,
            "instantiate_permission": {
                "any_of_addresses": {
                    "addresses": [user1.id().to_string(), user2.id().to_string()]
                }
            }
        }))
        .max_gas()
        .transact()
        .await?;
    
    let any_of_code_id: u64 = store_result.json()?;
    
    // Both users should succeed
    let user1_result = user1
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": serde_json::to_vec(&json!({}))?,
            "funds": [],
            "label": "User1 AnyOf",
            "admin": user1.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(user1_result.is_success());
    
    let user2_result = user2
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": serde_json::to_vec(&json!({}))?,
            "funds": [],
            "label": "User2 AnyOf",
            "admin": user2.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(user2_result.is_success());
    println!("✅ Both allowed users can instantiate through router");
    
    // Admin should fail
    let admin_result = admin
        .call(router.id(), "wasm_instantiate")
        .args_json(json!({
            "code_id": any_of_code_id,
            "msg": serde_json::to_vec(&json!({}))?,
            "funds": [],
            "label": "Should Fail",
            "admin": admin.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!admin_result.is_success());
    println!("✅ Non-allowed user cannot instantiate through router");
    
    println!("🎉 Router permissions test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_router_module_info() -> Result<()> {
    println!("🧪 Testing Module Info Through Router");
    
    let worker = near_workspaces::sandbox().await?;
    let (router, wasm_module) = setup_full_deployment(&worker).await?;
    
    // Get module info through router
    let modules_result = router
        .view("get_modules")
        .await?;
    
    let modules: serde_json::Value = modules_result.json()?;
    println!("DEBUG: modules = {:?}", modules);
    assert!(modules["wasm"].is_object());
    
    let wasm_info = &modules["wasm"];
    assert_eq!(wasm_info["contract_id"], wasm_module.id().to_string());
    assert_eq!(wasm_info["version"], "0.1.0");
    println!("✅ Module info accessible through router: {:?}", wasm_info);
    
    // Get module version through router
    let version_result = router
        .view("get_module_version")
        .args_json(json!({
            "module_type": "wasm"
        }))
        .await?;
    
    let version: String = version_result.json()?;
    assert_eq!(version, "0.1.0");
    println!("✅ Module version: {}", version);
    
    // Check if module is registered
    let is_registered_result = router
        .view("is_module_registered")
        .args_json(json!({
            "module_type": "wasm"
        }))
        .await?;
    
    let is_registered: bool = is_registered_result.json()?;
    assert!(is_registered);
    println!("✅ Module is registered");
    
    println!("🎉 Router module info test completed successfully!");
    Ok(())
}