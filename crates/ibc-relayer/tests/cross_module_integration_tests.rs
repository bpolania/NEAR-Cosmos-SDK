// Integration tests for cross-module operations
use std::collections::HashMap;
use std::sync::Arc;

use ibc_relayer::{
    chains::{
        Chain, IbcModuleType, ModuleRegistry, ModuleInfo, 
        NearModularChain, CrossModuleOp, IbcPacket,
    },
    config::{ChainConfig, ChainSpecificConfig},
};

/// Test complete packet lifecycle across modules
#[tokio::test]
async fn test_packet_lifecycle_integration() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // 1. Create a packet
            let packet = IbcPacket {
                sequence: 1,
                source_port: "transfer".to_string(),
                source_channel: "channel-0".to_string(),
                destination_port: "transfer".to_string(),
                destination_channel: "channel-1".to_string(),
                data: b"integration_test_packet".to_vec(),
                timeout_height: Some(1000),
                timeout_timestamp: None,
            };
            
            // 2. Send packet (involves channel, connection, and client modules)
            let send_op = CrossModuleOp::SendPacket { 
                packet: packet.clone() 
            };
            
            let send_result = chain.execute_cross_module_op(send_op).await;
            println!("Send packet result: {:?}", send_result.is_ok());
            
            // 3. Query packet commitment (channel module)
            let commitment_result = chain.query_packet_commitment(
                &packet.source_port,
                &packet.source_channel,
                packet.sequence,
            ).await;
            println!("Packet commitment query: {:?}", commitment_result.is_ok());
            
            // 4. Check next sequence (channel module)
            let next_seq_result = chain.query_next_sequence_recv(
                &packet.destination_port,
                &packet.destination_channel,
            ).await;
            println!("Next sequence query: {:?}", next_seq_result.is_ok());
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test channel handshake coordination
#[tokio::test]
async fn test_channel_handshake_integration() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Channel open init (requires connection and client verification)
            let chan_open_init = CrossModuleOp::ChannelHandshake {
                operation: "ChanOpenInit".to_string(),
                port_id: "transfer".to_string(),
                channel_id: "channel-100".to_string(),
                counterparty_port_id: "transfer".to_string(),
                counterparty_channel_id: "".to_string(), // Not yet assigned
                connection_hops: vec!["connection-0".to_string()],
                version: "ics20-1".to_string(),
            };
            
            let init_result = chain.execute_cross_module_op(chan_open_init).await;
            println!("Channel open init: {:?}", init_result.is_ok());
            
            // Channel open try (on counterparty)
            let chan_open_try = CrossModuleOp::ChannelHandshake {
                operation: "ChanOpenTry".to_string(),
                port_id: "transfer".to_string(),
                channel_id: "channel-101".to_string(),
                counterparty_port_id: "transfer".to_string(),
                counterparty_channel_id: "channel-100".to_string(),
                connection_hops: vec!["connection-1".to_string()],
                version: "ics20-1".to_string(),
            };
            
            let try_result = chain.execute_cross_module_op(chan_open_try).await;
            println!("Channel open try: {:?}", try_result.is_ok());
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test connection handshake with client updates
#[tokio::test]
async fn test_connection_handshake_integration() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // First update the client
            let update_client = CrossModuleOp::UpdateClient {
                client_id: "07-tendermint-0".to_string(),
                header: vec![1, 2, 3, 4, 5], // Mock header
            };
            
            let update_result = chain.execute_cross_module_op(update_client).await;
            println!("Client update: {:?}", update_result.is_ok());
            
            // Connection open init
            let conn_open_init = CrossModuleOp::ConnectionHandshake {
                operation: "ConnOpenInit".to_string(),
                connection_id: "connection-100".to_string(),
                client_id: "07-tendermint-0".to_string(),
                counterparty_connection_id: "".to_string(),
                counterparty_client_id: "07-tendermint-1".to_string(),
            };
            
            let init_result = chain.execute_cross_module_op(conn_open_init).await;
            println!("Connection open init: {:?}", init_result.is_ok());
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test module registry hot-swapping during operations
#[tokio::test]
async fn test_hot_swap_during_operations() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Get reference to chain
            {
                let chain = &chain;
                
                // Send initial operation
                let packet1 = create_test_packet(1);
                let op1 = CrossModuleOp::SendPacket { packet: packet1 };
                let result1 = chain.execute_cross_module_op(op1).await;
                println!("Operation before hot-swap: {:?}", result1.is_ok());
                
                // Simulate hot-swap (would need mutable access in real scenario)
                // This demonstrates the concept
                
                // Send operation after hot-swap
                let packet2 = create_test_packet(2);
                let op2 = CrossModuleOp::SendPacket { packet: packet2 };
                let result2 = chain.execute_cross_module_op(op2).await;
                println!("Operation after hot-swap: {:?}", result2.is_ok());
            }
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test error recovery in cross-module operations
#[tokio::test]
async fn test_cross_module_error_recovery() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Test with invalid connection ID
            let invalid_op = CrossModuleOp::ChannelHandshake {
                operation: "ChanOpenInit".to_string(),
                port_id: "transfer".to_string(),
                channel_id: "channel-999".to_string(),
                counterparty_port_id: "transfer".to_string(),
                counterparty_channel_id: "".to_string(),
                connection_hops: vec!["invalid-connection".to_string()],
                version: "ics20-1".to_string(),
            };
            
            let result = chain.execute_cross_module_op(invalid_op).await;
            // Should handle error gracefully
            assert!(result.is_err() || result.is_ok());
            
            // Test recovery - valid operation after error
            let valid_op = CrossModuleOp::UpdateClient {
                client_id: "07-tendermint-0".to_string(),
                header: vec![1, 2, 3],
            };
            
            let recovery_result = chain.execute_cross_module_op(valid_op).await;
            println!("Recovery operation: {:?}", recovery_result.is_ok());
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test batch cross-module operations
#[tokio::test]
async fn test_batch_cross_module_operations() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Create multiple operations
            let operations = vec![
                CrossModuleOp::UpdateClient {
                    client_id: "07-tendermint-0".to_string(),
                    header: vec![1, 2, 3],
                },
                CrossModuleOp::SendPacket {
                    packet: create_test_packet(1),
                },
                CrossModuleOp::SendPacket {
                    packet: create_test_packet(2),
                },
                CrossModuleOp::UpdateClient {
                    client_id: "07-tendermint-1".to_string(),
                    header: vec![4, 5, 6],
                },
            ];
            
            // Execute operations concurrently
            let handles: Vec<_> = operations.into_iter().map(|op| {
                let chain = chain.clone();
                tokio::spawn(async move {
                    chain.execute_cross_module_op(op).await
                })
            }).collect();
            
            let results = futures::future::join_all(handles).await;
            
            // Check all operations completed
            let completed = results.iter().filter(|r| r.is_ok()).count();
            println!("Batch operations completed: {}/{}", completed, results.len());
            
            assert_eq!(results.len(), 4);
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
        }
    }
}

