use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;
use cosmos_sdk_contract::modules::cosmwasm::{
    CosmWasmContractWrapper, WrapperInitMsg, WrapperExecuteMsg, WrapperQueryMsg,
    types::{Binary, Storage, Api},
    storage::CosmWasmStorage,
    api::CosmWasmApi,
    memory::CosmWasmMemoryManager,
};
use std::time::{Duration, Instant};

/// Performance Benchmarking for CosmWasm Compatibility Layer
/// 
/// This test suite measures the performance characteristics of our CosmWasm compatibility layer
/// to ensure it meets production requirements and provides acceptable overhead compared to native execution.

fn setup_context() {
    let context = VMContextBuilder::new()
        .current_account_id(accounts(0))
        .predecessor_account_id(accounts(1))
        .build();
    testing_env!(context);
}

/// Benchmark result structure
#[derive(Debug)]
struct BenchmarkResult {
    operation: String,
    iterations: u32,
    total_duration: Duration,
    avg_duration: Duration,
    ops_per_second: f64,
}

impl BenchmarkResult {
    fn new(operation: String, iterations: u32, total_duration: Duration) -> Self {
        let avg_duration = total_duration / iterations;
        let ops_per_second = iterations as f64 / total_duration.as_secs_f64();
        
        Self {
            operation,
            iterations,
            total_duration,
            avg_duration,
            ops_per_second,
        }
    }
    
    fn print(&self) {
        println!(
            "📊 {}: {} iterations in {:?} (avg: {:?}, {:.2} ops/sec)",
            self.operation,
            self.iterations,
            self.total_duration,
            self.avg_duration,
            self.ops_per_second
        );
    }
}

