# IBC Relayer Redesign for Modular Architecture

## Overview
The IBC relayer must be redesigned to work with the new modular contract architecture where IBC functionality is split across multiple contracts instead of a single monolithic contract.

## Current vs New Architecture

### Current (Monolithic)
```
NEAR Blockchain
└── cosmos-sdk.near (Single Contract)
    ├── IBC Client Module
    ├── IBC Connection Module  
    ├── IBC Channel Module
    └── IBC Transfer Module

Relayer → Single contract call to cosmos-sdk.near
```

### New (Modular)
```
NEAR Blockchain
├── cosmos-router.near (Main Router)
├── cosmos-ibc-client.near
├── cosmos-ibc-connection.near
├── cosmos-ibc-channel.near
└── cosmos-ibc-transfer.near

Relayer → Router or direct module calls
```

## Key Changes Required

### 1. Module Discovery System

The relayer needs to discover which contracts handle which IBC functions:

```rust
// New module registry in relayer
pub struct ModuleRegistry {
    router_contract: AccountId,
    modules: HashMap<String, ModuleInfo>,
}

#[derive(Clone, Debug)]
pub struct ModuleInfo {
    pub contract_id: AccountId,
    pub module_type: IbcModuleType,
    pub methods: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum IbcModuleType {
    Client,
    Connection,
    Channel,
    Transfer,
}

impl ModuleRegistry {
    /// Query the router contract to discover all IBC modules
    pub async fn discover_modules(
        router_contract: &AccountId,
        rpc_client: &JsonRpcClient,
    ) -> Result<Self, Box<dyn Error>> {
        // Query router for module addresses
        let modules = query_router_modules(router_contract, rpc_client).await?;
        
        let mut registry = Self {
            router_contract: router_contract.clone(),
            modules: HashMap::new(),
        };
        
        // Identify each module's type and capabilities
        for (name, address) in modules {
            let module_info = query_module_metadata(&address, rpc_client).await?;
            registry.modules.insert(name, module_info);
        }
        
        Ok(registry)
    }
}
```

### 2. Updated Chain Interface

The NEAR chain implementation needs to handle multi-contract calls:

```rust
// Updated NearChain implementation
pub struct NearChain {
    chain_id: String,
    module_registry: ModuleRegistry,
    rpc_client: JsonRpcClient,
    network_id: String,
}

impl NearChain {
    /// Route IBC method calls to appropriate module contracts
    async fn call_ibc_method(
        &self,
        module_type: IbcModuleType,
        method: &str,
        args: serde_json::Value,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // Find the appropriate module
        let module = self.module_registry
            .modules
            .values()
            .find(|m| m.module_type == module_type)
            .ok_or("Module not found")?;
        
        // Call the specific module contract
        self.call_contract_view(&module.contract_id, method, args).await
    }
    
    /// Handle cross-contract calls for operations spanning multiple modules
    async fn call_cross_module_operation(
        &self,
        operation: CrossModuleOp,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // Route through the main router contract
        let args = json!({
            "operation": operation,
        });
        
        self.call_contract_view(
            &self.module_registry.router_contract,
            "execute_cross_module_op",
            args
        ).await
    }
}
```

### 3. IBC Operation Routing

Different IBC operations need to be routed to different contracts:

```rust
#[async_trait]
impl Chain for NearChain {
    /// Query packet commitment - routes to IBC Channel module
    async fn query_packet_commitment(
        &self,
        port_id: &str,
        channel_id: &str,
        sequence: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn Error + Send + Sync>> {
        let args = json!({
            "port_id": port_id,
            "channel_id": channel_id,
            "sequence": sequence,
        });
        
        let result = self.call_ibc_method(
            IbcModuleType::Channel,
            "query_packet_commitment",
            args
        ).await?;
        
        // Parse result
        Ok(serde_json::from_slice(&result)?)
    }
    
    /// Update client - routes to IBC Client module
    async fn update_client(
        &self,
        client_id: &str,
        header: Vec<u8>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let args = json!({
            "client_id": client_id,
            "header": base64::encode(&header),
        });
        
        self.call_ibc_method(
            IbcModuleType::Client,
            "update_client",
            args
        ).await?;
        
        Ok(())
    }
    
    /// Send packet - may require cross-module coordination
    async fn send_packet(
        &self,
        packet: IbcPacket,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // This operation touches multiple modules:
        // 1. Channel module for packet commitment
        // 2. Connection module for connection state
        // 3. Client module for consensus state
        
        let operation = CrossModuleOp::SendPacket {
            packet: packet.clone(),
        };
        
        self.call_cross_module_operation(operation).await?;
        Ok(())
    }
}
```