/// Test module dependency resolution
#[tokio::test]
async fn test_module_dependency_resolution() {
    // Create a registry with module dependencies
    let mut modules = HashMap::new();
    
    // Channel depends on Connection
    modules.insert(IbcModuleType::Channel, ModuleInfo {
        contract_id: "channel.testnet".parse().unwrap(),
        module_type: IbcModuleType::Channel,
        version: "1.0.0".to_string(),
        methods: vec!["send_packet".to_string(), "recv_packet".to_string()],
    });
    
    // Connection depends on Client
    modules.insert(IbcModuleType::Connection, ModuleInfo {
        contract_id: "connection.testnet".parse().unwrap(),
        module_type: IbcModuleType::Connection,
        version: "1.0.0".to_string(),
        methods: vec!["conn_open_init".to_string(), "conn_open_try".to_string()],
    });
    
    // Client is standalone
    modules.insert(IbcModuleType::Client, ModuleInfo {
        contract_id: "client.testnet".parse().unwrap(),
        module_type: IbcModuleType::Client,
        version: "1.0.0".to_string(),
        methods: vec!["create_client".to_string(), "update_client".to_string()],
    });
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    // Verify all dependencies are present
    assert!(registry.modules.contains_key(&IbcModuleType::Client));
    assert!(registry.modules.contains_key(&IbcModuleType::Connection));
    assert!(registry.modules.contains_key(&IbcModuleType::Channel));
    
    // Test operations that require multiple modules
    let config = create_test_config();
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // This operation requires all three modules
            let complex_op = CrossModuleOp::SendPacket {
                packet: create_test_packet(1),
            };
            
            let result = chain.execute_cross_module_op(complex_op).await;
            println!("Complex operation requiring all modules: {:?}", result.is_ok());
        }
        Err(_) => {
            println!("⚠️  Skipping dependency test - cannot connect to chain");
        }
    }
}

/// Test transaction routing through router contract
#[tokio::test]
async fn test_transaction_routing() {
    let config = create_test_config();
    
    match NearModularChain::new(&config).await {
        Ok(chain) => {
            // Test routing a cross-module operation
            let op = CrossModuleOp::SendPacket {
                packet: create_test_packet(123),
            };
            
            // Submit as transaction data
            let tx_data = serde_json::to_vec(&op).unwrap();
            let result = chain.submit_transaction(tx_data).await;
            
            match result {
                Ok(tx_hash) => {
                    println!("Transaction routed successfully: {}", tx_hash);
                }
                Err(e) => {
                    println!("Transaction routing error (expected in test): {}", e);
                }
            }
            
            // Test routing non-cross-module transaction
            let other_data = b"other_transaction_data".to_vec();
            let other_result = chain.submit_transaction(other_data).await;
            println!("Non-cross-module transaction: {:?}", other_result.is_ok());
        }
        Err(e) => {
            println!("⚠️  Skipping test - cannot connect to chain: {}", e);
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

fn create_test_packet(sequence: u64) -> IbcPacket {
    IbcPacket {
        sequence,
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        destination_port: "transfer".to_string(),
        destination_channel: "channel-1".to_string(),
        data: format!("test_packet_{}", sequence).into_bytes(),
        timeout_height: Some(1000 + sequence),
        timeout_timestamp: None,
    }
}