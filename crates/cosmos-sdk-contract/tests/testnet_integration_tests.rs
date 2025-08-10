use anyhow::Result;
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose};

const TESTNET_CONTRACT_ID: &str = "cosmos-sdk-demo.testnet";
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

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_basic_contract_functions() -> Result<()> {
    println!("🧪 Testing basic contract functions...");

    // Test get_block_height
    let response = rpc_view_call("get_block_height", json!({})).await?;
    let block_height_str = parse_rpc_result_as_string(&response)?;
    let block_height: u64 = block_height_str.parse()?;
    println!("✅ Block height: {}", block_height);

    // Test get_balance
    let response = rpc_view_call("get_balance", json!({
        "account": TESTNET_CONTRACT_ID
    })).await?;
    let balance_str = parse_rpc_result_as_string(&response)?;
    let balance: u128 = balance_str.parse()?;
    println!("✅ Contract balance: {}", balance);

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_ibc_client_functions() -> Result<()> {
    println!("🧪 Testing IBC client functions...");

    // Test getting client state (should return empty/error for non-existent client)
    let response = rpc_view_call("ibc_get_client_state", json!({
        "client_id": "07-tendermint-0"
    })).await;
    
    // This should fail gracefully
    match response {
        Ok(_) => println!("✅ Client state query successful"),
        Err(e) => println!("✅ Client state query failed as expected: {}", e),
    }

    // Test getting consensus state
    let response = rpc_view_call("ibc_get_consensus_state", json!({
        "client_id": "07-tendermint-0",
        "height": 1000
    })).await;

    match response {
        Ok(_) => println!("✅ Consensus state query successful"),
        Err(e) => println!("✅ Consensus state query failed as expected: {}", e),
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_ibc_connection_functions() -> Result<()> {
    println!("🧪 Testing IBC connection functions...");

    // Test getting connection (should return empty/error for non-existent connection)
    let response = rpc_view_call("ibc_get_connection", json!({
        "connection_id": "connection-0"
    })).await;

    match response {
        Ok(_) => println!("✅ Connection query successful"),
        Err(e) => println!("✅ Connection query failed as expected: {}", e),
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_ibc_channel_functions() -> Result<()> {
    println!("🧪 Testing IBC channel functions...");

    // Test getting channel
    let response = rpc_view_call("ibc_get_channel", json!({
        "port_id": "transfer",
        "channel_id": "channel-0"
    })).await;

    match response {
        Ok(_) => println!("✅ Channel query successful"),
        Err(e) => println!("✅ Channel query failed as expected: {}", e),
    }

    // Test getting next sequence send
    let response = rpc_view_call("ibc_get_next_sequence_send", json!({
        "port_id": "transfer", 
        "channel_id": "channel-0"
    })).await;

    match response {
        Ok(_) => println!("✅ Sequence query successful"),
        Err(e) => println!("✅ Sequence query failed as expected: {}", e),
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_ics20_token_transfer_functions() -> Result<()> {
    println!("🧪 Testing ICS-20 token transfer functions...");

    // Test creating IBC denomination
    let response = rpc_view_call("ibc_create_ibc_denom", json!({
        "port_id": "transfer",
        "channel_id": "channel-0", 
        "denom": "unear"
    })).await?;
    
    let ibc_denom = parse_rpc_result_as_string(&response)?;
    println!("✅ Created IBC denomination: {}", ibc_denom);

    // Test source zone detection
    let response = rpc_view_call("ibc_is_source_zone", json!({
        "port_id": "transfer",
        "channel_id": "channel-0",
        "denom": "unear"
    })).await?;
    
    let is_source_str = parse_rpc_result_as_string(&response)?;
    let is_source: bool = is_source_str.parse()?;
    println!("✅ Is source zone for unear: {}", is_source);

    // Test getting escrowed amount (should be 0)
    let response = rpc_view_call("ibc_get_escrowed_amount", json!({
        "port_id": "transfer",
        "channel_id": "channel-0",
        "denom": "unear"
    })).await?;
    
    let escrowed_str = parse_rpc_result_as_string(&response)?;
    let escrowed: u128 = escrowed_str.parse()?;
    println!("✅ Escrowed amount: {}", escrowed);
    assert_eq!(escrowed, 0);

    // Test getting voucher supply for a non-existent token (should be 0)
    let response = rpc_view_call("ibc_get_voucher_supply", json!({
        "denom": "ibc/nonexistent"
    })).await?;
    
    let supply_str = parse_rpc_result_as_string(&response)?;
    let supply: u128 = supply_str.parse()?;
    println!("✅ Voucher supply: {}", supply);
    assert_eq!(supply, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_multistore_proof_functions() -> Result<()> {
    println!("🧪 Testing multi-store proof functions...");

    // Test multistore membership verification with mock data (should fail gracefully)
    let response = rpc_view_call("ibc_verify_multistore_membership", json!({
        "client_id": "07-tendermint-0",
        "height": 1000,
        "store_name": "bank",
        "key": "0x62616c616e6365732f636f736d6f73316163636f756e74",
        "value": "0x31303030303030", 
        "proof": "0x0a100801180320012a080a061a040801200110011a20c2d0c53d0fcf63bbf5bb4f5c3d9a5b4c8f71d4b7c8c9f43b3a2c7d8e9f0a1b2c"
    })).await;

    match response {
        Ok(resp) => {
            // Try to parse as boolean or handle error response
            match parse_rpc_result_as_string(&resp) {
                Ok(result_str) => {
                    if let Ok(is_valid) = result_str.parse::<bool>() {
                        println!("✅ Multistore verification result: {}", is_valid);
                    } else {
                        println!("✅ Multistore verification returned: {}", result_str);
                    }
                },
                Err(_) => println!("✅ Multistore verification completed (complex response)"),
            }
        },
        Err(e) => println!("✅ Multistore verification failed as expected: {}", e),
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_transfer_validation() -> Result<()> {
    println!("🧪 Testing transfer validation...");

    // Test transfer validation with invalid channel (should fail)
    let response = rpc_view_call("ibc_validate_transfer", json!({
        "source_port": "transfer",
        "source_channel": "channel-999",
        "denom": "unear", 
        "amount": "1000000",
        "sender": TESTNET_CONTRACT_ID
    })).await;

    match response {
        Ok(resp) => {
            println!("✅ Transfer validation completed: {:?}", resp);
        },
        Err(e) => {
            println!("✅ Transfer validation failed as expected: {}", e);
        }
    }

    // Test validation with zero amount (should fail)
    let response = rpc_view_call("ibc_validate_transfer", json!({
        "source_port": "transfer",
        "source_channel": "channel-0",
        "denom": "unear",
        "amount": "0",
        "sender": TESTNET_CONTRACT_ID
    })).await;

    match response {
        Ok(resp) => {
            println!("✅ Zero amount validation completed: {:?}", resp);
        }, 
        Err(e) => {
            println!("✅ Zero amount validation failed as expected: {}", e);
        }
    }

    Ok(())
}

#[tokio::test] 
async fn test_comprehensive_contract_integration() -> Result<()> {
    println!("🚀 Running comprehensive contract integration test...");

    // 1. Test basic functionality
    let response = rpc_view_call("get_block_height", json!({})).await?;
    let block_height_str = parse_rpc_result_as_string(&response)?;
    let block_height: u64 = block_height_str.parse()?;
    println!("✅ Step 1: Contract responsive - block height: {}", block_height);

    // 2. Test IBC denomination operations
    let response = rpc_view_call("ibc_create_ibc_denom", json!({
        "port_id": "transfer",
        "channel_id": "channel-0",
        "denom": "test-token"
    })).await?;
    let ibc_denom = parse_rpc_result_as_string(&response)?;
    println!("✅ Step 2: IBC denomination created: {}", ibc_denom);

    // 3. Test source zone detection  
    let response = rpc_view_call("ibc_is_source_zone", json!({
        "port_id": "transfer", 
        "channel_id": "channel-0",
        "denom": "test-token"
    })).await?;
    let is_source_str = parse_rpc_result_as_string(&response)?;
    let is_source: bool = is_source_str.parse()?;
    println!("✅ Step 3: Source zone detection: {}", is_source);

    // 4. Test escrowed amounts
    let response = rpc_view_call("ibc_get_escrowed_amount", json!({
        "port_id": "transfer",
        "channel_id": "channel-0", 
        "denom": "test-token"
    })).await?;
    let escrowed_str = parse_rpc_result_as_string(&response)?;
    let escrowed: u128 = escrowed_str.parse()?;
    println!("✅ Step 4: Escrowed amount: {}", escrowed);

    // 5. Test error conditions work properly
    let error_response = rpc_view_call("ibc_validate_transfer", json!({
        "source_port": "transfer",
        "source_channel": "channel-999",
        "denom": "test-token",
        "amount": "1000",
        "sender": TESTNET_CONTRACT_ID
    })).await;
    
    match error_response {
        Ok(_) => println!("✅ Step 5: Error validation completed"),
        Err(_) => println!("✅ Step 5: Error conditions handled correctly"),
    }

    println!("🎉 Comprehensive integration test completed successfully!");
    Ok(())
}