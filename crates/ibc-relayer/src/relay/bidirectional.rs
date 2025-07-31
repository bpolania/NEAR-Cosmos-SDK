// Bidirectional packet relay with proper sequencing and state synchronization
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

use crate::chains::{Chain, IbcPacket};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::{RelayEvent, PacketKey};
use super::processor::PacketProcessor;

/// Configuration for bidirectional relay
#[derive(Debug, Clone)]
pub struct BidirectionalConfig {
    /// Maximum number of packets to process in parallel per direction
    pub max_parallel_packets: usize,
    /// Sequence tracking window size
    pub sequence_window_size: u64,
    /// How often to check for sequence gaps (seconds)
    pub sequence_check_interval: u64,
    /// Maximum time to wait for out-of-order packets (seconds)
    pub max_out_of_order_wait: u64,
    /// Enable strict ordering (wait for all previous packets)
    pub strict_ordering: bool,
    /// Batch size for processing multiple packets
    pub batch_size: usize,
}

impl Default for BidirectionalConfig {
    fn default() -> Self {
        Self {
            max_parallel_packets: 10,
            sequence_window_size: 1000,
            sequence_check_interval: 10,
            max_out_of_order_wait: 300, // 5 minutes
            strict_ordering: true,
            batch_size: 5,
        }
    }
}

/// Direction of packet flow
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelayDirection {
    /// NEAR to Cosmos
    NearToCosmos,
    /// Cosmos to NEAR
    CosmosToNear,
}

impl Default for RelayDirection {
    fn default() -> Self {
        RelayDirection::NearToCosmos
    }
}

impl RelayDirection {
    pub fn source_chain(&self) -> &str {
        match self {
            RelayDirection::NearToCosmos => "near",
            RelayDirection::CosmosToNear => "cosmos",
        }
    }

    pub fn dest_chain(&self) -> &str {
        match self {
            RelayDirection::NearToCosmos => "cosmos",
            RelayDirection::CosmosToNear => "near",
        }
    }

    pub fn reverse(&self) -> Self {
        match self {
            RelayDirection::NearToCosmos => RelayDirection::CosmosToNear,
            RelayDirection::CosmosToNear => RelayDirection::NearToCosmos,
        }
    }
}

/// State of a packet in the bidirectional relay system
#[derive(Debug, Clone, PartialEq)]
pub enum PacketRelayState {
    /// Packet detected but not yet processed
    Detected,
    /// Packet is being processed
    Processing,
    /// Packet successfully relayed
    Relayed,
    /// Packet acknowledged by destination
    Acknowledged,
    /// Packet timed out
    TimedOut,
    /// Packet failed to relay
    Failed(String),
}

/// Sequenced packet with metadata
#[derive(Debug, Clone)]
pub struct SequencedPacket {
    /// The IBC packet
    pub packet: IbcPacket,
    /// Relay direction
    pub direction: RelayDirection,
    /// Source chain ID
    pub source_chain_id: String,
    /// Destination chain ID
    pub dest_chain_id: String,
    /// Current state
    pub state: PacketRelayState,
    /// When the packet was detected
    pub detected_at: Instant,
    /// When the packet was last processed
    pub last_processed: Option<Instant>,
    /// Number of processing attempts
    pub attempts: u32,
    /// Channel identifier (port:channel)
    pub channel_id: String,
}

impl SequencedPacket {
    pub fn new(
        packet: IbcPacket,
        direction: RelayDirection,
        source_chain_id: String,
        dest_chain_id: String,
    ) -> Self {
        let channel_id = format!("{}:{}", packet.source_port, packet.source_channel);
        
        Self {
            packet,
            direction,
            source_chain_id,
            dest_chain_id,
            state: PacketRelayState::Detected,
            detected_at: Instant::now(),
            last_processed: None,
            attempts: 0,
            channel_id,
        }
    }

    pub fn packet_key(&self) -> PacketKey {
        PacketKey {
            source_chain: self.source_chain_id.clone(),
            source_port: self.packet.source_port.clone(),
            source_channel: self.packet.source_channel.clone(),
            sequence: self.packet.sequence,
        }
    }
}

