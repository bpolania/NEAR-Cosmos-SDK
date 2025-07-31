// Packet relay coordinator that orchestrates scanning, proof generation, and relay automation
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;

use crate::chains::Chain;
use crate::config::RelayerConfig;
use crate::metrics::RelayerMetrics;
use super::{RelayEvent, PacketKey};
use super::scanner::{PacketScanner, ScannerConfig, ScanStats};
use super::engine::{RelayEngine, RelayStats};
use super::processor::PacketProcessor;
use super::timeout::{TimeoutManager, TimeoutConfig, TimeoutStats};
use super::bidirectional::{BidirectionalRelayManager, BidirectionalConfig, BidirectionalStats};

/// Comprehensive packet relay coordinator
pub struct PacketRelayCoordinator {
    /// Configuration
    config: RelayerConfig,
    /// Metrics collection
    metrics: Arc<RelayerMetrics>,
    /// Chain implementations
    chains: HashMap<String, Arc<dyn Chain>>,
    /// Event channel for relay events
    event_sender: mpsc::Sender<RelayEvent>,
    event_receiver: Option<mpsc::Receiver<RelayEvent>>,
    /// Shutdown signals
    shutdown_sender: tokio::sync::watch::Sender<bool>,
    shutdown_receiver: tokio::sync::watch::Receiver<bool>,
}

impl PacketRelayCoordinator {
    /// Create a new packet relay coordinator
    pub fn new(
        config: RelayerConfig,
        chains: HashMap<String, Arc<dyn Chain>>,
        metrics: Arc<RelayerMetrics>,
    ) -> Self {
        let (event_sender, event_receiver) = mpsc::channel(10000);
        let (shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
        
        Self {
            config,
            metrics,
            chains,
            event_sender,
            event_receiver: Some(event_receiver),
            shutdown_sender,
            shutdown_receiver,
        }
    }
    
    /// Start the complete packet relay system
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting comprehensive packet relay coordinator");
        
        // Create scanner configuration
        let scanner_config = ScannerConfig {
            scan_interval: 3, // Scan every 3 seconds
            start_height: None,
            max_blocks_per_scan: 50,
            max_packets_per_scan: 25,
            monitored_channels: vec![
                ("transfer".to_string(), "channel-0".to_string()),
            ],
        };
        
        // Create and start packet scanner
        let mut scanner = PacketScanner::new(
            self.chains.clone(),
            scanner_config,
            self.config.clone(),
            self.event_sender.clone(),
            self.metrics.clone(),
            self.shutdown_receiver.clone(),
        );
        
        // Create and start relay engine
        let (mut relay_engine, _engine_event_sender, _engine_shutdown_sender) = RelayEngine::new(
            self.config.clone(),
            self.chains.clone(),
            self.metrics.clone(),
        );

        // Create and start timeout manager
        let timeout_config = TimeoutConfig {
            check_interval: 30,    // Check every 30 seconds
            grace_period: 300,     // 5 minute grace period
            max_cleanup_retries: 3,
            cleanup_retry_delay_ms: 5000,
            max_completed_age_hours: 24,
        };

        let mut timeout_manager = TimeoutManager::new(
            timeout_config,
            self.config.clone(),
            self.chains.clone(),
            self.metrics.clone(),
            self.event_sender.clone(),
            self.shutdown_receiver.clone(),
        );

        // Create and start bidirectional relay manager
        let bidirectional_config = BidirectionalConfig {
            max_parallel_packets: 5,
            sequence_window_size: 1000,
            sequence_check_interval: 15,
            max_out_of_order_wait: 300,
            strict_ordering: true,
            batch_size: 3,
        };

        let mut bidirectional_manager = BidirectionalRelayManager::new(
            bidirectional_config,
            self.config.clone(),
            self.chains.clone(),
            self.metrics.clone(),
            self.event_sender.clone(),
            self.shutdown_receiver.clone(),
        );
        
        // Take the event receiver (we can only have one consumer)
        let event_receiver = self.event_receiver.take()
            .ok_or("Event receiver already consumed")?;
        
        // Start components in separate tasks
        let scanner_handle = task::spawn(async move {
            if let Err(e) = scanner.run().await {
                eprintln!("Scanner error: {}", e);
            }
        });
        
        let engine_handle = task::spawn(async move {
            if let Err(e) = relay_engine.run().await {
                eprintln!("Relay engine error: {}", e);
            }
        });
        
        let timeout_handle = task::spawn(async move {
            if let Err(e) = timeout_manager.run().await {
                eprintln!("Timeout manager error: {}", e);
            }
        });

        let bidirectional_handle = task::spawn(async move {
            if let Err(e) = bidirectional_manager.run().await {
                eprintln!("Bidirectional relay manager error: {}", e);
            }
        });

        let coordinator_handle = task::spawn(async move {
            Self::run_event_dispatcher(event_receiver).await;
        });
        
        println!("✅ All packet relay components started");
        
        // Wait for shutdown signal or component completion
        tokio::select! {
            _ = scanner_handle => {
                println!("📡 Scanner completed");
            }
            _ = engine_handle => {
                println!("⚙️  Relay engine completed");
            }
            _ = timeout_handle => {
                println!("⏰ Timeout manager completed");
            }
            _ = bidirectional_handle => {
                println!("🔄 Bidirectional relay manager completed");
            }
            _ = coordinator_handle => {
                println!("🎯 Event coordinator completed");
            }
            _ = tokio::signal::ctrl_c() => {
                println!("🛑 Shutdown signal received");
                self.stop().await?;
            }
        }
        
        Ok(())
    }
    
