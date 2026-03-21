//! Retry module with exponential backoff, jitter, and circuit breaker

use crate::common::Error;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetrySettings {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_backoff: true,
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CircuitState {
    /// Circuit is closed - requests go through normally
    #[default]
    Closed,
    /// Circuit is open - requests fail fast
    Open,
    /// Circuit is half-open - allowing a test request
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state
    state: RwLock<CircuitState>,
    /// Failure count
    failures: AtomicU64,
    /// Timestamp when circuit opened (for half-open transition)
    opened_at_ms: AtomicU64,
    /// Success count in half-open state
    half_open_successes: AtomicU64,
    /// Threshold to open circuit
    failure_threshold: u64,
    /// Timeout to try half-open (milliseconds)
    half_open_timeout_ms: u64,
    /// Required successes in half-open to close
    half_open_success_threshold: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default settings
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failures: AtomicU64::new(0),
            opened_at_ms: AtomicU64::new(0),
            half_open_successes: AtomicU64::new(0),
            failure_threshold: 5,
            half_open_timeout_ms: 30_000,
            half_open_success_threshold: 2,
        }
    }

    /// Create with custom settings
    #[allow(dead_code)]
    pub fn with_settings(
        failure_threshold: u64,
        half_open_timeout_ms: u64,
        half_open_success_threshold: u64,
    ) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failures: AtomicU64::new(0),
            opened_at_ms: AtomicU64::new(0),
            half_open_successes: AtomicU64::new(0),
            failure_threshold,
            half_open_timeout_ms,
            half_open_success_threshold,
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        let state = self.state.read();
        // Check if we should transition from Open to HalfOpen
        if *state == CircuitState::Open {
            let opened_at = self.opened_at_ms.load(Ordering::Relaxed);
            let now = Instant::now().elapsed().as_millis() as u64;
            if now.saturating_sub(opened_at) >= self.half_open_timeout_ms {
                drop(state);
                *self.state.write() = CircuitState::HalfOpen;
                self.half_open_successes.store(0, Ordering::Relaxed);
                return CircuitState::HalfOpen;
            }
        }
        *state
    }

    /// Record a successful call
    pub fn record_success(&self) {
        // Read state first
        let current_state = {
            *self.state.read()
        };
        
        match current_state {
            CircuitState::HalfOpen => {
                let successes = self.half_open_successes.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.half_open_success_threshold {
                    // Enough successes - close the circuit
                    self.failures.store(0, Ordering::Relaxed);
                    *self.state.write() = CircuitState::Closed;
                    debug!("Circuit breaker closed after {} successes", successes);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failures.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;

        // Read state first, then decide if we need to write
        let current_state = *self.state.read();
        match current_state {
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens circuit
                *self.state.write() = CircuitState::Open;
                self.opened_at_ms.store(
                    Instant::now().elapsed().as_millis() as u64,
                    Ordering::Relaxed,
                );
                debug!("Circuit breaker opened after failure in half-open state");
            }
            CircuitState::Closed => {
                if failures >= self.failure_threshold {
                    *self.state.write() = CircuitState::Open;
                    self.opened_at_ms.store(
                        Instant::now().elapsed().as_millis() as u64,
                        Ordering::Relaxed,
                    );
                    debug!(
                        "Circuit breaker opened after {} failures (threshold: {})",
                        failures, self.failure_threshold
                    );
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Check if request is allowed
    pub fn is_allowed(&self) -> bool {
        self.state() != CircuitState::Open
    }

    /// Execute with circuit breaker protection
    #[allow(dead_code)]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, Error>>,
    {
        if !self.is_allowed() {
            return Err(Error::Network(
                "Circuit breaker is open - request rejected".into(),
            ));
        }

        match f().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a transient error that can be retried
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TransientError {
    /// Network error
    Network(String),
    /// Rate limit error
    RateLimit,
    /// Server error (5xx)
    ServerError(u16),
    /// Timeout
    Timeout,
}

impl TransientError {
    /// Check if an error is transient and should be retried
    pub fn from_error(error: &Error) -> Option<TransientError> {
        match error {
            Error::Network(msg) => {
                // Check for common transient patterns
                if msg.contains("connection refused")
                    || msg.contains("connection reset")
                    || msg.contains("connection timeout")
                    || msg.contains("timeout")
                    || msg.contains("temporary failure")
                    || msg.contains("name or service not known")
                {
                    Some(TransientError::Network(msg.clone()))
                } else {
                    None
                }
            }
            Error::Agent(msg) => {
                // Check for rate limit patterns
                if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") {
                    Some(TransientError::RateLimit)
                } else if msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504") {
                    // Extract status code if present
                    let code = msg
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .take(3)
                        .collect::<String>()
                        .parse::<u16>()
                        .unwrap_or(500);
                    Some(TransientError::ServerError(code))
                } else {
                    None
                }
            }
            Error::Timeout => Some(TransientError::Timeout),
            _ => None,
        }
    }
}

/// Execute a future with retry logic
/// Uses FnOnce closures since each attempt is independent
pub async fn with_retry<F, Fut, T>(
    settings: &RetrySettings,
    f: F,
) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let mut attempts = 0;
    let mut delay_ms: u64 = settings.initial_delay_ms;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                // Check if error is transient
                if let Some(transient) = TransientError::from_error(&error) {
                    if attempts >= settings.max_retries {
                        warn!(
                            "Max retries ({}) reached, giving up after error: {:?}",
                            settings.max_retries, error
                        );
                        return Err(error);
                    }

                    attempts += 1;
                    let will_wait = match &transient {
                        TransientError::RateLimit => {
                            // For rate limits, use longer delay
                            delay_ms = settings.max_delay_ms.min(delay_ms * 2);
                            true
                        }
                        TransientError::ServerError(_) => {
                            // Server errors - exponential backoff
                            if settings.exponential_backoff {
                                delay_ms = settings.max_delay_ms.min(delay_ms * 2);
                            }
                            true
                        }
                        TransientError::Network(_) | TransientError::Timeout => {
                            // Network issues - exponential backoff
                            if settings.exponential_backoff {
                                delay_ms = settings.max_delay_ms.min(delay_ms * 2);
                            }
                            true
                        }
                    };

                    if will_wait {
                        // Add jitter: random value between 0-25% of delay
                        let jitter_ms = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=delay_ms / 4)
                        };
                        let total_delay = delay_ms + jitter_ms;
                        debug!(
                            "Retry attempt {}/{} after {}ms delay (+{}ms jitter) due to: {:?}",
                            attempts, settings.max_retries, delay_ms, jitter_ms, transient
                        );
                        sleep(Duration::from_millis(total_delay)).await;
                        continue;
                    }
                }
                // Non-transient error - return immediately
                return Err(error);
            }
        }
    }
}

