//! HTTP routes

use crate::config::Config;
use crate::gateway::session::SessionManager;
use crate::agent::Agent;
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
use tracing::info;

/// HTTP Server state
pub struct HttpState {
    pub config: Arc<RwLock<Config>>,
    pub session_manager: Arc<SessionManager>,
    #[allow(dead_code)]
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub start_time: Instant,
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

/// Create the router
pub fn create_router(state: Arc<HttpState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/config", get(config_get))
        .route("/api/config", axum::routing::patch(config_patch))
        .route("/api/shutdown", axum::routing::post(shutdown))
        .route("/api/sessions", get(sessions_list))
        .with_state(state)
}
