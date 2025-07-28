// Real-time event monitoring for IBC relayer
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use futures::StreamExt;
use base64::Engine;

use crate::chains::{Chain, ChainEvent};
use crate::relay::RelayEvent;

/// Event monitor for tracking blockchain events across multiple chains
pub struct EventMonitor {
    /// Chain implementations being monitored
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Event sender for relay engine
    event_sender: mpsc::Sender<RelayEvent>,
    /// Monitoring configuration
    config: MonitorConfig,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

/// Configuration for event monitoring
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Polling interval for block events (when streaming is not available)
    pub polling_interval_ms: u64,
    /// Number of blocks to check in each polling cycle
    pub blocks_per_poll: u64,
    /// Whether to use streaming (WebSocket) when available
    pub prefer_streaming: bool,
    /// Maximum concurrent chain monitors
    pub max_concurrent_monitors: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            polling_interval_ms: 1000, // 1 second
            blocks_per_poll: 10,
            prefer_streaming: true,
            max_concurrent_monitors: 10,
        }
    }
}

impl EventMonitor {
    /// Create a new event monitor
    pub fn new(
        chains: HashMap<String, Arc<dyn Chain>>,
        event_sender: mpsc::Sender<RelayEvent>,
        config: MonitorConfig,
    ) -> (Self, tokio::sync::watch::Sender<bool>) {
        let (shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
        
        let monitor = Self {
            chains,
            event_sender,
            config,
            shutdown: shutdown_receiver,
        };
        
        (monitor, shutdown_sender)
    }
    
    /// Start monitoring all configured chains
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Starting event monitoring for {} chains", self.chains.len());
        
        // Start monitoring each chain concurrently
        let mut handles = Vec::new();
        
        for (chain_id, chain) in &self.chains {
            let chain_clone = Arc::clone(chain);
            let event_sender = self.event_sender.clone();
            let config = self.config.clone();
            let shutdown_receiver = self.shutdown.clone();
            let chain_id_clone = chain_id.clone();
            
            let handle = tokio::spawn(async move {
                Self::monitor_chain(
                    chain_id_clone,
                    chain_clone,
                    event_sender,
                    config,
                    shutdown_receiver,
                ).await
            });
            
            handles.push(handle);
        }
        
        // Wait for shutdown signal
        tokio::select! {
            _ = self.shutdown.changed() => {
                if *self.shutdown.borrow() {
                    println!("🛑 Event monitor shutdown requested");
                }
            }
        }
        
        // Wait for all monitors to complete
        for handle in handles {
            if let Err(e) = handle.await {
                eprintln!("Error in chain monitor: {}", e);
            }
        }
        
        println!("✅ Event monitor stopped gracefully");
        Ok(())
    }
    
