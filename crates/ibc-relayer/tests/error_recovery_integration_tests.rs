// Integration tests for error recovery and retry mechanisms
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc as StdArc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use anyhow::Result;

use ibc_relayer::relay::error_recovery::{
    ErrorRecoveryManager, ErrorRecoveryConfig, RecoverableError, CircuitBreakerState
};

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_delay_ms: 50,  // Fast for testing
        backoff_multiplier: 2.0,
        max_delay_ms: 500,
        jitter_factor: 0.0, // No jitter for predictable timing
        operation_timeout_ms: 1000,
        circuit_breaker_threshold: 5,
        circuit_breaker_timeout_s: 60,
        rate_limit_delay_ms: 100,
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);
    let call_count = StdArc::new(AtomicU32::new(0));
    let start_time = Instant::now();

    let call_count_clone = call_count.clone();
    let operation = move || {
        let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            if count < 2 {
                // Simulate network error for first 2 attempts
                anyhow::bail!("Network connection failed")
            } else {
                Ok("Success after retries")
            }
        }
    };

    let result = recovery_manager.execute_with_retry("backoff_test", operation).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success after retries");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    // Network errors have a base delay of 1000ms, so timing will be:
    // First retry: 1000ms (base delay for network error)
    // Second retry: 100ms (configured delay) 
    // Total: ~1000ms + some processing time
    println!("Elapsed time: {:?}", elapsed);
    assert!(elapsed >= Duration::from_millis(900)); // At least 1000ms - some tolerance
    assert!(elapsed <= Duration::from_millis(1500)); // Allow buffer for processing and jitter

    let stats = recovery_manager.get_stats("backoff_test").unwrap();
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.successful_retries, 1);
    assert_eq!(stats.failed_retries, 0);
}

#[tokio::test]
async fn test_circuit_breaker_prevents_cascading_failures() {
    let config = ErrorRecoveryConfig {
        max_retries: 1,
        initial_delay_ms: 10,
        circuit_breaker_threshold: 2,
        circuit_breaker_timeout_s: 1, // Short timeout for testing
        operation_timeout_ms: 100,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // Operation that always fails
    let failing_operation = || async { anyhow::bail!("Persistent failure") };

    // First operation - should fail and increment failure count
    let result1: Result<String> = recovery_manager
        .execute_with_retry("circuit_test", failing_operation)
        .await;
    assert!(result1.is_err());

    // Second operation - should fail and trigger circuit breaker
    let result2: Result<String> = recovery_manager
        .execute_with_retry("circuit_test", failing_operation)
        .await;
    assert!(result2.is_err());

    // Verify circuit breaker is open
    assert_eq!(
        recovery_manager.get_circuit_breaker_status("circuit_test"),
        Some(CircuitBreakerState::Open)
    );

    // Third operation - should be blocked by circuit breaker
    let successful_operation = || async { Ok("Should not execute") };
    let result3 = recovery_manager
        .execute_with_retry("circuit_test", successful_operation)
        .await;
    
    assert!(result3.is_err());
    assert!(result3.unwrap_err().to_string().contains("Circuit breaker is open"));

    let stats = recovery_manager.get_stats("circuit_test").unwrap();
    assert_eq!(stats.circuit_breaker_activations, 1);
}

#[tokio::test]
async fn test_circuit_breaker_recovery() {
    let config = ErrorRecoveryConfig {
        max_retries: 1,
        initial_delay_ms: 10,
        circuit_breaker_threshold: 1,
        circuit_breaker_timeout_s: 1, // 1 second recovery time
        operation_timeout_ms: 100,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // First operation fails and opens circuit breaker
    let failing_operation = || async { anyhow::bail!("Network error") };
    let result1: Result<String> = recovery_manager
        .execute_with_retry("recovery_test", failing_operation)
        .await;
    assert!(result1.is_err());

    // Verify circuit breaker is open
    assert_eq!(
        recovery_manager.get_circuit_breaker_status("recovery_test"),
        Some(CircuitBreakerState::Open)
    );

    // Wait for circuit breaker recovery timeout
    sleep(Duration::from_secs(2)).await;

    // Operation should now succeed and reset circuit breaker
    let successful_operation = || async { Ok("Recovery successful") };
    let result2 = recovery_manager
        .execute_with_retry("recovery_test", successful_operation)
        .await;
    
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "Recovery successful");

    // Circuit breaker should be closed again
    assert_eq!(
        recovery_manager.get_circuit_breaker_status("recovery_test"),
        Some(CircuitBreakerState::Closed)
    );
}

#[tokio::test]
async fn test_rate_limit_error_handling() {
    let config = ErrorRecoveryConfig {
        max_retries: 2,
        initial_delay_ms: 10,
        rate_limit_delay_ms: 100, // Specific delay for rate limits
        operation_timeout_ms: 500,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);
    let call_count = StdArc::new(AtomicU32::new(0));
    let start_time = Instant::now();

    let call_count_clone = call_count.clone();
    let operation = move || {
        let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            if count == 0 {
                anyhow::bail!("Rate limit exceeded - too many requests")
            } else {
                Ok("Success after rate limit")
            }
        }
    };

    let result = recovery_manager.execute_with_retry("rate_limit_test", operation).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success after rate limit");
    
    // Should take at least the rate limit delay (100ms)
    assert!(elapsed >= Duration::from_millis(100));

    let stats = recovery_manager.get_stats("rate_limit_test").unwrap();
    assert_eq!(stats.rate_limit_delays, 1);
    assert_eq!(stats.successful_retries, 1);
}

