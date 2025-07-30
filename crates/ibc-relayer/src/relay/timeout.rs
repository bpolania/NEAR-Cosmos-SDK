// Timeout detection and cleanup mechanisms for failed packet relay operations
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;

use crate::chains::{Chain, IbcPacket};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::{RelayEvent, PacketKey};
use super::processor::PacketProcessor;

/// Configuration for timeout detection
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// How often to check for timeouts (seconds)
    pub check_interval: u64,
    /// Grace period after packet timeout before cleanup (seconds)
    pub grace_period: u64,
    /// Maximum number of cleanup retries
    pub max_cleanup_retries: u32,
    /// Delay between cleanup retries (milliseconds)
    pub cleanup_retry_delay_ms: u64,
    /// Maximum age of completed timeout records to keep (hours)
    pub max_completed_age_hours: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            check_interval: 30,      // Check every 30 seconds
            grace_period: 300,       // 5 minute grace period
            max_cleanup_retries: 3,
            cleanup_retry_delay_ms: 5000, // 5 seconds between retries
            max_completed_age_hours: 24,   // Keep completed records for 24 hours
        }
    }
}

/// Status of a packet timeout operation
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutStatus {
    /// Packet is still within timeout period
    Active,
    /// Packet has timed out but within grace period
    Expired,
    /// Cleanup in progress
    CleaningUp,
    /// Cleanup completed successfully
    Completed,
    /// Cleanup failed after retries
    Failed(String),
}

/// Tracked timeout information for a packet
#[derive(Debug, Clone)]
pub struct TimeoutTracker {
    /// The packet being tracked
    pub packet: IbcPacket,
    /// Source chain ID
    pub source_chain: String,
    /// Destination chain ID
    pub dest_chain: String,
    /// When the packet was first detected
    pub detected_at: Instant,
    /// Current timeout status
    pub status: TimeoutStatus,
    /// Number of cleanup attempts made
    pub cleanup_attempts: u32,
    /// Last cleanup attempt time
    pub last_cleanup_attempt: Option<Instant>,
    /// Error message if cleanup failed
    pub error_message: Option<String>,
}

impl TimeoutTracker {
    pub fn new(packet: IbcPacket, source_chain: String, dest_chain: String) -> Self {
        Self {
            packet,
            source_chain,
            dest_chain,
            detected_at: Instant::now(),
            status: TimeoutStatus::Active,
            cleanup_attempts: 0,
            last_cleanup_attempt: None,
            error_message: None,
        }
    }

    /// Check if packet has exceeded its timeout
    pub fn is_expired(&self) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Check timeout height if specified
        if let Some(timeout_height) = self.packet.timeout_height {
            // Would need to check current chain height - simplified for now
            println!("📋 Checking timeout height: {} (current check simplified)", timeout_height);
        }

        // Check timeout timestamp if specified
        if let Some(timeout_timestamp) = self.packet.timeout_timestamp {
            if current_time >= timeout_timestamp {
                return true;
            }
        }

        false
    }

    /// Check if packet is past grace period and needs cleanup
    pub fn needs_cleanup(&self, grace_period: Duration) -> bool {
        self.is_expired() && self.detected_at.elapsed() >= grace_period
    }
}

/// Timeout detection and cleanup manager
pub struct TimeoutManager {
    /// Configuration
    config: TimeoutConfig,
    /// Relayer configuration
    relayer_config: RelayerConfig,
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Packet processor for cleanup operations
    processor: PacketProcessor,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
    /// Event sender for timeout events
    event_sender: mpsc::Sender<RelayEvent>,
    /// Tracked timeouts (packet_key -> tracker)
    tracked_timeouts: HashMap<PacketKey, TimeoutTracker>,
    /// Completed timeouts for history tracking
    completed_timeouts: VecDeque<(PacketKey, TimeoutTracker, Instant)>,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl TimeoutManager {
    /// Create a new timeout manager
    pub fn new(
        config: TimeoutConfig,
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
            tracked_timeouts: HashMap::new(),
            completed_timeouts: VecDeque::new(),
            shutdown,
        }
    }

