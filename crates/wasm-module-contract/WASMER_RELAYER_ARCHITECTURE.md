# Wasmer-Enabled Relayer Architecture for True CosmWasm Execution

## Executive Summary

This document proposes a comprehensive architecture that extends the existing IBC relayer infrastructure to provide true CosmWasm contract execution on NEAR, delivering an authentic Cosmos developer experience.

## Current State Analysis

### Existing Infrastructure

#### 1. IBC Relayer System
- **Purpose**: Relays IBC packets between NEAR and Cosmos chains
- **Components**:
  - `RelayEngine`: Orchestrates packet processing
  - `Chain` trait: Abstracts chain operations
  - `PacketProcessor`: Handles relay logic
  - Event monitoring and packet lifecycle tracking

#### 2. NEAR Smart Contracts
- **Router Contract**: Routes messages to appropriate modules
- **WASM Module Contract**: Stores CosmWasm bytecode but can't execute it
- **Pattern Matching**: Simulates execution for known contract types

### The Gap
Currently, uploaded CosmWasm contracts are stored but not executed. The pattern matching only works for known contract types (CW20, CW721, CW1).

## Proposed Architecture: Wasmer-Enabled Relayer

### Overview
```
┌─────────────────────────────────────────────────────────────┐
│                     Cosmos Developer                         │
└────────────────────┬────────────────────────────────────────┘
                     │ cosmwasm-cli deploy contract.wasm
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Enhanced IBC Relayer                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            Wasmer Execution Engine                    │  │
│  │  - Compiles WASM                                     │  │
│  │  - Executes with full CosmWasm semantics            │  │
│  │  - Manages state transitions                         │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            Consensus Layer (3+ nodes)                │  │
│  │  - Distributed execution                             │  │
│  │  - Result verification                               │  │
│  │  - Byzantine fault tolerance                         │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────────┘
                     │ Verified results + state changes
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              NEAR Blockchain                                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         WASM Module Contract (on-chain)              │  │
│  │  - Stores WASM bytecode                              │  │
│  │  - Verifies relayer consensus                        │  │
│  │  - Applies state changes                             │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Detailed Implementation Plan

### Phase 1: Extend Relayer with Wasmer Execution

#### 1.1 Create WasmerExecutionService
```rust
// crates/ibc-relayer/src/cosmwasm/execution_service.rs

use wasmer::{Store, Module, Instance, imports};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct WasmerExecutionService {
    /// Wasmer store for all instances
    store: Store,
    
    /// Cache of compiled modules
    module_cache: HashMap<Vec<u8>, Module>, // hash -> Module
    
    /// Active contract instances
    instances: HashMap<String, ContractInstance>,
    
    /// State management
    state_manager: StateManager,
    
    /// Host function implementations
    host_functions: HostFunctions,
}

pub struct ContractInstance {
    instance: Instance,
    contract_address: String,
    code_hash: Vec<u8>,
    state: ContractState,
}

impl WasmerExecutionService {
    /// Execute a CosmWasm contract function
    pub async fn execute_contract(
        &mut self,
        contract_address: &str,
        wasm_code: &[u8],
        entry_point: &str,
        msg: &[u8],
        env: CosmWasmEnv,
    ) -> Result<ExecutionResult, ExecutionError> {
        // 1. Compile or get cached module
        let module = self.get_or_compile_module(wasm_code)?;
        
        // 2. Create/get instance with host functions
        let instance = self.get_or_create_instance(
            contract_address,
            module,
            &env,
        )?;
        
        // 3. Execute the entry point
        let result = self.invoke_entry_point(
            instance,
            entry_point,
            msg,
            env,
        )?;
        
        // 4. Collect state changes
        let state_changes = self.state_manager.get_changes(contract_address);
        
        Ok(ExecutionResult {
            data: result,
            state_changes,
            events: self.collect_events(),
            gas_used: self.calculate_gas_used(),
        })
    }
}
```

#### 1.2 Integrate with Chain Trait
```rust
// crates/ibc-relayer/src/chains/cosmwasm_chain.rs

pub struct CosmWasmChain {
    near_chain: NearModularChain,
    execution_service: Arc<RwLock<WasmerExecutionService>>,
    state_cache: StateCache,
}

