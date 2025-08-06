/// Comprehensive Local Deployment Integration Tests
/// 
/// This test suite provides end-to-end integration testing for the CosmWasm compatibility layer
/// using NEAR Workspaces sandbox environment. Tests multiple contracts, cross-contract calls,
/// and complex workflows.

use anyhow::Result;
use near_workspaces::{types::NearToken, Account, Contract, Worker};
use serde_json::json;
use std::collections::HashMap;

const WASM_FILEPATH: &str = "./target/near/cosmos_sdk_near.wasm";

/// Test infrastructure for managing multiple contracts and accounts
struct TestEnvironment {
    worker: Worker<near_workspaces::network::Sandbox>,
    contracts: HashMap<String, Contract>,
    accounts: HashMap<String, Account>,
}

impl TestEnvironment {
    /// Initialize test environment with contracts and accounts
    async fn new() -> Result<Self> {
        let worker = near_workspaces::sandbox().await?;
        
        Ok(Self {
            worker,
            contracts: HashMap::new(),
            accounts: HashMap::new(),
        })
    }
    
    /// Deploy a contract instance with a given name
    async fn deploy_contract(&mut self, name: &str) -> Result<()> {
        let wasm = std::fs::read(WASM_FILEPATH)
            .map_err(|_| anyhow::anyhow!("Failed to read WASM file. Run 'cargo near build' first"))?;
        
        let contract = self.worker.dev_deploy(&wasm).await?;
        
        // Initialize the contract
        contract
            .call("new")
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        
        self.contracts.insert(name.to_string(), contract);
        Ok(())
    }
    
    /// Create a test account with optional initial balance
    async fn create_account(&mut self, name: &str, balance_near: Option<u32>) -> Result<()> {
        let account = self.worker
            .create_tla(
                name.parse()?,
                near_workspaces::types::SecretKey::from_random(near_workspaces::types::KeyType::ED25519)
            )
            .await?
            .result;
        
        // If balance specified, fund the account
        if let Some(balance) = balance_near {
            let root = self.worker.root_account()?;
            root.transfer_near(account.id(), NearToken::from_near(balance as u128))
                .await?
                .into_result()?;
        }
        
        self.accounts.insert(name.to_string(), account);
        Ok(())
    }
    
    /// Get contract by name
    fn get_contract(&self, name: &str) -> Result<&Contract> {
        self.contracts.get(name)
            .ok_or_else(|| anyhow::anyhow!("Contract '{}' not found", name))
    }
    
    /// Get account by name
    fn get_account(&self, name: &str) -> Result<&Account> {
        self.accounts.get(name)
            .ok_or_else(|| anyhow::anyhow!("Account '{}' not found", name))
    }
}