    /// Start the timeout detection and cleanup loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("⏰ Starting timeout detection manager");
        
        let mut check_interval = time::interval(Duration::from_secs(self.config.check_interval));

        loop {
            tokio::select! {
                // Perform periodic timeout checks
                _ = check_interval.tick() => {
                    if let Err(e) = self.check_timeouts().await {
                        eprintln!("Error during timeout check: {}", e);
                    }
                }
                
                // Check for shutdown signal
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        println!("🛑 Timeout manager shutdown requested");
                        break;
                    }
                }
            }
        }

        println!("✅ Timeout manager stopped gracefully");
        Ok(())
    }

    /// Add a packet to timeout tracking
    pub fn track_packet(&mut self, packet: IbcPacket, source_chain: String, dest_chain: String) {
        let packet_key = PacketKey {
            source_chain: source_chain.clone(),
            source_port: packet.source_port.clone(),
            source_channel: packet.source_channel.clone(),
            sequence: packet.sequence,
        };

        let tracker = TimeoutTracker::new(packet, source_chain, dest_chain);
        
        println!("📋 Tracking packet timeout: seq={} from {} to {}", 
                 tracker.packet.sequence, tracker.source_chain, tracker.dest_chain);
        
        self.tracked_timeouts.insert(packet_key, tracker);
    }

    /// Remove a packet from timeout tracking (when successfully acknowledged)
    pub fn untrack_packet(&mut self, packet_key: &PacketKey) -> bool {
        if let Some(tracker) = self.tracked_timeouts.remove(packet_key) {
            println!("✅ Removed packet from timeout tracking: seq={}", tracker.packet.sequence);
            true
        } else {
            false
        }
    }

    /// Check all tracked packets for timeouts and trigger cleanup
    async fn check_timeouts(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut expired_packets = Vec::new();
        let mut cleanup_needed = Vec::new();
        let grace_period = Duration::from_secs(self.config.grace_period);

        // Check each tracked packet
        for (packet_key, tracker) in &mut self.tracked_timeouts {
            match tracker.status {
                TimeoutStatus::Active => {
                    if tracker.is_expired() {
                        tracker.status = TimeoutStatus::Expired;
                        expired_packets.push((packet_key.clone(), tracker.clone()));
                        println!("⏰ Packet expired: seq={} from {} to {}", 
                                 tracker.packet.sequence, tracker.source_chain, tracker.dest_chain);
                    }
                }
                TimeoutStatus::Expired => {
                    if tracker.needs_cleanup(grace_period) {
                        tracker.status = TimeoutStatus::CleaningUp;
                        cleanup_needed.push((packet_key.clone(), tracker.clone()));
                        println!("🧹 Packet needs cleanup: seq={} (expired {} ago)", 
                                 tracker.packet.sequence, tracker.detected_at.elapsed().as_secs());
                    }
                }
                TimeoutStatus::CleaningUp => {
                    // Check if we should retry cleanup
                    if let Some(last_attempt) = tracker.last_cleanup_attempt {
                        let retry_delay = Duration::from_millis(self.config.cleanup_retry_delay_ms);
                        if last_attempt.elapsed() >= retry_delay && 
                           tracker.cleanup_attempts < self.config.max_cleanup_retries {
                            cleanup_needed.push((packet_key.clone(), tracker.clone()));
                        }
                    }
                }
                TimeoutStatus::Completed | TimeoutStatus::Failed(_) => {
                    // These are handled by cleanup_completed_timeouts
                }
            }
        }

        // Send timeout events for expired packets
        for (_packet_key, tracker) in expired_packets {
            let timeout_event = RelayEvent::PacketTimedOut {
                chain_id: tracker.dest_chain.clone(),
                packet: tracker.packet.clone(),
            };

            if let Err(e) = self.event_sender.send(timeout_event).await {
                eprintln!("Failed to send timeout event for packet seq={}: {}", 
                          tracker.packet.sequence, e);
            }
        }

        // Perform cleanup operations
        for (packet_key, tracker) in cleanup_needed {
            self.cleanup_packet(&packet_key, tracker).await;
        }

        // Clean up old completed records
        self.cleanup_completed_timeouts();

        // Update metrics
        let active_count = self.tracked_timeouts.len();
        let expired_count = self.tracked_timeouts.values()
            .filter(|t| matches!(t.status, TimeoutStatus::Expired | TimeoutStatus::CleaningUp))
            .count();

        println!("📊 Timeout check complete: {} active, {} expired, {} completed", 
                 active_count, expired_count, self.completed_timeouts.len());

        Ok(())
    }

    /// Perform cleanup for a timed-out packet
    async fn cleanup_packet(&mut self, packet_key: &PacketKey, mut tracker: TimeoutTracker) {
        tracker.cleanup_attempts += 1;
        tracker.last_cleanup_attempt = Some(Instant::now());

        println!("🧹 Attempting cleanup for packet seq={} (attempt {}/{})", 
                 tracker.packet.sequence, tracker.cleanup_attempts, self.config.max_cleanup_retries);

        // Attempt to process the timeout
        match self.processor.process_timeout(
            &tracker.source_chain,
            &tracker.dest_chain,
            &tracker.packet,
        ).await {
            Ok(tx_hash) => {
                tracker.status = TimeoutStatus::Completed;
                tracker.error_message = None;
                
                println!("✅ Timeout cleanup successful for packet seq={}: {}", 
                         tracker.packet.sequence, tx_hash);

                // Move to completed list
                self.completed_timeouts.push_back((
                    packet_key.clone(),
                    tracker.clone(),
                    Instant::now(),
                ));
                self.tracked_timeouts.remove(packet_key);

                // Update metrics
                self.metrics.total_packets_timed_out.inc();
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracker.error_message = Some(error_msg.clone());

                if tracker.cleanup_attempts >= self.config.max_cleanup_retries {
                    tracker.status = TimeoutStatus::Failed(error_msg.clone());
                    
                    println!("❌ Timeout cleanup failed permanently for packet seq={}: {}", 
                             tracker.packet.sequence, error_msg);

                    // Move to completed list as failed
                    self.completed_timeouts.push_back((
                        packet_key.clone(),
                        tracker.clone(),
                        Instant::now(),
                    ));
                    self.tracked_timeouts.remove(packet_key);

                    // Update metrics
                    self.metrics.total_packets_failed.inc();
                } else {
                    tracker.status = TimeoutStatus::CleaningUp; // Will retry later
                    
                    println!("⚠️  Timeout cleanup failed for packet seq={}, will retry: {}", 
                             tracker.packet.sequence, error_msg);
                }

                // Update the tracker in the map
                self.tracked_timeouts.insert(packet_key.clone(), tracker);
            }
        }
    }

    /// Clean up old completed timeout records
    fn cleanup_completed_timeouts(&mut self) {
        let max_age = Duration::from_secs(self.config.max_completed_age_hours * 3600);
        let cutoff_time = Instant::now() - max_age;

        let original_count = self.completed_timeouts.len();
        
        // Remove old completed records
        while let Some((_, _, completed_at)) = self.completed_timeouts.front() {
            if *completed_at < cutoff_time {
                self.completed_timeouts.pop_front();
            } else {
                break;
            }
        }

        let removed_count = original_count - self.completed_timeouts.len();
        if removed_count > 0 {
            println!("🧹 Cleaned up {} old timeout records", removed_count);
        }
    }

    /// Get timeout statistics for monitoring
    pub fn get_timeout_stats(&self) -> TimeoutStats {
        let mut stats = TimeoutStats::default();

        stats.total_tracked = self.tracked_timeouts.len();
        stats.total_completed = self.completed_timeouts.len();

        for tracker in self.tracked_timeouts.values() {
            match tracker.status {
                TimeoutStatus::Active => stats.active += 1,
                TimeoutStatus::Expired => stats.expired += 1,
                TimeoutStatus::CleaningUp => stats.cleaning_up += 1,
                TimeoutStatus::Completed => stats.completed += 1,
                TimeoutStatus::Failed(_) => stats.failed += 1,
            }

            if tracker.cleanup_attempts > 0 {
                stats.cleanup_attempts += tracker.cleanup_attempts as usize;
            }
        }

        // Add completed records stats
        for (_, tracker, _) in &self.completed_timeouts {
            match tracker.status {
                TimeoutStatus::Completed => stats.completed += 1,
                TimeoutStatus::Failed(_) => stats.failed += 1,
                _ => {}
            }
        }

        stats
    }

    /// Force cleanup of a specific packet (for testing/debugging)
    pub async fn force_cleanup_packet(
        &mut self,
        packet_key: &PacketKey,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tracker) = self.tracked_timeouts.get(packet_key).cloned() {
            println!("🔧 Force cleanup requested for packet seq={}", tracker.packet.sequence);
            
            let result = self.processor.process_timeout(
                &tracker.source_chain,
                &tracker.dest_chain,
                &tracker.packet,
            ).await;

            match &result {
                Ok(tx_hash) => {
                    // Remove from tracking and add to completed
                    if let Some(mut tracker) = self.tracked_timeouts.remove(packet_key) {
                        tracker.status = TimeoutStatus::Completed;
                        self.completed_timeouts.push_back((
                            packet_key.clone(),
                            tracker,
                            Instant::now(),
                        ));
                    }
                    println!("✅ Force cleanup successful: {}", tx_hash);
                }
                Err(e) => {
                    println!("❌ Force cleanup failed: {}", e);
                }
            }

            result
        } else {
            Err("Packet not found in timeout tracking".into())
        }
    }
}

