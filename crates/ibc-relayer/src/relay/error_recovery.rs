// Error recovery and retry logic with exponential backoff for network failures
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Configuration for error recovery and retry logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum retry delay in milliseconds (backoff cap)
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor to avoid thundering herd (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Timeout for individual operations in milliseconds
    pub operation_timeout_ms: u64,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker recovery timeout in seconds
    pub circuit_breaker_timeout_s: u64,
    /// Rate limit recovery delay in milliseconds
    pub rate_limit_delay_ms: u64,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 1000,      // Start with 1 second
            max_delay_ms: 60000,         // Cap at 1 minute
            backoff_multiplier: 2.0,     // Double the delay each time
            jitter_factor: 0.1,          // 10% jitter
            operation_timeout_ms: 30000, // 30 second timeout
            circuit_breaker_threshold: 5, // Break after 5 consecutive failures
            circuit_breaker_timeout_s: 300, // 5 minute recovery
            rate_limit_delay_ms: 5000,   // 5 second delay for rate limits
        }
    }
}

/// Types of recoverable errors
#[derive(Debug, Clone, PartialEq)]
pub enum RecoverableError {
    /// Network connectivity issues
    NetworkError(String),
    /// Rate limiting from external APIs
    RateLimitError(String),
    /// Temporary service unavailability
    ServiceUnavailable(String),
    /// Timeout waiting for response
    TimeoutError(String),
    /// Transient blockchain errors (mempool full, etc.)
    TransientChainError(String),
    /// Unknown recoverable error
    Unknown(String),
}

impl RecoverableError {
    /// Check if an error is recoverable
    pub fn from_error(error: &dyn std::error::Error) -> Option<Self> {
        let error_msg = error.to_string().to_lowercase();
        
        // Network-related errors
        if error_msg.contains("connection") || 
           error_msg.contains("timeout") || 
           error_msg.contains("network") ||
           error_msg.contains("dns") ||
           error_msg.contains("unreachable") {
            return Some(RecoverableError::NetworkError(error.to_string()));
        }
        
        // Rate limiting
        if error_msg.contains("rate limit") || 
           error_msg.contains("too many requests") ||
           error_msg.contains("429") {
            return Some(RecoverableError::RateLimitError(error.to_string()));
        }
        
        // Service unavailable
        if error_msg.contains("service unavailable") ||
           error_msg.contains("502") ||
           error_msg.contains("503") ||
           error_msg.contains("504") {
            return Some(RecoverableError::ServiceUnavailable(error.to_string()));
        }
        
        // Blockchain transient errors
        if error_msg.contains("mempool") ||
           error_msg.contains("nonce") ||
           error_msg.contains("sequence") ||
           error_msg.contains("insufficient fee") {
            return Some(RecoverableError::TransientChainError(error.to_string()));
        }
        
        None // Not recoverable
    }
    
    /// Get recommended retry delay for this error type
    pub fn get_base_delay_ms(&self) -> u64 {
        match self {
            RecoverableError::RateLimitError(_) => 5000,  // Wait longer for rate limits
            RecoverableError::NetworkError(_) => 1000,   // Standard delay for network
            RecoverableError::ServiceUnavailable(_) => 2000, // Medium delay for service issues
            RecoverableError::TimeoutError(_) => 1500,   // Slightly longer for timeouts
            RecoverableError::TransientChainError(_) => 3000, // Wait for chain state
            RecoverableError::Unknown(_) => 1000,        // Standard delay
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,  // Normal operation
    Open,    // Blocking requests due to failures
    HalfOpen, // Testing recovery
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    threshold: u32,
    last_failure_time: Option<Instant>,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            threshold,
            last_failure_time: None,
            timeout,
        }
    }
    
    /// Check if operation should be allowed
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.timeout {
                        debug!("Circuit breaker transitioning to half-open");
                        self.state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }
    
