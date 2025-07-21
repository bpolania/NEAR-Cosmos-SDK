use anyhow::Result;
use near_workspaces::{types::NearToken, Account, Contract, Worker};
use serde_json::json;

const COSMOS_SDK_NEAR_WASM: &[u8] = include_bytes!("../target/near/cosmos_sdk_near.wasm");

/// Helper function to deploy the Cosmos SDK NEAR contract
async fn deploy_cosmos_contract(worker: &Worker<near_workspaces::network::Sandbox>) -> Result<Contract> {
    let contract = worker.dev_deploy(COSMOS_SDK_NEAR_WASM).await?;
    
    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({}))
        .transact()
        .await?
        .into_result()?;
    
    Ok(contract)
}

/// Helper function to create a test account
async fn create_test_account(worker: &Worker<near_workspaces::network::Sandbox>, name: &str) -> Result<Account> {
    let root = worker.root_account()?;
    let account = root
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    
    Ok(account)
}

/// Helper function to create a sample Tendermint header for testing
fn create_sample_header() -> serde_json::Value {
    json!({
        "signed_header": {
            "header": {
                "version": { "block": 11, "app": 0 },
                "chain_id": "test-chain-1",
                "height": 100,
                "time": 1640995200, // Unix timestamp
                "last_block_id": {
                    "hash": [18, 52, 86, 120, 144, 171, 205, 239],
                    "part_set_header": { "total": 1, "hash": [171, 205, 239, 18, 52, 86, 120, 144] }
                },
                "last_commit_hash": [17, 17, 17, 17, 17, 17, 17, 17],
                "data_hash": [34, 34, 34, 34, 34, 34, 34, 34],
                "validators_hash": [51, 51, 51, 51, 51, 51, 51, 51],
                "next_validators_hash": [68, 68, 68, 68, 68, 68, 68, 68],
                "consensus_hash": [85, 85, 85, 85, 85, 85, 85, 85],
                "app_hash": [102, 102, 102, 102, 102, 102, 102, 102],
                "last_results_hash": [119, 119, 119, 119, 119, 119, 119, 119],
                "evidence_hash": [136, 136, 136, 136, 136, 136, 136, 136],
                "proposer_address": [153, 153, 153, 153, 153, 153, 153, 153]
            },
            "commit": {
                "height": 100,
                "round": 0,
                "block_id": {
                    "hash": [18, 52, 86, 120, 144, 171, 205, 239],
                    "part_set_header": { "total": 1, "hash": [171, 205, 239, 18, 52, 86, 120, 144] }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8],
                        "timestamp": 1640995200,
                        "signature": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64]
                    }
                ]
            }
        },
        "validator_set": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": { "revision_number": 0, "revision_height": 99 },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    })
}

