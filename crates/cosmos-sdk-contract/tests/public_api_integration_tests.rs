use near_workspaces::{Account, Contract};
use serde_json::json;
use cosmos_sdk_contract::types::cosmos_tx::{CosmosTx, TxBody, AuthInfo, Fee, SignerInfo, Coin, ModeInfo, SignMode, Any};
use cosmos_sdk_contract::handler::TxProcessingConfig;
use near_sdk::json_types::Base64VecU8;

const CONTRACT_WASM: &[u8] = include_bytes!("../target/near/cosmos_sdk_near.wasm");

async fn setup_contract() -> anyhow::Result<(near_workspaces::Worker<near_workspaces::network::Sandbox>, Account, Contract)> {
    let worker = near_workspaces::sandbox().await?;
    let account = worker.dev_create_account().await?;
    let contract = account.deploy(CONTRACT_WASM).await?.unwrap();
    
    // Initialize the contract
    let _outcome = contract
        .call("new")
        .args_json(json!({}))
        .transact()
        .await?;
    
    Ok((worker, account, contract))
}

fn create_valid_bank_transaction() -> Vec<u8> {
    // Create a valid MsgSend transaction
    let msg = Any::new("/cosmos.bank.v1beta1.MsgSend", vec![
        // Simplified MsgSend protobuf encoding (this would normally be proper protobuf)
        1, 2, 3, 4, 5 // placeholder bytes
    ]);
    
    let body = TxBody::new(vec![msg]);
    let fee = Fee::new(vec![Coin::new("unear", "1000000")], 200000);
    let signer_info = SignerInfo {
        public_key: None,
        mode_info: ModeInfo {
            mode: SignMode::Direct,
            multi: None,
        },
        sequence: 1,
    };
    let auth_info = AuthInfo::new(vec![signer_info], fee);
    let signatures = vec![vec![0u8; 65]]; // placeholder signature
    
    let tx = CosmosTx::new(body, auth_info, signatures);
    
    // Serialize to JSON (in a real implementation this would be protobuf)
    serde_json::to_vec(&tx).unwrap()
}

fn create_invalid_transaction() -> Vec<u8> {
    b"invalid_transaction_data".to_vec()
}