    /// Record successful operation
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitBreakerState::Closed;
        debug!("Circuit breaker reset to closed state");
    }
    
    /// Record failed operation
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        
        if self.failure_count >= self.threshold {
            if self.state == CircuitBreakerState::HalfOpen {
                warn!("Circuit breaker opening after half-open failure");
            } else {
                warn!("Circuit breaker opening after {} failures", self.failure_count);
            }
            self.state = CircuitBreakerState::Open;
        }
    }
}

/// Retry statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    pub total_attempts: u64,
    pub successful_retries: u64,
    pub failed_retries: u64,
    pub circuit_breaker_activations: u64,
    pub rate_limit_delays: u64,
}

/// Error recovery manager with exponential backoff and circuit breaker
pub struct ErrorRecoveryManager {
    config: ErrorRecoveryConfig,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    stats: HashMap<String, RetryStats>,
}

impl ErrorRecoveryManager {
    /// Create a new error recovery manager
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        Self {
            config,
            circuit_breakers: HashMap::new(),
            stats: HashMap::new(),
        }
    }
    
    /// Execute an operation with retry logic and error recovery
    pub async fn execute_with_retry<F, Fut, T>(
        &mut self,
        operation_name: &str,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        let mut attempt = 0;
        let mut delay_ms = self.config.initial_delay_ms;
        
        // Get or create circuit breaker
        if !self.circuit_breakers.contains_key(operation_name) {
            self.circuit_breakers.insert(
                operation_name.to_string(),
                CircuitBreaker::new(
                    self.config.circuit_breaker_threshold,
                    Duration::from_secs(self.config.circuit_breaker_timeout_s),
                ),
            );
        }
        
        // Get or create stats
        if !self.stats.contains_key(operation_name) {
            self.stats.insert(operation_name.to_string(), RetryStats::default());
        }
        
        loop {
            attempt += 1;
            
            // Check circuit breaker
            let circuit_breaker = self.circuit_breakers.get_mut(operation_name).unwrap();
            if !circuit_breaker.can_execute() {
                let stats = self.stats.get_mut(operation_name).unwrap();
                stats.circuit_breaker_activations += 1;
                
                return Err(anyhow::anyhow!(
                    "Circuit breaker is open for operation '{}' - too many failures",
                    operation_name
                ));
            }
            
            let stats = self.stats.get_mut(operation_name).unwrap();
            stats.total_attempts += 1;
            
            debug!("Executing '{}' attempt {}/{}", operation_name, attempt, self.config.max_retries + 1);
            
            // Execute operation with timeout
            let operation_future = operation();
            let timeout_duration = Duration::from_millis(self.config.operation_timeout_ms);
            
            let result = timeout(timeout_duration, operation_future).await;
            
            match result {
                Ok(Ok(success)) => {
                    // Operation succeeded
                    let circuit_breaker = self.circuit_breakers.get_mut(operation_name).unwrap();
                    circuit_breaker.record_success();
                    
                    if attempt > 1 {
                        let stats = self.stats.get_mut(operation_name).unwrap();
                        stats.successful_retries += 1;
                        info!("✅ Operation '{}' succeeded after {} attempts", operation_name, attempt);
                    }
                    
                    return Ok(success);
                }
                Ok(Err(error)) => {
                    // Operation failed - check if recoverable
                    if let Some(recoverable_error) = RecoverableError::from_error(error.as_ref()) {
                        if attempt <= self.config.max_retries {
                            warn!("⚠️ Operation '{}' failed (attempt {}/{}): {:?}", 
                                  operation_name, attempt, self.config.max_retries + 1, recoverable_error);
                            
                            // Calculate retry delay with exponential backoff and jitter
                            let base_delay = recoverable_error.get_base_delay_ms().max(delay_ms);
                            let jitter = self.calculate_jitter(base_delay);
                            let final_delay = (base_delay + jitter).min(self.config.max_delay_ms);
                            
                            debug!("Retrying '{}' in {}ms (base: {}ms, jitter: {}ms)", 
                                   operation_name, final_delay, base_delay, jitter);
                            
                            // Special handling for rate limits
                            if matches!(recoverable_error, RecoverableError::RateLimitError(_)) {
                                let stats = self.stats.get_mut(operation_name).unwrap();
                                stats.rate_limit_delays += 1;
                                sleep(Duration::from_millis(self.config.rate_limit_delay_ms)).await;
                            } else {
                                sleep(Duration::from_millis(final_delay)).await;
                            }
                            
                            // Update delay for next iteration
                            delay_ms = (delay_ms as f64 * self.config.backoff_multiplier) as u64;
                            
                            continue;
                        } else {
                            // Max retries exceeded
                            let circuit_breaker = self.circuit_breakers.get_mut(operation_name).unwrap();
                            circuit_breaker.record_failure();
                            
                            let stats = self.stats.get_mut(operation_name).unwrap();
                            stats.failed_retries += 1;
                            
                            error!("❌ Operation '{}' failed after {} attempts with recoverable error: {}", 
                                   operation_name, attempt, error);
                            return Err(error);
                        }
                    } else {
                        // Non-recoverable error
                        let circuit_breaker = self.circuit_breakers.get_mut(operation_name).unwrap();
                        circuit_breaker.record_failure();
                        
                        let stats = self.stats.get_mut(operation_name).unwrap();
                        stats.failed_retries += 1;
                        
                        error!("❌ Operation '{}' failed with non-recoverable error: {}", operation_name, error);
                        return Err(error);
                    }
                }
                Err(_timeout_error) => {
                    // Operation timed out
                    if attempt <= self.config.max_retries {
                        warn!("⏰ Operation '{}' timed out (attempt {}/{})", 
                              operation_name, attempt, self.config.max_retries + 1);
                        
                        let delay_with_jitter = delay_ms + self.calculate_jitter(delay_ms);
                        sleep(Duration::from_millis(delay_with_jitter)).await;
                        delay_ms = (delay_ms as f64 * self.config.backoff_multiplier) as u64;
                        
                        continue;
                    } else {
                        let circuit_breaker = self.circuit_breakers.get_mut(operation_name).unwrap();
                        circuit_breaker.record_failure();
                        
                        let stats = self.stats.get_mut(operation_name).unwrap();
                        stats.failed_retries += 1;
                        
                        error!("❌ Operation '{}' timed out after {} attempts", operation_name, attempt);
                        return Err(anyhow::anyhow!(
                            "Operation '{}' timed out after {} attempts",
                            operation_name, attempt
                        ));
                    }
                }
            }
        }
    }
    
    /// Calculate jitter to avoid thundering herd
    pub fn calculate_jitter(&self, base_delay_ms: u64) -> u64 {
        if self.config.jitter_factor <= 0.0 {
            return 0;
        }
        
        let max_jitter = (base_delay_ms as f64 * self.config.jitter_factor) as u64;
        fastrand::u64(0..=max_jitter)
    }
    
    /// Get retry statistics for an operation
    pub fn get_stats(&self, operation_name: &str) -> Option<&RetryStats> {
        self.stats.get(operation_name)
    }
    
    /// Get all retry statistics
    pub fn get_all_stats(&self) -> &HashMap<String, RetryStats> {
        &self.stats
    }
    
    /// Reset statistics for an operation
    pub fn reset_stats(&mut self, operation_name: &str) {
        if let Some(stats) = self.stats.get_mut(operation_name) {
            *stats = RetryStats::default();
        }
    }
    
    /// Reset circuit breaker for an operation
    pub fn reset_circuit_breaker(&mut self, operation_name: &str) {
        if let Some(circuit_breaker) = self.circuit_breakers.get_mut(operation_name) {
            circuit_breaker.record_success();
            info!("🔄 Circuit breaker reset for operation '{}'", operation_name);
        }
    }
    
    /// Get circuit breaker status
    pub fn get_circuit_breaker_status(&self, operation_name: &str) -> Option<CircuitBreakerState> {
        self.circuit_breakers.get(operation_name).map(|cb| cb.state.clone())
    }
}