#[async_trait]
impl Chain for CosmWasmChain {
    async fn submit_transaction(
        &self,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let tx = decode_cosmwasm_tx(&data)?;
        
        match tx.msg_type {
            MsgType::StoreCode { wasm_byte_code } => {
                // Store on NEAR
                self.near_chain.store_code(wasm_byte_code).await
            }
            MsgType::Instantiate { code_id, init_msg } => {
                // Get WASM from NEAR
                let wasm = self.near_chain.get_code(code_id).await?;
                
                // Execute with Wasmer
                let result = self.execution_service.write().await
                    .execute_contract(
                        &generate_address(code_id),
                        &wasm,
                        "instantiate",
                        &init_msg,
                        create_env(&tx),
                    ).await?;
                
                // Submit state changes to NEAR
                self.near_chain.apply_state_changes(result.state_changes).await
            }
            MsgType::Execute { contract, exec_msg } => {
                // Similar pattern for execute
            }
        }
    }
}
```

### Phase 2: Consensus Layer for Trust

#### 2.1 Multi-Node Execution
```rust
// crates/ibc-relayer/src/cosmwasm/consensus.rs

pub struct ConsensusExecutor {
    /// Multiple execution nodes
    nodes: Vec<ExecutionNode>,
    
    /// Minimum nodes for consensus (e.g., 2/3)
    threshold: usize,
}

pub struct ExecutionNode {
    id: String,
    execution_service: WasmerExecutionService,
    reputation: f64,
}

impl ConsensusExecutor {
    pub async fn execute_with_consensus(
        &self,
        request: ExecutionRequest,
    ) -> Result<ConsensusResult, ConsensusError> {
        // 1. Execute on all nodes in parallel
        let futures: Vec<_> = self.nodes.iter()
            .map(|node| node.execute(request.clone()))
            .collect();
        
        let results = futures::future::join_all(futures).await;
        
        // 2. Verify consensus
        let consensus = self.find_consensus(results)?;
        
        // 3. Create proof of consensus
        let proof = self.create_consensus_proof(&consensus);
        
        Ok(ConsensusResult {
            result: consensus.result,
            proof,
            participating_nodes: consensus.nodes,
        })
    }
    
    fn find_consensus(&self, results: Vec<ExecutionResult>) -> Result<Consensus, Error> {
        // Group results by hash
        let mut groups: HashMap<Vec<u8>, Vec<ExecutionResult>> = HashMap::new();
        
        for result in results {
            let hash = hash_result(&result);
            groups.entry(hash).or_default().push(result);
        }
        
        // Find group with enough agreement
        for (hash, group) in groups {
            if group.len() >= self.threshold {
                return Ok(Consensus {
                    result: group[0].clone(),
                    nodes: group.iter().map(|r| r.node_id.clone()).collect(),
                    agreement_hash: hash,
                });
            }
        }
        
        Err(Error::NoConsensus)
    }
}
```

#### 2.2 On-Chain Verification
```rust
// Update wasm-module-contract/src/lib.rs

impl WasmModuleContract {
    pub fn execute_with_consensus_proof(
        &mut self,
        contract_addr: String,
        msg: String,
        consensus_proof: ConsensusProof,
    ) -> ExecuteResponse {
        // 1. Verify consensus proof
        assert!(
            self.verify_consensus_proof(&consensus_proof),
            "Invalid consensus proof"
        );
        
        // 2. Verify participating nodes are authorized
        assert!(
            self.verify_relayer_authorization(&consensus_proof.nodes),
            "Unauthorized relayers"
        );
        
        // 3. Apply state changes from consensus result
        self.apply_state_changes(
            contract_addr,
            consensus_proof.state_changes,
        );
        
        // 4. Emit events
        ExecuteResponse {
            data: Some(consensus_proof.result_data),
            events: consensus_proof.events,
        }
    }
}
```

### Phase 3: Developer Experience

#### 3.1 CosmWasm CLI Wrapper
```rust
// crates/cosmwasm-near-cli/src/main.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cosmwasm-near")]
#[command(about = "CosmWasm CLI for NEAR deployment")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Relayer endpoint
    #[arg(long, default_value = "http://localhost:8545")]
    relayer: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Store WASM code
    Store {
        /// Path to .wasm file
        file: String,
    },
    /// Instantiate contract
    Instantiate {
        /// Code ID
        code_id: u64,
        /// Initialization message (JSON)
        msg: String,
        /// Label
        label: String,
    },
    /// Execute contract
    Execute {
        /// Contract address
        contract: String,
        /// Execution message (JSON)
        msg: String,
    },
    /// Query contract
    Query {
        /// Contract address
        contract: String,
        /// Query message (JSON)
        msg: String,
    },
}