#[tokio::test]
async fn test_timeout_handling_with_retries() {
    let config = ErrorRecoveryConfig {
        max_retries: 2,
        initial_delay_ms: 10,
        operation_timeout_ms: 50, // Very short timeout
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);
    let call_count = StdArc::new(AtomicU32::new(0));

    let call_count_clone = call_count.clone();
    let operation = move || {
        let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            if count < 2 {
                // First two attempts will timeout
                sleep(Duration::from_millis(100)).await;
                Ok("Should timeout")
            } else {
                // Third attempt succeeds quickly
                Ok("Success after timeouts")
            }
        }
    };

    let result = recovery_manager.execute_with_retry("timeout_test", operation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success after timeouts");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    let stats = recovery_manager.get_stats("timeout_test").unwrap();
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.successful_retries, 1);
}

#[tokio::test]
async fn test_non_recoverable_error_handling() {
    let config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_delay_ms: 10,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // Operation that fails with non-recoverable error
    let operation = || async {
        anyhow::bail!("Unauthorized access - invalid credentials")
    };

    let start_time = Instant::now();
    let result: Result<String> = recovery_manager
        .execute_with_retry("non_recoverable_test", operation)
        .await;
    let elapsed = start_time.elapsed();

    // Should fail immediately without retries
    assert!(result.is_err());
    assert!(elapsed < Duration::from_millis(50)); // Should be very quick

    let stats = recovery_manager.get_stats("non_recoverable_test").unwrap();
    assert_eq!(stats.total_attempts, 1); // Only one attempt
    assert_eq!(stats.failed_retries, 1);
}

