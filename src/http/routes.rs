//! HTTP routes

use crate::config::Config;
use crate::gateway::session::SessionManager;
use crate::agent::Agent;
use crate::metrics::{MetricsCollector, collector::SystemMetrics};
use crate::ratelimit::RateLimiter;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use tracing::info;
use std::collections::HashMap;

/// HTTP Server state
pub struct HttpState {
    pub config: Arc<RwLock<Config>>,
    pub session_manager: Arc<SessionManager>,
    pub history_manager: Arc<crate::gateway::history::HistoryManager>,
    #[allow(dead_code)]
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub start_time: Instant,
    pub metrics: Arc<MetricsCollector>,
    pub rate_limiter: Arc<RateLimiter>,
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub sessions: usize,
    pub uptime_seconds: u64,
    pub memory_usage: Option<MemoryInfo>,
}

/// Memory info
#[derive(Serialize)]
pub struct MemoryInfo {
    pub resident_set_kb: u64,
}

/// Status response
#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub model: String,
    pub sessions: usize,
    pub uptime_seconds: u64,
    pub gateway: GatewayStatus,
    pub tools: ToolsStatus,
}

/// Gateway status
#[derive(Serialize)]
pub struct GatewayStatus {
    pub bind_address: String,
    pub verbose: bool,
}

/// Tools status
#[derive(Serialize)]
pub struct ToolsStatus {
    pub exec_enabled: bool,
}

/// Health check handler
async fn health(State(state): State<Arc<HttpState>>) -> Json<HealthResponse> {
    let sessions = state.session_manager.list().len();
    let uptime = state.start_time.elapsed().as_secs();
    
    let memory = get_memory_info();
    
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        sessions,
        uptime_seconds: uptime,
        memory_usage: memory,
    })
}

/// Status handler
async fn status(State(state): State<Arc<HttpState>>) -> Json<StatusResponse> {
    let config = state.config.read();
    let sessions = state.session_manager.list().len();
    let uptime = state.start_time.elapsed().as_secs();
    
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: config.agent.model.clone(),
        sessions,
        uptime_seconds: uptime,
        gateway: GatewayStatus {
            bind_address: config.gateway.bind.clone(),
            verbose: config.gateway.verbose,
        },
        tools: ToolsStatus {
            exec_enabled: config.tools.exec_enabled,
        },
    })
}

/// Config get handler
async fn config_get(State(state): State<Arc<HttpState>>) -> Json<Config> {
    Json(state.config.read().clone())
}

/// Config patch handler
async fn config_patch(
    State(state): State<Arc<HttpState>>,
    Json(new_config): Json<Config>,
) -> (StatusCode, Json<Config>) {
    let mut config = state.config.write();
    *config = new_config.clone();
    info!("Configuration updated");
    (StatusCode::OK, Json(config.clone()))
}

/// Shutdown handler
async fn shutdown(State(state): State<Arc<HttpState>>) -> (StatusCode, Json<serde_json::Value>) {
    info!("Shutdown requested via HTTP");
    let _ = state.shutdown_tx.send(());
    (StatusCode::OK, Json(serde_json::json!({ "shuttingDown": true })))
}

/// Sessions list handler
async fn sessions_list(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let sessions = state.session_manager.list();
    let session_infos: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            let session = s.read();
            serde_json::json!({
                "id": session.id,
                "label": session.label,
                "kind": match &session.kind {
                    crate::gateway::session::SessionKind::Main => "main",
                    crate::gateway::session::SessionKind::Isolated => "isolated",
                    crate::gateway::session::SessionKind::Channel { channel } => {
                        return serde_json::json!({
                            "kind": "channel",
                            "channel": channel
                        });
                    }
                },
                "createdAt": session.created_at.to_rfc3339(),
                "lastActive": session.last_active.to_rfc3339(),
            })
        })
        .collect();

    Json(serde_json::json!({ "sessions": session_infos }))
}