### 4. Event Monitoring Updates

Events will now come from multiple contracts:

```rust
pub struct ModularEventMonitor {
    modules: Vec<AccountId>,
    event_streams: Vec<Box<dyn Stream<Item = ChainEvent>>>,
}

impl ModularEventMonitor {
    pub async fn new(
        module_registry: &ModuleRegistry,
        rpc_client: &JsonRpcClient,
    ) -> Result<Self, Box<dyn Error>> {
        let mut event_streams = Vec::new();
        
        // Subscribe to events from each IBC module
        for module in module_registry.modules.values() {
            let stream = create_event_stream(
                &module.contract_id,
                rpc_client.clone()
            ).await?;
            event_streams.push(Box::new(stream));
        }
        
        Ok(Self {
            modules: module_registry.modules.values()
                .map(|m| m.contract_id.clone())
                .collect(),
            event_streams,
        })
    }
    
    /// Merge events from all module contracts
    pub fn merged_event_stream(&self) -> impl Stream<Item = ChainEvent> {
        futures::stream::select_all(self.event_streams.iter())
    }
}
```

### 5. Configuration Updates

The relayer config needs to support modular architecture:

```toml
# config.toml
[[chains]]
chain_id = "near-testnet"
rpc_endpoint = "https://rpc.testnet.near.org"

[chains.config.near]
# Router contract that knows about all modules
router_contract = "cosmos-router.testnet"

# Optional: Direct module addresses for optimization
[chains.config.near.modules]
ibc_client = "cosmos-ibc-client.testnet"
ibc_connection = "cosmos-ibc-connection.testnet"
ibc_channel = "cosmos-ibc-channel.testnet"
ibc_transfer = "cosmos-ibc-transfer.testnet"

# Module discovery settings
[chains.config.near.discovery]
auto_discover = true
cache_duration_secs = 3600
```

### 6. Relayer Initialization

```rust
// Updated relayer initialization
pub async fn initialize_near_chain(
    config: &ChainConfig,
) -> Result<Box<dyn Chain>, Box<dyn Error>> {
    let near_config = match &config.config {
        ChainSpecificConfig::Near { router_contract, modules, .. } => {
            (router_contract, modules)
        }
        _ => return Err("Invalid config".into()),
    };
    
    let rpc_client = JsonRpcClient::connect(&config.rpc_endpoint);
    
    // Discover or load module registry
    let module_registry = if let Some(modules) = near_config.1 {
        // Use configured modules
        ModuleRegistry::from_config(near_config.0, modules)
    } else {
        // Auto-discover modules from router
        ModuleRegistry::discover_modules(
            &near_config.0.parse()?,
            &rpc_client
        ).await?
    };
    
    Ok(Box::new(NearChain {
        chain_id: config.chain_id.clone(),
        module_registry,
        rpc_client,
        network_id: config.network_id.clone(),
    }))
}
```

## Migration Path

### Phase 1: Dual Support
- Support both monolithic and modular contracts
- Detect contract type during initialization
- Use appropriate calling convention

### Phase 2: Modular Optimization
- Implement parallel module queries
- Add module-specific caching
- Optimize cross-module operations

### Phase 3: Full Migration
- Remove monolithic support
- Add advanced features like module hot-swapping
- Implement module health monitoring

## Benefits

1. **Resilience**: If one module fails, others continue working
2. **Performance**: Parallel queries to different modules
3. **Flexibility**: Easy to add/update individual modules
4. **Debugging**: Clearer separation of concerns

## Testing Strategy

1. **Unit Tests**: Test each module interface separately
2. **Integration Tests**: Test cross-module operations
3. **E2E Tests**: Full packet lifecycle across modular contracts
4. **Migration Tests**: Ensure smooth transition from monolithic

## Next Steps

1. Implement `ModuleRegistry` system
2. Update `NearChain` for multi-contract calls
3. Create module discovery mechanism
4. Update event monitoring
5. Write comprehensive tests