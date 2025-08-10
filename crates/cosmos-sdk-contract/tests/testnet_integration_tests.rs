use anyhow::Result;
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose};

const TESTNET_CONTRACT_ID: &str = "cosmos-sdk-demo-1754812961.testnet";
const NEAR_TESTNET_RPC: &str = "https://rpc.testnet.near.org";

/// Helper to make RPC view calls
async fn rpc_view_call(method_name: &str, args: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    
    let args_base64 = if args.is_null() {
        String::new()
    } else {
        general_purpose::STANDARD.encode(args.to_string())
    };

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": "dontcare",
        "method": "query",
        "params": {
            "request_type": "call_function",
            "finality": "final",
            "account_id": TESTNET_CONTRACT_ID,
            "method_name": method_name,
            "args_base64": args_base64
        }
    });

    let response = client
        .post(NEAR_TESTNET_RPC)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("RPC request failed with status: {}", response.status());
    }

    let response_json: Value = response.json().await?;
    
    if let Some(error) = response_json.get("error") {
        anyhow::bail!("RPC error: {}", error);
    }

    Ok(response_json)
}

/// Helper to parse RPC result as string
fn parse_rpc_result_as_string(response: &Value) -> Result<String> {
    if let Some(result) = response.get("result") {
        if let Some(result_data) = result.get("result") {
            if let Some(result_array) = result_data.as_array() {
                let result_bytes: Vec<u8> = result_array.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                return Ok(String::from_utf8(result_bytes)?);
            }
        }
    }
    anyhow::bail!("Could not parse RPC result")
}

async fn test_basic_contract_functions() -> Result<()> {
    println!("🧪 Testing basic router contract functions on testnet...");

    // Test health_check
    let response = rpc_view_call("health_check", json!({})).await?;
    let health_str = parse_rpc_result_as_string(&response)?;
    let health: Value = serde_json::from_str(&health_str)?;
    assert_eq!(health["router"], true);
    println!("✅ Health check passed: {}", health);

    // Test get_metadata
    let response = rpc_view_call("get_metadata", json!({})).await?;
    let metadata_str = parse_rpc_result_as_string(&response)?;
    let metadata: Value = serde_json::from_str(&metadata_str)?;
    assert_eq!(metadata["type"], "modular_router");
    println!("✅ Metadata: {}", metadata["name"]);

    // Test test_function
    let response = rpc_view_call("test_function", json!({})).await?;
    let test_str = parse_rpc_result_as_string(&response)?;
    assert!(test_str.contains("Modular Router is working!"));
    println!("✅ Test function: {}", test_str);

    Ok(())
}

async fn test_router_module_functions() -> Result<()> {
    println!("🧪 Testing router module functions...");

    // Test get_modules (should be empty initially)
    let response = rpc_view_call("get_modules", json!({})).await?;
    let modules_str = parse_rpc_result_as_string(&response)?;
    let modules: Value = serde_json::from_str(&modules_str)?;
    println!("✅ Registered modules: {}", modules);

    // Test get_owner
    let response = rpc_view_call("get_owner", json!({})).await?;
    let owner_str = parse_rpc_result_as_string(&response)?;
    assert_eq!(owner_str.trim_matches('"'), TESTNET_CONTRACT_ID);
    println!("✅ Contract owner: {}", owner_str);

    // Test get_stats
    let response = rpc_view_call("get_stats", json!({})).await?;
    let stats_str = parse_rpc_result_as_string(&response)?;
    let stats: Value = serde_json::from_str(&stats_str)?;
    println!("✅ Contract stats: modules_registered={}", stats["modules_registered"]);

    Ok(())
}

#[tokio::test]
async fn test_all_testnet_functions() -> Result<()> {
    println!("🧪 Running comprehensive testnet integration test...");
    println!("Contract: {}", TESTNET_CONTRACT_ID);
    
    // Run all tests
    test_basic_contract_functions().await?;
    test_router_module_functions().await?;
    
    println!("🎉 All testnet integration tests passed!");
    Ok(())
}

// IBC tests will be added when IBC modules are deployed
#[tokio::test]
#[ignore = "IBC modules not yet deployed"]
async fn test_ibc_module_integration() -> Result<()> {
    println!("🧪 IBC module tests will be implemented when IBC modules are deployed");
    // TODO: Add tests for IBC client, connection, channel, and transfer modules
    Ok(())
}