    /// Stop the packet relay coordinator
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🛑 Stopping packet relay coordinator");
        
        // Send shutdown signal to all components
        if let Err(e) = self.shutdown_sender.send(true) {
            eprintln!("Failed to send shutdown signal: {}", e);
        }
        
        // Give components time to shut down gracefully
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        println!("✅ Packet relay coordinator stopped");
        Ok(())
    }
    
    /// Run the event dispatcher (internal)
    async fn run_event_dispatcher(mut event_receiver: mpsc::Receiver<RelayEvent>) {
        println!("🎯 Starting event dispatcher for packet relay coordination");
        
        let mut processed_events = 0;
        
        while let Some(event) = event_receiver.recv().await {
            processed_events += 1;
            
            match &event {
                RelayEvent::PacketDetected { chain_id, packet, .. } => {
                    println!("📦 Event dispatcher: Packet detected on {} seq={}", 
                             chain_id, packet.sequence);
                }
                RelayEvent::PacketRelayed { source_chain, dest_chain, sequence } => {
                    println!("✅ Event dispatcher: Packet relayed {} -> {} seq={}", 
                             source_chain, dest_chain, sequence);
                }
                RelayEvent::PacketAcknowledged { chain_id, packet, .. } => {
                    println!("🎯 Event dispatcher: Packet acknowledged on {} seq={}", 
                             chain_id, packet.sequence);
                }
                RelayEvent::PacketTimedOut { chain_id, packet } => {
                    println!("⏰ Event dispatcher: Packet timed out on {} seq={}", 
                             chain_id, packet.sequence);
                }
                RelayEvent::ChainDisconnected { chain_id } => {
                    println!("🔌 Event dispatcher: Chain disconnected: {}", chain_id);
                }
                RelayEvent::ChainReconnected { chain_id } => {
                    println!("🔗 Event dispatcher: Chain reconnected: {}", chain_id);
                }
            }
            
            // Log periodic statistics
            if processed_events % 100 == 0 {
                println!("📊 Event dispatcher: Processed {} events", processed_events);
            }
        }
        
        println!("🎯 Event dispatcher completed after processing {} events", processed_events);
    }
    
    /// Get comprehensive relay statistics
    pub async fn get_relay_statistics(&self) -> RelayCoordinatorStats {
        RelayCoordinatorStats {
            chains_configured: self.chains.len(),
            event_channel_capacity: 10000, // Our channel capacity
            uptime_seconds: 0, // Would track actual uptime in production
            total_events_processed: 0, // Would track from metrics
            timeout_stats: None, // Would be populated with actual timeout manager stats
            bidirectional_stats: None, // Would be populated with actual bidirectional manager stats
        }
    }
    
    /// Health check for all components
    pub async fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus::default();
        
        // Check chain connectivity
        for (chain_id, chain) in &self.chains {
            match chain.get_latest_height().await {
                Ok(height) => {
                    status.healthy_chains += 1;
                    println!("✅ Chain {} healthy at height {}", chain_id, height);
                }
                Err(e) => {
                    status.unhealthy_chains += 1;
                    println!("❌ Chain {} unhealthy: {}", chain_id, e);
                    status.chain_errors.push(format!("{}: {}", chain_id, e));
                }
            }
        }
        
        status.total_chains = self.chains.len();
        status.is_healthy = status.unhealthy_chains == 0;
        
        status
    }
    
    /// Force packet relay for testing (bypasses normal scanning)
    pub async fn force_relay_packet(
        &self,
        source_chain: &str,
        dest_chain: &str,
        port: &str,
        channel: &str,
        sequence: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("🔧 Force relaying packet: {} -> {} {}:{} seq={}", 
                 source_chain, dest_chain, port, channel, sequence);
        
        // This would create a mock packet and force it through the relay process
        // Useful for testing and debugging
        
        let mock_packet = crate::chains::IbcPacket {
            sequence,
            source_port: port.to_string(),
            source_channel: channel.to_string(),
            destination_port: port.to_string(),
            destination_channel: "channel-1".to_string(), // Assume counterparty channel
            data: b"test_packet_data".to_vec(),
            timeout_height: None,
            timeout_timestamp: None,
        };
        
        // Create processor for this relay
        let processor = PacketProcessor::new(
            self.chains.clone(),
            self.config.clone(),
            self.metrics.clone(),
        );
        
        // Process the packet
        let tx_hash = processor.process_packet(source_chain, dest_chain, &mock_packet).await?;
        
        println!("✅ Force relay completed: {}", tx_hash);
        Ok(tx_hash)
    }
}

