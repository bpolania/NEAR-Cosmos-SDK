# CosmWasm Compatibility Layer Architecture Design

## Overview

This document outlines the architecture for implementing CosmWasm compatibility within Proxima, enabling existing Cosmos ecosystem contracts to run on NEAR with minimal modifications while maintaining performance and security guarantees.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    CosmWasm Contract                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │ instantiate │ │   execute   │ │    query    │ │   migrate   ││
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                 CosmWasm Compatibility Layer                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │           API Translation Module                            ││
│  │  • Deps/DepsMut → NEAR env functions                       ││
│  │  • Storage trait → NEAR collections                        ││
│  │  • MessageInfo → Proxima context                           ││
│  │  • Response → NEAR method response                         ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │           Memory Management Bridge                          ││
│  │  • CosmWasm allocate/deallocate → NEAR registers           ││
│  │  • Binary data handling                                    ││
│  │  • Serialization compatibility                             ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │           Contract Lifecycle Manager                        ││
│  │  • Entry point routing                                     ││
│  │  • State management                                        ││
│  │  • Migration handling                                      ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Proxima Runtime                               │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │    Bank     │ │   Staking   │ │     Gov     │ │     IBC     ││
│  │   Module    │ │   Module    │ │   Module    │ │   Module    ││
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      NEAR Runtime                                │
│           Storage • Gas Metering • Host Functions               │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. API Translation Module

#### 1.1 Dependencies Structure Translation

**CosmWasm Deps/DepsMut Implementation:**

```rust
use near_sdk::env;
use crate::cosmwasm::storage::CosmWasmStorage;
use crate::cosmwasm::api::CosmWasmApi;
use crate::cosmwasm::querier::CosmWasmQuerier;

pub struct CosmWasmDeps<'a> {
    pub storage: &'a CosmWasmStorage,
    pub api: &'a CosmWasmApi,
    pub querier: CosmWasmQuerier<'a>,
}

pub struct CosmWasmDepsMut<'a> {
    pub storage: &'a mut CosmWasmStorage,
    pub api: &'a CosmWasmApi,
    pub querier: CosmWasmQuerier<'a>,
}

impl<'a> CosmWasmDeps<'a> {
    pub fn new(storage: &'a CosmWasmStorage, api: &'a CosmWasmApi) -> Self {
        Self {
            storage,
            api,
            querier: CosmWasmQuerier::new(),
        }
    }
}
```

#### 1.2 Environment Information Translation

**Env Structure Implementation:**

```rust
use near_sdk::env;
use cosmwasm_std::{Env, BlockInfo, ContractInfo, TransactionInfo};

pub fn get_cosmwasm_env() -> Env {
    Env {
        block: BlockInfo {
            height: env::block_height(),
            time: Timestamp::from_nanos(env::block_timestamp()),
            chain_id: "near-testnet".to_string(), // Configurable
        },
        transaction: Some(TransactionInfo {
            index: None, // NEAR doesn't have transaction index concept
        }),
        contract: ContractInfo {
            address: Addr::unchecked(env::current_account_id().to_string()),
        },
    }
}
```

#### 1.3 Message Info Translation

**MessageInfo from NEAR Context:**

```rust
use cosmwasm_std::{MessageInfo, Addr, Coin};
use near_sdk::{env, AccountId};

pub fn get_message_info() -> MessageInfo {
    MessageInfo {
        sender: Addr::unchecked(env::predecessor_account_id().to_string()),
        funds: get_attached_funds(), // Convert NEAR attached deposit to Cosmos coins
    }
}

fn get_attached_funds() -> Vec<Coin> {
    let attached_deposit = env::attached_deposit();
    if attached_deposit > 0 {
        vec![Coin {
            denom: "near".to_string(),
            amount: attached_deposit.into(),
        }]
    } else {
        vec![]
    }
}
```

### 2. Storage Translation Layer

#### 2.1 CosmWasm Storage Trait Implementation

