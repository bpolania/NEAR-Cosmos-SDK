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
async fn test_add_validator() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let validator = create_test_account(&worker, "validator").await?;

    // Add validator
    let result = contract
        .call("add_validator")
        .args_json(json!({
            "validator": validator.id()
        }))
        .transact()
        .await?;

    assert!(result.is_success());
    println!("✅ Staking: Add validator test passed - Validator: {}", validator.id());
    Ok(())
}

#[tokio::test]
async fn test_delegate_tokens() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let validator = create_test_account(&worker, "validator").await?;
    let delegator = create_test_account(&worker, "delegator").await?;

    // Add validator
    contract
        .call("add_validator")
        .args_json(json!({
            "validator": validator.id()
        }))
        .transact()
        .await?
        .into_result()?;

    // Mint tokens to delegator
    contract
        .call("mint")
        .args_json(json!({
            "receiver": delegator.id(),
            "amount": 1000
        }))
        .transact()
        .await?
        .into_result()?;

    // Delegate tokens
    let result = delegator
        .call(contract.id(), "delegate")
        .args_json(json!({
            "validator": validator.id(),
            "amount": 500
        }))
        .transact()
        .await?;

    assert!(result.is_success());

    // Check delegator balance (should be reduced)
    let balance: u128 = contract
        .view("get_balance")
        .args_json(json!({
            "account": delegator.id()
        }))
        .await?
        .json()?;

    assert_eq!(balance, 500);
    println!("✅ Staking: Delegate test passed - Remaining balance: {}", balance);
    Ok(())
}

#[tokio::test]
async fn test_undelegate_tokens() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let validator = create_test_account(&worker, "validator").await?;
    let delegator = create_test_account(&worker, "delegator").await?;

    // Setup: add validator, mint tokens, delegate
    contract
        .call("add_validator")
        .args_json(json!({"validator": validator.id()}))
        .transact()
        .await?
        .into_result()?;

    contract
        .call("mint")
        .args_json(json!({
            "receiver": delegator.id(),
            "amount": 1000
        }))
        .transact()
        .await?
        .into_result()?;

    delegator
        .call(contract.id(), "delegate")
        .args_json(json!({
            "validator": validator.id(),
            "amount": 500
        }))
        .transact()
        .await?
        .into_result()?;

    // Undelegate tokens
    let result = delegator
        .call(contract.id(), "undelegate")
        .args_json(json!({
            "validator": validator.id(),
            "amount": 200
        }))
        .transact()
        .await?;

    assert!(result.is_success());
    println!("✅ Staking: Undelegate test passed - Undelegated 200 tokens");
    Ok(())
}