/// Manages packet sequences for a specific channel and direction
#[derive(Debug)]
pub struct SequenceTracker {
    /// Channel identifier
    pub channel_id: String,
    /// Relay direction
    pub direction: RelayDirection,
    /// Next expected sequence number
    pub next_expected: u64,
    /// Highest processed sequence number
    pub highest_processed: u64,
    /// Out-of-order packets waiting for processing
    pub pending_packets: BTreeMap<u64, SequencedPacket>,
    /// Recently processed packets (for duplicate detection)
    pub processed_window: VecDeque<u64>,
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl SequenceTracker {
    pub fn new(channel_id: String, direction: RelayDirection, start_sequence: u64) -> Self {
        Self {
            channel_id,
            direction,
            next_expected: start_sequence,
            highest_processed: start_sequence.saturating_sub(1),
            pending_packets: BTreeMap::new(),
            processed_window: VecDeque::new(),
            last_activity: Instant::now(),
        }
    }

    /// Add a packet to the sequence tracker
    pub fn add_packet(&mut self, packet: SequencedPacket) -> Vec<SequencedPacket> {
        let sequence = packet.packet.sequence;
        let mut ready_packets = Vec::new();

        self.last_activity = Instant::now();

        // Check if we've already processed this packet
        if self.processed_window.contains(&sequence) || sequence <= self.highest_processed {
            println!("🔄 Duplicate packet detected: seq={}", sequence);
            return ready_packets;
        }

        if sequence == self.next_expected {
            // This is the next packet we're waiting for
            ready_packets.push(packet);
            self.next_expected += 1;
            self.highest_processed = sequence;
            self.add_to_processed_window(sequence);

            // Check if we can process any pending packets
            while let Some((&seq, _)) = self.pending_packets.first_key_value() {
                if seq == self.next_expected {
                    if let Some(pending_packet) = self.pending_packets.remove(&seq) {
                        ready_packets.push(pending_packet);
                        self.next_expected += 1;
                        self.highest_processed = seq;
                        self.add_to_processed_window(seq);
                    }
                } else {
                    break;
                }
            }
        } else if sequence > self.next_expected {
            // Out-of-order packet - store for later
            println!("📋 Out-of-order packet: seq={}, expected={}", sequence, self.next_expected);
            self.pending_packets.insert(sequence, packet);
        } else {
            // Old packet - ignore
            println!("⚠️  Old packet ignored: seq={}, expected={}", sequence, self.next_expected);
        }

        ready_packets
    }

    /// Add sequence to processed window
    fn add_to_processed_window(&mut self, sequence: u64) {
        const MAX_WINDOW_SIZE: usize = 1000;
        
        self.processed_window.push_back(sequence);
        if self.processed_window.len() > MAX_WINDOW_SIZE {
            self.processed_window.pop_front();
        }
    }

    /// Get packets that have been waiting too long
    pub fn get_expired_packets(&mut self, max_wait: Duration) -> Vec<SequencedPacket> {
        let cutoff_time = Instant::now() - max_wait;
        let mut expired = Vec::new();

        // Find expired packets
        let expired_sequences: Vec<u64> = self.pending_packets
            .iter()
            .filter(|(_, packet)| packet.detected_at < cutoff_time)
            .map(|(&seq, _)| seq)
            .collect();

        // Remove and return expired packets
        for seq in expired_sequences {
            if let Some(packet) = self.pending_packets.remove(&seq) {
                expired.push(packet);
            }
        }

        expired
    }

    /// Get statistics for this sequence tracker
    pub fn get_stats(&self) -> SequenceStats {
        SequenceStats {
            channel_id: self.channel_id.clone(),
            direction: self.direction.clone(),
            next_expected: self.next_expected,
            highest_processed: self.highest_processed,
            pending_count: self.pending_packets.len(),
            processed_window_size: self.processed_window.len(),
            sequence_gap: if self.pending_packets.is_empty() {
                0
            } else {
                self.pending_packets.keys().next().unwrap() - self.next_expected
            },
        }
    }
}

/// Bidirectional packet relay manager
pub struct BidirectionalRelayManager {
    /// Configuration
    config: BidirectionalConfig,
    /// Relayer configuration
    relayer_config: RelayerConfig,
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Packet processor
    processor: PacketProcessor,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
    /// Event sender for relay events
    event_sender: mpsc::Sender<RelayEvent>,
    /// Sequence trackers per channel and direction
    sequence_trackers: HashMap<(String, RelayDirection), SequenceTracker>,
    /// Currently processing packets
    processing_packets: HashMap<PacketKey, SequencedPacket>,
    /// Completed packets (for statistics)
    completed_packets: VecDeque<(PacketKey, SequencedPacket, Instant)>,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl BidirectionalRelayManager {
    /// Create a new bidirectional relay manager
    pub fn new(
        config: BidirectionalConfig,
        relayer_config: RelayerConfig,
        chains: HashMap<String, Arc<dyn Chain>>,
        metrics: Arc<RelayerMetrics>,
        event_sender: mpsc::Sender<RelayEvent>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        let processor = PacketProcessor::new(
            chains.clone(),
            relayer_config.clone(),
            metrics.clone(),
        );

        Self {
            config,
            relayer_config,
            chains,
            processor,
            metrics,
            event_sender,
            sequence_trackers: HashMap::new(),
            processing_packets: HashMap::new(),
            completed_packets: VecDeque::new(),
            shutdown,
        }
    }

    /// Start the bidirectional relay manager
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔄 Starting bidirectional packet relay manager");
        
        let mut sequence_check_interval = time::interval(Duration::from_secs(self.config.sequence_check_interval));

        loop {
            tokio::select! {
                // Periodic sequence checking and cleanup
                _ = sequence_check_interval.tick() => {
                    if let Err(e) = self.check_sequences().await {
                        eprintln!("Error during sequence check: {}", e);
                    }
                }
                
                // Check for shutdown signal
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        println!("🛑 Bidirectional relay manager shutdown requested");
                        break;
                    }
                }
            }
        }

