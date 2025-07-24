use anyhow::Result;
use near_workspaces::{Contract, Worker};
use serde_json::json;

const COSMOS_CONTRACT_WASM: &[u8] = include_bytes!("../target/near/cosmos_sdk_near.wasm");

/// Helper function to deploy the Cosmos contract
async fn deploy_cosmos_contract(worker: &Worker<near_workspaces::network::Sandbox>) -> Result<Contract> {
    let contract = worker.dev_deploy(COSMOS_CONTRACT_WASM).await?;
    
    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({}))
        .transact()
        .await?
        .into_result()?;
    
    Ok(contract)
}


#[tokio::test]
async fn test_process_block() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;

    // Get initial block height
    let initial_height: u64 = contract
        .view("get_block_height")
        .await?
        .json()?;

    assert_eq!(initial_height, 0);

    // Process a block
    let result = contract
        .call("process_block")
        .args_json(json!({}))
        .transact()
        .await?;

    assert!(result.is_success());

    // Check block height increased
    let new_height: u64 = contract
        .view("get_block_height")
        .await?
        .json()?;

    assert_eq!(new_height, 1);
    println!("✅ Block: Process block test passed - Height: {} -> {}", initial_height, new_height);
    Ok(())
}

#[tokio::test]
async fn test_multiple_block_processing() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;

    // Process multiple blocks
    for i in 1..=5 {
        contract
            .call("process_block")
            .args_json(json!({}))
            .transact()
            .await?
            .into_result()?;

        let height: u64 = contract
            .view("get_block_height")
            .await?
            .json()?;

        assert_eq!(height, i);
    }
    println!("✅ Block: Multiple block processing test passed - Processed 5 blocks");
    Ok(())
}