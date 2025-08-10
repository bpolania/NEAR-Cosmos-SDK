// Performance benchmarks for modular IBC relayer
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use ibc_relayer::{
    chains::{
        Chain, IbcModuleType, ModuleRegistry, ModuleInfo,
        NearModularChain, CrossModuleOp, IbcPacket,
    },
    config::{ChainConfig, ChainSpecificConfig},
};

// Extension trait to expose call_module for tests
#[async_trait::async_trait]
trait NearModularChainExt {
    async fn call_module(&self, module_type: IbcModuleType, method_name: &str, args: serde_json::Value) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait::async_trait]
impl NearModularChainExt for NearModularChain {
    async fn call_module(&self, module_type: IbcModuleType, method_name: &str, args: serde_json::Value) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.call_module(module_type, method_name, args).await
    }
}

/// Benchmark parallel queries vs sequential queries
#[tokio::test]
async fn benchmark_parallel_vs_sequential_queries() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Define test queries
            let test_queries = vec![
                (IbcModuleType::Client, "get_client_state", serde_json::json!({"client_id": "07-tendermint-0"})),
                (IbcModuleType::Connection, "get_connection", serde_json::json!({"connection_id": "connection-0"})),
                (IbcModuleType::Channel, "get_channel", serde_json::json!({"port_id": "transfer", "channel_id": "channel-0"})),
                (IbcModuleType::Transfer, "get_escrow_address", serde_json::json!({"port_id": "transfer", "channel_id": "channel-0"})),
            ];
            
            // Benchmark sequential queries
            let start_seq = Instant::now();
            let mut seq_results = vec![];
            for (module_type, method, args) in test_queries.clone() {
                let chain_ref = &chain;
                let result: Result<serde_json::Value, _> = async move {
                    let data = chain_ref.call_module(module_type, method, args).await?;
                    serde_json::from_slice(&data).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }.await;
                seq_results.push(result);
            }
            let seq_duration = start_seq.elapsed();
            
            // Benchmark parallel queries
            let start_par = Instant::now();
            let par_results: Vec<_> = chain.query_modules_parallel::<serde_json::Value>(test_queries).await;
            let par_duration = start_par.elapsed();
            
            println!("🏁 Performance Benchmark Results:");
            println!("   Sequential queries: {:?}", seq_duration);
            println!("   Parallel queries: {:?}", par_duration);
            println!("   Speedup: {:.2}x", seq_duration.as_millis() as f64 / par_duration.as_millis().max(1) as f64);
            
            // Verify same number of results
            assert_eq!(seq_results.len(), par_results.len());
        }
        Err(_) => {
            println!("⚠️  Skipping benchmark - cannot connect to testnet");
        }
    }
}

/// Benchmark module discovery with different cache sizes
#[tokio::test]
async fn benchmark_module_discovery_caching() {
    let config = create_test_config();
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect(&config.rpc_endpoint);
    let router = "cosmos-router.testnet".parse().unwrap();
    
    // First discovery (cold cache)
    let start_cold = Instant::now();
    let result1 = ModuleRegistry::discover_modules(&router, &rpc_client).await;
    let cold_duration = start_cold.elapsed();
    
    if let Ok(registry) = result1 {
        // Test different cache durations
        let cache_tests = vec![
            ("No cache", Duration::from_secs(0)),
            ("1 second cache", Duration::from_secs(1)),
            ("5 minute cache", Duration::from_secs(300)),
        ];
        
        for (name, cache_duration) in cache_tests {
            let cached_at = Instant::now();
            
            // Wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let start = Instant::now();
            let _result = ModuleRegistry::discover_with_cache(
                &router,
                &rpc_client,
                cache_duration,
                Some((registry.clone(), cached_at)),
            ).await;
            let duration = start.elapsed();
            
            println!("🏁 Discovery with {}: {:?}", name, duration);
        }
        
        println!("   Initial discovery (cold): {:?}", cold_duration);
    }
}