impl Cli {
    async fn run(self) -> Result<()> {
        let client = RelayerClient::new(&self.relayer);
        
        match self.command {
            Commands::Store { file } => {
                let wasm = std::fs::read(&file)?;
                let result = client.store_code(wasm).await?;
                println!("Code stored with ID: {}", result.code_id);
            }
            Commands::Instantiate { code_id, msg, label } => {
                let result = client.instantiate(code_id, msg, label).await?;
                println!("Contract instantiated at: {}", result.address);
            }
            Commands::Execute { contract, msg } => {
                let result = client.execute(contract, msg).await?;
                println!("Execution result: {}", result);
            }
            Commands::Query { contract, msg } => {
                let result = client.query(contract, msg).await?;
                println!("Query result: {}", result);
            }
        }
        Ok(())
    }
}
```

#### 3.2 Relayer RPC Interface
```rust
// crates/ibc-relayer/src/rpc/cosmwasm.rs

use jsonrpc_core::{IoHandler, Result};
use jsonrpc_derive::rpc;

#[rpc]
pub trait CosmWasmRpc {
    /// Store WASM code
    #[rpc(name = "cosmwasm_storeCode")]
    fn store_code(&self, wasm_base64: String) -> Result<StoreCodeResponse>;
    
    /// Instantiate contract
    #[rpc(name = "cosmwasm_instantiate")]
    fn instantiate(&self, 
        code_id: u64, 
        msg: String,
        label: String,
    ) -> Result<InstantiateResponse>;
    
    /// Execute contract
    #[rpc(name = "cosmwasm_execute")]
    fn execute(&self,
        contract: String,
        msg: String,
    ) -> Result<ExecuteResponse>;
    
    /// Query contract (read-only)
    #[rpc(name = "cosmwasm_query")]
    fn query(&self,
        contract: String,
        msg: String,
    ) -> Result<String>;
}

impl CosmWasmRpc for RelayerRpcImpl {
    fn execute(&self, contract: String, msg: String) -> Result<ExecuteResponse> {
        // 1. Get contract's WASM code from NEAR
        let code = self.near_client.get_contract_code(&contract)?;
        
        // 2. Execute with consensus
        let consensus_result = self.consensus_executor
            .execute_with_consensus(ExecutionRequest {
                contract_address: contract.clone(),
                wasm_code: code,
                entry_point: "execute",
                msg: msg.as_bytes(),
            })?;
        
        // 3. Submit to NEAR with proof
        let tx_hash = self.near_client
            .submit_with_consensus_proof(consensus_result)?;
        
        Ok(ExecuteResponse {
            tx_hash,
            data: consensus_result.data,
            events: consensus_result.events,
        })
    }
}
```

### Phase 4: State Management

#### 4.1 State Synchronization
```rust
// crates/ibc-relayer/src/cosmwasm/state.rs

pub struct StateManager {
    /// Local state cache
    cache: HashMap<String, ContractStateCache>,
    
    /// NEAR state reader
    near_reader: NearStateReader,
    
    /// Pending changes
    pending: HashMap<String, Vec<StateChange>>,
}

pub struct ContractStateCache {
    /// Last synced height
    last_height: u64,
    
    /// Cached key-value pairs
    storage: HashMap<Vec<u8>, Vec<u8>>,
    
    /// Merkle tree for proofs
    merkle_tree: MerkleTree,
}

impl StateManager {
    /// Get value from contract storage
    pub async fn get(&self, contract: &str, key: &[u8]) -> Option<Vec<u8>> {
        // 1. Check cache
        if let Some(cached) = self.cache.get(contract) {
            if let Some(value) = cached.storage.get(key) {
                return Some(value.clone());
            }
        }
        
        // 2. Read from NEAR
        self.near_reader.get_storage(contract, key).await
    }
    
    /// Set value in contract storage (pending)
    pub fn set(&mut self, contract: &str, key: Vec<u8>, value: Vec<u8>) {
        self.pending
            .entry(contract.to_string())
            .or_default()
            .push(StateChange::Set { key, value });
    }
    