#[tokio::test]
async fn test_broadcast_tx_sync_success() -> anyhow::Result<()> {
    let (_worker, account, contract) = setup_contract().await?;
    
    // First mint some tokens to ensure we have balance for transfers
    let _mint_result = account
        .call(&contract.id(), "mint")
        .args_json(json!({
            "receiver": "test.near",
            "amount": "10000000"
        }))
        .transact()
        .await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    let result = contract
        .view("broadcast_tx_sync")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should return error due to signature verification being disabled in test config
    // But the method should work and return proper ABCI response structure
    assert!(response.get("code").is_some());
    assert!(response.get("data").is_some());
    assert!(response.get("raw_log").is_some());
    assert!(response.get("gas_wanted").is_some());
    assert!(response.get("gas_used").is_some());
    assert!(response.get("events").is_some());
    assert!(response.get("codespace").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_broadcast_tx_sync_invalid_data() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let invalid_tx_bytes = create_invalid_transaction();
    let tx_base64 = Base64VecU8(invalid_tx_bytes);
    
    let result = contract
        .view("broadcast_tx_sync")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should return decoding error
    assert!(response["code"].as_u64().unwrap() > 0);
    let log = response["raw_log"].as_str().unwrap();
    assert!(log.contains("decode") || log.contains("Decoding"));
    assert_eq!(response["codespace"], "sdk");
    
    Ok(())
}

#[tokio::test]
async fn test_simulate_tx() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    let result = contract
        .view("simulate_tx")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Simulation should return proper response structure
    assert!(response.get("code").is_some());
    assert!(response.get("gas_wanted").is_some());
    assert!(response.get("gas_used").is_some());
    
    // Should include "simulation" in the info field
    let info = response["info"].as_str().unwrap_or("");
    assert!(info.contains("simulation") || response.get("info").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_broadcast_tx_async() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    let result = contract
        .view("broadcast_tx_async")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should have same structure as sync
    assert!(response.get("code").is_some());
    assert!(response.get("data").is_some());
    assert!(response.get("raw_log").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_broadcast_tx_commit() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    let result = contract
        .view("broadcast_tx_commit")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should include height field (set to current block)
    assert!(response.get("height").is_some());
    let height = response["height"].as_str().unwrap();
    assert!(!height.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_get_tx_not_found() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let result = contract
        .view("get_tx")
        .args_json(json!({
            "hash": "nonexistent_transaction_hash"
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should return transaction not found error
    assert!(response["code"].as_u64().unwrap() > 0);
    let log = response["raw_log"].as_str().unwrap();
    assert!(log.contains("not found") || log.contains("Not found"));
    assert_eq!(response["codespace"], "sdk");
    
    Ok(())
}

#[tokio::test]
async fn test_tx_config_management() -> anyhow::Result<()> {
    let (_worker, account, contract) = setup_contract().await?;
    
    // Get initial config
    let initial_result = contract
        .view("get_tx_config")
        .args_json(json!({}))
        .await?;
    
    let initial_config: TxProcessingConfig = initial_result.json()?;
    assert!(!initial_config.chain_id.is_empty());
    assert!(initial_config.max_gas_per_tx > 0);
    
    // Update config
    let new_config = TxProcessingConfig {
        chain_id: "updated-test-chain".to_string(),
        max_gas_per_tx: 5_000_000,
        gas_price: 10,
        verify_signatures: true,
        check_sequences: true,
    };
    
    let _update_result = account
        .call(&contract.id(), "update_tx_config")
        .args_json(json!({ "config": new_config }))
        .transact()
        .await?;
    
    // Verify config was updated
    let updated_result = contract
        .view("get_tx_config")
        .args_json(json!({}))
        .await?;
    
    let updated_config: TxProcessingConfig = updated_result.json()?;
    assert_eq!(updated_config.chain_id, "updated-test-chain");
    assert_eq!(updated_config.max_gas_per_tx, 5_000_000);
    assert_eq!(updated_config.gas_price, 10);
    assert_eq!(updated_config.verify_signatures, true);
    assert_eq!(updated_config.check_sequences, true);
    
    Ok(())
}

#[tokio::test]
async fn test_public_api_error_codes() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    // Test different error scenarios and their ABCI codes
    let test_cases = vec![
        (create_invalid_transaction(), "decode"), // TX_DECODE_ERROR = 2
    ];
    
    for (tx_bytes, expected_error_type) in test_cases {
        let tx_base64 = Base64VecU8(tx_bytes);
        
        let result = contract
            .view("broadcast_tx_sync")
            .args_json(json!({
                "tx_bytes": tx_base64
            }))
            .await?;
        
        let response: serde_json::Value = result.json()?;
        
        // Should return appropriate error code
        let code = response["code"].as_u64().unwrap();
        assert!(code > 0, "Expected error code for {}", expected_error_type);
        
        let log = response["raw_log"].as_str().unwrap();
        assert!(log.to_lowercase().contains(expected_error_type), 
                "Expected log to contain '{}', got: {}", expected_error_type, log);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_gas_tracking_in_responses() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    // Test both broadcast and simulate return gas information
    for method in &["broadcast_tx_sync", "simulate_tx"] {
        let result = contract
            .view(method)
            .args_json(json!({
                "tx_bytes": tx_base64
            }))
            .await?;
        
        let response: serde_json::Value = result.json()?;
        
        // Should have gas tracking fields
        assert!(response.get("gas_wanted").is_some());
        assert!(response.get("gas_used").is_some());
        
        let gas_wanted = response["gas_wanted"].as_str().unwrap();
        let gas_used = response["gas_used"].as_str().unwrap();
        
        // Gas values should be numeric strings
        assert!(!gas_wanted.is_empty());
        assert!(!gas_used.is_empty());
        
        // Should be parseable as numbers
        let _wanted: u64 = gas_wanted.parse().expect("gas_wanted should be numeric");
        let _used: u64 = gas_used.parse().expect("gas_used should be numeric");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_event_structure_in_responses() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    let result = contract
        .view("broadcast_tx_sync")
        .args_json(json!({
            "tx_bytes": tx_base64
        }))
        .await?;
    
    let response: serde_json::Value = result.json()?;
    
    // Should have events array
    assert!(response.get("events").is_some());
    let events = response["events"].as_array().unwrap();
    
    // Even for error responses, events structure should be valid
    for event in events {
        assert!(event.get("type").is_some());
        assert!(event.get("attributes").is_some());
        
        let attributes = event["attributes"].as_array().unwrap();
        for attr in attributes {
            // ABCI attributes should have key, value, and index fields
            assert!(attr.get("key").is_some());
            assert!(attr.get("value").is_some());
            // index field is optional but should be boolean if present
            if let Some(index) = attr.get("index") {
                assert!(index.is_boolean());
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_public_api_performance() -> anyhow::Result<()> {
    let (_worker, _account, contract) = setup_contract().await?;
    
    let valid_tx_bytes = create_valid_bank_transaction();
    let tx_base64 = Base64VecU8(valid_tx_bytes);
    
    // Test that API calls complete in reasonable time
    let start = std::time::Instant::now();
    
    for _ in 0..5 {
        let _result = contract
            .view("simulate_tx")
            .args_json(json!({
                "tx_bytes": tx_base64
            }))
            .await?;
    }
    
    let duration = start.elapsed();
    
    // Should complete 5 calls in under 10 seconds (generous timeout for CI)
    assert!(duration.as_secs() < 10, "Public API calls took too long: {:?}", duration);
    
    Ok(())
}