        println!("✅ Bidirectional relay manager stopped gracefully");
        Ok(())
    }

    /// Add a packet for bidirectional relay processing
    pub async fn add_packet(
        &mut self,
        packet: IbcPacket,
        source_chain_id: String,
        dest_chain_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Determine relay direction
        let direction = if source_chain_id.contains("near") && dest_chain_id.contains("cosmos") {
            RelayDirection::NearToCosmos
        } else if source_chain_id.contains("cosmos") && dest_chain_id.contains("near") {
            RelayDirection::CosmosToNear
        } else {
            return Err(format!("Unsupported relay direction: {} -> {}", source_chain_id, dest_chain_id).into());
        };

        let sequenced_packet = SequencedPacket::new(
            packet,
            direction.clone(),
            source_chain_id,
            dest_chain_id,
        );

        let channel_id = sequenced_packet.channel_id.clone();
        let tracker_key = (channel_id.clone(), direction.clone());

        // Get or create sequence tracker
        if !self.sequence_trackers.contains_key(&tracker_key) {
            let tracker = SequenceTracker::new(channel_id, direction, 1);
            self.sequence_trackers.insert(tracker_key.clone(), tracker);
        }

        // Process packet through sequence tracker
        let tracker = self.sequence_trackers.get_mut(&tracker_key).unwrap();
        let ready_packets = tracker.add_packet(sequenced_packet);

        // Process ready packets
        for ready_packet in ready_packets {
            self.process_packet(ready_packet).await?;
        }

        Ok(())
    }

    /// Process a packet that's ready for relay
    async fn process_packet(
        &mut self,
        mut packet: SequencedPacket,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let packet_key = packet.packet_key();
        
        // Check if we're already processing this packet
        if self.processing_packets.contains_key(&packet_key) {
            return Ok(());
        }

        // Check if we've reached the parallel processing limit
        if self.processing_packets.len() >= self.config.max_parallel_packets {
            println!("⚠️  Parallel processing limit reached, queuing packet seq={}", packet.packet.sequence);
            return Ok(());
        }

        packet.state = PacketRelayState::Processing;
        packet.last_processed = Some(Instant::now());
        packet.attempts += 1;

        println!("🔄 Processing bidirectional packet: seq={} {} -> {}", 
                 packet.packet.sequence, 
                 packet.direction.source_chain(), 
                 packet.direction.dest_chain());

        // Add to processing map
        self.processing_packets.insert(packet_key.clone(), packet.clone());

        // Process the packet based on direction
        let result = match packet.direction {
            RelayDirection::NearToCosmos => {
                self.processor.process_send_packet(
                    &packet.source_chain_id,
                    &packet.dest_chain_id,
                    &packet.packet,
                ).await
            }
            RelayDirection::CosmosToNear => {
                self.processor.process_recv_packet(
                    &packet.source_chain_id,
                    &packet.dest_chain_id,
                    &packet.packet,
                ).await
            }
        };

        // Update packet state based on result
        match result {
            Ok(tx_hash) => {
                packet.state = PacketRelayState::Relayed;
                
                println!("✅ Bidirectional packet relay successful: seq={} tx={}", 
                         packet.packet.sequence, tx_hash);

                // Send relay event
                let relay_event = RelayEvent::PacketRelayed {
                    source_chain: packet.source_chain_id.clone(),
                    dest_chain: packet.dest_chain_id.clone(),
                    sequence: packet.packet.sequence,
                };

                if let Err(e) = self.event_sender.send(relay_event).await {
                    eprintln!("Failed to send packet relayed event: {}", e);
                }

                // Update metrics
                self.metrics.total_packets_relayed.inc();

                // Move to completed
                self.completed_packets.push_back((packet_key.clone(), packet, Instant::now()));
            }
            Err(e) => {
                packet.state = PacketRelayState::Failed(e.to_string());
                
                println!("❌ Bidirectional packet relay failed: seq={} error={}", 
                         packet.packet.sequence, e);

                // Update metrics
                self.metrics.total_packets_failed.inc();

                // Move to completed as failed
                self.completed_packets.push_back((packet_key.clone(), packet, Instant::now()));
            }
        }

        // Remove from processing
        self.processing_packets.remove(&packet_key);

        Ok(())
    }

    /// Check sequences and handle out-of-order packets
    async fn check_sequences(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let max_wait = Duration::from_secs(self.config.max_out_of_order_wait);
        let mut expired_packets = Vec::new();

        // Check each sequence tracker for expired packets
        for tracker in self.sequence_trackers.values_mut() {
            let expired = tracker.get_expired_packets(max_wait);
            expired_packets.extend(expired);
        }

        // Process expired packets (potentially out of order if strict ordering is disabled)
        for mut expired_packet in expired_packets {
            if !self.config.strict_ordering {
                println!("🔄 Processing expired out-of-order packet: seq={}", expired_packet.packet.sequence);
                self.process_packet(expired_packet).await?;
            } else {
                println!("⚠️  Dropping expired out-of-order packet due to strict ordering: seq={}", 
                         expired_packet.packet.sequence);
                
                expired_packet.state = PacketRelayState::Failed("Expired out-of-order packet".to_string());
                let packet_key = expired_packet.packet_key();
                self.completed_packets.push_back((packet_key, expired_packet, Instant::now()));
                self.metrics.total_packets_failed.inc();
            }
        }

        // Clean up old completed packets
        self.cleanup_completed_packets();

        // Log statistics
        self.log_sequence_stats();

        Ok(())
    }

    /// Clean up old completed packets
    fn cleanup_completed_packets(&mut self) {
        const MAX_COMPLETED_PACKETS: usize = 10000;
        const MAX_AGE: Duration = Duration::from_secs(3600); // 1 hour

        let cutoff_time = Instant::now() - MAX_AGE;

        // Remove old packets
        while let Some((_, _, completed_at)) = self.completed_packets.front() {
            if *completed_at < cutoff_time || self.completed_packets.len() > MAX_COMPLETED_PACKETS {
                self.completed_packets.pop_front();
            } else {
                break;
            }
        }
    }

    /// Log sequence statistics
    fn log_sequence_stats(&self) {
        let mut total_pending = 0;
        let mut max_gap = 0;

        for tracker in self.sequence_trackers.values() {
            let stats = tracker.get_stats();
            total_pending += stats.pending_count;
            max_gap = max_gap.max(stats.sequence_gap);
        }

        if total_pending > 0 || max_gap > 0 {
            println!("📊 Sequence stats: {} pending packets, max gap: {}, {} trackers", 
                     total_pending, max_gap, self.sequence_trackers.len());
        }
    }

    /// Get comprehensive bidirectional relay statistics
    pub fn get_bidirectional_stats(&self) -> BidirectionalStats {
        let mut stats = BidirectionalStats::default();

        stats.total_trackers = self.sequence_trackers.len();
        stats.processing_packets = self.processing_packets.len();
        stats.completed_packets = self.completed_packets.len();

        // Aggregate sequence tracker stats
        for tracker in self.sequence_trackers.values() {
            let tracker_stats = tracker.get_stats();
            stats.total_pending += tracker_stats.pending_count;
            stats.max_sequence_gap = stats.max_sequence_gap.max(tracker_stats.sequence_gap);
        }

        // Count packets by state from completed packets
        for (_, packet, _) in &self.completed_packets {
            match packet.state {
                PacketRelayState::Relayed => stats.successful_relays += 1,
                PacketRelayState::Acknowledged => stats.acknowledged_packets += 1,
                PacketRelayState::TimedOut => stats.timed_out_packets += 1,
                PacketRelayState::Failed(_) => stats.failed_relays += 1,
                _ => {}
            }
        }

        // Separate stats by direction
        for tracker in self.sequence_trackers.values() {
            match tracker.direction {
                RelayDirection::NearToCosmos => {
                    stats.near_to_cosmos_packets += tracker.highest_processed;
                }
                RelayDirection::CosmosToNear => {
                    stats.cosmos_to_near_packets += tracker.highest_processed;
                }
            }
        }

        stats
    }

    /// Force process a specific packet (for testing/debugging)
    pub async fn force_process_packet(
        &mut self,
        source_chain_id: String,
        dest_chain_id: String,
        port: String,
        channel: String,
        sequence: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("🔧 Force processing bidirectional packet: {} -> {} {}:{} seq={}", 
                 source_chain_id, dest_chain_id, port, channel, sequence);

        // Create a mock packet
        let packet = IbcPacket {
            sequence,
            source_port: port,
            source_channel: channel,
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: b"force_test_packet".to_vec(),
            timeout_height: None,
            timeout_timestamp: None,
        };

        // Determine direction and process
        if source_chain_id.contains("near") && dest_chain_id.contains("cosmos") {
            self.processor.process_send_packet(&source_chain_id, &dest_chain_id, &packet).await
        } else if source_chain_id.contains("cosmos") && dest_chain_id.contains("near") {
            self.processor.process_recv_packet(&source_chain_id, &dest_chain_id, &packet).await
        } else {
            Err(format!("Unsupported force relay direction: {} -> {}", source_chain_id, dest_chain_id).into())
        }
    }
}

