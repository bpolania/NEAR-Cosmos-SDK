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

/// Helper function to create a connection version
fn create_connection_version() -> serde_json::Value {
    json!({
        "identifier": "1",
        "features": ["ORDER_ORDERED", "ORDER_UNORDERED"]
    })
}

/// Helper to create IBC client for connection tests
async fn create_test_client(
    user: &Account, 
    contract: &Contract,
    chain_id: &str
) -> Result<String> {
    let header = create_sample_header();
    
    let result = user
        .call(contract.id(), "ibc_create_client")
        .args_json(json!({
            "chain_id": chain_id,
            "trust_period": 86400,
            "unbonding_period": 1814400,
            "max_clock_drift": 600,
            "initial_header": header
        }))
        .transact()
        .await?;

    Ok(result.json()?)
}

#[cfg(test)]
mod connection_tests {
    use super::*;

    #[tokio::test]
    async fn test_conn_open_init() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Initialize a connection
        let result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        assert!(result.is_success());

        let connection_id: String = result.json()?;
        assert!(connection_id.starts_with("connection-"));

        // Verify the connection was created
        let get_result = contract
            .view("ibc_get_connection")
            .args_json(json!({
                "connection_id": connection_id
            }))
            .await?;

        let connection: Option<serde_json::Value> = get_result.json()?;
        assert!(connection.is_some());

        let conn = connection.unwrap();
        assert_eq!(conn["state"], "Init");
        assert_eq!(conn["client_id"], client_id);
        assert_eq!(conn["counterparty"]["client_id"], "07-tendermint-0");

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_conn_open_try() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Open a connection with ConnOpenTry
        let result = user
            .call(contract.id(), "ibc_conn_open_try")
            .args_json(json!({
                "previous_connection_id": null,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_connection_id": "connection-0",
                "counterparty_prefix": null,
                "delay_period": 0,
                "client_id": client_id,
                "client_state_proof": [1, 2, 3, 4],
                "consensus_state_proof": [5, 6, 7, 8],
                "connection_proof": [9, 10, 11, 12],
                "proof_height": 100,
                "version": create_connection_version()
            }))
            .transact()
            .await?;

        assert!(result.is_success());

        let connection_id: String = result.json()?;
        assert!(connection_id.starts_with("connection-"));

        // Verify the connection was created in TryOpen state
        let get_result = contract
            .view("ibc_get_connection")
            .args_json(json!({
                "connection_id": connection_id
            }))
            .await?;

        let connection: Option<serde_json::Value> = get_result.json()?;
        assert!(connection.is_some());

