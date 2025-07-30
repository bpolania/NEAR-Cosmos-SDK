// Core relay engine for packet routing
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use crate::chains::{Chain, IbcPacket, ChainEvent};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;

// Sub-modules
pub mod engine;
pub mod packet;
pub mod processor;
pub mod proof;
pub mod near_proof;
pub mod handshake;

// Re-export enhanced types
pub use engine::RelayEngine as EnhancedRelayEngine;
pub use packet::{PacketLifecycle, PacketState, PacketMetadata, ProcessingTimes};
pub use processor::PacketProcessor;
pub use proof::ProofGenerator;

/// Events that drive the relay engine
#[derive(Debug, Clone)]
pub enum RelayEvent {
    /// New packet detected on source chain
    PacketDetected {
        chain_id: String,
        packet: IbcPacket,
        _event: ChainEvent,
    },
    /// Packet successfully relayed
    PacketRelayed {
        source_chain: String,
        dest_chain: String,
        sequence: u64,
    },
    /// Packet acknowledgment received
    PacketAcknowledged {
        chain_id: String,
        packet: IbcPacket,
        ack_data: Vec<u8>,
    },
    /// Packet timed out
    PacketTimedOut {
        chain_id: String,
        packet: IbcPacket,
    },
    /// Chain connection lost
    ChainDisconnected {
        chain_id: String,
    },
    /// Chain reconnected
    ChainReconnected {
        chain_id: String,
    },
}

/// Unique identifier for a packet
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PacketKey {
    pub source_chain: String,
    pub source_port: String,
    pub source_channel: String,
    pub sequence: u64,
}

/// Packet with relay metadata
#[derive(Debug, Clone)]
pub struct PendingPacket {
    pub packet: IbcPacket,
    pub dest_chain: String,
    pub retry_count: u32,
    pub next_retry: Option<std::time::Instant>,
}

/// Tracks packet state across the relay process
#[derive(Debug)]
pub struct PacketTracker {
    /// Packets pending relay (source_chain -> packets)
    pub pending_packets: HashMap<String, Vec<PendingPacket>>,
    /// Packets waiting for acknowledgment
    pub awaiting_ack: HashMap<PacketKey, PendingPacket>,
    /// Completed packets (for cleanup)
    pub completed_packets: Vec<PacketKey>,
}

impl PacketTracker {
    pub fn new() -> Self {
        Self {
            pending_packets: HashMap::new(),
            awaiting_ack: HashMap::new(),
            completed_packets: Vec::new(),
        }
    }
}

/// Main relay engine that coordinates packet flow between chains
pub struct RelayEngine {
    pub chains: HashMap<String, Arc<dyn Chain>>,
}

impl RelayEngine {
    pub fn new(
        _config: RelayerConfig,
        chains: HashMap<String, Arc<dyn Chain>>,
        _metrics: Arc<RelayerMetrics>,
    ) -> Self {
        Self {
            chains,
        }
    }
    
    /// Extract IBC packet from chain event
    pub fn extract_packet_from_event(event: &ChainEvent) -> Option<IbcPacket> {
        // Helper function to find attribute value
        let find_attr = |key: &str| -> Option<String> {
            event.attributes.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        
        // Extract packet data from event attributes
        let source_port = find_attr("packet_src_port")?;
        let source_channel = find_attr("packet_src_channel")?;
        let destination_port = find_attr("packet_dst_port")?;
        let destination_channel = find_attr("packet_dst_channel")?;
        let sequence = find_attr("packet_sequence")?.parse().ok()?;
        let data = find_attr("packet_data")
            .map(|s| s.as_bytes().to_vec())
            .unwrap_or_default();
        
        Some(IbcPacket {
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data,
            timeout_height: None,
            timeout_timestamp: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_relay_types_compile() {
        // Test RelayEvent variants
        let event = ChainEvent {
            event_type: "test".to_string(),
            attributes: vec![],
            height: 0,
            tx_hash: None,
        };
        
        let packet = IbcPacket {
            sequence: 1,
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![],
            timeout_height: None,
            timeout_timestamp: None,
        };
        
        let _detected = RelayEvent::PacketDetected {
            chain_id: "test".to_string(),
            packet: packet.clone(),
            _event: event,
        };
        
        let _relayed = RelayEvent::PacketRelayed {
            source_chain: "test".to_string(),
            dest_chain: "test2".to_string(),
            sequence: 1,
        };
        
        // Test PendingPacket
        let pending = PendingPacket {
            packet: packet.clone(),
            dest_chain: "test".to_string(),
            retry_count: 0,
            next_retry: None,
        };
        
        // Test PacketTracker
        let mut tracker = PacketTracker::new();
        tracker.pending_packets.insert("test".to_string(), vec![pending]);
        
        // Test PacketKey
        let key = PacketKey {
            source_chain: "test".to_string(),
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            sequence: 1,
        };
        
        assert_eq!(key.sequence, 1);
        
        // Test RelayEngine
        let config = RelayerConfig {
            global: crate::config::GlobalConfig {
                log_level: "info".to_string(),
                max_retries: 3,
                retry_delay_ms: 1000,
                health_check_interval: 30,
            },
            chains: HashMap::new(),
            connections: vec![],
            metrics: crate::config::MetricsConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
        };
        
        let metrics = Arc::new(RelayerMetrics::new().unwrap());
        let chains = HashMap::new();
        let engine = RelayEngine::new(config, chains, metrics);
        assert!(engine.chains.is_empty());
    }
}