**Storage Interface:**

```rust
use near_sdk::collections::UnorderedMap;
use cosmwasm_std::Storage;

pub struct CosmWasmStorage {
    // Raw key-value storage to match CosmWasm's byte-oriented approach
    data: UnorderedMap<Vec<u8>, Vec<u8>>,
}

impl Storage for CosmWasmStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(&key.to_vec())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data.insert(&key.to_vec(), &value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.remove(&key.to_vec());
    }
}

impl CosmWasmStorage {
    pub fn new() -> Self {
        Self {
            data: UnorderedMap::new(b"cosmwasm_storage"),
        }
    }
}
```

#### 2.2 Advanced Storage Operations

**Iterator Support for Range Queries:**

```rust
use cosmwasm_std::{Order, Record};

impl CosmWasmStorage {
    pub fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // Implementation for range queries
        // This is complex due to NEAR's collection limitations
        // May require custom implementation or caching layer
        todo!("Implement range queries")
    }
}
```

### 3. Memory Management Bridge

#### 3.1 Allocation Translation

**Memory Manager:**

```rust
use std::collections::HashMap;

pub struct CosmWasmMemoryManager {
    // Simulate CosmWasm's allocate/deallocate using NEAR's register system
    registers: HashMap<usize, Vec<u8>>,
    next_id: usize,
}

impl CosmWasmMemoryManager {
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn allocate(&mut self, size: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.registers.insert(id, vec![0; size]);
        id
    }

    pub fn deallocate(&mut self, ptr: usize) {
        self.registers.remove(&ptr);
    }

    pub fn write(&mut self, ptr: usize, data: &[u8]) -> Result<(), String> {
        if let Some(buffer) = self.registers.get_mut(&ptr) {
            if data.len() <= buffer.len() {
                buffer[..data.len()].copy_from_slice(data);
                Ok(())
            } else {
                Err("Buffer overflow".to_string())
            }
        } else {
            Err("Invalid pointer".to_string())
        }
    }

    pub fn read(&self, ptr: usize, len: usize) -> Result<Vec<u8>, String> {
        if let Some(buffer) = self.registers.get(&ptr) {
            if len <= buffer.len() {
                Ok(buffer[..len].to_vec())
            } else {
                Err("Read beyond buffer".to_string())
            }
        } else {
            Err("Invalid pointer".to_string())
        }
    }
}
```

### 4. Contract Lifecycle Manager

#### 4.1 Entry Point Router

**Contract Wrapper:**

```rust
use near_sdk::{near_bindgen, PanicOnDefault};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

#[near_bindgen]
#[derive(PanicOnDefault)]
pub struct CosmWasmContract {
    storage: CosmWasmStorage,
    memory_manager: CosmWasmMemoryManager,
    initialized: bool,
}

#[near_bindgen]
impl CosmWasmContract {
    #[init]
    pub fn new() -> Self {
        Self {
            storage: CosmWasmStorage::new(),
            memory_manager: CosmWasmMemoryManager::new(),
            initialized: false,
        }
    }

    // CosmWasm instantiate entry point
    pub fn cosmwasm_instantiate(&mut self, msg: String) -> String {
        if self.initialized {
            env::panic_str("Contract already initialized");
        }

        let deps_mut = self.get_deps_mut();
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        // Call the actual CosmWasm contract's instantiate function
        let result = self.call_instantiate(deps_mut, env, info, msg);
        
        match result {
            Ok(response) => {
                self.initialized = true;
                self.process_response(response)
            },
            Err(e) => env::panic_str(&e.to_string()),
        }
    }

    // CosmWasm execute entry point
    pub fn cosmwasm_execute(&mut self, msg: String) -> String {
        if !self.initialized {
            env::panic_str("Contract not initialized");
        }

        let deps_mut = self.get_deps_mut();
        let env = get_cosmwasm_env();
        let info = get_message_info();
        
        let result = self.call_execute(deps_mut, env, info, msg);
        
        match result {
            Ok(response) => self.process_response(response),
            Err(e) => env::panic_str(&e.to_string()),
        }
    }

    // CosmWasm query entry point
    pub fn cosmwasm_query(&self, msg: String) -> String {
        let deps = self.get_deps();
        let env = get_cosmwasm_env();
        
        let result = self.call_query(deps, env, msg);
        
        match result {
            Ok(binary) => base64::encode(binary.as_slice()),
            Err(e) => env::panic_str(&e.to_string()),
        }
    }

    // CosmWasm migrate entry point
    pub fn cosmwasm_migrate(&mut self, msg: String) -> String {
        let deps_mut = self.get_deps_mut();
        let env = get_cosmwasm_env();
        
        let result = self.call_migrate(deps_mut, env, msg);
        
        match result {
            Ok(response) => self.process_response(response),
            Err(e) => env::panic_str(&e.to_string()),
        }
    }
}
```

