// Utility functions and helpers

pub mod crypto;

use anyhow::Result;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Convert system time to nanoseconds since Unix epoch
pub fn system_time_to_nanos(time: SystemTime) -> Result<u64> {
    let duration = time.duration_since(UNIX_EPOCH)?;
    Ok(duration.as_nanos() as u64)
}

/// Convert nanoseconds since Unix epoch to system time
pub fn nanos_to_system_time(nanos: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_nanos(nanos)
}

/// Exponential backoff delay calculator
pub fn exponential_backoff(retry_count: u32, base_delay_ms: u64, max_delay_ms: u64) -> Duration {
    let delay_ms = base_delay_ms * 2_u64.pow(retry_count);
    Duration::from_millis(delay_ms.min(max_delay_ms))
}

/// Parse connection identifier from string
pub fn parse_connection_id(conn_id: &str) -> Result<u64> {
    if let Some(id_str) = conn_id.strip_prefix("connection-") {
        Ok(id_str.parse()?)
    } else {
        anyhow::bail!("Invalid connection ID format: {}", conn_id)
    }
}

/// Parse channel identifier from string  
pub fn parse_channel_id(chan_id: &str) -> Result<u64> {
    if let Some(id_str) = chan_id.strip_prefix("channel-") {
        Ok(id_str.parse()?)
    } else {
        anyhow::bail!("Invalid channel ID format: {}", chan_id)
    }
}

/// Generate connection identifier
pub fn generate_connection_id(id: u64) -> String {
    format!("connection-{}", id)
}

/// Generate channel identifier
pub fn generate_channel_id(id: u64) -> String {
    format!("channel-{}", id)
}