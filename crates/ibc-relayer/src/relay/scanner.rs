// Packet scanning and detection system for IBC relay operations
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

use crate::chains::{Chain, ChainEvent, IbcPacket};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::{RelayEvent, PacketKey};

/// Configuration for packet scanning behavior
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// How often to scan for new packets (seconds)
    pub scan_interval: u64,
    /// Starting block/height to scan from
    pub start_height: Option<u64>,
    /// Maximum number of blocks to scan in one batch
    pub max_blocks_per_scan: u64,
    /// Maximum number of packets to process per scan
    pub max_packets_per_scan: usize,
    /// Channels to monitor (port/channel pairs)
    pub monitored_channels: Vec<(String, String)>, // (port, channel)
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_interval: 5, // Every 5 seconds
            start_height: None,
            max_blocks_per_scan: 100,
            max_packets_per_scan: 50,
            monitored_channels: vec![
                ("transfer".to_string(), "channel-0".to_string()),
            ],
        }
    }
}

/// Tracks scanning state for each chain
#[derive(Debug)]
struct ChainScanState {
    chain_id: String,
    last_scanned_height: u64,
    last_scan_time: Instant,
    consecutive_empty_scans: u32,
    total_packets_found: u64,
    last_error: Option<String>,
    scan_errors: u32,
}

impl ChainScanState {
    fn new(chain_id: String, start_height: u64) -> Self {
        Self {
            chain_id,
            last_scanned_height: start_height,
            last_scan_time: Instant::now(),
            consecutive_empty_scans: 0,
            total_packets_found: 0,
            last_error: None,
            scan_errors: 0,
        }
    }
    
    fn update_success(&mut self, new_height: u64, packets_found: usize) {
        self.last_scanned_height = new_height;
        self.last_scan_time = Instant::now();
        self.total_packets_found += packets_found as u64;
        self.last_error = None;
        
        if packets_found == 0 {
            self.consecutive_empty_scans += 1;
        } else {
            self.consecutive_empty_scans = 0;
        }
    }
    
    fn update_error(&mut self, error: String) {
        self.last_scan_time = Instant::now();
        self.last_error = Some(error);
        self.scan_errors += 1;
    }
}