#[cfg(test)]
mod light_client_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_client() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create a new Tendermint light client
        let result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        assert!(result.is_success());

        // Extract the client ID from the result
        let client_id: String = result.json()?;
        assert!(client_id.starts_with("07-tendermint-"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_client_state() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Get client state
        let client_state_result = contract
            .view("ibc_get_client_state")
            .args_json(json!({
                "client_id": client_id
            }))
            .await?;

        let client_state: Option<serde_json::Value> = client_state_result.json()?;
        assert!(client_state.is_some());

        let state = client_state.unwrap();
        assert_eq!(state["chain_id"], "test-chain-1");
        assert_eq!(state["trust_period"], 86400);
        assert_eq!(state["unbonding_period"], 1814400);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_consensus_state() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Get consensus state at height 100
        let consensus_state_result = contract
            .view("ibc_get_consensus_state")
            .args_json(json!({
                "client_id": client_id,
                "height": 100
            }))
            .await?;

        let consensus_state: Option<serde_json::Value> = consensus_state_result.json()?;
        assert!(consensus_state.is_some());

        let state = consensus_state.unwrap();
        assert_eq!(state["timestamp"], 1640995200);
        assert!(!state["root"].as_array().unwrap().is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_latest_height() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Get latest height
        let height_result = contract
            .view("ibc_get_latest_height")
            .args_json(json!({
                "client_id": client_id
            }))
            .await?;

        let height: Option<serde_json::Value> = height_result.json()?;
        assert!(height.is_some());

        let h = height.unwrap();
        assert_eq!(h["revision_number"], 0);
        assert_eq!(h["revision_height"], 100);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_client_basic() -> Result<()> {
        // Add delay to avoid port conflicts with other test files
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Create a new header with higher height and later timestamp
        let mut new_header = create_sample_header();
        new_header["signed_header"]["header"]["height"] = json!(101);
        new_header["signed_header"]["commit"]["height"] = json!(101);
        new_header["signed_header"]["header"]["time"] = json!(1640995300); // Later timestamp
        new_header["trusted_height"]["revision_height"] = json!(100); // Set trusted height to previous header

        // Update client with new header
        let update_result = user
            .call(contract.id(), "ibc_update_client")
            .args_json(json!({
                "client_id": client_id,
                "header": new_header
            }))
            .transact()
            .await?;

        assert!(update_result.is_success());

        let success: bool = update_result.json()?;
        assert!(success);

        // Verify latest height was updated
        let height_result = contract
            .view("ibc_get_latest_height")
            .args_json(json!({
                "client_id": client_id
            }))
            .await?;

        let height: Option<serde_json::Value> = height_result.json()?;
        let h = height.unwrap();
        assert_eq!(h["revision_height"], 101);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_client_invalid_height() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Try to update with same height (should fail)
        let same_header = create_sample_header();

        let update_result = user
            .call(contract.id(), "ibc_update_client")
            .args_json(json!({
                "client_id": client_id,
                "header": same_header
            }))
            .transact()
            .await?;

        assert!(update_result.is_success());

        let success: bool = update_result.json()?;
        assert!(!success); // Should return false for invalid update

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_membership_placeholder() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Test verify_membership (placeholder implementation returns true for non-empty root)
        let verify_result = contract
            .view("ibc_verify_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "key": [1, 2, 3, 4],
                "value": [5, 6, 7, 8],
                "proof": [9, 10, 11, 12]
            }))
            .await?;

        let result: bool = verify_result.json()?;
        assert!(result); // Should be true since app_hash is non-empty (placeholder logic)

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_non_membership_placeholder() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header = create_sample_header();

        // Create client first
        let create_result = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header
            }))
            .transact()
            .await?;

        let client_id: String = create_result.json()?;

        // Test verify_non_membership (placeholder implementation returns true for non-empty root)
        let verify_result = contract
            .view("ibc_verify_non_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "key": [1, 2, 3, 4],
                "proof": [9, 10, 11, 12]
            }))
            .await?;

        let result: bool = verify_result.json()?;
        assert!(result); // Should be true since app_hash is non-empty (placeholder logic)

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_clients() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        let header1 = create_sample_header();
        let mut header2 = create_sample_header();
        header2["signed_header"]["header"]["chain_id"] = json!("test-chain-2");

        // Create first client
        let result1 = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-1",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header1
            }))
            .transact()
            .await?;

        let client_id1: String = result1.json()?;

        // Create second client
        let result2 = user
            .call(contract.id(), "ibc_create_client")
            .args_json(json!({
                "chain_id": "test-chain-2",
                "trust_period": 86400,
                "unbonding_period": 1814400,
                "max_clock_drift": 600,
                "initial_header": header2
            }))
            .transact()
            .await?;

        let client_id2: String = result2.json()?;

        // Verify both clients exist and have different IDs
        assert_ne!(client_id1, client_id2);
        assert!(client_id1.starts_with("07-tendermint-"));
        assert!(client_id2.starts_with("07-tendermint-"));

        // Verify both clients can be queried
        let state1 = contract
            .view("ibc_get_client_state")
            .args_json(json!({"client_id": client_id1}))
            .await?
            .json::<Option<serde_json::Value>>()?;

        let state2 = contract
            .view("ibc_get_client_state")
            .args_json(json!({"client_id": client_id2}))
            .await?
            .json::<Option<serde_json::Value>>()?;

        assert!(state1.is_some());
        assert!(state2.is_some());

        assert_eq!(state1.unwrap()["chain_id"], "test-chain-1");
        assert_eq!(state2.unwrap()["chain_id"], "test-chain-2");

        Ok(())
    }
}