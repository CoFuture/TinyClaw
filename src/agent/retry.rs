//! Retry module with exponential backoff

use crate::common::Error;
use std::time::Duration;
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
                        debug!(
                            "Retry attempt {}/{} after {}ms delay due to: {:?}",
                            attempts, settings.max_retries, delay_ms, transient
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
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
}
