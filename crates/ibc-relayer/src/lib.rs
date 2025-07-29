// IBC Relayer Library
// This module structure exposes the relayer components for testing and external use

pub mod config;
pub mod chains;
pub mod keystore;
pub mod relay;
pub mod metrics;
pub mod monitor;

// Re-export commonly used types for convenience
pub use config::{RelayerConfig, ChainConfig, ChainSpecificConfig, ConnectionConfig};
pub use chains::{Chain, ChainEvent, IbcPacket};
pub use keystore::{KeyManager, KeyManagerConfig, KeyEntry, KeyError};
pub use relay::{RelayEngine, RelayEvent, PacketTracker, PendingPacket, PacketKey};
pub use metrics::RelayerMetrics;
pub use monitor::{EventMonitor, MonitorConfig};