/// Convenient macro for wrapping operations with retry logic
#[macro_export]
macro_rules! with_retry {
    ($recovery_manager:expr, $operation_name:expr, $operation:expr) => {
        $recovery_manager.execute_with_retry($operation_name, || {
            Box::pin(async move { $operation.await })
        }).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc as StdArc;
    
    #[tokio::test]
    async fn test_successful_retry_after_failures() {
        let config = ErrorRecoveryConfig {
            max_retries: 3,
            initial_delay_ms: 10, // Fast for testing
            ..Default::default()
        };
        
        let mut recovery_manager = ErrorRecoveryManager::new(config);
        let call_count = StdArc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        let operation = move || {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    anyhow::bail!("Network error")
                } else {
                    Ok("Success!")
                }
            }
        };
        
        let result = recovery_manager.execute_with_retry("test_op", operation).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success!");
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
        
        let stats = recovery_manager.get_stats("test_op").unwrap();
        assert_eq!(stats.total_attempts, 3);
        assert_eq!(stats.successful_retries, 1);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = ErrorRecoveryConfig {
            max_retries: 2,
            circuit_breaker_threshold: 2,
            initial_delay_ms: 1,
            ..Default::default()
        };
        
        let mut recovery_manager = ErrorRecoveryManager::new(config);
        
        // First operation - will fail and increment failure count
        let operation1 = || {
            async {
                anyhow::bail!("Network error")
            }
        };
        
        let result1: anyhow::Result<String> = recovery_manager.execute_with_retry("circuit_test", operation1).await;
        assert!(result1.is_err());
        
        // Second operation - will fail and open circuit breaker
        let operation2 = || {
            async {
                anyhow::bail!("Network error")
            }
        };
        
        let result2: anyhow::Result<String> = recovery_manager.execute_with_retry("circuit_test", operation2).await;
        assert!(result2.is_err());
        
        // Third operation - should be blocked by circuit breaker
        let operation3 = || {
            async { Ok("Should not execute") }
        };
        
        let result3 = recovery_manager.execute_with_retry("circuit_test", operation3).await;
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("Circuit breaker is open"));
    }
    
    #[tokio::test]
    async fn test_recoverable_error_detection() {
        let network_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let recoverable = RecoverableError::from_error(&network_error);
        assert!(matches!(recoverable, Some(RecoverableError::NetworkError(_))));
        
        let rate_limit_error = std::io::Error::new(std::io::ErrorKind::Other, "Rate limit exceeded");
        let recoverable = RecoverableError::from_error(&rate_limit_error);
        assert!(matches!(recoverable, Some(RecoverableError::RateLimitError(_))));
        
        // Non-recoverable error
        let auth_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Unauthorized");
        let recoverable = RecoverableError::from_error(&auth_error);
        assert!(recoverable.is_none());
    }
    
    #[tokio::test]
    async fn test_exponential_backoff_calculation() {
        let config = ErrorRecoveryConfig {
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_delay_ms: 1000,
            jitter_factor: 0.0, // No jitter for predictable testing
            ..Default::default()
        };
        
        let recovery_manager = ErrorRecoveryManager::new(config);
        
        // Test that jitter calculation doesn't panic with zero jitter
        let jitter = recovery_manager.calculate_jitter(100);
        assert_eq!(jitter, 0);
    }
    
    #[tokio::test]
    async fn test_timeout_handling() {
        let config = ErrorRecoveryConfig {
            max_retries: 1,
            operation_timeout_ms: 50, // Very short timeout
            initial_delay_ms: 1,
            ..Default::default()
        };
        
        let mut recovery_manager = ErrorRecoveryManager::new(config);
        
        let slow_operation = || {
            async {
                tokio::time::sleep(Duration::from_millis(100)).await; // Takes longer than timeout
                Ok("Should timeout")
            }
        };
        
        let result = recovery_manager.execute_with_retry("timeout_test", slow_operation).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
}