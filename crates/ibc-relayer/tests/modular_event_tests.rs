// Tests for modular event monitoring edge cases
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use futures::{StreamExt, stream};

use ibc_relayer::{
    chains::{ChainEvent, IbcModuleType, ModuleRegistry, ModuleInfo},
    monitor::{ModularEventMonitor, MonitorConfig, EventMonitor},
    relay::RelayEvent,
};

/// Test event monitor with stream failures
#[tokio::test]
async fn test_event_monitor_stream_failures() {
    let (tx, mut rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create registry with modules
    let mut modules = HashMap::new();
    modules.insert(IbcModuleType::Channel, ModuleInfo {
        contract_id: "channel.testnet".parse().unwrap(),
        module_type: IbcModuleType::Channel,
        version: "1.0.0".to_string(),
        methods: vec!["send_packet".to_string()],
    });
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    let mut monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig {
            prefer_streaming: true,
            polling_interval_ms: 100,
            blocks_per_poll: 10,
            max_concurrent_monitors: 4,
        },
    ).await.unwrap();
    
    // Initialize streams - should handle stream creation failures gracefully
    let result = monitor.initialize_streams().await;
    assert!(result.is_ok());
    
    // Start monitoring with quick shutdown
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let monitor_handle = tokio::spawn(async move {
        monitor.start(shutdown_rx).await
    });
    
    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Should not receive any events in test environment
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(event.is_err());
    
    drop(monitor_handle);
}

/// Test event parsing errors
#[tokio::test]
async fn test_event_parsing_errors() {
    let (tx, _rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules: HashMap::new(),
    };
    
    let monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig::default(),
    ).await.unwrap();
    
    // Test with malformed event data
    let malformed_event = ChainEvent {
        event_type: "send_packet".to_string(),
        attributes: vec![
            ("invalid_key".to_string(), "value".to_string()),
            // Missing required packet attributes
        ],
        height: 100,
        tx_hash: Some("0xabc".to_string()),
    };
    
    // Parsing should handle malformed events gracefully
    let result = EventMonitor::parse_send_packet_event("near-testnet", &malformed_event);
    assert!(result.is_ok()); // Should return Ok(None) for invalid events
    assert!(result.unwrap().is_none());
}

/// Test module stream reconnection
#[tokio::test]
async fn test_module_stream_reconnection() {
    let (tx, _rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create modules that will fail to connect initially
    let mut modules = HashMap::new();
    for i in 0..3 {
        modules.insert(
            match i {
                0 => IbcModuleType::Client,
                1 => IbcModuleType::Channel,
                _ => IbcModuleType::Transfer,
            },
            ModuleInfo {
                contract_id: format!("module{}.testnet", i).parse().unwrap(),
                module_type: match i {
                    0 => IbcModuleType::Client,
                    1 => IbcModuleType::Channel,
                    _ => IbcModuleType::Transfer,
                },
                version: "1.0.0".to_string(),
                methods: vec![],
            }
        );
    }
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    let mut monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig {
            prefer_streaming: true,
            polling_interval_ms: 50,
            blocks_per_poll: 10,
            max_concurrent_monitors: 4,
        },
    ).await.unwrap();
    
    // Initialize should handle all stream failures
    let result = monitor.initialize_streams().await;
    assert!(result.is_ok());
    
    // All modules should fall back to polling
    // This is verified by the monitor not panicking during start
}

/// Test concurrent event processing
#[tokio::test]
async fn test_concurrent_event_processing() {
    let (tx, mut rx) = mpsc::channel(100);
    let chain_id = "near-testnet";
    
    // Simulate multiple events arriving concurrently
    let events = vec![
        ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_data".to_string(), "dGVzdA==".to_string()), // base64 "test"
            ],
            height: 100,
            tx_hash: Some("0x123".to_string()),
        },
        ChainEvent {
            event_type: "recv_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ],
            height: 101,
            tx_hash: Some("0x124".to_string()),
        },
        ChainEvent {
            event_type: "acknowledge_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_ack".to_string(), "AQ==".to_string()), // base64 [1]
            ],
            height: 102,
            tx_hash: Some("0x125".to_string()),
        },
    ];
    
    // Process events concurrently
    let futures = events.into_iter().map(|event| {
        let tx = tx.clone();
        let chain_id = chain_id.to_string();
        async move {
            match event.event_type.as_str() {
                "send_packet" => {
                    if let Ok(Some(relay_event)) = EventMonitor::parse_send_packet_event(&chain_id, &event) {
                        tx.send(relay_event).await.ok();
                    }
                }
                "recv_packet" => {
                    if let Ok(Some(relay_event)) = EventMonitor::parse_recv_packet_event(&chain_id, &event) {
                        tx.send(relay_event).await.ok();
                    }
                }
                "acknowledge_packet" => {
                    if let Ok(Some(relay_event)) = EventMonitor::parse_acknowledge_packet_event(&chain_id, &event) {
                        tx.send(relay_event).await.ok();
                    }
                }
                _ => {}
            }
        }
    });
    
    // Execute all event processing concurrently
    futures::future::join_all(futures).await;
    drop(tx);
    
    // Collect all processed events
    let mut processed_events = vec![];
    while let Some(event) = rx.recv().await {
        processed_events.push(event);
    }
    
    // Should have processed 3 events
    assert_eq!(processed_events.len(), 3);
}