/// Packet scanner that monitors chains for new IBC packets
pub struct PacketScanner {
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Scanner configuration
    config: ScannerConfig,
    /// Relay configuration
    relay_config: RelayerConfig,
    /// Event sender for detected packets
    event_sender: mpsc::Sender<RelayEvent>,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
    /// Scanning state per chain
    chain_states: HashMap<String, ChainScanState>,
    /// Already processed packets (to avoid duplicates)
    processed_packets: HashSet<PacketKey>,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl PacketScanner {
    /// Create a new packet scanner
    pub fn new(
        chains: HashMap<String, Arc<dyn Chain>>,
        config: ScannerConfig,
        relay_config: RelayerConfig,
        event_sender: mpsc::Sender<RelayEvent>,
        metrics: Arc<RelayerMetrics>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        let mut chain_states = HashMap::new();
        
        // Initialize scan state for each chain
        for chain_id in chains.keys() {
            let start_height = config.start_height.unwrap_or(0);
            chain_states.insert(
                chain_id.clone(),
                ChainScanState::new(chain_id.clone(), start_height),
            );
        }
        
        Self {
            chains,
            config,
            relay_config,
            event_sender,
            metrics,
            chain_states,
            processed_packets: HashSet::new(),
            shutdown,
        }
    }
    
    /// Start the packet scanning loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Starting packet scanner for {} chains", self.chains.len());
        
        let mut scan_interval = time::interval(Duration::from_secs(self.config.scan_interval));
        
        loop {
            tokio::select! {
                // Perform periodic scans
                _ = scan_interval.tick() => {
                    if let Err(e) = self.scan_all_chains().await {
                        eprintln!("Error during chain scanning: {}", e);
                        // Note: scanner_errors metric would need to be added to RelayerMetrics
                        // self.metrics.scanner_errors.inc();
                    }
                }
                
                // Check for shutdown signal
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        println!("🛑 Packet scanner shutdown requested");
                        break;
                    }
                }
            }
        }
        
        println!("✅ Packet scanner stopped gracefully");
        Ok(())
    }
    
    /// Scan all chains for new packets
    async fn scan_all_chains(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut total_packets_found = 0;
        
        // Clone the chains map to avoid borrow checker issues
        let chains_clone = self.chains.clone();
        for (chain_id, chain) in chains_clone.iter() {
            match self.scan_chain(chain_id, chain.clone()).await {
                Ok(packets_found) => {
                    total_packets_found += packets_found;
                    
                    // Update chain state on success
                    if let Some(state) = self.chain_states.get_mut(chain_id) {
                        let latest_height = chain.get_latest_height().await.unwrap_or(state.last_scanned_height);
                        state.update_success(latest_height, packets_found);
                    }
                }
                Err(e) => {
                    eprintln!("Error scanning chain {}: {}", chain_id, e);
                    
                    // Update chain state on error
                    if let Some(state) = self.chain_states.get_mut(chain_id) {
                        state.update_error(e.to_string());
                    }
                }
            }
        }
        
        if total_packets_found > 0 {
            println!("🔍 Scan complete: found {} new packets across all chains", total_packets_found);
            // Add each packet individually since we don't have an add method
            for _ in 0..total_packets_found {
                self.metrics.total_packets_detected.inc();
            }
        }
        
        // Periodic cleanup of processed packets to prevent memory leaks
        if self.processed_packets.len() > 10000 {
            self.cleanup_processed_packets();
        }
        
        Ok(())
    }
    
    /// Scan a specific chain for new packets
    async fn scan_chain(
        &mut self,
        chain_id: &str,
        chain: Arc<dyn Chain>,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let state = self.chain_states.get(chain_id)
            .ok_or_else(|| format!("No scan state for chain {}", chain_id))?;
        
        let start_height = state.last_scanned_height;
        let latest_height = chain.get_latest_height().await?;
        
        if latest_height <= start_height {
            // No new blocks to scan
            return Ok(0);
        }
        
        let end_height = std::cmp::min(
            latest_height,
            start_height + self.config.max_blocks_per_scan,
        );
        
        println!("🔍 Scanning chain {} from height {} to {}", 
                 chain_id, start_height + 1, end_height);
        
        let mut packets_found = 0;
        
        // Scan blocks in the height range
        for height in (start_height + 1)..=end_height {
            if packets_found >= self.config.max_packets_per_scan {
                println!("⚠️  Reached max packets per scan ({}) for chain {}", 
                         self.config.max_packets_per_scan, chain_id);
                break;
            }
            
            match self.scan_block(chain_id, chain.clone(), height).await {
                Ok(block_packets) => {
                    packets_found += block_packets;
                }
                Err(e) => {
                    eprintln!("Error scanning block {} on chain {}: {}", height, chain_id, e);
                    // Continue with next block rather than failing the entire scan
                }
            }
        }
        
        Ok(packets_found)
    }
    
    /// Scan a specific block for IBC packets
    async fn scan_block(
        &mut self,
        chain_id: &str,
        chain: Arc<dyn Chain>,
        height: u64,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // Get events from the block range (single height)
        let events = chain.get_events(height, height).await?;
        let mut packets_found = 0;
        
        for event in events {
            if self.is_packet_event(&event) {
                if let Some(packet) = self.extract_packet_from_event(&event) {
                    if self.should_process_packet(chain_id, &packet) {
                        // Send packet detected event
                        let relay_event = RelayEvent::PacketDetected {
                            chain_id: chain_id.to_string(),
                            packet: packet.clone(),
                            _event: event.clone(),
                        };
                        
                        if let Err(e) = self.event_sender.send(relay_event).await {
                            eprintln!("Failed to send packet detected event: {}", e);
                        } else {
                            // Mark as processed to avoid duplicates
                            let packet_key = PacketKey {
                                source_chain: chain_id.to_string(),
                                source_port: packet.source_port.clone(),
                                source_channel: packet.source_channel.clone(),
                                sequence: packet.sequence,
                            };
                            self.processed_packets.insert(packet_key);
                            packets_found += 1;
                            
                            println!("📦 Detected packet on {} at height {}: seq={} port={} channel={}", 
                                     chain_id, height, packet.sequence, packet.source_port, packet.source_channel);
                        }
                    }
                }
            } else if self.is_acknowledgment_event(&event) {
                if let Some((packet, ack_data)) = self.extract_acknowledgment_from_event(&event) {
                    // Send acknowledgment event
                    let relay_event = RelayEvent::PacketAcknowledged {
                        chain_id: chain_id.to_string(),
                        packet,
                        ack_data,
                    };
                    
                    if let Err(e) = self.event_sender.send(relay_event).await {
                        eprintln!("Failed to send packet acknowledged event: {}", e);
                    } else {
                        println!("🎯 Detected acknowledgment on {} at height {}", chain_id, height);
                    }
                }
            } else if self.is_timeout_event(&event) {
                if let Some(packet) = self.extract_packet_from_event(&event) {
                    // Send timeout event
                    let relay_event = RelayEvent::PacketTimedOut {
                        chain_id: chain_id.to_string(),
                        packet,
                    };
                    
                    if let Err(e) = self.event_sender.send(relay_event).await {
                        eprintln!("Failed to send packet timeout event: {}", e);
                    } else {
                        println!("⏰ Detected timeout on {} at height {}", chain_id, height);
                    }
                }
            }
        }
        
        Ok(packets_found)
    }
    
    /// Check if an event represents an IBC packet
    fn is_packet_event(&self, event: &ChainEvent) -> bool {
        // Look for send_packet events
        event.event_type == "send_packet" || 
        event.event_type.contains("packet") ||
        event.attributes.iter().any(|(key, _)| 
            key.contains("packet") && (key.contains("src") || key.contains("sequence"))
        )
    }
    
    /// Check if an event represents a packet acknowledgment
    fn is_acknowledgment_event(&self, event: &ChainEvent) -> bool {
        event.event_type == "acknowledge_packet" ||
        event.event_type.contains("acknowledgment") ||
        event.attributes.iter().any(|(key, _)| key.contains("acknowledgment"))
    }
    
    /// Check if an event represents a packet timeout
    fn is_timeout_event(&self, event: &ChainEvent) -> bool {
        event.event_type == "timeout_packet" ||
        event.event_type.contains("timeout") ||
        event.attributes.iter().any(|(key, _)| key.contains("timeout"))
    }
    
    /// Extract packet information from a chain event
    fn extract_packet_from_event(&self, event: &ChainEvent) -> Option<IbcPacket> {
        // Helper function to find attribute value
        let find_attr = |key: &str| -> Option<String> {
            event.attributes.iter()
                .find(|(k, _)| k == key || k.contains(key))
                .map(|(_, v)| v.clone())
        };
        
        // Extract packet data from event attributes
        let source_port = find_attr("packet_src_port").or_else(|| find_attr("src_port"))?;
        let source_channel = find_attr("packet_src_channel").or_else(|| find_attr("src_channel"))?;
        let destination_port = find_attr("packet_dst_port").or_else(|| find_attr("dst_port"))?;
        let destination_channel = find_attr("packet_dst_channel").or_else(|| find_attr("dst_channel"))?;
        let sequence = find_attr("packet_sequence").or_else(|| find_attr("sequence"))?.parse().ok()?;
        
        let data = find_attr("packet_data")
            .or_else(|| find_attr("data"))
            .map(|s| s.as_bytes().to_vec())
            .unwrap_or_default();
        
        // Extract timeout information if available
        let timeout_height = find_attr("packet_timeout_height")
            .or_else(|| find_attr("timeout_height"))
            .and_then(|s| s.parse().ok());
            
        let timeout_timestamp = find_attr("packet_timeout_timestamp")
            .or_else(|| find_attr("timeout_timestamp"))
            .and_then(|s| s.parse().ok());
        
        Some(IbcPacket {
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data,
            timeout_height,
            timeout_timestamp,
        })
    }
    
    /// Extract acknowledgment information from a chain event
    fn extract_acknowledgment_from_event(&self, event: &ChainEvent) -> Option<(IbcPacket, Vec<u8>)> {
        let packet = self.extract_packet_from_event(event)?;
        
        // Find acknowledgment data
        let ack_data = event.attributes.iter()
            .find(|(k, _)| k.contains("acknowledgment") || k.contains("ack"))
            .map(|(_, v)| v.as_bytes().to_vec())
            .unwrap_or_default();
        
        Some((packet, ack_data))
    }
    
    /// Check if we should process this packet based on configuration
    fn should_process_packet(&self, chain_id: &str, packet: &IbcPacket) -> bool {
        // Check if we already processed this packet
        let packet_key = PacketKey {
            source_chain: chain_id.to_string(),
            source_port: packet.source_port.clone(),
            source_channel: packet.source_channel.clone(),
            sequence: packet.sequence,
        };
        
        if self.processed_packets.contains(&packet_key) {
            return false;
        }
        
        // Check if this channel is being monitored
        if !self.config.monitored_channels.is_empty() {
            let channel_monitored = self.config.monitored_channels.iter()
                .any(|(port, channel)| {
                    port == &packet.source_port && channel == &packet.source_channel
                });
            
            if !channel_monitored {
                return false;
            }
        }
        
        true
    }
    
    /// Clean up old processed packets to prevent memory leaks
    fn cleanup_processed_packets(&mut self) {
        let original_size = self.processed_packets.len();
        
        // For now, just clear half of them
        // In a production system, you'd want to keep packets processed recently
        if original_size > 5000 {
            let to_remove: Vec<_> = self.processed_packets.iter()
                .take(original_size / 2)
                .cloned()
                .collect();
            
            for key in to_remove {
                self.processed_packets.remove(&key);
            }
            
            println!("🧹 Cleaned up processed packets: {} -> {}", 
                     original_size, self.processed_packets.len());
        }
    }
    
    /// Get scanning statistics for monitoring
    pub fn get_scan_stats(&self) -> ScanStats {
        let mut stats = ScanStats::default();
        
        for state in self.chain_states.values() {
            stats.total_chains += 1;
            stats.total_packets_found += state.total_packets_found;
            stats.total_scan_errors += state.scan_errors;
            
            if state.last_error.is_some() {
                stats.chains_with_errors += 1;
            }
            
            stats.average_consecutive_empty_scans += state.consecutive_empty_scans as f64;
        }
        
        if stats.total_chains > 0 {
            stats.average_consecutive_empty_scans /= stats.total_chains as f64;
        }
        
        stats.processed_packets_count = self.processed_packets.len();
        stats
    }
}