/// Benchmark cross-module operation processing
#[tokio::test]
async fn benchmark_cross_module_operations() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Create different types of operations
            let operations = vec![
                ("SendPacket", CrossModuleOp::SendPacket {
                    packet: create_test_packet(1),
                }),
                ("ChannelHandshake", CrossModuleOp::ChannelHandshake {
                    operation: "ChanOpenInit".to_string(),
                    port_id: "transfer".to_string(),
                    channel_id: "channel-0".to_string(),
                    counterparty_port_id: "transfer".to_string(),
                    counterparty_channel_id: "channel-1".to_string(),
                    connection_hops: vec!["connection-0".to_string()],
                    version: "ics20-1".to_string(),
                }),
                ("UpdateClient", CrossModuleOp::UpdateClient {
                    client_id: "07-tendermint-0".to_string(),
                    header: vec![1, 2, 3, 4, 5],
                }),
            ];
            
            let mut results = vec![];
            
            for (name, operation) in operations {
                let start = Instant::now();
                let result = chain.execute_cross_module_op(operation).await;
                let duration = start.elapsed();
                
                results.push((name, duration, result.is_ok()));
                println!("🏁 {} operation: {:?}", name, duration);
            }
            
            // All operations should complete (success or expected failure)
            assert_eq!(results.len(), 3);
        }
        Err(_) => {
            println!("⚠️  Skipping benchmark - cannot connect to testnet");
        }
    }
}

/// Benchmark event processing throughput
#[tokio::test]
async fn benchmark_event_processing_throughput() {
    let (tx, mut rx) = mpsc::channel(10000);
    
    // Generate test events
    let event_counts = vec![100, 1000, 5000];
    
    for count in event_counts {
        let events = generate_test_events(count);
        let tx_clone = tx.clone();
        
        let start = Instant::now();
        
        // Process events concurrently
        let handles: Vec<_> = events.into_iter().map(|event| {
            let tx = tx_clone.clone();
            tokio::spawn(async move {
                // Simulate event processing
                if let Ok(parsed) = parse_test_event(&event) {
                    tx.send(parsed).await.ok();
                }
            })
        }).collect();
        
        // Wait for all processing to complete
        futures::future::join_all(handles).await;
        
        let duration = start.elapsed();
        let throughput = count as f64 / duration.as_secs_f64();
        
        println!("🏁 Processed {} events in {:?} ({:.0} events/sec)", 
                 count, duration, throughput);
        
        // Drain channel
        while tokio::time::timeout(Duration::from_millis(10), rx.recv()).await.is_ok() {}
    }
}

/// Benchmark module registry operations
#[tokio::test]
async fn benchmark_module_registry_operations() {
    let mut modules = HashMap::new();
    let module_counts = vec![5, 10, 20, 50];
    
    for count in module_counts {
        // Create registry with N modules
        modules.clear();
        for i in 0..count {
            let module_type = match i % 4 {
                0 => "ibc_client",
                1 => "ibc_connection",
                2 => "ibc_channel",
                _ => "ibc_transfer",
            };
            modules.insert(
                format!("{}{}", module_type, i),
                format!("module{}.testnet", i),
            );
        }
        
        let router = "router.testnet".parse().unwrap();
        
        // Benchmark registry creation
        let start = Instant::now();
        let registry = ModuleRegistry::from_config(router, &modules).unwrap();
        let create_duration = start.elapsed();
        
        // Benchmark lookups
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = registry.modules.get(&IbcModuleType::Channel);
            let _ = registry.modules.get(&IbcModuleType::Client);
            let _ = registry.modules.get(&IbcModuleType::Transfer);
        }
        let lookup_duration = start.elapsed();
        
        println!("🏁 Registry with {} modules:", count);
        println!("   Creation time: {:?}", create_duration);
        println!("   1000 lookups: {:?}", lookup_duration);
    }
}

