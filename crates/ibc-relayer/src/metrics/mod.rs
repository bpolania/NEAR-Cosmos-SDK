// Metrics and monitoring
#![allow(dead_code)]

use prometheus::{Counter, Histogram, Registry};
use std::sync::Arc;

/// Relayer metrics - only keeping fields used by tests
pub struct RelayerMetrics {
    // Packet metrics
    pub packets_relayed: Counter,
    pub packets_failed: Counter,
    pub packet_relay_duration: Histogram,
    
    // Chain metrics
    pub rpc_errors: Counter,
    
    registry: Arc<Registry>,
}

impl RelayerMetrics {
    pub fn new() -> prometheus::Result<Self> {
        let registry = Arc::new(Registry::new());
        
        let packets_relayed = Counter::new("ibc_packets_relayed_total", "Total packets relayed")?;
        let packets_failed = Counter::new("ibc_packets_failed_total", "Total packets that failed to relay")?;
        let packet_relay_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new("ibc_packet_relay_duration_seconds", "Time to relay a packet")
        )?;
        let rpc_errors = Counter::new("ibc_rpc_errors_total", "Total RPC errors")?;
        
        registry.register(Box::new(packets_relayed.clone()))?;
        registry.register(Box::new(packets_failed.clone()))?;
        registry.register(Box::new(packet_relay_duration.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        
        Ok(Self {
            packets_relayed,
            packets_failed,
            packet_relay_duration,
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