#### 4.2 Response Processing

**Response Translation:**

```rust
impl CosmWasmContract {
    fn process_response(&self, response: Response) -> String {
        // Convert CosmWasm Response to NEAR-compatible format
        
        // Log events as NEAR logs
        for event in response.events {
            let log_msg = format!("EVENT_TYPE={} {}", 
                event.ty, 
                event.attributes.iter()
                    .map(|attr| format!("{}={}", attr.key, attr.value))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            env::log_str(&log_msg);
        }

        // Log attributes
        for attr in response.attributes {
            env::log_str(&format!("ATTR: {}={}", attr.key, attr.value));
        }

        // Handle sub-messages (complex - may require promise scheduling)
        if !response.messages.is_empty() {
            // This is one of the most complex parts - CosmWasm sub-messages
            // need to be translated to NEAR cross-contract calls
            self.handle_sub_messages(response.messages);
        }

        // Return data
        response.data
            .map(|d| base64::encode(d.as_slice()))
            .unwrap_or_else(|| "{}".to_string())
    }

    fn handle_sub_messages(&self, messages: Vec<SubMsg>) {
        // Complex implementation needed for cross-contract calls
        // May require promise chaining and callback handling
        todo!("Implement sub-message handling")
    }
}
```

### 5. API Compatibility Layer

#### 5.1 Cryptographic Functions

**Crypto API Implementation:**

```rust
use cosmwasm_std::{
    secp256k1_verify, ed25519_verify, secp256k1_recover_pubkey,
    Secp256k1VerifyError, Ed25519VerifyError, RecoverPubkeyError
};
use near_sdk::env;

pub struct CosmWasmApi;

impl CosmWasmApi {
    pub fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Secp256k1VerifyError> {
        // Use NEAR's crypto functions or implement compatibility layer
        // This may require additional cryptographic libraries
        todo!("Implement secp256k1_verify")
    }

    pub fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Ed25519VerifyError> {
        // NEAR has built-in ed25519 support
        match env::ed25519_verify(signature, message, public_key) {
            true => Ok(true),
            false => Ok(false),
        }
    }

    pub fn addr_validate(&self, input: &str) -> Result<cosmwasm_std::Addr, cosmwasm_std::StdError> {
        // Validate addresses (may need to support both NEAR and Cosmos formats)
        if input.ends_with(".near") || input.len() == 64 {
            // NEAR address
            Ok(cosmwasm_std::Addr::unchecked(input))
        } else if input.starts_with("cosmos1") && input.len() == 45 {
            // Cosmos address
            Ok(cosmwasm_std::Addr::unchecked(input))
        } else {
            Err(cosmwasm_std::StdError::generic_err("Invalid address format"))
        }
    }
}
```

### 6. Integration with Proxima Modules

#### 6.1 Bank Module Integration

**Token Operations:**