#[tokio::test]
async fn test_recoverable_error_classification() {
    // Test network errors
    let network_error = std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused, 
        "Connection refused"
    );
    let recoverable = RecoverableError::from_error(&network_error);
    assert!(matches!(recoverable, Some(RecoverableError::NetworkError(_))));

    // Test rate limit errors
    let rate_limit_error = anyhow::anyhow!("HTTP 429: Too many requests");
    let recoverable = RecoverableError::from_error(rate_limit_error.as_ref());
    assert!(matches!(recoverable, Some(RecoverableError::RateLimitError(_))));

    // Test service unavailable
    let service_error = anyhow::anyhow!("HTTP 503: Service unavailable");
    let recoverable = RecoverableError::from_error(service_error.as_ref());
    assert!(matches!(recoverable, Some(RecoverableError::ServiceUnavailable(_))));

    // Test blockchain transient errors
    let mempool_error = anyhow::anyhow!("Mempool is full, try again later");
    let recoverable = RecoverableError::from_error(mempool_error.as_ref());
    assert!(matches!(recoverable, Some(RecoverableError::TransientChainError(_))));

    // Test non-recoverable error
    let auth_error = anyhow::anyhow!("Invalid signature");
    let recoverable = RecoverableError::from_error(auth_error.as_ref());
    assert!(recoverable.is_none());
}

#[tokio::test]
async fn test_concurrent_operations_with_separate_circuit_breakers() {
    let config = ErrorRecoveryConfig {
        max_retries: 1,
        initial_delay_ms: 10,
        circuit_breaker_threshold: 1,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // Make operation A fail to trigger its circuit breaker
    let failing_operation = || async { anyhow::bail!("Network error") };
    let result_a: Result<String> = recovery_manager
        .execute_with_retry("operation_a", failing_operation)
        .await;
    assert!(result_a.is_err());

    // Operation A circuit breaker should be open
    assert_eq!(
        recovery_manager.get_circuit_breaker_status("operation_a"),
        Some(CircuitBreakerState::Open)
    );

    // Operation B should still work (separate circuit breaker)
    let successful_operation = || async { Ok("Operation B success") };
    let result_b = recovery_manager
        .execute_with_retry("operation_b", successful_operation)
        .await;
    
    assert!(result_b.is_ok());
    assert_eq!(result_b.unwrap(), "Operation B success");

    // Operation B circuit breaker should be closed
    assert_eq!(
        recovery_manager.get_circuit_breaker_status("operation_b"),
        Some(CircuitBreakerState::Closed)
    );
}

#[tokio::test]
async fn test_statistics_tracking() {
    let config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_delay_ms: 10,
        circuit_breaker_threshold: 5, // High threshold to avoid circuit breaking
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
                Ok("Success")
            }
        }
    };

    let result = recovery_manager.execute_with_retry("stats_test", operation).await;
    assert!(result.is_ok());

    let stats = recovery_manager.get_stats("stats_test").unwrap();
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.successful_retries, 1);
    assert_eq!(stats.failed_retries, 0); // Operation ultimately succeeded
    assert_eq!(stats.circuit_breaker_activations, 0);

    // Test stats reset
    recovery_manager.reset_stats("stats_test");
    let reset_stats = recovery_manager.get_stats("stats_test").unwrap();
    assert_eq!(reset_stats.total_attempts, 0);
    assert_eq!(reset_stats.successful_retries, 0);
}

#[tokio::test]
async fn test_jitter_reduces_thundering_herd() {
    let config = ErrorRecoveryConfig {
        max_retries: 1,
        initial_delay_ms: 100,
        jitter_factor: 0.5, // 50% jitter
        backoff_multiplier: 1.0, // No backoff multiplication for predictable testing
        ..Default::default()
    };

    let recovery_manager = ErrorRecoveryManager::new(config);
    
    // Test jitter calculation multiple times
    let mut jitter_values = Vec::new();
    for _ in 0..50 {
        let jitter = recovery_manager.calculate_jitter(100);
        jitter_values.push(jitter);
    }

    // All jitter values should be within expected range (0-50ms with 50% jitter)
    assert!(jitter_values.iter().all(|&j| j <= 50));
    
    // Should have some variation (not all the same value)
    let unique_values: std::collections::HashSet<_> = jitter_values.iter().collect();
    assert!(unique_values.len() > 1, "Jitter should produce varied values");
}

