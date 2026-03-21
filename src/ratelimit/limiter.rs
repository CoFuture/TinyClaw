#![allow(dead_code)]

//! Rate limiter implementation

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,  // 60 requests per minute by default
            window: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    /// Create a new config with custom values
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Strict rate limit: 10 requests per minute
    pub fn strict() -> Self {
        Self::new(10, 60)
    }

    /// Relaxed rate limit: 120 requests per minute
    pub fn relaxed() -> Self {
        Self::new(120, 60)
    }
}

/// Rate limit entry for a client
#[derive(Debug)]
struct RateLimitEntry {
    /// Request timestamps
    requests: Vec<Instant>,
    /// Whether client is blocked
    blocked: bool,
    /// Blocked until
    blocked_until: Option<Instant>,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            blocked: false,
            blocked_until: None,
        }
    }

    /// Check if request is allowed and record it
    fn check_and_record(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();

        // Check if block has expired
        if let Some(until) = self.blocked_until {
            if now >= until {
                self.blocked = false;
                self.blocked_until = None;
            } else {
                return false; // Still blocked
            }
        }

        // Remove old requests outside the window
        let cutoff = now - config.window;
        self.requests.retain(|t| *t > cutoff);

        // Check if under limit
        if self.requests.len() >= config.max_requests as usize {
            return false;
        }

        // Record this request
        self.requests.push(now);
        true
    }

    /// Block the client
    fn block(&mut self, duration: Duration) {
        self.blocked = true;
        self.blocked_until = Some(Instant::now() + duration);
    }

    /// Get current request count
    fn request_count(&self) -> u32 {
        self.requests.len() as u32
    }
}

/// Rate limiter
pub struct RateLimiter {
    /// Per-client rate limit entries
    clients: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    /// Configuration
    config: RateLimitConfig,
    /// Block duration when limit exceeded
    block_duration: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with default config
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }

    /// Create a new rate limiter with custom config
    pub fn with_config(config: RateLimitConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
            block_duration: Duration::from_secs(60),
        }
    }

    /// Set block duration
    pub fn with_block_duration(mut self, duration: Duration) -> Self {
        self.block_duration = duration;
        self
    }

    /// Check if request is allowed for a client
    pub fn check(&self, client_id: &str) -> RateLimitResult {
        let mut clients = self.clients.write();
        let entry = clients.entry(client_id.to_string()).or_insert_with(RateLimitEntry::new);

        if entry.check_and_record(&self.config) {
            RateLimitResult {
                allowed: true,
                remaining: self.config.max_requests - entry.request_count(),
                reset_in: self.config.window,
            }
        } else {
            // Block the client if not already blocked
            if !entry.blocked {
                entry.block(self.block_duration);
            }

            let reset_in = entry.blocked_until
                .map(|until| until.saturating_duration_since(Instant::now()))
                .unwrap_or(self.config.window);

            RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_in,
            }
        }
    }

    /// Get remaining requests for a client
    pub fn get_remaining(&self, client_id: &str) -> u32 {
        let clients = self.clients.read();
        clients
            .get(client_id)
            .map(|e| self.config.max_requests - e.request_count())
            .unwrap_or(self.config.max_requests)
    }

    /// Reset rate limit for a client
    pub fn reset(&self, client_id: &str) {
        let mut clients = self.clients.write();
        clients.remove(client_id);
    }

    /// Get number of tracked clients
    pub fn tracked_clients(&self) -> usize {
        self.clients.read().len()
    }

    /// Clean up old entries
    pub fn cleanup(&self) {
        let now = Instant::now();
        let cutoff = now - (self.config.window * 2);
        
        let mut clients = self.clients.write();
        clients.retain(|_, entry| {
            // Keep if not blocked and has recent requests
            if entry.blocked {
                return true;
            }
            // Remove if no recent requests
            entry.requests.last().map(|t| *t > cutoff).unwrap_or(false)
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether request is allowed
    pub allowed: bool,
    /// Remaining requests in current window
    pub remaining: u32,
    /// Time until window resets
    pub reset_in: Duration,
}

impl RateLimitResult {
    /// Create a successful result
    pub fn success(remaining: u32, reset_in: Duration) -> Self {
        Self {
            allowed: true,
            remaining,
            reset_in,
        }
    }

    /// Create a rate limited result
    pub fn rate_limited(reset_in: Duration) -> Self {
        Self {
            allowed: false,
            remaining: 0,
            reset_in,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::with_config(RateLimitConfig::new(5, 1));
        
        // Should allow 5 requests
        for _ in 0..5 {
            let result = limiter.check("test_client");
            assert!(result.allowed);
        }
        
        // 6th request should be denied
        let result = limiter.check("test_client");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rate_limiter_different_clients() {
        let limiter = RateLimiter::with_config(RateLimitConfig::new(2, 1));
        
        // Each client has their own limit
        let result1 = limiter.check("client1");
        let result2 = limiter.check("client2");
        
        assert!(result1.allowed);
        assert!(result2.allowed);
    }
}
