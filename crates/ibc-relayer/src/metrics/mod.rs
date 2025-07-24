// Metrics and monitoring

use prometheus::{Counter, Histogram, Gauge, Registry};
use std::sync::Arc;

/// Relayer metrics
pub struct RelayerMetrics {
    // Packet metrics
    pub packets_relayed: Counter,
    pub packets_failed: Counter,
    pub packet_relay_duration: Histogram,
    
    // Chain metrics
    pub chain_heights: Gauge,
    pub rpc_requests: Counter,
    pub rpc_errors: Counter,
    
    // Connection metrics
    pub active_connections: Gauge,
    pub active_channels: Gauge,
    
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
        
        let chain_heights = Gauge::new("ibc_chain_height", "Current height of each chain")?;
        let rpc_requests = Counter::new("ibc_rpc_requests_total", "Total RPC requests made")?;
        let rpc_errors = Counter::new("ibc_rpc_errors_total", "Total RPC errors")?;
        
        let active_connections = Gauge::new("ibc_active_connections", "Number of active IBC connections")?;
        let active_channels = Gauge::new("ibc_active_channels", "Number of active IBC channels")?;
        
        registry.register(Box::new(packets_relayed.clone()))?;
        registry.register(Box::new(packets_failed.clone()))?;
        registry.register(Box::new(packet_relay_duration.clone()))?;
        registry.register(Box::new(chain_heights.clone()))?;
        registry.register(Box::new(rpc_requests.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(active_channels.clone()))?;
        
        Ok(Self {
            packets_relayed,
            packets_failed,
            packet_relay_duration,
            chain_heights,
            rpc_requests,
            rpc_errors,
            active_connections,
            active_channels,
            registry,
        })
    }
    
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}