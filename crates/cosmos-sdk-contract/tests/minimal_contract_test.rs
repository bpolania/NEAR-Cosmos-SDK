/// Minimal test to diagnose contract deployment issues
/// This will help us isolate whether the problem is contract size, initialization, or environment

use anyhow::Result;

const WASM_FILEPATH: &str = "./target/near/cosmos_sdk_near.wasm";

/// Test 1: Can we deploy the contract without calling new()?
#[tokio::test]
async fn test_contract_deployment_only() -> Result<()> {
    println!("🔍 Testing contract deployment without initialization...");
    
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(WASM_FILEPATH)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM file"))?;
    
    // Just deploy, don't initialize
    let contract = worker.dev_deploy(&wasm).await?;
    println!("✅ Contract deployed successfully: {}", contract.id());
    
    Ok(())
}

/// Test 2: Can we call a simple view method without initialization?
#[tokio::test]
async fn test_simple_view_call() -> Result<()> {
    println!("🔍 Testing simple view call...");
    
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(WASM_FILEPATH)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM file"))?;
    
    let contract = worker.dev_deploy(&wasm).await?;
    
    // Try a simple view call that doesn't require initialization
    let result = contract
        .view("get_block_height")
        .await;
    
    match result {
        Ok(height) => {
            let height: u64 = height.json()?;
            println!("✅ View call successful, block height: {}", height);
        }
        Err(e) => {
            println!("❌ View call failed: {}", e);
            // This is expected if the contract isn't initialized
        }
    }
    
    Ok(())
}

/// Test 3: Try initialization with detailed error reporting
#[tokio::test]
async fn test_initialization_with_details() -> Result<()> {
    println!("🔍 Testing contract initialization with detailed error reporting...");
    
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(WASM_FILEPATH)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM file"))?;
    
    println!("📦 WASM file size: {} bytes", wasm.len());
    
    let contract = worker.dev_deploy(&wasm).await?;
    println!("✅ Contract deployed: {}", contract.id());
    
    // Try initialization with maximum gas and detailed reporting
    let init_result = contract
        .call("new")
        .gas(near_workspaces::types::Gas::from_tgas(300)) // 300 TGas - maximum
        .transact()
        .await?;
    
    println!("📊 Initialization result:");
    println!("  Success: {}", init_result.is_success());
    println!("  Gas burnt: {:?}", init_result.total_gas_burnt);
    println!("  Logs: {:#?}", init_result.logs());
    
    if !init_result.is_success() {
        println!("❌ Detailed failure info:");
        println!("{:#?}", init_result);
        return Err(anyhow::anyhow!("Contract initialization failed"));
    }
    
    println!("✅ Contract initialized successfully!");
    Ok(())
}

/// Test 4: Test with a smaller gas limit to see if it's a gas issue
#[tokio::test]
async fn test_with_smaller_gas() -> Result<()> {
    println!("🔍 Testing with smaller gas limit...");
    
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(WASM_FILEPATH)
        .map_err(|_| anyhow::anyhow!("Failed to read WASM file"))?;
    
    let contract = worker.dev_deploy(&wasm).await?;
    
    // Try with progressively smaller gas limits
    let gas_limits = [10, 30, 50, 100]; // TGas
    
    for gas_limit in gas_limits {
        println!("  Trying with {} TGas...", gas_limit);
        
        let init_result = contract
            .call("new")
            .gas(near_workspaces::types::Gas::from_tgas(gas_limit))
            .transact()
            .await?;
        
        println!("    Gas burnt: {:?}", init_result.total_gas_burnt);
        
        if init_result.is_success() {
            println!("✅ Succeeded with {} TGas", gas_limit);
            return Ok(());
        } else {
            println!("❌ Failed with {} TGas", gas_limit);
        }
    }
    
    println!("❌ All gas limits failed");
    Ok(())
}