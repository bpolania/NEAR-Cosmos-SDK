// Metrics and monitoring
#![allow(dead_code)]

use prometheus::{Counter, Histogram, Registry};
use std::sync::Arc;

/// Relayer metrics - comprehensive packet tracking
pub struct RelayerMetrics {
    // Event metrics
    pub total_events_processed: Counter,
    
    // Packet metrics
    pub total_packets_detected: Counter,
    pub total_packets_relayed: Counter,
    pub total_packets_acknowledged: Counter,
    pub total_packets_timed_out: Counter,
    pub total_packets_failed: Counter,
    pub packet_relay_duration: Histogram,
    
    // Legacy metrics (for existing tests)
    pub packets_relayed: Counter,
    pub packets_failed: Counter,
    
    // Chain metrics
    pub rpc_errors: Counter,
    
    registry: Arc<Registry>,
}

impl RelayerMetrics {
    pub fn new() -> prometheus::Result<Self> {
        let registry = Arc::new(Registry::new());
        
        // Event metrics
        let total_events_processed = Counter::new("ibc_events_processed_total", "Total events processed")?;
        
        // Packet metrics
        let total_packets_detected = Counter::new("ibc_packets_detected_total", "Total packets detected")?;
        let total_packets_relayed = Counter::new("ibc_packets_relayed_total", "Total packets relayed")?;
        let total_packets_acknowledged = Counter::new("ibc_packets_acknowledged_total", "Total packets acknowledged")?;
        let total_packets_timed_out = Counter::new("ibc_packets_timed_out_total", "Total packets timed out")?;
        let total_packets_failed = Counter::new("ibc_packets_failed_total", "Total packets that failed to relay")?;
        let packet_relay_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new("ibc_packet_relay_duration_seconds", "Time to relay a packet")
        )?;
        
        // Legacy metrics
        let packets_relayed = total_packets_relayed.clone();
        let packets_failed = total_packets_failed.clone();
        
        // Chain metrics
        let rpc_errors = Counter::new("ibc_rpc_errors_total", "Total RPC errors")?;
        
        // Register all metrics
        registry.register(Box::new(total_events_processed.clone()))?;
        registry.register(Box::new(total_packets_detected.clone()))?;
        registry.register(Box::new(total_packets_relayed.clone()))?;
        registry.register(Box::new(total_packets_acknowledged.clone()))?;
        registry.register(Box::new(total_packets_timed_out.clone()))?;
        registry.register(Box::new(total_packets_failed.clone()))?;
        registry.register(Box::new(packet_relay_duration.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        
        Ok(Self {
            total_events_processed,
            total_packets_detected,
            total_packets_relayed,
            total_packets_acknowledged,
            total_packets_timed_out,
            total_packets_failed,
            packet_relay_duration,
            packets_relayed,
            packets_failed,
            rpc_errors,
            registry,
        })
    }
    
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_compile() {
        // Test that metrics can be created and used
        let metrics = RelayerMetrics::new().unwrap();
        
        // Test counter operations
        metrics.packets_relayed.inc();
        metrics.packets_failed.inc();
        metrics.rpc_errors.inc();
        
        // Test histogram
        metrics.packet_relay_duration.observe(1.5);
        
        // Test registry access
        let registry = metrics.registry();
        let families = registry.gather();
        assert!(!families.is_empty());
    }
}