/// Statistics for packet scanner monitoring
#[derive(Debug, Default)]
pub struct ScanStats {
    pub total_chains: usize,
    pub total_packets_found: u64,
    pub total_scan_errors: u32,
    pub chains_with_errors: usize,
    pub average_consecutive_empty_scans: f64,
    pub processed_packets_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainEvent;
    
    #[test]
    fn test_scanner_config_default() {
        let config = ScannerConfig::default();
        assert_eq!(config.scan_interval, 5);
        assert_eq!(config.max_blocks_per_scan, 100);
        assert_eq!(config.max_packets_per_scan, 50);
        assert_eq!(config.monitored_channels.len(), 1);
    }
    
    #[test]
    fn test_chain_scan_state() {
        let mut state = ChainScanState::new("test-chain".to_string(), 100);
        assert_eq!(state.last_scanned_height, 100);
        assert_eq!(state.consecutive_empty_scans, 0);
        
        // Test successful scan with packets
        state.update_success(105, 3);
        assert_eq!(state.last_scanned_height, 105);
        assert_eq!(state.total_packets_found, 3);
        assert_eq!(state.consecutive_empty_scans, 0);
        
        // Test successful scan without packets
        state.update_success(110, 0);
        assert_eq!(state.consecutive_empty_scans, 1);
        
        // Test error handling
        state.update_error("test error".to_string());
        assert_eq!(state.scan_errors, 1);
        assert!(state.last_error.is_some());
    }
    