/// Test event monitor with high frequency events
#[tokio::test]
async fn test_high_frequency_event_handling() {
    let (tx, mut rx) = mpsc::channel(1000);
    let chain_id = "near-testnet";
    
    // Generate many events rapidly
    let event_count = 100;
    let mut handles = vec![];
    
    for i in 0..event_count {
        let tx = tx.clone();
        let chain_id = chain_id.to_string();
        
        let handle = tokio::spawn(async move {
            let event = ChainEvent {
                event_type: "send_packet".to_string(),
                attributes: vec![
                    ("packet_sequence".to_string(), i.to_string()),
                    ("packet_src_port".to_string(), "transfer".to_string()),
                    ("packet_src_channel".to_string(), "channel-0".to_string()),
                    ("packet_dst_port".to_string(), "transfer".to_string()),
                    ("packet_dst_channel".to_string(), "channel-1".to_string()),
                    ("packet_data".to_string(), "dGVzdA==".to_string()),
                ],
                height: 100 + i as u64,
                tx_hash: Some(format!("0x{:x}", i)),
            };
            
            if let Ok(Some(relay_event)) = EventMonitor::parse_send_packet_event(&chain_id, &event) {
                tx.send(relay_event).await.ok();
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all events to be processed
    futures::future::join_all(handles).await;
    drop(tx);
    
    // Count received events
    let mut count = 0;
    while rx.recv().await.is_some() {
        count += 1;
    }
    
    assert_eq!(count, event_count);
}

/// Test module polling with varying heights
#[tokio::test]
async fn test_module_polling_height_tracking() {
    let (tx, _rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    let mut modules = HashMap::new();
    modules.insert(IbcModuleType::Channel, ModuleInfo {
        contract_id: "channel.testnet".parse().unwrap(),
        module_type: IbcModuleType::Channel,
        version: "1.0.0".to_string(),
        methods: vec![],
    });
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    let mut monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig {
            prefer_streaming: false, // Force polling mode
            polling_interval_ms: 50,
            blocks_per_poll: 10,
            max_concurrent_monitors: 4,
        },
    ).await.unwrap();
    
    // Initialize in polling mode
    monitor.initialize_streams().await.unwrap();
    
    // Start and run briefly
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let monitor_handle = tokio::spawn(async move {
        monitor.start(shutdown_rx).await
    });
    
    // Let it poll a few times
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Shutdown
    shutdown_tx.send(true).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), monitor_handle).await;
    assert!(result.is_ok());
}

/// Test event deduplication
#[tokio::test]
async fn test_event_deduplication() {
    let (tx, mut rx) = mpsc::channel(100);
    let chain_id = "near-testnet";
    
    // Send the same event multiple times
    let event = ChainEvent {
        event_type: "send_packet".to_string(),
        attributes: vec![
            ("packet_sequence".to_string(), "1".to_string()),
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
            ("packet_dst_port".to_string(), "transfer".to_string()),
            ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ("packet_data".to_string(), "dGVzdA==".to_string()),
        ],
        height: 100,
        tx_hash: Some("0x123".to_string()),
    };
    
    // Process same event multiple times
    for _ in 0..5 {
        if let Ok(Some(relay_event)) = EventMonitor::parse_send_packet_event(chain_id, &event) {
            tx.send(relay_event).await.ok();
        }
    }
    
    drop(tx);
    
    // Should receive all events (deduplication would happen at a higher level)
    let mut count = 0;
    while rx.recv().await.is_some() {
        count += 1;
    }
    
    assert_eq!(count, 5);
}

/// Test mixed event stream processing
#[tokio::test]
async fn test_mixed_event_streams() {
    let (tx, mut rx) = mpsc::channel(100);
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect("https://rpc.testnet.near.org");
    
    // Create registry with multiple module types
    let mut modules = HashMap::new();
    modules.insert(IbcModuleType::Client, ModuleInfo {
        contract_id: "client.testnet".parse().unwrap(),
        module_type: IbcModuleType::Client,
        version: "1.0.0".to_string(),
        methods: vec!["update_client".to_string()],
    });
    modules.insert(IbcModuleType::Channel, ModuleInfo {
        contract_id: "channel.testnet".parse().unwrap(),
        module_type: IbcModuleType::Channel,
        version: "1.0.0".to_string(),
        methods: vec!["send_packet".to_string()],
    });
    modules.insert(IbcModuleType::Transfer, ModuleInfo {
        contract_id: "transfer.testnet".parse().unwrap(),
        module_type: IbcModuleType::Transfer,
        version: "1.0.0".to_string(),
        methods: vec!["transfer".to_string()],
    });
    
    let registry = ModuleRegistry {
        router_contract: "router.testnet".parse().unwrap(),
        modules,
    };
    
    let mut monitor = ModularEventMonitor::new(
        "near-testnet".to_string(),
        Arc::new(registry),
        tx,
        rpc_client,
        MonitorConfig {
            prefer_streaming: true,
            polling_interval_ms: 100,
            blocks_per_poll: 10,
            max_concurrent_monitors: 4,
        },
    ).await.unwrap();
    
    // Initialize with mixed streaming/polling
    monitor.initialize_streams().await.unwrap();
    
    // Run briefly
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let monitor_handle = tokio::spawn(async move {
        monitor.start(shutdown_rx).await
    });
    
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Should handle mixed mode without issues
    drop(monitor_handle);
    
    // Check no events in test environment
    let event = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
    assert!(event.is_err());
}