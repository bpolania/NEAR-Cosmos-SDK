# Deployment Solution: Modular Contract Architecture

## Problem
The current monolithic Cosmos SDK contract has 2,251 functions, exceeding NEAR's runtime limits.

## Solution: Split into Multiple Contracts

### 1. Core Contract Architecture
```
cosmos-sdk-main.near (Main Router Contract)
├── cosmos-sdk-bank.near (Bank Module)
├── cosmos-sdk-staking.near (Staking Module)
├── cosmos-sdk-gov.near (Governance Module)
├── cosmos-sdk-wasm.near (x/wasm Module)
├── cosmos-sdk-ibc-client.near (IBC Client)
├── cosmos-sdk-ibc-connection.near (IBC Connection)
├── cosmos-sdk-ibc-channel.near (IBC Channel)
└── cosmos-sdk-ibc-transfer.near (IBC Transfer)
```

### 2. Implementation Plan

#### Phase 1: Create Individual Module Contracts
Each module becomes its own deployable contract:

**Example: x/wasm Module Contract**
```rust
// crates/cosmos-sdk-wasm-contract/src/lib.rs
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct WasmModuleContract {
    wasm_module: WasmModule,
    main_contract: AccountId, // Reference to main contract
}

#[near_bindgen]
impl WasmModuleContract {
    #[init]
    pub fn new(main_contract: AccountId) -> Self {
        Self {
            wasm_module: WasmModule::new(),
            main_contract,
        }
    }
    
    // All x/wasm methods here
    pub fn store_code(...) -> CodeID { ... }
    pub fn instantiate(...) -> InstantiateResponse { ... }
    // etc...
}
```

#### Phase 2: Main Router Contract
```rust
// crates/cosmos-sdk-main-contract/src/lib.rs
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosMainContract {
    // Module contract addresses
    bank_contract: AccountId,
    staking_contract: AccountId,
    gov_contract: AccountId,
    wasm_contract: AccountId,
    ibc_client_contract: AccountId,
    // ... etc
}

#[near_bindgen]
impl CosmosMainContract {
    // Route calls to appropriate module contracts
    pub fn bank_transfer(&self, receiver: AccountId, amount: Balance) -> Promise {
        ext_bank::ext(self.bank_contract.clone())
            .transfer(receiver, amount)
    }
    
    pub fn wasm_store_code(&self, wasm_byte_code: Vec<u8>, ...) -> Promise {
        ext_wasm::ext(self.wasm_contract.clone())
            .store_code(wasm_byte_code, ...)
    }
}
```

### 3. Benefits
- Each contract stays under NEAR's function limit
- Modules can be updated independently
- Better gas efficiency (only load what you need)
- Easier testing and maintenance
- Follows microservices architecture pattern

### 4. Cross-Contract Communication
Use NEAR's cross-contract calls for module interaction:
```rust
#[ext_contract(ext_bank)]
trait ExtBank {
    fn transfer(&mut self, receiver: AccountId, amount: Balance) -> String;
    fn get_balance(&self, account: AccountId) -> Balance;
}

#[ext_contract(ext_wasm)]
trait ExtWasm {
    fn store_code(&mut self, wasm_byte_code: Vec<u8>, ...) -> CodeID;
    fn instantiate(&mut self, code_id: CodeID, ...) -> InstantiateResponse;
}
```

### 5. Deployment Script
```bash
#!/bin/bash
# Deploy all module contracts
near deploy cosmos-sdk-bank.testnet --wasmFile target/bank.wasm
near deploy cosmos-sdk-staking.testnet --wasmFile target/staking.wasm
near deploy cosmos-sdk-gov.testnet --wasmFile target/gov.wasm
near deploy cosmos-sdk-wasm.testnet --wasmFile target/wasm.wasm
# ... etc

# Deploy main router with module addresses
near deploy cosmos-sdk-main.testnet --wasmFile target/main.wasm \
  --initFunction new \
  --initArgs '{
    "bank_contract": "cosmos-sdk-bank.testnet",
    "staking_contract": "cosmos-sdk-staking.testnet",
    "gov_contract": "cosmos-sdk-gov.testnet",
    "wasm_contract": "cosmos-sdk-wasm.testnet"
  }'
```

## Alternative Solutions

### Option 2: Feature Flags (Quick Fix)
Use Rust feature flags to compile different versions:
```toml
[features]
default = ["bank", "staking"]
full = ["bank", "staking", "gov", "wasm", "ibc"]
bank = []
staking = []
gov = []
wasm = []
ibc = ["ibc-client", "ibc-connection", "ibc-channel", "ibc-transfer"]
```

Then conditionally compile modules:
```rust
#[cfg(feature = "wasm")]
pub mod wasm;
```

### Option 3: Dynamic Module Loading
Store module code as contract state and load dynamically (more complex).

## Recommendation
**Go with Option 1 (Modular Architecture)** because:
- It's the most scalable solution
- Follows best practices for large systems
- Allows independent module updates
- Better for gas optimization
- Easier to test and maintain

## Next Steps
1. Create separate crate for each module contract
2. Implement cross-contract communication interfaces
3. Create deployment scripts
4. Update tests to work with multi-contract setup
5. Document the new architecture