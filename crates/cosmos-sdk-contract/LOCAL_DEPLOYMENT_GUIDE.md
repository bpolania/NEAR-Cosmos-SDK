# Local Deployment Testing Guide

This guide explains how to perform local deployment tests for the CosmWasm compatibility layer and CW20 contracts.

## Overview

The project supports three levels of testing:

1. **Mock Testing** - In-memory simulation using `near_sdk::testing_env`
2. **Local Deployment Testing** - Using NEAR Workspaces sandbox (this guide)  
3. **Testnet Testing** - Deploying to NEAR testnet

## Current Testing Setup

### Mock Testing (Current Implementation)
- **Location**: `src/modules/cosmwasm/real_cw20_wrapper.rs` tests
- **How it works**: Uses `near_sdk::testing_env` for in-memory simulation
- **Pros**: Fast, no network required, full CosmWasm compatibility testing
- **Cons**: Not testing actual deployment

```bash
# Run mock tests
cargo test real_cw20
```

### Local Deployment Testing (This Guide)
- **Location**: `tests/cw20_local_deployment_test.rs`
- **How it works**: Uses NEAR Workspaces sandbox to deploy actual WASM locally
- **Pros**: Tests real deployment process, network interactions
- **Cons**: Requires WASM build, slower than mock tests

```bash
# Run local deployment tests  
cargo test --test cw20_local_deployment_test
```

## Prerequisites

1. **WASM File**: Ensure contract is built
   ```bash
   # If you have cargo-near installed:
   cargo near build
   
   # Or check if WASM exists:
   ls -la target/near/cosmos_sdk_near.wasm
   ```

2. **Dependencies**: The project includes `near-workspaces` in dev-dependencies

## Available Local Deployment Tests

### 1. Basic Cosmos SDK Deployment
```bash
cargo test --test cw20_local_deployment_test test_local_cosmos_deployment
```

**What it tests:**
- Contract deployment to local sandbox
- Bank module functions (mint, transfer, balance queries)
- Block height functionality
- Basic contract initialization

### 2. CosmWasm Compatibility Framework
```bash  
cargo test --test cw20_local_deployment_test test_cosmwasm_compatibility_framework
```

**What it demonstrates:**
- Shows all implemented CosmWasm compatibility components
- Provides roadmap for full CW20 local deployment
- Documents the architecture

### 3. Separate CW20 Contract Pattern (Template)
```bash
cargo test --test cw20_local_deployment_test test_separate_cw20_contract_deployment --ignored
```

**What it shows:**
- Pattern for deploying CW20 wrapper as separate contract
- Framework for future implementation

## How Local Deployment Works

### 1. Sandbox Environment
```rust
let worker = near_workspaces::sandbox().await?;
```
- Creates isolated NEAR blockchain environment
- Runs locally without network connectivity
- Provides full NEAR protocol simulation

### 2. Contract Deployment
```rust
let wasm = std::fs::read("./target/near/cosmos_sdk_near.wasm")?;
let contract = worker.dev_deploy(&wasm).await?;
```
- Deploys actual WASM bytecode
- Gets unique contract account ID
- Initializes contract state

### 3. Account Creation
```rust
let account = worker
    .create_tla(name.parse()?, SecretKey::from_random(KeyType::ED25519))
    .await?
    .result;
```
- Creates test accounts with keys
- Provides accounts for transaction signing
- Simulates real user interactions

### 4. Function Calls
```rust
// Mutable call
let result = account
    .call(contract.id(), "mint")
    .args_json(json!({"receiver": user.id(), "amount": 1000000u64}))
    .max_gas()
    .transact()
    .await?;

// View call  
let balance = contract
    .view("get_balance")
    .args_json(json!({"account": user.id()}))
    .await?;
```

## Current Capabilities

### ✅ Working
- Basic Cosmos SDK contract deployment
- Bank module testing (mint, transfer, balance)
- Block height queries
- Contract initialization
- Account management
- Transaction simulation

### 🚧 In Development  
- Direct CW20 wrapper deployment (requires contract interface updates)
- Full CW20 lifecycle testing via local deployment
- Cross-contract communication testing

## Extending for Full CW20 Testing

To enable complete CW20 local deployment testing, choose one of these approaches:

### Option 1: Add CW20 Methods to Main Contract
Update `src/lib.rs` to expose CW20 wrapper functions:

```rust
#[near_bindgen]
impl CosmosContract {
    // Add these methods
    pub fn new_cw20_wrapper(&mut self, init_msg: Cw20WrapperInitMsg) -> Cw20WrapperResponse {
        // Implementation
    }
    
    pub fn cw20_execute(&mut self, msg: Cw20WrapperExecuteMsg) -> Cw20WrapperResponse {
        // Implementation  
    }
    
    pub fn cw20_query(&self, msg: Cw20WrapperQueryMsg) -> Cw20WrapperResponse {
        // Implementation
    }
}
```

### Option 2: Separate CW20 Contract Build
Create a separate WASM build for just the CW20 wrapper:

```rust
// Create src/cw20_main.rs
use crate::modules::cosmwasm::real_cw20_wrapper::RealCw20Wrapper;

#[near_bindgen]
impl RealCw20Wrapper {
    // Expose as main contract
}
```

## Testnet Deployment

For testnet deployment, use the existing infrastructure:

```bash
# Deploy to testnet
./scripts/deploy_testnet.sh

# Run testnet integration tests
cargo test testnet_integration_tests --ignored
```

## Best Practices

1. **Build First**: Always ensure WASM is up-to-date before testing
2. **Account Management**: Use descriptive account names for debugging
3. **Error Handling**: Check transaction results and parse errors
4. **Gas Limits**: Use `.max_gas()` for testing to avoid gas issues
5. **JSON Format**: Ensure correct JSON serialization for contract calls

## Example Test Structure

```rust
#[tokio::test]
async fn test_my_feature() -> Result<()> {
    // 1. Setup
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let user = create_test_account(&worker, "user").await?;
    
    // 2. Execute operations
    let result = user
        .call(contract.id(), "my_function")
        .args_json(json!({"param": "value"}))
        .max_gas()
        .transact()
        .await?;
    
    // 3. Verify results
    assert!(result.is_success());
    let response = result.json::<MyResponse>()?;
    assert_eq!(response.field, expected_value);
    
    Ok(())
}
```

## Debugging Tips

1. **Transaction Logs**: Use `println!` or `env::log_str` in contract code
2. **Gas Usage**: Check `result.total_gas_burnt` for optimization
3. **State Inspection**: Query contract state after operations
4. **Error Messages**: Parse `result.receipt_failures()` for detailed errors

## Next Steps

1. Choose CW20 integration approach (Option 1 or 2 above)
2. Implement chosen approach
3. Update tests in `cw20_local_deployment_test.rs`
4. Extend to CW721 and other CosmWasm standards
5. Add performance benchmarking to local tests

This framework provides the foundation for comprehensive local deployment testing of CosmWasm contracts on NEAR.