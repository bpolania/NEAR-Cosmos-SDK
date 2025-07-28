// Packet lifecycle management for IBC relay
use std::time::{Duration, Instant};
use crate::chains::IbcPacket;

/// States a packet can be in during the relay process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketState {
    /// Packet detected on source chain
    Detected,
    /// Proof generated for packet
    ProofGenerated,
    /// Transaction submitted to destination chain
    Submitted,
    /// Transaction confirmed on destination chain
    Confirmed,
    /// Acknowledgment received
    Acknowledged,
    /// Packet timed out
    TimedOut,
    /// Packet processing failed
    Failed(String),
}

/// Manages the complete lifecycle of a packet through the relay process
#[derive(Debug, Clone)]
pub struct PacketLifecycle {
    /// The IBC packet being relayed
    pub packet: IbcPacket,
    /// Current state of the packet
    pub state: PacketState,
    /// Source chain ID
    pub source_chain: String,
    /// Destination chain ID
    pub dest_chain: String,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Timestamp when packet was first detected
    pub detected_at: Instant,
    /// Timestamp of last state transition
    pub last_updated: Instant,
    /// Next retry time (if applicable)
    pub next_retry: Option<Instant>,
    /// Additional metadata
    pub metadata: PacketMetadata,
}

/// Additional metadata for packet tracking
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    /// Transaction hash when submitted (if available)
    pub tx_hash: Option<String>,
    /// Proof data (cached for retries)
    pub proof_data: Option<Vec<u8>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Processing duration tracking
    pub processing_times: ProcessingTimes,
}

/// Track timing information for performance monitoring
#[derive(Debug, Clone)]
pub struct ProcessingTimes {
    /// Time to generate proof
    pub proof_generation: Option<Duration>,
    /// Time to submit transaction
    pub transaction_submission: Option<Duration>,
    /// Time to confirm transaction
    pub confirmation_time: Option<Duration>,
    /// Total relay time
    pub total_relay_time: Option<Duration>,
}

impl PacketLifecycle {
    /// Create a new packet lifecycle tracker
    pub fn new(
        packet: IbcPacket,
        source_chain: String,
        dest_chain: String,
    ) -> Self {
        let now = Instant::now();
        
        Self {
            packet,
            state: PacketState::Detected,
            source_chain,
            dest_chain,
            retry_count: 0,
            detected_at: now,
            last_updated: now,
            next_retry: None,
            metadata: PacketMetadata {
                tx_hash: None,
                proof_data: None,
                error_message: None,
                processing_times: ProcessingTimes {
                    proof_generation: None,
                    transaction_submission: None,
                    confirmation_time: None,
                    total_relay_time: None,
                },
            },
        }
    }
    
    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: PacketState) -> Result<(), String> {
        if !self.is_valid_transition(&self.state, &new_state) {
            return Err(format!(
                "Invalid state transition from {:?} to {:?}",
                self.state, new_state
            ));
        }
        
        let now = Instant::now();
        
        // Update processing times based on state transition
        match (&self.state, &new_state) {
            (PacketState::Detected, PacketState::ProofGenerated) => {
                self.metadata.processing_times.proof_generation = 
                    Some(now.duration_since(self.last_updated));
            }
            (PacketState::ProofGenerated, PacketState::Submitted) => {
                self.metadata.processing_times.transaction_submission = 
                    Some(now.duration_since(self.last_updated));
            }
            (PacketState::Submitted, PacketState::Confirmed) => {
                self.metadata.processing_times.confirmation_time = 
                    Some(now.duration_since(self.last_updated));
            }
            (_, PacketState::Acknowledged) => {
                self.metadata.processing_times.total_relay_time = 
                    Some(now.duration_since(self.detected_at));
            }
            _ => {}
        }
        
        self.state = new_state;
        self.last_updated = now;
        
        Ok(())
    }
    
    /// Check if a state transition is valid
    fn is_valid_transition(&self, from: &PacketState, to: &PacketState) -> bool {
        use PacketState::*;
        
        match (from, to) {
            // Normal flow
            (Detected, ProofGenerated) => true,
            (ProofGenerated, Submitted) => true,
            (Submitted, Confirmed) => true,
            (Confirmed, Acknowledged) => true,
            
            // Error transitions
            (Detected, Failed(_)) => true,
            (ProofGenerated, Failed(_)) => true,
            (Submitted, Failed(_)) => true,
            (Confirmed, Failed(_)) => true,
            
            // Timeout transitions
            (Detected, TimedOut) => true,
            (ProofGenerated, TimedOut) => true,
            (Submitted, TimedOut) => true,
            (Confirmed, TimedOut) => true,
            
            // Retry transitions (back to earlier states)
            (Failed(_), Detected) => true,
            (Failed(_), ProofGenerated) => true,
            
            // Same state (no change)
            (a, b) if a == b => true,
            
            // Invalid transitions
            _ => false,
        }
    }
    
    /// Schedule next retry
    pub fn schedule_retry(&mut self, retry_delay: Duration) {
        self.retry_count += 1;
        self.next_retry = Some(Instant::now() + retry_delay);
        
        // Reset to detected state for retry
        if let Err(e) = self.transition_to(PacketState::Detected) {
            eprintln!("Error scheduling retry: {}", e);
        }
    }
    
    /// Check if packet is ready for retry
    pub fn is_ready_for_retry(&self) -> bool {
        self.next_retry.map_or(false, |retry_time| Instant::now() >= retry_time)
    }
    
    /// Mark packet as failed with error message
    pub fn mark_failed(&mut self, error: String) {
        self.metadata.error_message = Some(error.clone());
        if let Err(e) = self.transition_to(PacketState::Failed(error)) {
            eprintln!("Error marking packet as failed: {}", e);
        }
    }
    
    /// Set transaction hash
    pub fn set_tx_hash(&mut self, tx_hash: String) {
        self.metadata.tx_hash = Some(tx_hash);
    }
    
    /// Set proof data for caching
    pub fn set_proof_data(&mut self, proof_data: Vec<u8>) {
        self.metadata.proof_data = Some(proof_data);
    }
    
    /// Get cached proof data
    pub fn get_proof_data(&self) -> Option<&[u8]> {
        self.metadata.proof_data.as_deref()
    }
    
    /// Check if packet has timed out
    pub fn is_timed_out(&self, timeout_duration: Duration) -> bool {
        self.detected_at.elapsed() > timeout_duration
    }
    
    /// Get processing summary for logging/metrics
    pub fn get_processing_summary(&self) -> String {
        let total_time = self.last_updated.duration_since(self.detected_at);
        
        format!(
            "Packet seq={} state={:?} retries={} time={:.2}s",
            self.packet.sequence,
            self.state,
            self.retry_count,
            total_time.as_secs_f64()
        )
    }
    
    /// Check if packet is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self.state, 
            PacketState::Acknowledged | 
            PacketState::TimedOut | 
            PacketState::Failed(_)
        )
    }
    
    /// Check if packet is actively being processed
    pub fn is_processing(&self) -> bool {
        matches!(self.state,
            PacketState::Detected |
            PacketState::ProofGenerated |
            PacketState::Submitted |
            PacketState::Confirmed
        )
    }
}