/// Statistics for the relay coordinator
#[derive(Debug, Default)]
pub struct RelayCoordinatorStats {
    pub chains_configured: usize,
    pub event_channel_capacity: usize,
    pub uptime_seconds: u64,
    pub total_events_processed: u64,
    pub timeout_stats: Option<TimeoutStats>,
    pub bidirectional_stats: Option<BidirectionalStats>,
}

/// Health status for all relay components
#[derive(Debug, Default)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub total_chains: usize,
    pub healthy_chains: usize,
    pub unhealthy_chains: usize,
    pub chain_errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GlobalConfig, MetricsConfig};
    
    #[test]
    fn test_coordinator_creation() {
        let config = RelayerConfig {
            global: GlobalConfig {
                log_level: "info".to_string(),
                max_retries: 3,
                retry_delay_ms: 1000,
                health_check_interval: 30,
            },
            chains: HashMap::new(),
            connections: vec![],
            metrics: MetricsConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
        };
        
        let metrics = Arc::new(RelayerMetrics::new().unwrap());
        let chains = HashMap::new();
        
        let coordinator = PacketRelayCoordinator::new(config, chains, metrics);
        assert_eq!(coordinator.chains.len(), 0);
    }
    
    #[test]
    fn test_health_status() {
        let status = HealthStatus {
            is_healthy: true,
            total_chains: 2,
            healthy_chains: 2,
            unhealthy_chains: 0,
            chain_errors: vec![],
        };
        
        assert!(status.is_healthy);
        assert_eq!(status.total_chains, 2);
    }
    
    #[test]
    fn test_relay_coordinator_stats() {
        let stats = RelayCoordinatorStats {
            chains_configured: 2,
            event_channel_capacity: 10000,
            uptime_seconds: 3600,
            total_events_processed: 150,
            timeout_stats: None,
            bidirectional_stats: None,
        };
        
        assert_eq!(stats.chains_configured, 2);
        assert_eq!(stats.event_channel_capacity, 10000);
    }
}