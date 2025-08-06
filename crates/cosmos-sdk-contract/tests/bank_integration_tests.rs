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
#[ignore = "Old interface - needs update for modular architecture"]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_mint_tokens() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let user_account = create_test_account(&worker, "user").await?;

    // Mint tokens to user
    let result = contract
        .call("mint")
        .args_json(json!({
            "receiver": user_account.id(),
            "amount": 1000
        }))
        .transact()
        .await?;

    assert!(result.is_success());

    // Check balance
    let balance: u128 = contract
        .view("get_balance")
        .args_json(json!({
            "account": user_account.id()
        }))
        .await?
        .json()?;

    assert_eq!(balance, 1000);
    println!("✅ Bank: Mint test passed - User balance: {}", balance);
    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_transfer_tokens() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;
    let bob = create_test_account(&worker, "bob").await?;

    // Mint tokens to alice
    contract
        .call("mint")
        .args_json(json!({
            "receiver": alice.id(),
            "amount": 1000
        }))
        .transact()
        .await?
        .into_result()?;

    // Transfer from alice to bob
    let result = alice
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": bob.id(),
            "amount": 300
        }))
        .transact()
        .await?;

    assert!(result.is_success());

    // Check balances
    let alice_balance: u128 = contract
        .view("get_balance")
        .args_json(json!({"account": alice.id()}))
        .await?
        .json()?;

    let bob_balance: u128 = contract
        .view("get_balance")
        .args_json(json!({"account": bob.id()}))
        .await?
        .json()?;

    assert_eq!(alice_balance, 700);
    assert_eq!(bob_balance, 300);
    println!("✅ Bank: Transfer test passed - Alice: {}, Bob: {}", alice_balance, bob_balance);
    Ok(())
}

#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_insufficient_balance_transfer() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;
    let bob = create_test_account(&worker, "bob").await?;

    // Mint only 100 tokens to alice
    contract
        .call("mint")
        .args_json(json!({
            "receiver": alice.id(),
            "amount": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Try to transfer more than balance
    let result = alice
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": bob.id(),
            "amount": 500
        }))
        .transact()
        .await?;

    // Transaction should fail
    assert!(result.is_failure());
    println!("✅ Bank: Insufficient balance test passed - Transfer properly rejected");
    Ok(())
}