impl Default for ProcessingTimes {
    fn default() -> Self {
        Self {
            proof_generation: None,
            transaction_submission: None,
            confirmation_time: None,
            total_relay_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::IbcPacket;
    
    fn create_test_packet() -> IbcPacket {
        IbcPacket {
            sequence: 1,
            source_port: "transfer".to_string(),
            source_channel: "channel-0".to_string(),
            destination_port: "transfer".to_string(),
            destination_channel: "channel-1".to_string(),
            data: vec![1, 2, 3],
            timeout_height: Some(1000),
            timeout_timestamp: Some(1234567890),
        }
    }
    
    #[test]
    fn test_packet_lifecycle_creation() {
        let packet = create_test_packet();
        let lifecycle = PacketLifecycle::new(
            packet.clone(),
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        assert_eq!(lifecycle.packet.sequence, 1);
        assert_eq!(lifecycle.state, PacketState::Detected);
        assert_eq!(lifecycle.retry_count, 0);
        assert_eq!(lifecycle.source_chain, "near-testnet");
        assert_eq!(lifecycle.dest_chain, "cosmoshub-testnet");
    }
    
    #[test]
    fn test_valid_state_transitions() {
        let packet = create_test_packet();
        let mut lifecycle = PacketLifecycle::new(
            packet,
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        // Normal flow
        assert!(lifecycle.transition_to(PacketState::ProofGenerated).is_ok());
        assert!(lifecycle.transition_to(PacketState::Submitted).is_ok());
        assert!(lifecycle.transition_to(PacketState::Confirmed).is_ok());
        assert!(lifecycle.transition_to(PacketState::Acknowledged).is_ok());
    }
    
    #[test]
    fn test_invalid_state_transitions() {
        let packet = create_test_packet();
        let mut lifecycle = PacketLifecycle::new(
            packet,
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        // Invalid transitions
        assert!(lifecycle.transition_to(PacketState::Acknowledged).is_err());
        assert!(lifecycle.transition_to(PacketState::Confirmed).is_err());
    }
    
    #[test]
    fn test_retry_scheduling() {
        let packet = create_test_packet();
        let mut lifecycle = PacketLifecycle::new(
            packet,
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        lifecycle.schedule_retry(Duration::from_secs(5));
        
        assert_eq!(lifecycle.retry_count, 1);
        assert!(lifecycle.next_retry.is_some());
        assert_eq!(lifecycle.state, PacketState::Detected);
    }
    
    #[test]
    fn test_packet_metadata() {
        let packet = create_test_packet();
        let mut lifecycle = PacketLifecycle::new(
            packet,
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        lifecycle.set_tx_hash("abc123".to_string());
        lifecycle.set_proof_data(vec![1, 2, 3, 4]);
        lifecycle.mark_failed("Network error".to_string());
        
        assert_eq!(lifecycle.metadata.tx_hash, Some("abc123".to_string()));
        assert_eq!(lifecycle.get_proof_data(), Some([1, 2, 3, 4].as_slice()));
        assert_eq!(lifecycle.state, PacketState::Failed("Network error".to_string()));
    }
    
    #[test]
    fn test_terminal_states() {
        let packet = create_test_packet();
        let mut lifecycle = PacketLifecycle::new(
            packet,
            "near-testnet".to_string(),
            "cosmoshub-testnet".to_string(),
        );
        
        assert!(!lifecycle.is_terminal());
        assert!(lifecycle.is_processing());
        
        // Follow the normal flow to reach Acknowledged state
        lifecycle.transition_to(PacketState::ProofGenerated).unwrap();
        lifecycle.transition_to(PacketState::Submitted).unwrap();
        lifecycle.transition_to(PacketState::Confirmed).unwrap();
        lifecycle.transition_to(PacketState::Acknowledged).unwrap();
        
        assert!(lifecycle.is_terminal());
        assert!(!lifecycle.is_processing());
    }
}