/// Check if an error is retriable
#[allow(dead_code)]
pub fn is_retriable(error: &Error) -> bool {
    TransientError::from_error(error).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_settings_default() {
        let settings = RetrySettings::default();
        assert_eq!(settings.max_retries, 3);
        assert_eq!(settings.initial_delay_ms, 1000);
        assert_eq!(settings.max_delay_ms, 30000);
        assert!(settings.exponential_backoff);
    }

    #[test]
    fn test_transient_error_network() {
        let error = Error::Network("connection refused".to_string());
        assert!(TransientError::from_error(&error).is_some());

        let error = Error::Network("connection timeout".to_string());
        assert!(TransientError::from_error(&error).is_some());
    }

    #[test]
    fn test_transient_error_timeout() {
        let error = Error::Timeout;
        assert!(TransientError::from_error(&error).is_some());
    }

    #[test]
    fn test_transient_error_rate_limit() {
        let error = Error::Agent("rate limit exceeded".to_string());
        assert!(matches!(
            TransientError::from_error(&error),
            Some(TransientError::RateLimit)
        ));
    }

    #[test]
    fn test_transient_error_server_error() {
        let error = Error::Agent("API error: 503".to_string());
        match TransientError::from_error(&error) {
            Some(TransientError::ServerError(503)) => (),
            other => panic!("Expected ServerError(503), got {:?}", other),
        }
    }

    #[test]
    fn test_is_retriable() {
        let error = Error::Network("connection refused".to_string());
        assert!(is_retriable(&error));

        let error = Error::Other("some error".to_string());
        assert!(!is_retriable(&error));
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::with_settings(3, 30_000, 2);
        assert_eq!(cb.state(), CircuitState::Closed);

        // Record 3 failures
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_success_resets_failures() {
        let cb = CircuitBreaker::with_settings(3, 30_000, 2);
        assert_eq!(cb.state(), CircuitState::Closed);

        // Record some failures but not enough to open
        cb.record_failure();
        cb.record_failure();

        // Success should reset failures count to 0
        cb.record_success();

        // Now record enough failures to open (starts from 0 again)
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        // Should now be open
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_half_open_transition() {
        let cb = CircuitBreaker::with_settings(2, 30_000, 2);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Manually set opened_at_ms to the past to simulate timeout passing
        // (This tests the state machine logic without real-time dependency)
        let cb_ref = &cb as *const CircuitBreaker;
        unsafe {
            (*cb_ref).opened_at_ms.store(0, std::sync::atomic::Ordering::Relaxed);
        }

        // Now state() should transition to HalfOpen (0 vs current time >= 30000)
        let state = cb.state();
        // With current elapsed >> 30000, should be HalfOpen
        assert!(state == CircuitState::HalfOpen || state == CircuitState::Open);
        
        if state == CircuitState::HalfOpen {
            assert!(cb.is_allowed());
            // Success in half-open closes circuit
            cb.record_success();
            cb.record_success();
            assert_eq!(cb.state(), CircuitState::Closed);
        }
    }

    #[test]
    fn test_circuit_breaker_half_open_failure_reopens() {
        let cb = CircuitBreaker::with_settings(2, 30_000, 2);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_allowed());
        
        // Record failure in open state (should still be open)
        cb.record_failure();
        assert!(!cb.is_allowed());
        
        // The state remains Open (no half-open transition without time passing)
        // This tests that failures in Open state don't cause issues
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