/// Statistics for sequence tracker
#[derive(Debug, Default)]
pub struct SequenceStats {
    pub channel_id: String,
    pub direction: RelayDirection,
    pub next_expected: u64,
    pub highest_processed: u64,
    pub pending_count: usize,
    pub processed_window_size: usize,
    pub sequence_gap: u64,
}

/// Statistics for bidirectional relay
#[derive(Debug, Default)]
pub struct BidirectionalStats {
    pub total_trackers: usize,
    pub processing_packets: usize,
    pub completed_packets: usize,
    pub total_pending: usize,
    pub max_sequence_gap: u64,
    pub successful_relays: usize,
    pub failed_relays: usize,
    pub acknowledged_packets: usize,
    pub timed_out_packets: usize,
    pub near_to_cosmos_packets: u64,
    pub cosmos_to_near_packets: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::IbcPacket;

    fn create_test_packet(sequence: u64, port: &str, channel: &str) -> IbcPacket {
        IbcPacket {
            sequence,
            source_port: port.to_string(),
            source_channel: channel.to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }
    }

    #[test]
    fn test_bidirectional_config_default() {
        let config = BidirectionalConfig::default();
        assert_eq!(config.max_parallel_packets, 10);
        assert_eq!(config.sequence_window_size, 1000);
        assert!(config.strict_ordering);
        assert_eq!(config.batch_size, 5);
    }