/// Benchmark serialization/deserialization of cross-module operations
#[tokio::test]
async fn benchmark_cross_module_op_serialization() {
    let operations = vec![
        ("Small packet", CrossModuleOp::SendPacket {
            packet: create_test_packet(1),
        }),
        ("Large packet", CrossModuleOp::SendPacket {
            packet: IbcPacket {
                sequence: 1,
                source_port: "transfer".to_string(),
                source_channel: "channel-0".to_string(),
                destination_port: "transfer".to_string(),
                destination_channel: "channel-1".to_string(),
                data: vec![0u8; 10000], // 10KB data
                timeout_height: Some(1000),
                timeout_timestamp: Some(1234567890),
            },
        }),
    ];
    
    for (name, operation) in operations {
        // Benchmark serialization
        let start_ser = Instant::now();
        let mut serialized = vec![];
        for _ in 0..1000 {
            serialized = serde_json::to_vec(&operation).unwrap();
        }
        let ser_duration = start_ser.elapsed();
        
        // Benchmark deserialization
        let start_de = Instant::now();
        for _ in 0..1000 {
            let _: CrossModuleOp = serde_json::from_slice(&serialized).unwrap();
        }
        let de_duration = start_de.elapsed();
        
        println!("🏁 {} serialization benchmark:", name);
        println!("   Serialization (1000x): {:?}", ser_duration);
        println!("   Deserialization (1000x): {:?}", de_duration);
        println!("   Serialized size: {} bytes", serialized.len());
    }
}

/// Benchmark health check performance with varying module counts
#[tokio::test]
async fn benchmark_health_check_scaling() {
    // Test with different numbers of modules
    let module_counts = vec![1, 5, 10, 20];
    
    for count in module_counts {
        let config = create_config_with_modules(count);
        
        match NearModularChain::new(&config).await {
            Ok(chain) => {
                let start = Instant::now();
                let result = chain.health_check().await;
                let duration = start.elapsed();
                
                println!("🏁 Health check with {} modules: {:?} ({})",
                         count, duration, 
                         if result.is_ok() { "success" } else { "failed" });
            }
            Err(_) => {
                println!("⚠️  Skipping {} module test - cannot create chain", count);
            }
        }
    }
}

// Helper functions

fn create_test_config() -> ChainConfig {
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-router.testnet".to_string(),
            modules: Some(HashMap::from([
                ("ibc_client".to_string(), "client.testnet".to_string()),
                ("ibc_connection".to_string(), "connection.testnet".to_string()),
                ("ibc_channel".to_string(), "channel.testnet".to_string()),
                ("ibc_transfer".to_string(), "transfer.testnet".to_string()),
            ])),
            signer_account_id: "relayer.testnet".to_string(),
            private_key: None,
            network_id: "testnet".to_string(),
            modular: true,
        },
    }
}

fn create_config_with_modules(count: usize) -> ChainConfig {
    let mut modules = HashMap::new();
    
    // Always include base modules
    modules.insert("ibc_client".to_string(), "client.testnet".to_string());
    modules.insert("ibc_connection".to_string(), "connection.testnet".to_string());
    modules.insert("ibc_channel".to_string(), "channel.testnet".to_string());
    modules.insert("ibc_transfer".to_string(), "transfer.testnet".to_string());
    
    // Add additional modules
    for i in 4..count {
        modules.insert(
            format!("custom_module_{}", i),
            format!("module{}.testnet", i),
        );
    }
    
    ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-router.testnet".to_string(),
            modules: Some(modules),
            signer_account_id: "relayer.testnet".to_string(),
            private_key: None,
            network_id: "testnet".to_string(),
            modular: true,
        },
    }
}

fn create_test_packet(sequence: u64) -> IbcPacket {
    IbcPacket {
        sequence,
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: b"test_packet_data".to_vec(),
        timeout_height: Some(1000),
        timeout_timestamp: Some(1234567890),
    }
}

fn generate_test_events(count: usize) -> Vec<ibc_relayer::chains::ChainEvent> {
    (0..count).map(|i| {
        ibc_relayer::chains::ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), i.to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
            ],
            height: 100 + i as u64,
            tx_hash: Some(format!("0x{:x}", i)),
        }
    }).collect()
}

fn parse_test_event(event: &ibc_relayer::chains::ChainEvent) -> Result<String, ()> {
    // Simulate event parsing
    Ok(format!("{}-{}", event.event_type, event.height))
}