        let conn = connection.unwrap();
        assert_eq!(conn["state"], "TryOpen");
        assert_eq!(conn["client_id"], client_id);
        assert_eq!(conn["counterparty"]["connection_id"], "connection-0");

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_handshake_init_to_ack() -> Result<()> {
        sleep(Duration::from_millis(1400)).await;
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Step 1: ConnOpenInit
        let init_result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        let connection_id: String = init_result.json()?;

        // Verify connection is in Init state
        let get_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let connection: Option<serde_json::Value> = get_result.json()?;
        assert_eq!(connection.unwrap()["state"], "Init");

        // Step 2: ConnOpenAck
        let ack_result = user
            .call(contract.id(), "ibc_conn_open_ack")
            .args_json(json!({
                "connection_id": connection_id,
                "counterparty_connection_id": "connection-0",
                "version": create_connection_version(),
                "client_state_proof": [1, 2, 3, 4],
                "connection_proof": [5, 6, 7, 8],
                "consensus_state_proof": [9, 10, 11, 12],
                "proof_height": 100
            }))
            .transact()
            .await?;

        assert!(ack_result.is_success());

        // Verify connection is now Open
        let final_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let final_connection: Option<serde_json::Value> = final_result.json()?;
        let conn = final_connection.unwrap();
        assert_eq!(conn["state"], "Open");
        assert_eq!(conn["counterparty"]["connection_id"], "connection-0");

        // Test is_connection_open
        let open_result = contract
            .view("ibc_is_connection_open")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let is_open: bool = open_result.json()?;
        assert!(is_open);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_handshake_try_to_confirm() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Step 1: ConnOpenTry
        let try_result = user
            .call(contract.id(), "ibc_conn_open_try")
            .args_json(json!({
                "previous_connection_id": null,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_connection_id": "connection-0",
                "counterparty_prefix": null,
                "delay_period": 0,
                "client_id": client_id,
                "client_state_proof": [1, 2, 3, 4],
                "consensus_state_proof": [5, 6, 7, 8],
                "connection_proof": [9, 10, 11, 12],
                "proof_height": 100,
                "version": create_connection_version()
            }))
            .transact()
            .await?;

        let connection_id: String = try_result.json()?;

        // Verify connection is in TryOpen state
        let get_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let connection: Option<serde_json::Value> = get_result.json()?;
        assert_eq!(connection.unwrap()["state"], "TryOpen");

        // Step 2: ConnOpenConfirm
        let confirm_result = user
            .call(contract.id(), "ibc_conn_open_confirm")
            .args_json(json!({
                "connection_id": connection_id,
                "connection_proof": [1, 2, 3, 4],
                "proof_height": 101
            }))
            .transact()
            .await?;

        assert!(confirm_result.is_success());

        // Verify connection is now Open
        let final_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let final_connection: Option<serde_json::Value> = final_result.json()?;
        assert_eq!(final_connection.unwrap()["state"], "Open");

        // Test is_connection_open
        let open_result = contract
            .view("ibc_is_connection_open")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let is_open: bool = open_result.json()?;
        assert!(is_open);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_conn_open_ack_invalid_state() -> Result<()> {
        sleep(Duration::from_millis(500)).await; // Add delay to avoid port conflicts
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Create a connection in TryOpen state
        let try_result = user
            .call(contract.id(), "ibc_conn_open_try")
            .args_json(json!({
                "previous_connection_id": null,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_connection_id": "connection-0",
                "counterparty_prefix": null,
                "delay_period": 0,
                "client_id": client_id,
                "client_state_proof": [1, 2, 3, 4],
                "consensus_state_proof": [5, 6, 7, 8],
                "connection_proof": [9, 10, 11, 12],
                "proof_height": 100,
                "version": create_connection_version()
            }))
            .transact()
            .await?;

        let connection_id: String = try_result.json()?;

        // Try to call ConnOpenAck on a connection in TryOpen state (should fail)
        let ack_result = user
            .call(contract.id(), "ibc_conn_open_ack")
            .args_json(json!({
                "connection_id": connection_id,
                "counterparty_connection_id": "connection-0",
                "version": create_connection_version(),
                "client_state_proof": [1, 2, 3, 4],
                "connection_proof": [5, 6, 7, 8],
                "consensus_state_proof": [9, 10, 11, 12],
                "proof_height": 100
            }))
            .transact()
            .await?;

        // Should fail because connection is not in Init state
        assert!(!ack_result.is_success());

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_conn_open_confirm_invalid_state() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Create a connection in Init state
        let init_result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        let connection_id: String = init_result.json()?;

        // Try to call ConnOpenConfirm on a connection in Init state (should fail)
        let confirm_result = user
            .call(contract.id(), "ibc_conn_open_confirm")
            .args_json(json!({
                "connection_id": connection_id,
                "connection_proof": [1, 2, 3, 4],
                "proof_height": 101
            }))
            .transact()
            .await?;

        // Should fail because connection is not in TryOpen state
        assert!(!confirm_result.is_success());

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_connection_nonexistent() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;

        // Try to get a non-existent connection
        let result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": "connection-999"}))
            .await?;

        let connection: Option<serde_json::Value> = result.json()?;
        assert!(connection.is_none());

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_is_connection_open_false() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create a client first
        let client_id = create_test_client(&user, &contract, "test-chain-1").await?;

        // Create a connection in Init state
        let init_result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        let connection_id: String = init_result.json()?;

        // Test is_connection_open - should be false for Init state
        let open_result = contract
            .view("ibc_is_connection_open")
            .args_json(json!({"connection_id": connection_id}))
            .await?;

        let is_open: bool = open_result.json()?;
        assert!(!is_open);

        // Test for non-existent connection
        let nonexistent_result = contract
            .view("ibc_is_connection_open")
            .args_json(json!({"connection_id": "connection-999"}))
            .await?;

        let is_nonexistent_open: bool = nonexistent_result.json()?;
        assert!(!is_nonexistent_open);

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_connections() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let contract = deploy_cosmos_contract(&worker).await?;
        let user = create_test_account(&worker, "user").await?;

        // Create clients first
        let client_id1 = create_test_client(&user, &contract, "test-chain-1").await?;
        let client_id2 = create_test_client(&user, &contract, "test-chain-2").await?;

        // Create first connection
        let conn1_result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id1,
                "counterparty_client_id": "07-tendermint-0",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        let connection_id1: String = conn1_result.json()?;

        // Create second connection
        let conn2_result = user
            .call(contract.id(), "ibc_conn_open_init")
            .args_json(json!({
                "client_id": client_id2,
                "counterparty_client_id": "07-tendermint-1",
                "counterparty_prefix": null,
                "version": create_connection_version(),
                "delay_period": 0
            }))
            .transact()
            .await?;

        let connection_id2: String = conn2_result.json()?;

        // Verify both connections exist and are different
        assert_ne!(connection_id1, connection_id2);
        assert!(connection_id1.starts_with("connection-"));
        assert!(connection_id2.starts_with("connection-"));

        // Verify both connections can be retrieved
        let get1_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id1}))
            .await?;

        let get2_result = contract
            .view("ibc_get_connection")
            .args_json(json!({"connection_id": connection_id2}))
            .await?;

        let conn1: Option<serde_json::Value> = get1_result.json()?;
        let conn2: Option<serde_json::Value> = get2_result.json()?;

        assert!(conn1.is_some());
        assert!(conn2.is_some());

        let c1 = conn1.unwrap();
        let c2 = conn2.unwrap();

        assert_eq!(c1["client_id"], client_id1);
        assert_eq!(c2["client_id"], client_id2);
        assert_eq!(c1["counterparty"]["client_id"], "07-tendermint-0");
        assert_eq!(c2["counterparty"]["client_id"], "07-tendermint-1");

        // Add delay to avoid port conflicts with other test files
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }
}