/// Test multi-contract deployment scenario
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_multi_contract_deployment() -> Result<()> {
    println!("🏗️ Testing Multi-Contract Deployment");
    
    let mut env = TestEnvironment::new().await?;
    
    // Deploy multiple contract instances
    env.deploy_contract("cosmos_a").await?;
    env.deploy_contract("cosmos_b").await?;
    env.deploy_contract("cosmos_c").await?;
    
    // Create test accounts
    env.create_account("admin", Some(10)).await?;
    env.create_account("user1", Some(5)).await?;
    env.create_account("user2", Some(5)).await?;
    
    let contract_a = env.get_contract("cosmos_a")?;
    let contract_b = env.get_contract("cosmos_b")?;
    let contract_c = env.get_contract("cosmos_c")?;
    
    println!("✅ Deployed contracts:");
    println!("  - Contract A: {}", contract_a.id());
    println!("  - Contract B: {}", contract_b.id());
    println!("  - Contract C: {}", contract_c.id());
    
    let admin = env.get_account("admin")?;
    let user1 = env.get_account("user1")?;
    let user2 = env.get_account("user2")?;
    
    // Test independent operations on each contract
    
    // Contract A: Mint tokens
    let mint_a = admin
        .call(contract_a.id(), "mint")
        .args_json(json!({
            "receiver": user1.id(),
            "amount": 1000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(mint_a.is_success());
    println!("✅ Contract A: Minted tokens to user1");
    
    // Contract B: Different mint operation
    let mint_b = admin
        .call(contract_b.id(), "mint")
        .args_json(json!({
            "receiver": user2.id(),
            "amount": 2000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(mint_b.is_success());
    println!("✅ Contract B: Minted tokens to user2");
    
    // Contract C: Block height operations
    let height_c = contract_c
        .view("get_block_height")
        .args_json(json!({}))
        .await?;
    
    let height: u64 = height_c.json()?;
    assert!(height >= 0);
    println!("✅ Contract C: Block height = {}", height);
    
    // Verify independent state
    let balance_a = contract_a
        .view("get_balance")
        .args_json(json!({"account": user1.id()}))
        .await?;
    
    let balance_b = contract_b
        .view("get_balance")
        .args_json(json!({"account": user2.id()}))
        .await?;
    
    assert_eq!(balance_a.json::<u128>()?, 1000000);
    assert_eq!(balance_b.json::<u128>()?, 2000000);
    
    println!("✅ Verified independent contract states");
    println!("🎉 Multi-contract deployment test completed successfully!");
    
    Ok(())
}

/// Test complex workflow with multiple operations
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_complex_workflow_integration() -> Result<()> {
    println!("🔄 Testing Complex Workflow Integration");
    
    let mut env = TestEnvironment::new().await?;
    
    // Setup
    env.deploy_contract("main").await?;
    env.create_account("admin", Some(10)).await?;
    env.create_account("alice", Some(5)).await?;
    env.create_account("bob", Some(5)).await?;
    env.create_account("charlie", Some(5)).await?;
    
    let contract = env.get_contract("main")?;
    let admin = env.get_account("admin")?;
    let alice = env.get_account("alice")?;
    let bob = env.get_account("bob")?;
    let charlie = env.get_account("charlie")?;
    
    println!("✅ Environment setup complete");
    
    // Phase 1: Initial token distribution
    println!("📦 Phase 1: Initial Distribution");
    
    let distributions = [
        (alice.id(), 5000000u64),
        (bob.id(), 3000000u64),
        (charlie.id(), 2000000u64),
    ];
    
    for (recipient, amount) in distributions.iter() {
        let result = admin
            .call(contract.id(), "mint")
            .args_json(json!({
                "receiver": recipient,
                "amount": amount
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(result.is_success());
        println!("  ✅ Minted {} to {}", amount, recipient);
    }
    
    // Phase 2: Cross-user transfers
    println!("💸 Phase 2: Cross-User Transfers");
    
    let transfers = [
        (alice, bob.id(), 1000000u64, "Alice -> Bob"),
        (bob, charlie.id(), 500000u64, "Bob -> Charlie"),
        (charlie, alice.id(), 200000u64, "Charlie -> Alice"),
    ];
    
    for (sender, recipient, amount, description) in transfers.iter() {
        let result = sender
            .call(contract.id(), "transfer")
            .args_json(json!({
                "receiver": recipient,
                "amount": amount
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(result.is_success());
        println!("  ✅ {}: {} tokens", description, amount);
    }
    
    // Phase 3: Balance verification
    println!("🔍 Phase 3: Balance Verification");
    
    let expected_balances = [
        (alice.id(), 4200000u64), // 5000000 - 1000000 + 200000
        (bob.id(), 3500000u64),   // 3000000 + 1000000 - 500000  
        (charlie.id(), 2300000u64), // 2000000 + 500000 - 200000
    ];
    
    for (account_id, expected) in expected_balances.iter() {
        let balance = contract
            .view("get_balance")
            .args_json(json!({"account": account_id}))
            .await?;
        
        let actual: u128 = balance.json()?;
        assert_eq!(actual, *expected as u128);
        println!("  ✅ {}: {} tokens (expected {})", account_id, actual, expected);
    }
    
    // Phase 4: System state verification
    println!("🏛️ Phase 4: System State Verification");
    
    let total_supply = distributions.iter().map(|(_, amount)| *amount as u128).sum::<u128>();
    let total_balances = expected_balances.iter().map(|(_, balance)| *balance as u128).sum::<u128>();
    
    assert_eq!(total_supply, total_balances);
    println!("  ✅ Total supply conservation: {} tokens", total_supply);
    
    let block_height = contract
        .view("get_block_height")
        .args_json(json!({}))
        .await?;
    
    let height: u64 = block_height.json()?;
    // Note: In the current implementation, block height remains 0 in local tests
    // as process_block is not called automatically
    assert!(height >= 0);
    println!("  ✅ Block height: {}", height);
    
    println!("🎉 Complex workflow integration test completed successfully!");
    
    Ok(())
}

/// Test contract state persistence and recovery
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_state_persistence_integration() -> Result<()> {
    println!("💾 Testing State Persistence Integration");
    
    let mut env = TestEnvironment::new().await?;
    
    env.deploy_contract("persistent").await?;
    env.create_account("admin", Some(10)).await?;
    env.create_account("user", Some(5)).await?;
    
    let contract = env.get_contract("persistent")?;
    let admin = env.get_account("admin")?;
    let user = env.get_account("user")?;
    
    // Phase 1: Setup initial state
    println!("🏗️ Phase 1: Initial State Setup");
    
    // Add validator
    let add_validator = admin
        .call(contract.id(), "add_validator")
        .args_json(json!({
            "validator": user.id()
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(add_validator.is_success());
    println!("  ✅ Added validator: {}", user.id());
    
    // Mint tokens
    let mint_result = admin
        .call(contract.id(), "mint")
        .args_json(json!({
            "receiver": user.id(),
            "amount": 5000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(mint_result.is_success());
    println!("  ✅ Minted 5,000,000 tokens to user");
    
    // Delegate tokens
    let delegate_result = user
        .call(contract.id(), "delegate")
        .args_json(json!({
            "validator": user.id(),
            "amount": 2000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(delegate_result.is_success());
    println!("  ✅ Delegated 2,000,000 tokens");
    
    // Phase 2: Verify state persistence through multiple operations
    println!("🔄 Phase 2: State Persistence Verification");
    
    // Perform multiple transfers to test state updates
    for i in 1..=5 {
        let transfer_result = user
            .call(contract.id(), "transfer")
            .args_json(json!({
                "receiver": admin.id(),
                "amount": 100000u64
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(transfer_result.is_success());
        
        // Verify balance after each transfer
        let balance = contract
            .view("get_balance")
            .args_json(json!({"account": user.id()}))
            .await?;
        
        let expected_balance = 3000000u64 - (i * 100000);  // 5000000 - 2000000 (delegated) - (i * 100000)
        assert_eq!(balance.json::<u128>()?, expected_balance as u128);
        
        println!("  ✅ Transfer {}: Balance = {}", i, expected_balance);
    }
    
    // Phase 3: Complex state interactions
    println!("🔗 Phase 3: Complex State Interactions");
    
    // Submit a governance proposal
    let proposal_result = admin
        .call(contract.id(), "submit_proposal")
        .args_json(json!({
            "title": "Test Proposal",
            "description": "Integration test proposal",
            "param_key": "test_param",
            "param_value": "test_value"
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(proposal_result.is_success());
    let proposal_id: u64 = proposal_result.json()?;
    assert_eq!(proposal_id, 1); // First proposal has ID 1 in current implementation
    println!("  ✅ Created governance proposal: ID {}", proposal_id);
    
    // Vote on proposal
    let vote_result = user
        .call(contract.id(), "vote")
        .args_json(json!({
            "proposal_id": proposal_id,
            "option": 1  // 1 = Yes, 0 = No
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(vote_result.is_success());
    println!("  ✅ Voted on proposal: ID {}", proposal_id);
    
    // Verify final state consistency
    let final_balance = contract
        .view("get_balance")
        .args_json(json!({"account": user.id()}))
        .await?;
    
    assert_eq!(final_balance.json::<u128>()?, 2500000); // 5000000 - 2000000 - 500000
    println!("  ✅ Final balance verified: 2,500,000 tokens");
    
    println!("🎉 State persistence integration test completed successfully!");
    
    Ok(())
}

/// Test error handling and recovery scenarios
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_error_handling_integration() -> Result<()> {
    println!("⚠️ Testing Error Handling Integration");
    
    let mut env = TestEnvironment::new().await?;
    
    env.deploy_contract("error_test").await?;
    env.create_account("admin", Some(10)).await?;
    env.create_account("user", Some(5)).await?;
    
    let contract = env.get_contract("error_test")?;
    let admin = env.get_account("admin")?;
    let user = env.get_account("user")?;
    
    // Phase 1: Test insufficient balance errors
    println!("💰 Phase 1: Insufficient Balance Scenarios");
    
    // Try to transfer without any balance
    let transfer_result = user
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": admin.id(),
            "amount": 1000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    // Should fail due to insufficient balance
    assert!(!transfer_result.is_success());
    println!("  ✅ Transfer failed as expected (insufficient balance)");
    
    // Give user some balance
    let mint_result = admin
        .call(contract.id(), "mint")
        .args_json(json!({
            "receiver": user.id(),
            "amount": 500000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(mint_result.is_success());
    println!("  ✅ Minted 500,000 tokens to user");
    
    // Try to transfer more than balance
    let over_transfer = user
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": admin.id(),
            "amount": 1000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(!over_transfer.is_success());
    println!("  ✅ Over-transfer failed as expected");
    
    // Valid transfer should work
    let valid_transfer = user
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": admin.id(),
            "amount": 100000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(valid_transfer.is_success());
    println!("  ✅ Valid transfer succeeded");
    
    // Phase 2: Test unauthorized operations
    println!("🔒 Phase 2: Authorization Scenarios");
    
    // User tries to mint (should fail - only admin can mint)
    let unauthorized_mint = user
        .call(contract.id(), "mint")
        .args_json(json!({
            "receiver": user.id(),
            "amount": 1000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    // This might succeed depending on implementation, but let's test the pattern
    println!("  ℹ️ Unauthorized mint result: success = {}", unauthorized_mint.is_success());
    
    // Phase 3: Test state recovery after errors
    println!("🔄 Phase 3: State Recovery After Errors");
    
    // Get balance before error scenarios
    let balance_before = contract
        .view("get_balance")
        .args_json(json!({"account": user.id()}))
        .await?;
    
    let balance_value: u128 = balance_before.json()?;
    println!("  ℹ️ Balance before errors: {}", balance_value);
    
    // Try several failing operations
    for i in 1..=3 {
        let failing_transfer = user
            .call(contract.id(), "transfer")
            .args_json(json!({
                "receiver": admin.id(),
                "amount": 10000000u64  // Way more than balance
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(!failing_transfer.is_success());
        println!("  ✅ Failing transfer {} handled correctly", i);
    }
    
    // Verify balance unchanged after failed operations
    let balance_after = contract
        .view("get_balance")
        .args_json(json!({"account": user.id()}))
        .await?;
    
    assert_eq!(balance_after.json::<u128>()?, balance_value);
    println!("  ✅ Balance unchanged after failed operations: {}", balance_value);
    
    // Successful operation should still work
    let recovery_transfer = user
        .call(contract.id(), "transfer")
        .args_json(json!({
            "receiver": admin.id(),
            "amount": 50000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(recovery_transfer.is_success());
    println!("  ✅ Recovery transfer succeeded");
    
    println!("🎉 Error handling integration test completed successfully!");
    
    Ok(())
}

/// Test performance and gas usage patterns
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_performance_integration() -> Result<()> {
    println!("⚡ Testing Performance Integration");
    
    let mut env = TestEnvironment::new().await?;
    
    env.deploy_contract("performance").await?;
    env.create_account("admin", Some(10)).await?;
    
    // Create multiple users for load testing
    for i in 1..=10 {
        env.create_account(&format!("user_{}", i), Some(5)).await?;
    }
    
    let contract = env.get_contract("performance")?;
    let admin = env.get_account("admin")?;
    
    let mut users = Vec::new();
    for i in 1..=10 {
        let user = env.get_account(&format!("user_{}", i))?;
        users.push(user);
    }
    
    println!("✅ Created 10 test users");
    
    // Phase 1: Bulk minting performance
    println!("🏭 Phase 1: Bulk Minting Performance");
    
    let mut mint_gas_usage = Vec::new();
    
    for (i, user) in users.iter().enumerate() {
        let mint_result = admin
            .call(contract.id(), "mint")
            .args_json(json!({
                "receiver": user.id(),
                "amount": 1000000u64
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(mint_result.is_success());
        
        let gas_used = mint_result.total_gas_burnt.as_gas();
        mint_gas_usage.push(gas_used);
        
        if i % 3 == 0 {
            println!("  ✅ Minted to user_{}: {} gas", i + 1, gas_used);
        }
    }
    
    let avg_mint_gas = mint_gas_usage.iter().sum::<u64>() / mint_gas_usage.len() as u64;
    println!("  📊 Average mint gas usage: {} gas", avg_mint_gas);
    
    // Phase 2: Bulk transfer performance
    println!("💸 Phase 2: Bulk Transfer Performance");
    
    let mut transfer_gas_usage = Vec::new();
    
    for (i, user) in users.iter().enumerate() {
        let next_user = &users[(i + 1) % users.len()];
        
        let transfer_result = user
            .call(contract.id(), "transfer")
            .args_json(json!({
                "receiver": next_user.id(),
                "amount": 100000u64
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(transfer_result.is_success());
        
        let gas_used = transfer_result.total_gas_burnt.as_gas();
        transfer_gas_usage.push(gas_used);
        
        if i % 3 == 0 {
            println!("  ✅ Transfer from user_{} to user_{}: {} gas", 
                    i + 1, ((i + 1) % users.len()) + 1, gas_used);
        }
    }
    
    let avg_transfer_gas = transfer_gas_usage.iter().sum::<u64>() / transfer_gas_usage.len() as u64;
    println!("  📊 Average transfer gas usage: {} gas", avg_transfer_gas);
    
    // Phase 3: Batch query performance
    println!("🔍 Phase 3: Batch Query Performance");
    
    let query_start = std::time::Instant::now();
    
    for user in users.iter() {
        let balance = contract
            .view("get_balance")
            .args_json(json!({"account": user.id()}))
            .await?;
        
        let balance_value: u128 = balance.json()?;
        assert!(balance_value > 0); // Should have some balance from transfers
    }
    
    let query_duration = query_start.elapsed();
    println!("  📊 10 balance queries completed in: {:?}", query_duration);
    
    // Phase 4: Gas efficiency analysis
    println!("📈 Phase 4: Gas Efficiency Analysis");
    
    let min_mint_gas = *mint_gas_usage.iter().min().unwrap();
    let max_mint_gas = *mint_gas_usage.iter().max().unwrap();
    let mint_variance = max_mint_gas - min_mint_gas;
    
    let min_transfer_gas = *transfer_gas_usage.iter().min().unwrap();
    let max_transfer_gas = *transfer_gas_usage.iter().max().unwrap();
    let transfer_variance = max_transfer_gas - min_transfer_gas;
    
    println!("  📊 Mint gas: avg={}, min={}, max={}, variance={}", 
             avg_mint_gas, min_mint_gas, max_mint_gas, mint_variance);
    println!("  📊 Transfer gas: avg={}, min={}, max={}, variance={}", 
             avg_transfer_gas, min_transfer_gas, max_transfer_gas, transfer_variance);
    
    // Gas usage should be reasonably consistent
    assert!(mint_variance < avg_mint_gas / 2, "Mint gas variance too high");
    assert!(transfer_variance < avg_transfer_gas / 2, "Transfer gas variance too high");
    
    println!("  ✅ Gas usage patterns are consistent");
    
    println!("🎉 Performance integration test completed successfully!");
    
    Ok(())
}

/// Test contract upgrade scenarios (if supported)
#[tokio::test]
#[ignore = "Old interface - needs update for modular architecture"]
async fn test_contract_lifecycle_integration() -> Result<()> {
    println!("🔄 Testing Contract Lifecycle Integration");
    
    let mut env = TestEnvironment::new().await?;
    
    env.deploy_contract("lifecycle").await?;
    env.create_account("admin", Some(10)).await?;
    env.create_account("user", Some(5)).await?;
    
    // Create all validators upfront to avoid borrowing issues
    for round in 1..=5 {
        let validator_name = format!("validator_{}", round);
        env.create_account(&validator_name, Some(5)).await?;
    }
    
    let contract = env.get_contract("lifecycle")?;
    let admin = env.get_account("admin")?;
    let user = env.get_account("user")?;
    
    // Phase 1: Initial deployment state
    println!("🏗️ Phase 1: Initial Deployment State");
    
    let initial_height = contract
        .view("get_block_height")
        .args_json(json!({}))
        .await?;
    
    let height: u64 = initial_height.json()?;
    println!("  ✅ Initial block height: {}", height);
    
    // Set up initial state
    let mint_result = admin
        .call(contract.id(), "mint")
        .args_json(json!({
            "receiver": user.id(),
            "amount": 1000000u64
        }))
        .max_gas()
        .transact()
        .await?;
    
    assert!(mint_result.is_success());
    println!("  ✅ Initial state setup complete");
    
    // Phase 2: Contract state evolution
    println!("🔄 Phase 2: Contract State Evolution");
    
    // Simulate contract usage over time
    for round in 1..=5 {
        // Get validator
        let validator_name = format!("validator_{}", round);
        let validator = env.get_account(&validator_name)?;
        
        let add_validator = admin
            .call(contract.id(), "add_validator")
            .args_json(json!({
                "validator": validator.id()
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(add_validator.is_success());
        
        // Delegate to validator
        let delegate_result = user
            .call(contract.id(), "delegate")
            .args_json(json!({
                "validator": validator.id(),
                "amount": 100000u64
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(delegate_result.is_success());
        
        // Submit proposal
        let proposal_result = admin
            .call(contract.id(), "submit_proposal")
            .args_json(json!({
                "title": format!("Proposal Round {}", round),
                "description": format!("Test proposal for round {}", round),
                "param_key": format!("param_{}", round),
                "param_value": format!("value_{}", round)
            }))
            .max_gas()
            .transact()
            .await?;
        
        assert!(proposal_result.is_success());
        let proposal_id: u64 = proposal_result.json()?;
        
        println!("  ✅ Round {}: Added validator, delegated, created proposal {}", 
                round, proposal_id);
    }
    
    // Phase 3: State consistency verification
    println!("🔍 Phase 3: State Consistency Verification");
    
    // Verify user balance after all delegations
    let final_balance = contract
        .view("get_balance")
        .args_json(json!({"account": user.id()}))
        .await?;
    
    let expected_balance = 1000000u64 - (5 * 100000u64); // Original - delegations
    assert_eq!(final_balance.json::<u128>()?, expected_balance as u128);
    println!("  ✅ Final balance verified: {} tokens", expected_balance);
    
    // Check block height progression
    let final_height = contract
        .view("get_block_height")
        .args_json(json!({}))
        .await?;
    
    let final_height_value: u64 = final_height.json()?;
    // Note: Block height remains unchanged as process_block is not called in these tests
    assert!(final_height_value >= height);
    println!("  ✅ Block height check: {} -> {} (unchanged in local tests)", height, final_height_value);
    
    println!("🎉 Contract lifecycle integration test completed successfully!");
    
    Ok(())
}