```rust
impl CosmWasmQuerier {
    pub fn query_balance(&self, address: &str, denom: &str) -> StdResult<cosmwasm_std::Coin> {
        // Query Proxima's Bank module for balance
        // This requires integration with the existing Bank module
        
        if denom == "near" {
            // Query NEAR balance
            let balance = self.query_near_balance(address)?;
            Ok(cosmwasm_std::Coin {
                denom: "near".to_string(),
                amount: balance.into(),
            })
        } else {
            // Query custom token balance from Bank module
            let balance = self.query_bank_balance(address, denom)?;
            Ok(cosmwasm_std::Coin {
                denom: denom.to_string(),
                amount: balance.into(),
            })
        }
    }
}
```

#### 6.2 IBC Integration

**Cross-Chain Queries:**

```rust
impl CosmWasmQuerier {
    pub fn query_ibc_channel(&self, port_id: &str, channel_id: &str) -> StdResult<cosmwasm_std::IbcChannel> {
        // Query Proxima's IBC module
        // Convert IBC channel information to CosmWasm format
        todo!("Implement IBC channel queries")
    }
}
```

## Implementation Challenges & Solutions

### 1. Sub-Message Handling

**Challenge**: CosmWasm's sub-messages enable complex cross-contract interactions that don't directly map to NEAR's promise system.

**Solution**: 
- Implement a message queue system
- Use NEAR promises for cross-contract calls
- Implement callback handlers for sub-message responses

### 2. Gas Metering Compatibility

**Challenge**: Different gas models between CosmWasm and NEAR.

**Solution**:
- Create gas conversion table
- Pre-calculate approximate NEAR gas requirements
- Implement gas limiting wrappers

### 3. Iterator Support

**Challenge**: CosmWasm contracts rely heavily on storage iteration, which is expensive on NEAR.

**Solution**:
- Implement efficient range query caching
- Use NEAR's UnorderedMap with careful key design
- Potentially limit iterator usage or implement pagination

### 4. Address Format Compatibility

**Challenge**: Different address formats between Cosmos and NEAR.

**Solution**:
- Support both address formats in validation
- Implement address translation utilities
- Maintain address mapping for cross-ecosystem interactions

## Performance Considerations

### Memory Usage
- CosmWasm contracts typically use less memory than NEAR contracts
- The compatibility layer adds overhead
- Need to optimize memory allocations and buffer management

### Gas Efficiency
- Translation layer introduces gas overhead
- Critical paths need optimization
- May require special handling for gas-intensive operations

### Storage Costs
- NEAR storage costs are different from Cosmos
- May need to implement storage optimization patterns
- Consider trade-offs between storage and computation costs

## Security Considerations

### Isolation
- Ensure CosmWasm contracts can't break out of compatibility layer
- Validate all inputs and outputs
- Prevent access to NEAR-specific functions that could compromise security

### State Consistency
- Ensure atomic operations across the compatibility layer
- Prevent state corruption during failures
- Implement proper rollback mechanisms

### Access Control
- Translate CosmWasm's sender validation to NEAR's predecessor system
- Ensure proper permission checking
- Validate cross-contract call permissions

## Migration Path

### For Existing CosmWasm Contracts

1. **Minimal Changes Required**:
   - Update Cargo.toml dependencies
   - Add compatibility layer imports
   - Potentially adjust gas usage patterns

2. **Testing Strategy**:
   - Unit tests should pass without modification
   - Integration tests may require adaptation
   - Performance testing needed for gas optimization

3. **Deployment Process**:
   - Compile with NEAR toolchain
   - Deploy to NEAR testnet
   - Validate functionality against original Cosmos deployment

### For Developers

1. **Development Environment**:
   - Same CosmWasm development patterns
   - NEAR-specific testing tools
   - Compatibility layer documentation

2. **Ecosystem Integration**:
   - Access to NEAR's DeFi ecosystem
   - Integration with Proxima's Cosmos SDK modules
   - Cross-chain functionality via IBC

This architecture provides a comprehensive foundation for CosmWasm compatibility while maintaining the performance and security benefits of the NEAR ecosystem.