/// Statistics for timeout manager monitoring
#[derive(Debug, Default)]
pub struct TimeoutStats {
    pub total_tracked: usize,
    pub total_completed: usize,
    pub active: usize,
    pub expired: usize,
    pub cleaning_up: usize,
    pub completed: usize,
    pub failed: usize,
    pub cleanup_attempts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::IbcPacket;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_packet_with_timeout(sequence: u64, timeout_timestamp: Option<u64>) -> IbcPacket {
        IbcPacket {
            sequence,
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000000),
            timeout_timestamp,
        }
    }

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.check_interval, 30);
        assert_eq!(config.grace_period, 300);
        assert_eq!(config.max_cleanup_retries, 3);
    }

    #[test]
    fn test_timeout_tracker_creation() {
        let packet = create_test_packet_with_timeout(1, None);
        let tracker = TimeoutTracker::new(
            packet,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        assert_eq!(tracker.packet.sequence, 1);
        assert_eq!(tracker.source_chain, "near-testnet");
        assert_eq!(tracker.dest_chain, "provider");
        assert_eq!(tracker.status, TimeoutStatus::Active);
        assert_eq!(tracker.cleanup_attempts, 0);
    }

    #[test]
    fn test_timeout_detection() {
        // Create packet with past timeout
        let past_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64 - 1000000000; // 1 second ago

        let packet = create_test_packet_with_timeout(1, Some(past_time));
        let tracker = TimeoutTracker::new(
            packet,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        assert!(tracker.is_expired());
    }

    #[test]
    fn test_timeout_status_transitions() {
        let packet = create_test_packet_with_timeout(1, None);
        let mut tracker = TimeoutTracker::new(
            packet,
            "near-testnet".to_string(),
            "provider".to_string(),
        );

        // Test status transitions
        assert_eq!(tracker.status, TimeoutStatus::Active);

        tracker.status = TimeoutStatus::Expired;
        assert_eq!(tracker.status, TimeoutStatus::Expired);

        tracker.status = TimeoutStatus::CleaningUp;
        assert_eq!(tracker.status, TimeoutStatus::CleaningUp);

        tracker.status = TimeoutStatus::Completed;
        assert_eq!(tracker.status, TimeoutStatus::Completed);
    }

    #[test]
    fn test_timeout_stats() {
        let stats = TimeoutStats {
            total_tracked: 10,
            active: 5,
            expired: 3,
            cleaning_up: 1,
            completed: 8,
            failed: 2,
            cleanup_attempts: 15,
            total_completed: 10,
        };

        assert_eq!(stats.total_tracked, 10);
        assert_eq!(stats.active, 5);
        assert_eq!(stats.cleanup_attempts, 15);
    }
}