// Enhanced relay engine with packet processing capabilities
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

use crate::chains::{Chain, IbcPacket};
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::{RelayEvent, PacketTracker, PendingPacket, PacketKey};
use super::processor::PacketProcessor;
use super::packet::{PacketLifecycle, PacketState};

/// Enhanced relay engine with full packet processing capabilities
pub struct RelayEngine {
    /// Chain implementations mapped by chain ID
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Packet processor for handling relay logic
    packet_processor: PacketProcessor,
    /// Tracks packet state across the relay process
    packet_tracker: PacketTracker,
    /// Enhanced packet lifecycle tracking
    packet_lifecycles: HashMap<PacketKey, PacketLifecycle>,
    /// Event receiver for relay events
    event_receiver: mpsc::Receiver<RelayEvent>,
    /// Event sender for testing and internal communication
    event_sender: mpsc::Sender<RelayEvent>,
    /// Configuration
    config: RelayerConfig,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl RelayEngine {
    /// Create a new enhanced relay engine
    pub fn new(
        config: RelayerConfig,
        chains: HashMap<String, Arc<dyn Chain>>,
        metrics: Arc<RelayerMetrics>,
    ) -> (Self, mpsc::Sender<RelayEvent>, tokio::sync::watch::Sender<bool>) {
        let (event_sender, event_receiver) = mpsc::channel(1000);
        let (shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
        
        let packet_processor = PacketProcessor::new(
            chains.clone(),
            config.clone(),
            metrics.clone(),
        );
        
        let event_sender_clone = event_sender.clone();
        
        let engine = Self {
            chains,
            packet_processor,
            packet_tracker: PacketTracker::new(),
            packet_lifecycles: HashMap::new(),
            event_receiver,
            event_sender,
            config,
            metrics,
            shutdown: shutdown_receiver,
        };
        
        (engine, event_sender_clone, shutdown_sender)
    }
    
    /// Main relay loop - processes events and pending packets
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting IBC Relay Engine...");
        
        let mut tick_interval = time::interval(Duration::from_secs(1));
        
        loop {
            tokio::select! {
                // Handle incoming relay events
                Some(event) = self.event_receiver.recv() => {
                    if let Err(e) = self.handle_relay_event(event).await {
                        eprintln!("Error handling relay event: {}", e);
                        // Continue processing even if one event fails
                    }
                }
                
                // Process pending packets periodically
                _ = tick_interval.tick() => {
                    if let Err(e) = self.process_pending_packets().await {
                        eprintln!("Error processing pending packets: {}", e);
                    }
                }
                
                // Check for shutdown signal
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        println!("🛑 Relay engine shutdown requested");
                        break;
                    }
                }
            }
        }
        
