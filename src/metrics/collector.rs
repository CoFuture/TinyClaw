#![allow(dead_code)]

//! Metrics collector

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// System metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Total requests received
    pub requests_total: u64,
    /// Requests in the last minute
    pub requests_per_minute: u64,
    /// Total messages processed
    pub messages_total: u64,
    /// Active sessions
    pub active_sessions: usize,
    /// Total sessions created
    pub sessions_total: u64,
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    /// WebSocket connections
    pub ws_connections: usize,
    /// Plugin count
    pub plugins_loaded: usize,
    /// Error count
    pub errors_total: u64,
    /// Memory usage (bytes) - approximate
    pub memory_usage_bytes: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Per-endpoint metrics
#[derive(Debug, Clone, Default)]
pub struct EndpointMetrics {
    /// Request count
    pub requests: u64,
    /// Total response time
    pub total_response_time_ms: f64,
    /// Error count
    pub errors: u64,
}

/// Metrics collector
pub struct MetricsCollector {
    system: Arc<RwLock<SystemMetrics>>,
    endpoints: Arc<RwLock<HashMap<String, EndpointMetrics>>>,
    start_time: Instant,
    recent_requests: Arc<RwLock<Vec<Instant>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            system: Arc::new(RwLock::new(SystemMetrics::default())),
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            recent_requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a request
    pub fn record_request(&self, endpoint: &str, response_time_ms: f64, is_error: bool) {
        // Update system metrics
        {
            let mut system = self.system.write();
            system.requests_total += 1;
            system.avg_response_time_ms = (
                system.avg_response_time_ms * (system.requests_total - 1) as f64 
                + response_time_ms
            ) / system.requests_total as f64;
            
            if is_error {
                system.errors_total += 1;
            }
        }

        // Update recent requests for per-minute calculation
        {
            let now = Instant::now();
            let mut recent = self.recent_requests.write();
            recent.push(now);
            
            // Keep only requests from the last minute
            let cutoff = now - Duration::from_secs(60);
            recent.retain(|t| *t > cutoff);
            
            let mut system = self.system.write();
            system.requests_per_minute = recent.len() as u64;
        }

        // Update endpoint metrics
        {
            let mut endpoints = self.endpoints.write();
            let metrics = endpoints.entry(endpoint.to_string()).or_default();
            metrics.requests += 1;
            metrics.total_response_time_ms += response_time_ms;
            if is_error {
                metrics.errors += 1;
            }
        }
    }

    /// Record a message processed
    pub fn record_message(&self) {
        let mut system = self.system.write();
        system.messages_total += 1;
    }

    /// Update active sessions
    pub fn set_active_sessions(&self, count: usize) {
        let mut system = self.system.write();
        system.active_sessions = count;
    }

    /// Increment total sessions
    pub fn increment_sessions(&self) {
        let mut system = self.system.write();
        system.sessions_total += 1;
    }

    /// Update WebSocket connections
    pub fn set_ws_connections(&self, count: usize) {
        let mut system = self.system.write();
        system.ws_connections = count;
    }

    /// Update plugins loaded
    pub fn set_plugins_loaded(&self, count: usize) {
        let mut system = self.system.write();
        system.plugins_loaded = count;
    }

    /// Update memory usage
    pub fn update_memory_usage(&self) {
        #[cfg(target_os = "macos")]
        {
            // Approximate memory usage (in bytes)
            // This is a simple heuristic - in production you'd use more accurate methods
            let mut system = self.system.write();
            system.memory_usage_bytes = 50 * 1024 * 1024; // Placeholder: 50MB
        }
    }

    /// Update uptime
    pub fn update_uptime(&self) {
        let mut system = self.system.write();
        system.uptime_seconds = self.start_time.elapsed().as_secs();
    }

    /// Get system metrics
    pub fn get_system_metrics(&self) -> SystemMetrics {
        let mut system = self.system.read().clone();
        system.uptime_seconds = self.start_time.elapsed().as_secs();
        system
    }

    /// Get endpoint metrics
    pub fn get_endpoint_metrics(&self) -> HashMap<String, EndpointMetrics> {
        self.endpoints.read().clone()
    }

    /// Get average response time for an endpoint
    pub fn get_endpoint_avg_response_time(&self, endpoint: &str) -> Option<f64> {
        let endpoints = self.endpoints.read();
        endpoints.get(endpoint).and_then(|m| {
            if m.requests > 0 {
                Some(m.total_response_time_ms / m.requests as f64)
            } else {
                None
            }
        })
    }

    /// Reset all metrics
    pub fn reset(&self) {
        *self.system.write() = SystemMetrics::default();
        self.endpoints.write().clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
