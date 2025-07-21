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

#[cfg(test)]
mod bank_module_tests {
    use super::*;

    #[tokio::test]
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
        Ok(())
    }

    #[tokio::test]
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
        Ok(())
    }

    #[tokio::test]
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
        Ok(())
    }
}

#[cfg(test)]
mod staking_module_tests {
    use super::*;

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
        Ok(())
    }
}

#[cfg(test)]
mod governance_module_tests {
    use super::*;

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
        Ok(())
    }
}

#[cfg(test)]
mod block_processing_tests {
    use super::*;

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
        Ok(())
    }
}

#[cfg(test)]
mod end_to_end_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_cosmos_workflow() -> Result<()> {
        // Add delay to avoid port conflicts with other test files
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
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

        Ok(())
    }
}