        println!("✅ Relay engine stopped gracefully");
        Ok(())
    }
    
    /// Handle a relay event
    async fn handle_relay_event(&mut self, event: RelayEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.metrics.total_events_processed.inc();
        
        match event {
            RelayEvent::PacketDetected { chain_id, packet, .. } => {
                println!("📦 Packet detected on {}: seq={}", chain_id, packet.sequence);
                self.queue_packet_for_relay(chain_id, packet).await?;
            }
            RelayEvent::PacketRelayed { source_chain, dest_chain, sequence } => {
                println!("✅ Packet relayed: {} -> {} seq={}", source_chain, dest_chain, sequence);
                self.handle_packet_relayed(source_chain, dest_chain, sequence).await?;
            }
            RelayEvent::PacketAcknowledged { chain_id, packet, ack_data } => {
                println!("🎯 Packet acknowledged on {}: seq={}", chain_id, packet.sequence);
                self.handle_packet_acknowledged(chain_id, packet, ack_data).await?;
            }
            RelayEvent::PacketTimedOut { chain_id, packet } => {
                println!("⏰ Packet timed out on {}: seq={}", chain_id, packet.sequence);
                self.handle_packet_timeout(chain_id, packet).await?;
            }
            RelayEvent::ChainDisconnected { chain_id } => {
                println!("🔌 Chain disconnected: {}", chain_id);
                self.handle_chain_disconnected(chain_id).await?;
            }
            RelayEvent::ChainReconnected { chain_id } => {
                println!("🔗 Chain reconnected: {}", chain_id);
                self.handle_chain_reconnected(chain_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Queue a packet for relay to destination chain
    async fn queue_packet_for_relay(
        &mut self,
        source_chain: String,
        packet: IbcPacket,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Determine destination chain based on packet routing
        let dest_chain = self.determine_destination_chain(&source_chain, &packet)?;
        
        // Create packet key for tracking
        let packet_key = PacketKey {
            source_chain: source_chain.clone(),
            source_port: packet.source_port.clone(),
            source_channel: packet.source_channel.clone(),
            sequence: packet.sequence,
        };
        
        // Check if packet is already being tracked
        if self.packet_lifecycles.contains_key(&packet_key) {
            println!("⚠️  Packet seq={} already being tracked, ignoring duplicate", packet.sequence);
            return Ok(());
        }
        
        // Create lifecycle tracker for this packet
        let lifecycle = PacketLifecycle::new(
            packet.clone(),
            source_chain.clone(),
            dest_chain.clone(),
        );
        
        // Store lifecycle tracker
        self.packet_lifecycles.insert(packet_key.clone(), lifecycle);
        
        // Legacy tracking for compatibility
        let pending_packet = PendingPacket {
            packet: packet.clone(),
            dest_chain: dest_chain.clone(),
            retry_count: 0,
            next_retry: None,
        };
        
        self.packet_tracker.pending_packets
            .entry(source_chain.clone())
            .or_insert_with(Vec::new)
            .push(pending_packet);
        
        self.metrics.total_packets_detected.inc();
        
        println!("📥 Queued packet for relay: {:?} -> {} (state: {:?})", 
                 packet_key, dest_chain, PacketState::Detected);
        Ok(())
    }
    
    /// Process all pending packets
    async fn process_pending_packets(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut processed_count = 0;
        let mut packets_to_process = Vec::new();
        
        // Collect packets that are ready for processing
        for (source_chain, packets) in &mut self.packet_tracker.pending_packets {
            let mut ready_indices = Vec::new();
            
            for (i, packet) in packets.iter().enumerate() {
                let should_process = packet.next_retry.map_or(true, |retry_time| 
                    std::time::Instant::now() >= retry_time
                );
                
                if should_process {
                    ready_indices.push(i);
                }
            }
            
            // Remove ready packets in reverse order to maintain indices
            for &i in ready_indices.iter().rev() {
                let packet = packets.remove(i);
                packets_to_process.push((source_chain.clone(), packet));
            }
        }
        
        // Remove empty vectors
        self.packet_tracker.pending_packets.retain(|_, packets| !packets.is_empty());
        
        // Process collected packets
        for (source_chain, packet) in packets_to_process {
            if let Err(e) = self.process_single_packet(source_chain, packet).await {
                eprintln!("Error processing packet: {}", e);
            }
            processed_count += 1;
        }
        
        if processed_count > 0 {
            println!("🔄 Processed {} pending packets", processed_count);
        }
        
        Ok(())
    }
    
    /// Process a single packet through the relay pipeline with enhanced lifecycle tracking
    async fn process_single_packet(
        &mut self,
        source_chain: String,
        mut pending_packet: PendingPacket,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let packet_key = PacketKey {
            source_chain: source_chain.clone(),
            source_port: pending_packet.packet.source_port.clone(),
            source_channel: pending_packet.packet.source_channel.clone(),
            sequence: pending_packet.packet.sequence,
        };
        
        // Get the lifecycle tracker
        if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
            // Update lifecycle state to proof generation
            if let Err(e) = lifecycle.transition_to(PacketState::ProofGenerated) {
                eprintln!("Failed to transition packet to ProofGenerated: {}", e);
            }
        }
        
        // Process packet with enhanced error handling and state tracking
        match self.packet_processor.process_packet(
            &source_chain,
            &pending_packet.dest_chain,
            &pending_packet.packet,
        ).await {
            Ok(tx_hash) => {
                // Successfully processed - update lifecycle
                if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
                    lifecycle.set_tx_hash(tx_hash);
                    if let Err(e) = lifecycle.transition_to(PacketState::Submitted) {
                        eprintln!("Failed to transition packet to Submitted: {}", e);
                    }
                    // Then transition to Confirmed (simplified for now)
                    if let Err(e) = lifecycle.transition_to(PacketState::Confirmed) {
                        eprintln!("Failed to transition packet to Confirmed: {}", e);
                    }
                    
                    println!("📊 Packet processing summary: {}", lifecycle.get_processing_summary());
                }
                
                // Store values before moving pending_packet
                let packet_seq = pending_packet.packet.sequence;
                let dest_chain = pending_packet.dest_chain.clone();
                
                // Move to awaiting acknowledgment
                self.packet_tracker.awaiting_ack.insert(packet_key, pending_packet);
                self.metrics.total_packets_relayed.inc();
                
                println!("✅ Packet seq={} successfully relayed from {} to {}", 
                         packet_seq, source_chain, dest_chain);
            }
            Err(e) => {
                // Failed - update lifecycle and retry logic
                pending_packet.retry_count += 1;
                let packet_seq = pending_packet.packet.sequence;
                
                if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
                    lifecycle.mark_failed(format!("Processing failed: {}", e));
                }
                
                if pending_packet.retry_count < self.config.global.max_retries {
                    // Schedule retry with exponential backoff
                    let retry_delay = Duration::from_millis(
                        self.config.global.retry_delay_ms * (1 << pending_packet.retry_count.min(5))
                    );
                    pending_packet.next_retry = Some(std::time::Instant::now() + retry_delay);
                    
                    // Reset lifecycle state for retry
                    if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
                        lifecycle.schedule_retry(retry_delay);
                    }
                    
                    println!("🔄 Scheduling retry #{} for packet seq={} in {:.1}s", 
                             pending_packet.retry_count, packet_seq, retry_delay.as_secs_f64());
                    
                    // Re-queue for retry
                    self.packet_tracker.pending_packets
                        .entry(source_chain)
                        .or_insert_with(Vec::new)
                        .push(pending_packet);
                } else {
                    // Give up - mark as permanently failed
                    if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
                        lifecycle.mark_failed(format!("Max retries exceeded: {}", e));
                    }
                    
                    eprintln!("❌ Giving up on packet seq={} after {} retries: {}", 
                             packet_seq, pending_packet.retry_count, e);
                    self.metrics.total_packets_failed.inc();
                }
            }
        }
        
        Ok(())
    }
    
    /// Determine destination chain for a packet
    fn determine_destination_chain(
        &self,
        source_chain: &str,
        _packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Simple logic: if source is NEAR, dest is Cosmos, and vice versa
        // In production, this would use channel routing information
        if source_chain.contains("near") {
            Ok("cosmoshub-testnet".to_string())
        } else {
            Ok("near-testnet".to_string())
        }
    }
    
    /// Handle packet successfully relayed
    async fn handle_packet_relayed(
        &mut self,
        _source_chain: String,
        _dest_chain: String,
        _sequence: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Update metrics and state
        self.metrics.total_packets_relayed.inc();
        Ok(())
    }
    
    /// Handle packet acknowledgment with enhanced lifecycle tracking
    async fn handle_packet_acknowledged(
        &mut self,
        chain_id: String,
        packet: IbcPacket,
        ack_data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let packet_key = PacketKey {
            source_chain: chain_id.clone(),
            source_port: packet.source_port,
            source_channel: packet.source_channel,
            sequence: packet.sequence,
        };
        
        // Update lifecycle to acknowledged state
        if let Some(lifecycle) = self.packet_lifecycles.get_mut(&packet_key) {
            if let Err(e) = lifecycle.transition_to(PacketState::Acknowledged) {
                eprintln!("Failed to transition packet to Acknowledged: {}", e);
            } else {
                println!("🎯 Packet lifecycle complete: {}", lifecycle.get_processing_summary());
            }
        }
        
        // Remove from awaiting acknowledgment and add to completed
        if self.packet_tracker.awaiting_ack.remove(&packet_key).is_some() {
            self.packet_tracker.completed_packets.push(packet_key.clone());
            self.metrics.total_packets_acknowledged.inc();
            
            println!("✅ Packet seq={} acknowledged on chain {} (ack_data: {} bytes)", 
                     packet.sequence, chain_id, ack_data.len());
        } else {
            eprintln!("⚠️  Received acknowledgment for unknown packet seq={}", packet.sequence);
        }
        
        Ok(())
    }
    
    /// Handle packet timeout
    async fn handle_packet_timeout(
        &mut self,
        _chain_id: String,
        packet: IbcPacket,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Handle timeout - could trigger refund on source chain
        self.metrics.total_packets_timed_out.inc();
        println!("⏰ Handling timeout for packet seq={}", packet.sequence);
        Ok(())
    }
    
    /// Handle chain disconnection
    async fn handle_chain_disconnected(
        &mut self,
        chain_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔌 Handling disconnection for chain: {}", chain_id);
        // Could pause packet processing for this chain
        Ok(())
    }
    
    /// Handle chain reconnection
    async fn handle_chain_reconnected(
        &mut self,
        chain_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔗 Handling reconnection for chain: {}", chain_id);
        // Could resume packet processing for this chain
        Ok(())
    }
    
    /// Get current packet tracker state (for monitoring)
    pub fn get_packet_tracker(&self) -> &PacketTracker {
        &self.packet_tracker
    }
    
    /// Send an event to the relay engine (for testing)
    pub async fn send_event(&self, event: RelayEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.event_sender.send(event).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    /// Get packet lifecycle by key for monitoring
    pub fn get_packet_lifecycle(&self, packet_key: &PacketKey) -> Option<&PacketLifecycle> {
        self.packet_lifecycles.get(packet_key)
    }
    
    /// Get all packet lifecycles for monitoring
    pub fn get_all_packet_lifecycles(&self) -> &HashMap<PacketKey, PacketLifecycle> {
        &self.packet_lifecycles
    }
    
    /// Get packets by state for monitoring
    pub fn get_packets_by_state(&self, state: &PacketState) -> Vec<(&PacketKey, &PacketLifecycle)> {
        self.packet_lifecycles
            .iter()
            .filter(|(_, lifecycle)| &lifecycle.state == state)
            .collect()
    }
    
    /// Clean up completed packet lifecycles (retention policy)
    pub fn cleanup_completed_packets(&mut self, max_completed: usize) {
        let mut completed_keys: Vec<_> = self.packet_lifecycles
            .iter()
            .filter(|(_, lifecycle)| lifecycle.is_terminal())
            .map(|(key, _)| key.clone())
            .collect();
        
        if completed_keys.len() > max_completed {
            // Sort by completion time (latest first) and remove oldest
            completed_keys.sort_by_key(|key| {
                self.packet_lifecycles[key].last_updated
            });
            
            let to_remove = completed_keys.len() - max_completed;
            for key in completed_keys.iter().take(to_remove) {
                self.packet_lifecycles.remove(key);
                println!("🧹 Cleaned up completed packet lifecycle: {:?}", key);
            }
        }
    }
    
    /// Get relay engine statistics
    pub fn get_relay_stats(&self) -> RelayStats {
        let mut stats = RelayStats::default();
        
        for lifecycle in self.packet_lifecycles.values() {
            match lifecycle.state {
                PacketState::Detected => stats.detected += 1,
                PacketState::ProofGenerated => stats.proof_generated += 1,
                PacketState::Submitted => stats.submitted += 1,
                PacketState::Confirmed => stats.confirmed += 1,
                PacketState::Acknowledged => stats.acknowledged += 1,
                PacketState::TimedOut => stats.timed_out += 1,
                PacketState::Failed(_) => stats.failed += 1,
            }
            
            if lifecycle.retry_count > 0 {
                stats.retried += 1;
            }
        }
        
        stats.total_tracked = self.packet_lifecycles.len();
        stats.pending_in_tracker = self.packet_tracker.pending_packets.values()
            .map(|v| v.len())
            .sum();
        stats.awaiting_ack = self.packet_tracker.awaiting_ack.len();
        
        stats
    }
    
    /// Automatically determine routing based on channel configuration  
    /// This is a more sophisticated version that could use actual channel info
    fn determine_destination_chain_advanced(
        &self,
        source_chain: &str,
        packet: &IbcPacket,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // In a full implementation, this would:
        // 1. Look up the channel from the packet's source_channel
        // 2. Get the counterparty channel and connection
        // 3. Determine the destination chain from the connection
        
        // For now, use the simple logic but log the routing decision
        let dest_chain = if source_chain.contains("near") {
            "cosmoshub-testnet"
        } else {
            "near-testnet"
        };
        
        println!("🗺️  Routing packet seq={} from {} port={} channel={} -> {}", 
                 packet.sequence, source_chain, packet.source_port, packet.source_channel, dest_chain);
        
        Ok(dest_chain.to_string())
    }
}

/// Statistics for relay engine performance monitoring
#[derive(Debug, Default)]
pub struct RelayStats {
    pub total_tracked: usize,
    pub detected: usize,
    pub proof_generated: usize,
    pub submitted: usize,
    pub confirmed: usize,
    pub acknowledged: usize,
    pub timed_out: usize,
    pub failed: usize,
    pub retried: usize,
    pub pending_in_tracker: usize,
    pub awaiting_ack: usize,
}