// Example usage of the IBC relayer library
// This demonstrates how the types will be used in the actual implementation

use ibc_relayer::{
    chains::{Chain, ChainEvent},
    relay::{RelayEngine, RelayEvent, PacketTracker, PendingPacket, PacketKey},
    metrics::RelayerMetrics,
    config::{RelayerConfig, GlobalConfig, MetricsConfig},
};
use std::collections::HashMap;
use std::sync::Arc;

// Example chain implementation
struct ExampleChain {
    _name: String,
}

impl Chain for ExampleChain {}

fn main() {
    println!("IBC Relayer Example Usage");
    
    // Create configuration
    let config = RelayerConfig {
        global: GlobalConfig {
            log_level: "info".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval: 30,
        },
        chains: HashMap::new(),
        connections: vec![],
        metrics: MetricsConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 9090,
        },
    };
    
    // Initialize metrics
    let metrics = Arc::new(RelayerMetrics::new().expect("Failed to create metrics"));
    
    // Create chains
    let mut chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    chains.insert("example-chain".to_string(), Arc::new(ExampleChain {
        _name: "example".to_string(),
    }));
    
    // Create relay engine
    let engine = RelayEngine::new(config, chains, metrics.clone());
    println!("Created relay engine with {} chains", engine.chains.len());
    
    // Example: Create a chain event
    let event = ChainEvent {
        event_type: "send_packet".to_string(),
        attributes: vec![
            ("packet_src_port".to_string(), "transfer".to_string()),
            ("packet_src_channel".to_string(), "channel-0".to_string()),
            ("packet_dst_port".to_string(), "transfer".to_string()),
            ("packet_dst_channel".to_string(), "channel-1".to_string()),
            ("packet_sequence".to_string(), "1".to_string()),
            ("packet_data".to_string(), "test_data".to_string()),
        ],
        height: 100,
        tx_hash: Some("0x123".to_string()),
    };
    
    // Extract packet from event
    if let Some(packet) = RelayEngine::extract_packet_from_event(&event) {
        println!("Extracted packet with sequence: {}", packet.sequence);
        println!("  Source: {}/{}", packet.source_port, packet.source_channel);
        println!("  Destination: {}/{}", packet.destination_port, packet.destination_channel);
        println!("  Data length: {} bytes", packet.data.len());
        
        // Create relay events
        let detected_event = RelayEvent::PacketDetected {
            chain_id: "example-chain".to_string(),
            packet: packet.clone(),
            _event: event.clone(),
        };
        
        let relayed_event = RelayEvent::PacketRelayed {
            source_chain: "chain-a".to_string(),
            dest_chain: "chain-b".to_string(),
            sequence: packet.sequence,
        };
        
        let ack_event = RelayEvent::PacketAcknowledged {
            chain_id: "chain-b".to_string(),
            packet: packet.clone(),
            ack_data: b"success".to_vec(),
        };
        
        let timeout_event = RelayEvent::PacketTimedOut {
            chain_id: "chain-a".to_string(),
            packet: packet.clone(),
        };
        
        // Example of chain events
        let _disconnected = RelayEvent::ChainDisconnected {
            chain_id: "chain-a".to_string(),
        };
        
        let _reconnected = RelayEvent::ChainReconnected {
            chain_id: "chain-a".to_string(),
        };
        
        // Process events (in real implementation)
        match detected_event {
            RelayEvent::PacketDetected { chain_id, packet, .. } => {
                println!("Packet detected on chain: {}", chain_id);
                
                // Create pending packet
                let pending = PendingPacket {
                    packet: packet.clone(),
                    dest_chain: "chain-b".to_string(),
                    retry_count: 0,
                    next_retry: None,
                };
                
                // Track packet
                let mut tracker = PacketTracker::new();
                tracker.pending_packets
                    .entry(chain_id.clone())
                    .or_insert_with(Vec::new)
                    .push(pending.clone());
                
                // Create packet key
                let key = PacketKey {
                    source_chain: chain_id,
                    source_port: packet.source_port,
                    source_channel: packet.source_channel,
                    sequence: packet.sequence,
                };
                
                // Move to awaiting acknowledgment
                tracker.awaiting_ack.insert(key.clone(), pending);
                
                // After acknowledgment, mark as completed
                tracker.completed_packets.push(key);
                
                println!("Packet tracked and awaiting acknowledgment");
            }
            _ => {}
        }
        
        // Update metrics
        match relayed_event {
            RelayEvent::PacketRelayed { source_chain, dest_chain, sequence } => {
                metrics.packets_relayed.inc();
                metrics.packet_relay_duration.observe(0.5);
                println!("Packet {} relayed from {} to {}", sequence, source_chain, dest_chain);
            }
            _ => {}
        }
        
        // Handle acknowledgment
        match ack_event {
            RelayEvent::PacketAcknowledged { chain_id, packet, ack_data } => {
                println!("Packet {} acknowledged on {}: {:?}", 
                    packet.sequence, chain_id, 
                    String::from_utf8_lossy(&ack_data));
            }
            _ => {}
        }
        
        // Handle timeout
        match timeout_event {
            RelayEvent::PacketTimedOut { chain_id, packet } => {
                metrics.packets_failed.inc();
                println!("Packet {} timed out on {}", packet.sequence, chain_id);
                
                // Check timeout values
                if let Some(height) = packet.timeout_height {
                    println!("  Timeout height: {}", height);
                }
                if let Some(timestamp) = packet.timeout_timestamp {
                    println!("  Timeout timestamp: {}", timestamp);
                }
            }
            _ => {}
        }
    }
    
    // Error case
    metrics.rpc_errors.inc();
    
    // Access metrics registry
    let registry = metrics.registry();
    let metric_families = registry.gather();
    println!("\nMetrics collected: {} families", metric_families.len());
    
    println!("\nExample completed successfully!");
}