    /// Get all pending changes for a contract
    pub fn get_changes(&mut self, contract: &str) -> Vec<StateChange> {
        self.pending.remove(contract).unwrap_or_default()
    }
}
```

## User Experience Comparison

### Traditional Cosmos Experience
```bash
# Developer workflow
$ cargo build --release --target wasm32-unknown-unknown
$ cosmwasm-optimize
$ wasmd tx wasm store artifacts/contract.wasm --from wallet
$ wasmd tx wasm instantiate 1 '{"count":0}' --from wallet --label "my-contract"
$ wasmd tx wasm execute cosmos1... '{"increment":{}}' --from wallet
$ wasmd query wasm contract-state smart cosmos1... '{"get_count":{}}'
```

### Proposed NEAR-Cosmos Experience
```bash
# Nearly identical workflow
$ cargo build --release --target wasm32-unknown-unknown
$ cosmwasm-optimize
$ cosmwasm-near store artifacts/contract.wasm
$ cosmwasm-near instantiate 1 '{"count":0}' --label "my-contract"
$ cosmwasm-near execute proxima1... '{"increment":{}}'
$ cosmwasm-near query proxima1... '{"get_count":{}}'
```

### Key Differences
1. **Tool Name**: `cosmwasm-near` instead of `wasmd`
2. **Address Format**: `proxima1...` instead of `cosmos1...`
3. **Behind the Scenes**: Relayer consensus instead of native execution
4. **Performance**: ~2-3 second latency vs instant

## Security Considerations

### 1. Relayer Trust Model
- **Multi-node consensus**: Require 2/3 agreement
- **Reputation system**: Track relayer reliability
- **Slashing**: Penalize dishonest relayers
- **Rotation**: Regularly rotate relayer sets

### 2. State Integrity
- **Merkle proofs**: Verify state transitions
- **Checkpoint system**: Regular state snapshots
- **Rollback capability**: Revert invalid state changes

### 3. Resource Limits
- **Gas metering**: Track Wasmer execution costs
- **Memory limits**: Prevent excessive allocation
- **Time limits**: Timeout long-running executions

## Performance Optimization

### 1. Caching Strategy
```rust
pub struct ExecutionCache {
    /// Compiled WASM modules
    modules: LruCache<Hash, Module>,
    
    /// Contract instances
    instances: LruCache<String, Instance>,
    
    /// Query results (for deterministic queries)
    query_cache: TimedCache<QueryKey, Vec<u8>>,
}
```

### 2. Parallel Execution
- Execute independent contracts in parallel
- Batch state updates
- Pipeline consensus verification

### 3. State Prefetching
- Predict required state based on contract code
- Prefetch from NEAR before execution
- Reduce round-trip latency

## Migration Path

### Stage 1: Alpha (Months 1-2)
- Single relayer node with Wasmer
- Manual operation for testing
- Limited to testnet

### Stage 2: Beta (Months 3-4)
- 3-node consensus system
- Automated operation
- Public testnet deployment

### Stage 3: Production (Months 5-6)
- 5+ node consensus
- Full monitoring and alerting
- Mainnet deployment

## Monitoring and Operations

### Metrics to Track
```rust
pub struct RelayerMetrics {
    // Execution metrics
    pub executions_total: Counter,
    pub execution_duration: Histogram,
    pub execution_errors: Counter,
    
    // Consensus metrics
    pub consensus_rounds: Counter,
    pub consensus_failures: Counter,
    pub consensus_latency: Histogram,
    
    // State metrics
    pub state_sync_lag: Gauge,
    pub cache_hit_rate: Gauge,
    
    // Resource metrics
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
}
```

### Health Checks
```rust
pub async fn health_check(&self) -> HealthStatus {
    HealthStatus {
        wasmer_ready: self.execution_service.is_ready(),
        near_connected: self.near_client.is_connected().await,
        consensus_nodes: self.consensus_executor.active_nodes(),
        last_execution: self.last_execution_time,
        pending_executions: self.pending_queue.len(),
    }
}
```

## Cost Analysis

### Operational Costs
- **Relayer Infrastructure**: ~$500-1000/month for 3-5 nodes
- **NEAR Storage**: ~$0.01 per KB of contract state
- **NEAR Execution**: ~$0.0001 per transaction

### Comparison with Native Cosmos
- **Cosmos**: Direct execution, validator costs
- **NEAR-Cosmos**: Relayer costs + NEAR fees
- **Additional Cost**: ~20-30% overhead for consensus

## Conclusion

This architecture provides:

1. **True CosmWasm Execution**: Full WASM execution with Wasmer
2. **Authentic Developer Experience**: Nearly identical to Cosmos
3. **Trust through Consensus**: Multi-node verification
4. **Production Ready**: Scalable and secure design
5. **Clear Migration Path**: Staged rollout plan

The enhanced relayer acts as a **decentralized execution layer** that bridges the gap between CosmWasm's execution model and NEAR's storage capabilities, providing Cosmos developers with a familiar and powerful environment for deploying their contracts.

## Next Steps

1. **Prototype Development**: Build proof-of-concept with single node
2. **Consensus Implementation**: Add multi-node execution
3. **CLI Development**: Create developer tools
4. **Testing**: Comprehensive testing with real CosmWasm contracts
5. **Documentation**: Developer guides and API documentation
6. **Community Feedback**: Engage with Cosmos developers
7. **Production Deployment**: Launch on mainnet

This architecture leverages the existing IBC relayer infrastructure while adding the critical capability of executing arbitrary CosmWasm contracts, providing a complete solution for Cosmos developers on NEAR.