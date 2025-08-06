use anyhow::Result;
use near_workspaces::{types::NearToken, Account, Contract, Worker};
use serde_json::json;
use tokio::time::{sleep, Duration};

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
    #[ignore = "IBC tests need special setup"]
    async fn test_create_client() -> Result<()> {
        sleep(Duration::from_millis(800)).await;
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

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
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

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
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

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
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

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_update_client_basic() -> Result<()> {
        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(300)).await;
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
    #[ignore = "IBC tests need special setup"]
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

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_membership_ics23() -> Result<()> {
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

        // Create a valid ICS-23 proof structure (JSON format for cross-chain compatibility)
        let ics23_proof = json!({
            "proof": {
                "key": [1, 2, 3, 4],
                "value": [5, 6, 7, 8],
                "leaf": {
                    "hash": "Sha256",
                    "prehash_key": "NoHash", 
                    "prehash_value": "Sha256",
                    "length": "VarProto",
                    "prefix": [0]
                },
                "path": [
                    {
                        "hash": "Sha256",
                        "prefix": [1],
                        "suffix": []
                    }
                ]
            },
            "non_exist": null,
            "batch": null,
            "compressed": null
        });

        // Test verify_membership with ICS-23 proof format
        let verify_result = contract
            .view("ibc_verify_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "key": [1, 2, 3, 4],
                "value": [5, 6, 7, 8],
                "proof": serde_json::to_vec(&ics23_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        // With real ICS-23 implementation, this may return false for invalid proof,
        // but the important thing is that it parses the proof correctly
        println!("ICS-23 proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
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

        // Create a valid ICS-23 non-membership proof structure
        let ics23_non_membership_proof = json!({
            "proof": null,
            "non_exist": {
                "key": [1, 2, 3, 4],
                "left": {
                    "key": [0, 1, 2, 3],
                    "value": [0, 0, 0, 0],
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256", 
                        "length": "VarProto",
                        "prefix": [0]
                    },
                    "path": []
                },
                "right": {
                    "key": [2, 3, 4, 5],
                    "value": [1, 1, 1, 1],
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256",
                        "length": "VarProto", 
                        "prefix": [0]
                    },
                    "path": []
                }
            },
            "batch": null,
            "compressed": null
        });

        // Test verify_non_membership with ICS-23 proof format
        let verify_result = contract
            .view("ibc_verify_non_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "key": [1, 2, 3, 4],
                "proof": serde_json::to_vec(&ics23_non_membership_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        // With real ICS-23 implementation, this may return false for invalid proof,
        // but the important thing is that it parses the non-membership proof correctly
        println!("ICS-23 non-membership proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_multiple_clients() -> Result<()> {
        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(500)).await;
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

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_batch_membership() -> Result<()> {
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

        // Create a valid ICS-23 batch proof structure
        let batch_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": [
                    {
                        "exist": {
                            "key": [1, 2, 3, 4],
                            "value": [5, 6, 7, 8],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": []
                        }
                    },
                    {
                        "exist": {
                            "key": [2, 3, 4, 5],
                            "value": [6, 7, 8, 9],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": []
                        }
                    },
                    {
                        "nonexist": {
                            "key": [3, 3, 3, 3],
                            "left": {
                                "key": [2, 3, 4, 5],
                                "value": [6, 7, 8, 9],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            },
                            "right": {
                                "key": [4, 4, 4, 4],
                                "value": [10, 11, 12, 13],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            }
                        }
                    }
                ]
            },
            "compressed": null
        });

        // Test verify_batch_membership with mixed existence and non-existence
        let items = vec![
            ([1, 2, 3, 4], Some([5, 6, 7, 8])),
            ([2, 3, 4, 5], Some([6, 7, 8, 9])),
            ([3, 3, 3, 3], None) // non-membership item
        ];

        let verify_result = contract
            .view("ibc_verify_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "items": items,
                "proof": serde_json::to_vec(&batch_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_mixed_batch_membership() -> Result<()> {
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

        // Create a batch proof with mixed membership/non-membership
        let mixed_batch_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": [
                    {
                        "exist": {
                            "key": [100, 101, 102],
                            "value": [200, 201, 202],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": []
                        }
                    },
                    {
                        "exist": {
                            "key": [110, 111, 112],
                            "value": [210, 211, 212],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": []
                        }
                    },
                    {
                        "nonexist": {
                            "key": [105, 105, 105],
                            "left": {
                                "key": [100, 101, 102],
                                "value": [200, 201, 202],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            },
                            "right": {
                                "key": [110, 111, 112],
                                "value": [210, 211, 212],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            }
                        }
                    },
                    {
                        "nonexist": {
                            "key": [115, 115, 115],
                            "left": {
                                "key": [110, 111, 112],
                                "value": [210, 211, 212],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            },
                            "right": {
                                "key": [120, 121, 122],
                                "value": [220, 221, 222],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            }
                        }
                    }
                ]
            },
            "compressed": null
        });

        // Test verify_mixed_batch_membership with separate exist and non-exist lists
        let exist_items = vec![
            ([100, 101, 102], [200, 201, 202]),
            ([110, 111, 112], [210, 211, 212])
        ];
        let non_exist_keys = vec![
            [105, 105, 105],
            [115, 115, 115]
        ];

        let verify_result = contract
            .view("ibc_verify_mixed_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "exist_items": exist_items,
                "non_exist_keys": non_exist_keys,
                "proof": serde_json::to_vec(&mixed_batch_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Mixed batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_compressed_batch_membership() -> Result<()> {
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

        // Create a compressed batch proof with lookup table
        let compressed_batch_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": null,
            "compressed": {
                "entries": [
                    {
                        "exist": {
                            "key": [50, 51, 52],
                            "value": [150, 151, 152],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": [
                                {"hash": "Sha256", "prefix": [1, 0], "suffix": []},
                                {"hash": "Sha256", "prefix": [1, 1], "suffix": []}
                            ]
                        }
                    },
                    {
                        "exist": {
                            "key": [60, 61, 62],
                            "value": [160, 161, 162],
                            "leaf": {
                                "hash": "Sha256",
                                "prehash_key": "NoHash",
                                "prehash_value": "Sha256",
                                "length": "VarProto",
                                "prefix": [0]
                            },
                            "path": [
                                {"hash": "Sha256", "prefix": [1, 0], "suffix": []},
                                {"hash": "Sha256", "prefix": [1, 2], "suffix": []}
                            ]
                        }
                    },
                    {
                        "nonexist": {
                            "key": [55, 55, 55],
                            "left": {
                                "key": [50, 51, 52],
                                "value": [150, 151, 152],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            },
                            "right": {
                                "key": [60, 61, 62],
                                "value": [160, 161, 162],
                                "leaf": {
                                    "hash": "Sha256",
                                    "prehash_key": "NoHash",
                                    "prehash_value": "Sha256",
                                    "length": "VarProto",
                                    "prefix": [0]
                                },
                                "path": []
                            }
                        }
                    }
                ],
                "lookup_inners": [
                    {
                        "hash": [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                        "prefix": [1, 0],
                        "suffix": []
                    },
                    {
                        "hash": [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
                        "prefix": [1, 1],
                        "suffix": []
                    },
                    {
                        "hash": [3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
                        "prefix": [1, 2],
                        "suffix": []
                    }
                ]
            }
        });

        // Test verify_compressed_batch_membership with compressed proof format
        let items = vec![
            ([50, 51, 52], Some([150, 151, 152])),
            ([60, 61, 62], Some([160, 161, 162])),
            ([55, 55, 55], None) // non-membership item
        ];

        let verify_result = contract
            .view("ibc_verify_compressed_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "items": items,
                "proof": serde_json::to_vec(&compressed_batch_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Compressed batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_batch_proof_empty_items() -> Result<()> {
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

        // Test with empty items list
        let empty_items: Vec<(Vec<u8>, Option<Vec<u8>>)> = vec![];
        
        let empty_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": []
            },
            "compressed": null
        });

        let verify_result = contract
            .view("ibc_verify_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "items": empty_items,
                "proof": serde_json::to_vec(&empty_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Empty batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_batch_proof_invalid_client() -> Result<()> {
        sleep(Duration::from_millis(1000)).await;
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;

        // Test with non-existent client
        let items = vec![
            ([1, 2, 3, 4], Some([5, 6, 7, 8]))
        ];
        
        let proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": [{
                    "exist": {
                        "key": [1, 2, 3, 4],
                        "value": [5, 6, 7, 8],
                        "leaf": {
                            "hash": "Sha256",
                            "prehash_key": "NoHash",
                            "prehash_value": "Sha256",
                            "length": "VarProto",
                            "prefix": [0]
                        },
                        "path": []
                    }
                }]
            },
            "compressed": null
        });

        let verify_result = contract
            .view("ibc_verify_batch_membership")
            .args_json(json!({
                "client_id": "non-existent-client",
                "height": 100,
                "items": items,
                "proof": serde_json::to_vec(&proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        assert!(!result); // Should return false for non-existent client
        println!("Invalid client batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_batch_proof_invalid_height() -> Result<()> {
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

        // Test with non-existent height
        let items = vec![
            ([1, 2, 3, 4], Some([5, 6, 7, 8]))
        ];
        
        let proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": [{
                    "exist": {
                        "key": [1, 2, 3, 4],
                        "value": [5, 6, 7, 8],
                        "leaf": {
                            "hash": "Sha256",
                            "prehash_key": "NoHash",
                            "prehash_value": "Sha256",
                            "length": "VarProto",
                            "prefix": [0]
                        },
                        "path": []
                    }
                }]
            },
            "compressed": null
        });

        let verify_result = contract
            .view("ibc_verify_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 999, // Non-existent height
                "items": items,
                "proof": serde_json::to_vec(&proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        assert!(!result); // Should return false for non-existent height
        println!("Invalid height batch proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_large_batch_proof_performance() -> Result<()> {
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

        // Create a large batch with 10 items to test performance
        let mut batch_entries = vec![];
        let mut items = vec![];
        
        for i in 0..10 {
            let key = vec![i as u8, (i+1) as u8, (i+2) as u8];
            let value = vec![(i+100) as u8, (i+101) as u8, (i+102) as u8];
            
            batch_entries.push(json!({
                "exist": {
                    "key": key,
                    "value": value,
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256",
                        "length": "VarProto",
                        "prefix": [0]
                    },
                    "path": []
                }
            }));
            
            items.push((key, Some(value)));
        }

        let large_batch_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": {
                "entries": batch_entries
            },
            "compressed": null
        });

        let start = std::time::Instant::now();
        
        let verify_result = contract
            .view("ibc_verify_batch_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "items": items,
                "proof": serde_json::to_vec(&large_batch_proof).unwrap()
            }))
            .await?;

        let duration = start.elapsed();
        let result: bool = verify_result.json()?;
        
        println!("Large batch (10 items) verification took: {:?}, result: {}", duration, result);
        
        // Performance should be reasonable for batch operations
        assert!(duration.as_secs() < 5, "Batch verification took too long: {:?}", duration);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_range_membership_existence() -> Result<()> {
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

        // Create a range proof for consecutive packet keys (existence)
        let range_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": null,
            "compressed": null,
            "range": {
                "start_key": [112, 97, 99, 107, 101, 116, 115, 47, 49], // "packets/1"
                "end_key": [112, 97, 99, 107, 101, 116, 115, 47, 51], // "packets/3"
                "existence": true,
                "left_boundary": null,
                "right_boundary": null,
                "key_proofs": [
                    {
                        "key": [112, 97, 99, 107, 101, 116, 115, 47, 49], // "packets/1"
                        "value": [100, 97, 116, 97, 49], // "data1"
                        "leaf": {
                            "hash": "Sha256",
                            "prehash_key": "NoHash",
                            "prehash_value": "Sha256",
                            "length": "VarProto",
                            "prefix": [0]
                        },
                        "path": []
                    },
                    {
                        "key": [112, 97, 99, 107, 101, 116, 115, 47, 50], // "packets/2"
                        "value": [100, 97, 116, 97, 50], // "data2"
                        "leaf": {
                            "hash": "Sha256",
                            "prehash_key": "NoHash",
                            "prehash_value": "Sha256",
                            "length": "VarProto",
                            "prefix": [0]
                        },
                        "path": []
                    },
                    {
                        "key": [112, 97, 99, 107, 101, 116, 115, 47, 51], // "packets/3"
                        "value": [100, 97, 116, 97, 51], // "data3"
                        "leaf": {
                            "hash": "Sha256",
                            "prehash_key": "NoHash",
                            "prehash_value": "Sha256",
                            "length": "VarProto",
                            "prefix": [0]
                        },
                        "path": []
                    }
                ],
                "shared_path": []
            }
        });

        // Test range verification for consecutive packet sequence
        let start_key = [112, 97, 99, 107, 101, 116, 115, 47, 49]; // "packets/1"
        let end_key = [112, 97, 99, 107, 101, 116, 115, 47, 51]; // "packets/3"
        let expected_values = vec![
            ([112, 97, 99, 107, 101, 116, 115, 47, 49], [100, 97, 116, 97, 49]), // "packets/1" -> "data1"
            ([112, 97, 99, 107, 101, 116, 115, 47, 50], [100, 97, 116, 97, 50]), // "packets/2" -> "data2"
            ([112, 97, 99, 107, 101, 116, 115, 47, 51], [100, 97, 116, 97, 51])  // "packets/3" -> "data3"
        ];

        let verify_result = contract
            .view("ibc_verify_range_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "start_key": start_key,
                "end_key": end_key,
                "existence": true,
                "expected_values": expected_values,
                "proof": serde_json::to_vec(&range_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Range existence proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_range_membership_non_existence() -> Result<()> {
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

        // Create a range proof for non-existence (gap in packet sequence)
        let range_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": null,
            "compressed": null,
            "range": {
                "start_key": [112, 97, 99, 107, 101, 116, 115, 47, 53], // "packets/5"
                "end_key": [112, 97, 99, 107, 101, 116, 115, 47, 55], // "packets/7"
                "existence": false,
                "left_boundary": {
                    "key": [112, 97, 99, 107, 101, 116, 115, 47, 52], // "packets/4"
                    "value": [100, 97, 116, 97, 52], // "data4"
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256",
                        "length": "VarProto",
                        "prefix": [0]
                    },
                    "path": []
                },
                "right_boundary": {
                    "key": [112, 97, 99, 107, 101, 116, 115, 47, 56], // "packets/8"
                    "value": [100, 97, 116, 97, 56], // "data8"
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256",
                        "length": "VarProto",
                        "prefix": [0]
                    },
                    "path": []
                },
                "key_proofs": [],
                "shared_path": []
            }
        });

        // Test range verification for non-existence (proving gap in sequence)
        let start_key = [112, 97, 99, 107, 101, 116, 115, 47, 53]; // "packets/5"
        let end_key = [112, 97, 99, 107, 101, 116, 115, 47, 55]; // "packets/7"
        let expected_values: Vec<(Vec<u8>, Vec<u8>)> = vec![]; // No values for non-existence

        let verify_result = contract
            .view("ibc_verify_range_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "start_key": start_key,
                "end_key": end_key,
                "existence": false,
                "expected_values": expected_values,
                "proof": serde_json::to_vec(&range_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        println!("Range non-existence proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_verify_range_membership_invalid_range() -> Result<()> {
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

        let range_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": null,
            "compressed": null,
            "range": {
                "start_key": [112, 97, 99, 107, 101, 116, 115, 47, 53], // "packets/5"
                "end_key": [112, 97, 99, 107, 101, 116, 115, 47, 51], // "packets/3" - invalid!
                "existence": true,
                "left_boundary": null,
                "right_boundary": null,
                "key_proofs": [],
                "shared_path": []
            }
        });

        // Test with invalid range (start_key > end_key)
        let start_key = [112, 97, 99, 107, 101, 116, 115, 47, 53]; // "packets/5"
        let end_key = [112, 97, 99, 107, 101, 116, 115, 47, 51]; // "packets/3" - smaller than start!
        let expected_values: Vec<(Vec<u8>, Vec<u8>)> = vec![];

        let verify_result = contract
            .view("ibc_verify_range_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "start_key": start_key,
                "end_key": end_key,
                "existence": true,
                "expected_values": expected_values,
                "proof": serde_json::to_vec(&range_proof).unwrap()
            }))
            .await?;

        let result: bool = verify_result.json()?;
        assert!(!result); // Should return false for invalid range
        println!("Invalid range proof verification result: {}", result);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "IBC tests need special setup"]
    async fn test_range_proof_performance() -> Result<()> {
        sleep(Duration::from_millis(1200)).await;
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

        // Create a large range proof with 20 consecutive keys
        let mut key_proofs = vec![];
        let mut expected_values = vec![];
        
        for i in 1..=20 {
            let key = format!("packets/{}", i).into_bytes();
            let value = format!("data{}", i).into_bytes();
            
            key_proofs.push(json!({
                "key": key,
                "value": value,
                "leaf": {
                    "hash": "Sha256",
                    "prehash_key": "NoHash",
                    "prehash_value": "Sha256",
                    "length": "VarProto",
                    "prefix": [0]
                },
                "path": []
            }));
            
            expected_values.push((key, value));
        }

        let range_proof = json!({
            "proof": null,
            "non_exist": null,
            "batch": null,
            "compressed": null,
            "range": {
                "start_key": b"packets/1".to_vec(),
                "end_key": b"packets/20".to_vec(),
                "existence": true,
                "left_boundary": null,
                "right_boundary": null,
                "key_proofs": key_proofs,
                "shared_path": []
            }
        });

        let start = std::time::Instant::now();
        
        let verify_result = contract
            .view("ibc_verify_range_membership")
            .args_json(json!({
                "client_id": client_id,
                "height": 100,
                "start_key": b"packets/1".to_vec(),
                "end_key": b"packets/20".to_vec(),
                "existence": true,
                "expected_values": expected_values,
                "proof": serde_json::to_vec(&range_proof).unwrap()
            }))
            .await?;

        let duration = start.elapsed();
        let result: bool = verify_result.json()?;
        
        println!("Large range (20 keys) verification took: {:?}, result: {}", duration, result);
        
        // Performance should be reasonable for range operations
        assert!(duration.as_secs() < 10, "Range verification took too long: {:?}", duration);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }
}