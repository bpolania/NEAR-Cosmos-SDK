use anyhow::Result;
use near_workspaces::{types::NearToken, Account, Contract, Worker};
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

/// Helper function to create a test account with some NEAR tokens
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

#[tokio::test]
async fn test_submit_proposal() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let proposer = create_test_account(&worker, "proposer").await?;

    // Submit proposal
    let result = proposer
        .call(contract.id(), "submit_proposal")
        .args_json(json!({
            "title": "Test Proposal",
            "description": "A test governance proposal",
            "param_key": "test_param",
            "param_value": "test_value"
        }))
        .transact()
        .await?;

    assert!(result.is_success());

    // Extract proposal ID from result
    let proposal_id: u64 = result.json()?;
    assert_eq!(proposal_id, 1);
    println!("✅ Governance: Submit proposal test passed - Proposal ID: {}", proposal_id);
    Ok(())
}

#[tokio::test]
async fn test_vote_on_proposal() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let proposer = create_test_account(&worker, "proposer").await?;
    let voter = create_test_account(&worker, "voter").await?;

    // Submit proposal
    let proposal_result = proposer
        .call(contract.id(), "submit_proposal")
        .args_json(json!({
            "title": "Test Proposal",
            "description": "A test governance proposal",
            "param_key": "test_param",
            "param_value": "test_value"
        }))
        .transact()
        .await?;

    let proposal_id: u64 = proposal_result.json()?;

    // Vote on proposal (1 = yes, 0 = no)
    let result = voter
        .call(contract.id(), "vote")
        .args_json(json!({
            "proposal_id": proposal_id,
            "option": 1
        }))
        .transact()
        .await?;

    assert!(result.is_success());
    println!("✅ Governance: Vote test passed - Voted YES on proposal {}", proposal_id);
    Ok(())
}

#[tokio::test]
async fn test_get_parameter() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;

    // Get a parameter (should return empty string for non-existent)
    let param_value: String = contract
        .view("get_parameter")
        .args_json(json!({
            "key": "non_existent_param"
        }))
        .await?
        .json()?;

    assert_eq!(param_value, "");
    println!("✅ Governance: Get parameter test passed - Non-existent param returns empty string");
    Ok(())
}