#[tokio::test]
async fn test_max_delay_cap() {
    let config = ErrorRecoveryConfig {
        max_retries: 5,
        initial_delay_ms: 100,
        backoff_multiplier: 3.0, // Aggressive backoff
        max_delay_ms: 500, // Cap at 500ms
        jitter_factor: 0.0,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);
    let call_times = StdArc::new(std::sync::Mutex::new(Vec::new()));

    let call_times_clone = call_times.clone();
    let operation = move || {
        call_times_clone.lock().unwrap().push(Instant::now());
        async move {
            anyhow::bail!("connection timeout")
        }
    };

    let start_time = Instant::now();
    let result: Result<String> = recovery_manager
        .execute_with_retry("max_delay_test", operation)
        .await;
    let total_elapsed = start_time.elapsed();

    assert!(result.is_err());
    
    let times = call_times.lock().unwrap();
    assert_eq!(times.len(), 6); // Initial + 5 retries

    // Check that delays are capped
    // With initial_delay=100, multiplier=3.0, delays would be: 100, 300, 900 (capped to 500), 1500 (capped to 500), etc.
    // Total expected delay: ~100 + 300 + 500 + 500 + 500 = 1900ms minimum
    assert!(total_elapsed >= Duration::from_millis(1500)); // Allow tolerance for system variations
    assert!(total_elapsed <= Duration::from_millis(3000)); // Should not exceed reasonable bounds
}

#[tokio::test]
async fn test_error_recovery_integration_with_real_scenarios() {
    let config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_delay_ms: 50,
        backoff_multiplier: 2.0,
        circuit_breaker_threshold: 3,
        operation_timeout_ms: 200,
        ..Default::default()
    };

    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // Simulate a chain RPC that's initially down but recovers
    let attempt_count = StdArc::new(AtomicU32::new(0));
    let should_timeout = StdArc::new(AtomicBool::new(true));

    let attempt_count_clone = attempt_count.clone();
    let should_timeout_clone = should_timeout.clone();
    
    let chain_rpc_operation = move || {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        let timeout_flag = should_timeout_clone.clone();
        
        async move {
            match count {
                0 => {
                    // First attempt: connection refused
                    anyhow::bail!("Connection refused to chain RPC")
                }
                1 => {
                    // Second attempt: timeout
                    if timeout_flag.load(Ordering::SeqCst) {
                        sleep(Duration::from_millis(300)).await; // Exceeds 200ms timeout
                    }
                    Ok("Should timeout")
                }
                _ => {
                    // Third attempt: success
                    timeout_flag.store(false, Ordering::SeqCst);
                    Ok("Chain RPC response: block height 12345")
                }
            }
        }
    };

    let result = recovery_manager
        .execute_with_retry("chain_rpc_call", chain_rpc_operation)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Chain RPC response: block height 12345");
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);

    let stats = recovery_manager.get_stats("chain_rpc_call").unwrap();
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.successful_retries, 1);
}

#[tokio::test]
async fn test_multiple_operations_statistics() {
    let config = ErrorRecoveryConfig::default();
    let mut recovery_manager = ErrorRecoveryManager::new(config);

    // Run multiple different operations
    let op1 = || async { Ok("Operation 1 success") };
    let op2 = || async { anyhow::bail!("Network error") };
    let op3 = || async { Ok("Operation 3 success") };

    let _result1 = recovery_manager.execute_with_retry("op1", op1).await;
    let _result2: Result<String> = recovery_manager.execute_with_retry("op2", op2).await;
    let _result3 = recovery_manager.execute_with_retry("op3", op3).await;

    // Check overall statistics
    let all_stats = recovery_manager.get_all_stats();
    assert_eq!(all_stats.len(), 3);
    
    assert!(all_stats.contains_key("op1"));
    assert!(all_stats.contains_key("op2"));
    assert!(all_stats.contains_key("op3"));

    // Verify individual stats
    assert_eq!(all_stats["op1"].total_attempts, 1);
    assert_eq!(all_stats["op2"].failed_retries, 1);
    assert_eq!(all_stats["op3"].total_attempts, 1);
}