/// Metrics response
#[derive(Serialize)]
pub struct MetricsResponse {
    pub system: SystemMetrics,
    pub endpoints: HashMap<String, EndpointMetricsResponse>,
}

/// Endpoint metrics response
#[derive(Serialize)]
pub struct EndpointMetricsResponse {
    pub requests: u64,
    pub avg_response_time_ms: f64,
    pub errors: u64,
}

/// Metrics handler
async fn metrics(State(state): State<Arc<HttpState>>) -> Json<MetricsResponse> {
    let system = state.metrics.get_system_metrics();
    let endpoints = state.metrics.get_endpoint_metrics();
    
    let endpoint_responses: HashMap<String, EndpointMetricsResponse> = endpoints
        .into_iter()
        .map(|(k, v)| {
            let avg_response = if v.requests > 0 {
                v.total_response_time_ms / v.requests as f64
            } else {
                0.0
            };
            (k, EndpointMetricsResponse {
                requests: v.requests,
                avg_response_time_ms: avg_response,
                errors: v.errors,
            })
        })
        .collect();

    Json(MetricsResponse {
        system,
        endpoints: endpoint_responses,
    })
}

/// Rate limit check response
#[derive(Serialize)]
pub struct RateLimitCheckResponse {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_in_seconds: u64,
}

/// Rate limit check handler
async fn rate_limit_check(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(client_id): axum::extract::Path<String>,
) -> Json<RateLimitCheckResponse> {
    let result = state.rate_limiter.check(&client_id);
    
    Json(RateLimitCheckResponse {
        allowed: result.allowed,
        remaining: result.remaining,
        reset_in_seconds: result.reset_in.as_secs(),
    })
}

/// Get memory info (Linux/macOS)
fn get_memory_info() -> Option<MemoryInfo> {
    #[cfg(unix)]
    {
        use std::fs;
        
        // Try to read from /proc/self/status on Linux
        if let Ok(content) = fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(rss) = parts[1].parse::<u64>() {
                            return Some(MemoryInfo { resident_set_kb: rss });
                        }
                    }
                }
            }
        }
        
        // Fallback for macOS
        #[cfg(target_os = "macos")]
        {
            // On macOS, use ps command
            if let Ok(output) = std::process::Command::new("ps")
                .args(["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
            {
                let rss = String::from_utf8_lossy(&output.stdout);
                if let Ok(rss_kb) = rss.trim().parse::<u64>() {
                    return Some(MemoryInfo { resident_set_kb: rss_kb });
                }
            }
        }
    }
    
    None
}

/// Create the router with static files and API routes
pub fn create_router(state: Arc<HttpState>, static_dir: &str) -> Router {
    Router::new()
        .nest_service("/admin", ServeDir::new(static_dir))
        .route("/", get(root_redirect))
        .route("/admin.html", get(|| async { "/admin/admin.html" }))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/metrics", get(metrics))
        .route("/api/ratelimit/:client_id", get(rate_limit_check))
        .route("/api/config", get(config_get))
        .route("/api/config", axum::routing::patch(config_patch))
        .route("/api/shutdown", axum::routing::post(shutdown))
        .route("/api/sessions", get(sessions_list))
        .route("/api/sessions/:id/messages", get(session_messages))
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state)
}

/// Root path redirect to admin
async fn root_redirect() -> axum::response::Redirect {
    axum::response::Redirect::to("/admin/admin.html")
}

/// Session messages handler
async fn session_messages(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let messages = if let Some(history) = state.history_manager.get(&session_id) {
        let history = history.read();
        history.messages.iter()
            .map(|m| {
                serde_json::json!({
                    "role": format!("{:?}", m.role).to_lowercase(),
                    "content": m.content,
                    "timestamp": m.timestamp.to_rfc3339(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    Json(serde_json::json!({ "messages": messages, "sessionId": session_id }))
}
