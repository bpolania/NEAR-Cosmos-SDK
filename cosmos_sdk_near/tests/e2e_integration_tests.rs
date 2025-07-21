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
async fn test_full_cosmos_workflow() -> Result<()> {
    // Add delay to avoid port conflicts with other test files
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    
    // Create test accounts
    let validator = create_test_account(&worker, "validator").await?;
    let alice = create_test_account(&worker, "alice").await?;
    let bob = create_test_account(&worker, "bob").await?;

    // 1. Add validator
    contract
        .call("add_validator")
        .args_json(json!({"validator": validator.id()}))
        .transact()
        .await?
        .into_result()?;

    // 2. Mint tokens to alice
    contract
        .call("mint")
        .args_json(json!({
            "receiver": alice.id(),
            "amount": 1000
        }))
        .transact()
        .await?
        .into_result()?;

    // 3. Alice transfers some tokens to bob
    alice
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": bob.id(),
            "amount": 200
        }))
        .transact()
        .await?
        .into_result()?;

    // 4. Alice delegates tokens to validator
    alice
        .call(contract.id(), "delegate")
        .args_json(json!({
            "validator": validator.id(),
            "amount": 300
        }))
        .transact()
        .await?
        .into_result()?;

    // 5. Submit governance proposal
    let proposal_result = alice
        .call(contract.id(), "submit_proposal")
        .args_json(json!({
            "title": "Increase Rewards",
            "description": "Proposal to increase staking rewards",
            "param_key": "reward_rate",
            "param_value": "10"
        }))
        .transact()
        .await?;

    let proposal_id: u64 = proposal_result.json()?;

    // 6. Vote on proposal
    alice
        .call(contract.id(), "vote")
        .args_json(json!({
            "proposal_id": proposal_id,
            "option": 1
        }))
        .transact()
        .await?
        .into_result()?;

    // 7. Process blocks to advance time
    for _ in 0..10 {
        contract
            .call("process_block")
            .args_json(json!({}))
            .transact()
            .await?
            .into_result()?;
    }

    // 8. Verify final state
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

    let block_height: u64 = contract
        .view("get_block_height")
        .await?
        .json()?;

    // Verify expected outcomes
    assert_eq!(alice_balance, 650); // 1000 - 200 (transfer) - 300 (delegation) + 150 (rewards: 300*5%*10 blocks)
    assert_eq!(bob_balance, 200);   // received from transfer
    assert_eq!(block_height, 10);   // processed 10 blocks

    println!("✅ E2E: Full Cosmos workflow test passed");
    println!("   Alice final balance: {} (transfer: -200, delegation: -300, rewards: +150)", alice_balance);
    println!("   Bob final balance: {} (received transfer)", bob_balance);
    println!("   Processed {} blocks", block_height);
    println!("   Proposal {} submitted and voted on", proposal_id);

    Ok(())
}