# Modular Architecture Implementation Plan

## Executive Summary
Split the monolithic Cosmos SDK contract (2,251 functions) into 8 separate contracts (~281 functions each).

## Phase 1: Create Module Contracts (Week 1)

### 1.1 x/wasm Module Contract (Priority 1)
```bash
# Create new crate
cargo new --lib crates/cosmos-wasm-module
```

**Contract Features:**
- Store/query WASM code
- Instantiate/execute contracts  
- Access control management
- ~281 functions, ~0.17 MB

### 1.2 Bank Module Contract (Priority 2)
```bash
cargo new --lib crates/cosmos-bank-module
```

**Contract Features:**
- Token transfers
- Balance management
- Multi-asset support

### 1.3 Staking Module Contract (Priority 3)
```bash
cargo new --lib crates/cosmos-staking-module
```

**Contract Features:**
- Validator management
- Delegation/undelegation
- Rewards distribution

## Phase 2: Main Router Contract (Week 2)

### 2.1 Create Router Contract
```rust
// crates/cosmos-router/src/lib.rs
#[near_bindgen]
pub struct CosmosRouter {
    modules: HashMap<String, AccountId>,
}

impl CosmosRouter {
    pub fn route_message(&self, module: String, method: String, args: Vec<u8>) -> Promise {
        let module_addr = self.modules.get(&module)
            .expect("Module not found");
        
        // Cross-contract call
        Promise::new(module_addr.clone())
            .function_call(method, args, 0, Gas(50_000_000_000_000))
    }
}
```

### 2.2 Module Registry
```rust
pub fn register_module(&mut self, name: String, address: AccountId) {
    self.assert_owner();
    self.modules.insert(name, address);
}
```

## Phase 3: Cross-Contract Communication (Week 3)

### 3.1 Define Interfaces
```rust
// Common traits for all modules
#[ext_contract(ext_module)]
trait ModuleInterface {
    fn get_version(&self) -> String;
    fn get_metadata(&self) -> ModuleMetadata;
}

// Specific module interfaces
#[ext_contract(ext_wasm)]
trait WasmModuleInterface {
    fn store_code(&mut self, code: Vec<u8>, ...) -> CodeID;
    fn instantiate(&mut self, code_id: CodeID, ...) -> InstantiateResponse;
}
```

### 3.2 Callback Handling
```rust
#[near_bindgen]
impl CosmosRouter {
    #[private]
    pub fn on_module_call_complete(&mut self, result: String) {
        // Handle cross-contract call results
        env::log_str(&format!("Module call completed: {}", result));
    }
}
```

## Phase 4: Testing & Deployment (Week 4)

### 4.1 Integration Test Suite
```rust
#[tokio::test]
async fn test_modular_deployment() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    
    // Deploy all modules
    let wasm_module = deploy_wasm_module(&worker).await?;
    let bank_module = deploy_bank_module(&worker).await?;
    let router = deploy_router(&worker, vec![
        ("wasm", wasm_module.id()),
        ("bank", bank_module.id()),
    ]).await?;
    
    // Test cross-contract calls
    let result = router
        .call("route_message")
        .args_json(json!({
            "module": "wasm",
            "method": "store_code",
            "args": base64_encode(...)
        }))
        .transact()
        .await?;
        
    assert!(result.is_success());
    Ok(())
}
```

### 4.2 Deployment Script
```bash
#!/bin/bash
# deploy-modular.sh

# Build all modules
./build-modules.sh

# Deploy to testnet
near deploy cosmos-wasm.testnet --wasmFile target/wasm.wasm --initFunction new
near deploy cosmos-bank.testnet --wasmFile target/bank.wasm --initFunction new
near deploy cosmos-staking.testnet --wasmFile target/staking.wasm --initFunction new
near deploy cosmos-gov.testnet --wasmFile target/gov.wasm --initFunction new
near deploy cosmos-ibc-client.testnet --wasmFile target/ibc-client.wasm --initFunction new
near deploy cosmos-ibc-connection.testnet --wasmFile target/ibc-connection.wasm --initFunction new
near deploy cosmos-ibc-channel.testnet --wasmFile target/ibc-channel.wasm --initFunction new
near deploy cosmos-ibc-transfer.testnet --wasmFile target/ibc-transfer.wasm --initFunction new

# Deploy router with module registry
near deploy cosmos-router.testnet --wasmFile target/router.wasm \
  --initFunction new \
  --initArgs '{
    "wasm_module": "cosmos-wasm.testnet",
    "bank_module": "cosmos-bank.testnet",
    ...
  }'
```

## Benefits

1. **Deployable**: Each module <500 functions, well under NEAR limits
2. **Upgradeable**: Update individual modules without affecting others
3. **Gas Efficient**: Only load required modules
4. **Testable**: Test modules in isolation
5. **Scalable**: Add new modules without affecting existing ones

## Migration Path

1. **Existing Users**: Deploy router that mimics current API
2. **New Users**: Direct module access for better performance
3. **Gradual Migration**: Move functionality module by module

## Timeline

- **Week 1**: x/wasm, Bank, Staking modules
- **Week 2**: Router contract and remaining modules
- **Week 3**: Cross-contract communication
- **Week 4**: Testing and deployment

## Success Metrics

- [ ] All modules deploy successfully (<500 functions each)
- [ ] Integration tests pass with modular architecture
- [ ] Gas costs remain reasonable (<20% overhead)
- [ ] API compatibility maintained for existing users