    #[test]
    fn test_packet_event_detection() {
        let scanner = create_test_scanner();
        
        // Test packet event detection
        let packet_event = ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_sequence".to_string(), "1".to_string()),
            ],
            height: 100,
            tx_hash: Some("test_hash".to_string()),
        };
        
        assert!(scanner.is_packet_event(&packet_event));
        
        // Test acknowledgment event detection
        let ack_event = ChainEvent {
            event_type: "acknowledge_packet".to_string(),
            attributes: vec![
                ("acknowledgment".to_string(), "success".to_string()),
            ],
            height: 101,
            tx_hash: Some("test_hash_2".to_string()),
        };
        
        assert!(scanner.is_acknowledgment_event(&ack_event));
        
        // Test timeout event detection
        let timeout_event = ChainEvent {
            event_type: "timeout_packet".to_string(),
            attributes: vec![
                ("timeout".to_string(), "height".to_string()),
            ],
            height: 102,
            tx_hash: Some("test_hash_3".to_string()),
        };
        
        assert!(scanner.is_timeout_event(&timeout_event));
    }
    
    #[test]
    fn test_packet_extraction() {
        let scanner = create_test_scanner();
        
        let event = ChainEvent {
            event_type: "send_packet".to_string(),
            attributes: vec![
                ("packet_src_port".to_string(), "transfer".to_string()),
                ("packet_src_channel".to_string(), "channel-0".to_string()),
                ("packet_dst_port".to_string(), "transfer".to_string()),
                ("packet_dst_channel".to_string(), "channel-1".to_string()),
                ("packet_sequence".to_string(), "42".to_string()),
                ("packet_data".to_string(), "test_data".to_string()),
            ],
            height: 100,
            tx_hash: Some("test_hash".to_string()),
        };
        
        let packet = scanner.extract_packet_from_event(&event).unwrap();
        assert_eq!(packet.sequence, 42);
        assert_eq!(packet.source_port, "transfer");
        assert_eq!(packet.source_channel, "channel-0");
        assert_eq!(packet.destination_port, "transfer");
        assert_eq!(packet.destination_channel, "channel-1");
        assert_eq!(packet.data, b"test_data");
    }
    
    fn create_test_scanner() -> PacketScanner {
        let (event_sender, _) = mpsc::channel(100);
        let (_, shutdown_receiver) = tokio::sync::watch::channel(false);
        let metrics = Arc::new(crate::metrics::RelayerMetrics::new().unwrap());
        let config = ScannerConfig::default();
        let relay_config = crate::config::RelayerConfig {
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
        
        PacketScanner::new(
            HashMap::new(),
            config,
            relay_config,
            event_sender,
            metrics,
            shutdown_receiver,
        )
    }
}