    #[test]
    fn test_relay_direction() {
        let near_to_cosmos = RelayDirection::NearToCosmos;
        assert_eq!(near_to_cosmos.source_chain(), "near");
        assert_eq!(near_to_cosmos.dest_chain(), "cosmos");
        assert_eq!(near_to_cosmos.reverse(), RelayDirection::CosmosToNear);

        let cosmos_to_near = RelayDirection::CosmosToNear;
        assert_eq!(cosmos_to_near.source_chain(), "cosmos");
        assert_eq!(cosmos_to_near.dest_chain(), "near");
        assert_eq!(cosmos_to_near.reverse(), RelayDirection::NearToCosmos);
    }

    #[test]
    fn test_sequenced_packet_creation() {
        let packet = create_test_packet(42, "transfer", "channel-0");
        let sequenced = SequencedPacket::new(
            packet,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        assert_eq!(sequenced.packet.sequence, 42);
        assert_eq!(sequenced.direction, RelayDirection::NearToCosmos);
        assert_eq!(sequenced.state, PacketRelayState::Detected);
        assert_eq!(sequenced.attempts, 0);
        assert_eq!(sequenced.channel_id, "transfer:channel-0");
    }

    #[test]
    fn test_sequence_tracker_in_order() {
        let mut tracker = SequenceTracker::new(
            "transfer:channel-0".to_string(),
            RelayDirection::NearToCosmos,
            1,
        );

        // Add packets in order
        for seq in 1..=5 {
            let packet = create_test_packet(seq, "transfer", "channel-0");
            let sequenced = SequencedPacket::new(
                packet,
                RelayDirection::NearToCosmos,
                "near-testnet".to_string(),
                "provider".to_string(),
            );

            let ready = tracker.add_packet(sequenced);
            assert_eq!(ready.len(), 1);
            assert_eq!(ready[0].packet.sequence, seq);
        }

        assert_eq!(tracker.next_expected, 6);
        assert_eq!(tracker.highest_processed, 5);
        assert!(tracker.pending_packets.is_empty());
    }

    #[test]
    fn test_sequence_tracker_out_of_order() {
        let mut tracker = SequenceTracker::new(
            "transfer:channel-0".to_string(),
            RelayDirection::NearToCosmos,
            1,
        );

        // Add packet 3 first (out of order)
        let packet3 = create_test_packet(3, "transfer", "channel-0");
        let sequenced3 = SequencedPacket::new(
            packet3,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        let ready = tracker.add_packet(sequenced3);
        assert!(ready.is_empty()); // Not ready yet
        assert_eq!(tracker.pending_packets.len(), 1);

        // Add packet 1 (expected)
        let packet1 = create_test_packet(1, "transfer", "channel-0");
        let sequenced1 = SequencedPacket::new(
            packet1,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        let ready = tracker.add_packet(sequenced1);
        assert_eq!(ready.len(), 1); // Only packet 1 is ready
        assert_eq!(ready[0].packet.sequence, 1);

        // Add packet 2 (should release packet 3 too)
        let packet2 = create_test_packet(2, "transfer", "channel-0");
        let sequenced2 = SequencedPacket::new(
            packet2,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        let ready = tracker.add_packet(sequenced2);
        assert_eq!(ready.len(), 2); // Packets 2 and 3 are ready
        assert_eq!(ready[0].packet.sequence, 2);
        assert_eq!(ready[1].packet.sequence, 3);

        assert_eq!(tracker.next_expected, 4);
        assert_eq!(tracker.highest_processed, 3);
        assert!(tracker.pending_packets.is_empty());
    }

    #[test]
    fn test_sequence_stats() {
        let mut tracker = SequenceTracker::new(
            "transfer:channel-0".to_string(),
            RelayDirection::NearToCosmos,
            1,
        );

        // Add some packets
        let packet1 = create_test_packet(1, "transfer", "channel-0");
        let sequenced1 = SequencedPacket::new(
            packet1,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );
        tracker.add_packet(sequenced1);

        let packet3 = create_test_packet(3, "transfer", "channel-0");
        let sequenced3 = SequencedPacket::new(
            packet3,
            RelayDirection::NearToCosmos,
            "near-testnet".to_string(),
            "provider".to_string(),
        );
        tracker.add_packet(sequenced3);

        let stats = tracker.get_stats();
        assert_eq!(stats.next_expected, 2);
        assert_eq!(stats.highest_processed, 1);
        assert_eq!(stats.pending_count, 1);
        assert_eq!(stats.sequence_gap, 1); // Gap between 2 and 3
    }

    #[test]
    fn test_packet_relay_state_transitions() {
        assert_eq!(PacketRelayState::Detected, PacketRelayState::Detected);
        assert_ne!(PacketRelayState::Detected, PacketRelayState::Processing);
        
        let failed_state = PacketRelayState::Failed("test error".to_string());
        if let PacketRelayState::Failed(msg) = failed_state {
            assert_eq!(msg, "test error");
        }
    }
}