    /// Monitor a single chain for events
    async fn monitor_chain(
        chain_id: String,
        chain: Arc<dyn Chain>,
        event_sender: mpsc::Sender<RelayEvent>,
        config: MonitorConfig,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔄 Starting event monitor for chain: {}", chain_id);
        
        // Try streaming first if preferred and available
        if config.prefer_streaming {
            if let Ok(mut event_stream) = chain.subscribe_events().await {
                println!("📡 Using streaming events for chain: {}", chain_id);
                
                loop {
                    tokio::select! {
                        // Process incoming events
                        Some(chain_event) = event_stream.next() => {
                            if let Err(e) = Self::process_chain_event(
                                &chain_id,
                                chain_event,
                                &event_sender,
                            ).await {
                                eprintln!("Error processing event from {}: {}", chain_id, e);
                            }
                        }
                        
                        // Check for shutdown
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                println!("🛑 Stopping event monitor for chain: {}", chain_id);
                                break;
                            }
                        }
                    }
                }
                
                return Ok(());
            }
        }
        
        // Fallback to polling
        println!("⏱️  Using polling events for chain: {}", chain_id);
        Self::poll_chain_events(chain_id, chain, event_sender, config, shutdown).await
    }
    
    /// Poll chain events using block queries
    async fn poll_chain_events(
        chain_id: String,
        chain: Arc<dyn Chain>,
        event_sender: mpsc::Sender<RelayEvent>,
        config: MonitorConfig,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_height = chain.get_latest_height().await.unwrap_or(0);
        let mut poll_interval = time::interval(Duration::from_millis(config.polling_interval_ms));
        
        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    // Get current height
                    match chain.get_latest_height().await {
                        Ok(current_height) => {
                            if current_height > last_height {
                                // Query events from new blocks
                                let from_height = last_height + 1;
                                let to_height = (current_height).min(last_height + config.blocks_per_poll);
                                
                                match chain.get_events(from_height, to_height).await {
                                    Ok(events) => {
                                        let event_count = events.len();
                                        for event in events {
                                            if let Err(e) = Self::process_chain_event(
                                                &chain_id,
                                                event,
                                                &event_sender,
                                            ).await {
                                                eprintln!("Error processing event from {}: {}", chain_id, e);
                                            }
                                        }
                                        
                                        if event_count > 0 {
                                            println!("📦 Processed {} events from {} (blocks {}-{})", 
                                                     event_count, chain_id, from_height, to_height);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error querying events from {}: {}", chain_id, e);
                                    }
                                }
                                
                                last_height = to_height;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error getting latest height from {}: {}", chain_id, e);
                        }
                    }
                }
                
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        println!("🛑 Stopping polling monitor for chain: {}", chain_id);
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a chain event and convert to relay event if relevant
    async fn process_chain_event(
        chain_id: &str,
        chain_event: ChainEvent,
        event_sender: &mpsc::Sender<RelayEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse different event types and convert to relay events
        match chain_event.event_type.as_str() {
            "send_packet" => {
                if let Some(relay_event) = Self::parse_send_packet_event(chain_id, &chain_event)? {
                    event_sender.send(relay_event).await?;
                }
            }
            "recv_packet" => {
                if let Some(relay_event) = Self::parse_recv_packet_event(chain_id, &chain_event)? {
                    event_sender.send(relay_event).await?;
                }
            }
            "acknowledge_packet" => {
                if let Some(relay_event) = Self::parse_acknowledge_packet_event(chain_id, &chain_event)? {
                    event_sender.send(relay_event).await?;
                }
            }
            "timeout_packet" => {
                if let Some(relay_event) = Self::parse_timeout_packet_event(chain_id, &chain_event)? {
                    event_sender.send(relay_event).await?;
                }
            }
            _ => {
                // Ignore other event types for now
            }
        }
        
        Ok(())
    }
    
    /// Parse send_packet event into RelayEvent::PacketDetected
    pub fn parse_send_packet_event(
        chain_id: &str,
        chain_event: &ChainEvent,
    ) -> Result<Option<RelayEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // Extract packet information from event attributes
        let mut sequence = None;
        let mut source_port = None;
        let mut source_channel = None;
        let mut destination_port = None;
        let mut destination_channel = None;
        let mut data = None;
        let mut timeout_height = None;
        let mut timeout_timestamp = None;
        
        for (key, value) in &chain_event.attributes {
            match key.as_str() {
                "packet_sequence" => sequence = value.parse().ok(),
                "packet_src_port" => source_port = Some(value.clone()),
                "packet_src_channel" => source_channel = Some(value.clone()),
                "packet_dst_port" => destination_port = Some(value.clone()),
                "packet_dst_channel" => destination_channel = Some(value.clone()),
                "packet_data" => {
                    data = Some(base64::engine::general_purpose::STANDARD.decode(value).unwrap_or_else(|_| value.as_bytes().to_vec()));
                }
                "packet_timeout_height" => timeout_height = value.parse().ok(),
                "packet_timeout_timestamp" => timeout_timestamp = value.parse().ok(),
                _ => {}
            }
        }
        
        // Construct IBC packet if we have required fields
        if let (Some(seq), Some(src_port), Some(src_chan), Some(dst_port), Some(dst_chan)) = 
            (sequence, source_port, source_channel, destination_port, destination_channel) {
            
            let packet = crate::chains::IbcPacket {
                sequence: seq,
                source_port: src_port,
                source_channel: src_chan,
                destination_port: dst_port,
                destination_channel: dst_chan,
                data: data.unwrap_or_default(),
                timeout_height,
                timeout_timestamp,
            };
            
            println!("📤 Detected packet send: chain={} seq={} port={} channel={}", 
                     chain_id, seq, packet.source_port, packet.source_channel);
            
            Ok(Some(RelayEvent::PacketDetected {
                chain_id: chain_id.to_string(),
                packet,
                _event: chain_event.clone(),
            }))
        } else {
            // Missing required fields
            Ok(None)
        }
    }
    
    /// Parse recv_packet event into RelayEvent::PacketRelayed
    pub fn parse_recv_packet_event(
        chain_id: &str,
        chain_event: &ChainEvent,
    ) -> Result<Option<RelayEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sequence = None;
        let mut _source_port = None;
        let mut _source_channel = None;
        
        for (key, value) in &chain_event.attributes {
            match key.as_str() {
                "packet_sequence" => sequence = value.parse().ok(),
                "packet_src_port" => _source_port = Some(value.clone()),
                "packet_src_channel" => _source_channel = Some(value.clone()),
                _ => {}
            }
        }
        
        if let Some(seq) = sequence {
            // Determine source chain from the packet's origin
            let source_chain = if chain_id.contains("near") {
                "cosmoshub-testnet".to_string() // Packet was sent from Cosmos, received on NEAR
            } else {
                "near-testnet".to_string() // Packet was sent from NEAR, received on Cosmos
            };
            
            println!("📥 Detected packet receipt: chain={} seq={} from={}", 
                     chain_id, seq, source_chain);
            
            Ok(Some(RelayEvent::PacketRelayed {
                source_chain,
                dest_chain: chain_id.to_string(),
                sequence: seq,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Parse acknowledge_packet event into RelayEvent::PacketAcknowledged
    pub fn parse_acknowledge_packet_event(
        chain_id: &str,
        chain_event: &ChainEvent,
    ) -> Result<Option<RelayEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sequence = None;
        let mut source_port = None;
        let mut source_channel = None;
        let mut destination_port = None;
        let mut destination_channel = None;
        let mut ack_data = None;
        
        for (key, value) in &chain_event.attributes {
            match key.as_str() {
                "packet_sequence" => sequence = value.parse().ok(),
                "packet_src_port" => source_port = Some(value.clone()),
                "packet_src_channel" => source_channel = Some(value.clone()),
                "packet_dst_port" => destination_port = Some(value.clone()),
                "packet_dst_channel" => destination_channel = Some(value.clone()),
                "packet_ack" => {
                    ack_data = Some(base64::engine::general_purpose::STANDARD.decode(value).unwrap_or_else(|_| value.as_bytes().to_vec()));
                }
                _ => {}
            }
        }
        
        if let (Some(seq), Some(src_port), Some(src_chan), Some(dst_port), Some(dst_chan)) = 
            (sequence, source_port, source_channel, destination_port, destination_channel) {
            
            let packet = crate::chains::IbcPacket {
                sequence: seq,
                source_port: src_port,
                source_channel: src_chan,
                destination_port: dst_port,
                destination_channel: dst_chan,
                data: vec![], // Not needed for acknowledgment
                timeout_height: None,
                timeout_timestamp: None,
            };
            
            println!("🎯 Detected packet acknowledgment: chain={} seq={}", chain_id, seq);
            
            Ok(Some(RelayEvent::PacketAcknowledged {
                chain_id: chain_id.to_string(),
                packet,
                ack_data: ack_data.unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Parse timeout_packet event into RelayEvent::PacketTimedOut
    pub fn parse_timeout_packet_event(
        chain_id: &str,
        chain_event: &ChainEvent,
    ) -> Result<Option<RelayEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sequence = None;
        let mut source_port = None;
        let mut source_channel = None;
        let mut destination_port = None;
        let mut destination_channel = None;
        
        for (key, value) in &chain_event.attributes {
            match key.as_str() {
                "packet_sequence" => sequence = value.parse().ok(),
                "packet_src_port" => source_port = Some(value.clone()),
                "packet_src_channel" => source_channel = Some(value.clone()),
                "packet_dst_port" => destination_port = Some(value.clone()),
                "packet_dst_channel" => destination_channel = Some(value.clone()),
                _ => {}
            }
        }
        
        if let (Some(seq), Some(src_port), Some(src_chan), Some(dst_port), Some(dst_chan)) = 
            (sequence, source_port, source_channel, destination_port, destination_channel) {
            
            let packet = crate::chains::IbcPacket {
                sequence: seq,
                source_port: src_port,
                source_channel: src_chan,
                destination_port: dst_port,
                destination_channel: dst_chan,
                data: vec![],
                timeout_height: None,
                timeout_timestamp: None,
            };
            
            println!("⏰ Detected packet timeout: chain={} seq={}", chain_id, seq);
            
            Ok(Some(RelayEvent::PacketTimedOut {
                chain_id: chain_id.to_string(),
                packet,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainEvent;
    
    #[test]
    fn test_parse_send_packet_event() {
        let chain_event = ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "1".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_data".to_string(), "dGVzdA==".to_string()), // "test" in base64
            ],
            height: 100,
            tx_hash: Some("abc123".to_string()),
        };
        
        let result = EventMonitor::parse_send_packet_event("near-testnet", &chain_event).unwrap();
        assert!(result.is_some());
        
        if let Some(RelayEvent::PacketDetected { chain_id, packet, .. }) = result {
            assert_eq!(chain_id, "near-testnet");
            assert_eq!(packet.sequence, 1);
            assert_eq!(packet.source_port, "transfer");
            assert_eq!(packet.source_channel, "channel-0");
            assert_eq!(packet.data, b"test");
        } else {
            panic!("Expected PacketDetected event");
        }
    }
    
    #[test]
    fn test_parse_acknowledge_packet_event() {
        let chain_event = ChainEvent {
            event_type: "acknowledge_packet".to_string(),
            attributes: vec![
                ("packet_sequence".to_string(), "2".to_string()),
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_ack".to_string(), "AQ==".to_string()), // [1] in base64
            ],
            height: 105,
            tx_hash: Some("def456".to_string()),
        };
        
        let result = EventMonitor::parse_acknowledge_packet_event("cosmoshub-testnet", &chain_event).unwrap();
        assert!(result.is_some());
        
        if let Some(RelayEvent::PacketAcknowledged { chain_id, packet, ack_data }) = result {
            assert_eq!(chain_id, "cosmoshub-testnet");
            assert_eq!(packet.sequence, 2);
            assert_eq!(ack_data, vec![1]);
        } else {
            panic!("Expected PacketAcknowledged event");
        }
    }
}