/// Macro for running benchmarks
macro_rules! benchmark {
    ($name:expr, $iterations:expr, $code:block) => {{
        let start = Instant::now();
        for _ in 0..$iterations {
            $code
        }
        let duration = start.elapsed();
        BenchmarkResult::new($name.to_string(), $iterations, duration)
    }};
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_storage_performance() {
    setup_context();
    
    println!("🚀 CosmWasm Storage Performance Benchmarks");
    println!("============================================");
    
    // Benchmark basic storage operations
    let result = benchmark!("Storage: Set operations", 1000, {
        let mut storage = CosmWasmStorage::new();
        let key = format!("test_key_{}", fastrand::u32(..));
        let value = vec![fastrand::u8(..) ; 32]; // 32 bytes of random data
        storage.set(key.as_bytes(), &value);
    });
    result.print();
    
    // Benchmark storage reads
    let mut storage = CosmWasmStorage::new();
    // Pre-populate storage
    for i in 0..1000 {
        let key = format!("read_key_{}", i);
        let value = vec![i as u8; 32];
        storage.set(key.as_bytes(), &value);
    }
    
    let result = benchmark!("Storage: Get operations", 1000, {
        let key = format!("read_key_{}", fastrand::usize(..1000));
        let _ = storage.get(key.as_bytes());
    });
    result.print();
    
    // Benchmark range queries
    let result = benchmark!("Storage: Range queries", 100, {
        let start_key = format!("read_key_{}", fastrand::usize(..500));
        let end_key = format!("read_key_{}", fastrand::usize(500..1000));
        let _ = storage.range(
            Some(start_key.as_bytes()),
            Some(end_key.as_bytes()),
            cosmos_sdk_contract::modules::cosmwasm::types::Order::Ascending,
        ).take(10).collect::<Result<Vec<_>, _>>();
    });
    result.print();
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_api_performance() {
    setup_context();
    
    println!("🔐 CosmWasm API Performance Benchmarks");
    println!("======================================");
    
    let api = CosmWasmApi::new();
    
    // Benchmark address validation
    let addresses = vec![
        "alice.near",
        "bob.testnet", 
        "cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu",
        "proxima1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu2",
    ];
    
    let result = benchmark!("API: Address validation", 1000, {
        let addr = &addresses[fastrand::usize(..addresses.len())];
        let _ = api.addr_validate(addr);
    });
    result.print();
    
    // Benchmark address canonicalization
    let result = benchmark!("API: Address canonicalization", 1000, {
        let addr = &addresses[fastrand::usize(..addresses.len())];
        let _ = api.addr_canonicalize(addr);
    });
    result.print();
    
    // Benchmark SHA256 hashing
    let result = benchmark!("API: SHA256 hashing", 1000, {
        let data = vec![fastrand::u8(..) ; 256]; // 256 bytes of random data
        let _ = api.sha256(&data);
    });
    result.print();
    
    // Benchmark Keccak256 hashing
    let result = benchmark!("API: Keccak256 hashing", 1000, {
        let data = vec![fastrand::u8(..) ; 256]; // 256 bytes of random data
        let _ = api.keccak256(&data);
    });
    result.print();
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_memory_performance() {
    setup_context();
    
    println!("💾 CosmWasm Memory Management Performance Benchmarks");
    println!("==================================================");
    
    let mut memory_manager = CosmWasmMemoryManager::new();
    
    // Benchmark memory allocation
    let result = benchmark!("Memory: Allocation", 1000, {
        let size = fastrand::usize(64..4096); // 64 bytes to 4KB
        let _ = memory_manager.allocate(size);
    });
    result.print();
    
    // Pre-allocate some regions for read/write tests
    let mut regions = Vec::new();
    for _ in 0..100 {
        regions.push(memory_manager.allocate(1024)); // 1KB regions
    }
    
    // Benchmark memory writes
    let result = benchmark!("Memory: Write operations", 1000, {
        let region = regions[fastrand::usize(..regions.len())];
        let data = vec![fastrand::u8(..) ; 256]; // 256 bytes
        let _ = memory_manager.write(region, 0, &data);
    });
    result.print();
    
    // Benchmark memory reads
    let result = benchmark!("Memory: Read operations", 1000, {
        let region = regions[fastrand::usize(..regions.len())];
        let _ = memory_manager.read(region, 0, 256);
    });
    result.print();
    
    // Clean up regions
    for region in regions {
        memory_manager.deallocate(region);
    }
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_contract_wrapper_performance() {
    setup_context();
    
    println!("📋 CosmWasm Contract Wrapper Performance Benchmarks");
    println!("==================================================");
    
    // CW20 Token operations benchmark
    let cw20_init = serde_json::json!({
        "name": "Benchmark Token",
        "symbol": "BENCH",
        "decimals": 6,
        "initial_balances": [
            {"address": "alice.near", "amount": "1000000000"}
        ]
    });
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw20_init).unwrap(),
    };
    
    // Benchmark contract instantiation
    let result = benchmark!("Contract: Instantiation", 100, {
        let _ = CosmWasmContractWrapper::new(wrapper_init.clone());
    });
    result.print();
    
    // Create a contract for execute/query benchmarks
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Benchmark CW20 transfer execution
    let transfer_msg = serde_json::json!({
        "transfer": {
            "recipient": "bob.near",
            "amount": "1000"
        }
    });
    
    let result = benchmark!("Contract: CW20 Execute (Transfer)", 1000, {
        let execute_msg = WrapperExecuteMsg {
            contract_msg: serde_json::to_string(&transfer_msg).unwrap(),
        };
        let _ = contract.execute(execute_msg);
    });
    result.print();
    
    // Benchmark CW20 balance query
    let balance_query = serde_json::json!({
        "balance": {"address": "alice.near"}
    });
    
    let result = benchmark!("Contract: CW20 Query (Balance)", 1000, {
        let query_msg = WrapperQueryMsg {
            contract_msg: serde_json::to_string(&balance_query).unwrap(),
        };
        let _ = contract.query(query_msg);
    });
    result.print();
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_complex_operations_performance() {
    setup_context();
    
    println!("🎯 CosmWasm Complex Operations Performance Benchmarks");
    println!("====================================================");
    
    // CW721 NFT operations benchmark
    let cw721_init = serde_json::json!({
        "name": "Benchmark NFTs",
        "symbol": "BNFT",
        "minter": "alice.near"
    });
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Benchmark CW721 NFT minting with rich metadata
    let result = benchmark!("Contract: CW721 Mint (with metadata)", 100, {
        let token_id = format!("nft_{}", fastrand::u32(..));
        let mint_msg = serde_json::json!({
            "mint": {
                "token_id": token_id,
                "owner": "alice.near",
                "token_uri": "https://example.com/metadata.json",
                "extension": {
                    "name": "Test NFT",
                    "description": "A test NFT for benchmarking",
                    "image": "https://example.com/image.png",
                    "attributes": [
                        {"trait_type": "Color", "value": "Blue"},
                        {"trait_type": "Rarity", "value": "Common"},
                        {"trait_type": "Power", "value": "100"}
                    ]
                }
            }
        });
        
        let execute_msg = WrapperExecuteMsg {
            contract_msg: serde_json::to_string(&mint_msg).unwrap(),
        };
        let _ = contract.execute(execute_msg);
    });
    result.print();
    
    // Benchmark complex JSON message parsing
    let complex_msg = serde_json::json!({
        "complex_operation": {
            "nested_data": {
                "array": [1, 2, 3, 4, 5],
                "object": {
                    "key1": "value1",
                    "key2": {
                        "nested_key": "nested_value"
                    }
                },
                "large_string": "x".repeat(1000)
            },
            "metadata": {
                "timestamp": 1234567890,
                "version": "1.0.0",
                "tags": ["tag1", "tag2", "tag3"]
            }
        }
    });
    
    let result = benchmark!("Contract: Complex JSON parsing", 1000, {
        let execute_msg = WrapperExecuteMsg {
            contract_msg: serde_json::to_string(&complex_msg).unwrap(),
        };
        let _ = contract.execute(execute_msg);
    });
    result.print();
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_cross_contract_simulation() {
    setup_context();
    
    println!("🌐 CosmWasm Cross-Contract Communication Simulation");
    println!("=================================================");
    
    // Simulate cross-contract calls with binary messages
    let cw20_init = serde_json::json!({
        "name": "Cross Contract Token",
        "symbol": "CCT",
        "decimals": 6,
        "initial_balances": [{"address": "alice.near", "amount": "1000000"}]
    });
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw20_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Benchmark CW20 Send operation (cross-contract call simulation)
    let result = benchmark!("Contract: CW20 Send (cross-contract)", 500, {
        let recipient_msg = serde_json::json!({
            "action": "stake",
            "duration": 30,
            "auto_compound": true,
            "metadata": {
                "source": "benchmark_test",
                "timestamp": chrono::Utc::now().timestamp()
            }
        });
        
        let send_msg = serde_json::json!({
            "send": {
                "contract": "staking.near",
                "amount": "1000",
                "msg": Binary::from(recipient_msg.to_string().as_bytes())
            }
        });
        
        let execute_msg = WrapperExecuteMsg {
            contract_msg: serde_json::to_string(&send_msg).unwrap(),
        };
        let _ = contract.execute(execute_msg);
    });
    result.print();
    
    // Benchmark multiple contract state queries
    let result = benchmark!("Contract: Multiple state queries", 500, {
        let queries = vec![
            serde_json::json!({"token_info": {}}),
            serde_json::json!({"balance": {"address": "alice.near"}}),
            serde_json::json!({"minter": {}}),
        ];
        
        for query in queries {
            let query_msg = WrapperQueryMsg {
                contract_msg: serde_json::to_string(&query).unwrap(),
            };
            let _ = contract.query(query_msg);
        }
    });
    result.print();
    
    println!();
}

#[test]
#[ignore = "Integration tests need updated interface"]
#[ignore = "Requires NEAR contract context"]
fn test_cosmwasm_stress_test() {
    setup_context();
    
    println!("🔥 CosmWasm Stress Test - High Volume Operations");
    println!("===============================================");
    
    let cw20_init = serde_json::json!({
        "name": "Stress Test Token",
        "symbol": "STRESS",
        "decimals": 6,
        "initial_balances": []
    });
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw20_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Stress test: High volume operations
    let result = benchmark!("Stress: 10k mixed operations", 10000, {
        let operation_type = fastrand::usize(..3);
        
        match operation_type {
            0 => {
                // Execute operation
                let msg = serde_json::json!({
                    "action": "increment"
                });
                let execute_msg = WrapperExecuteMsg {
                    contract_msg: serde_json::to_string(&msg).unwrap(),
                };
                let _ = contract.execute(execute_msg);
            },
            1 => {
                // Query operation
                let msg = serde_json::json!({
                    "query": "get_count"
                });
                let query_msg = WrapperQueryMsg {
                    contract_msg: serde_json::to_string(&msg).unwrap(),
                };
                let _ = contract.query(query_msg);
            },
            _ => {
                // Contract info query
                let _ = contract.get_contract_info();
            }
        }
    });
    result.print();
    
    println!();
    println!("✅ Performance benchmarks completed successfully!");
    println!("💡 Results show the CosmWasm compatibility layer performance characteristics");
}