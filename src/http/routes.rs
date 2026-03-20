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
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

/// HTTP Server state
pub struct HttpState {
    pub config: Arc<RwLock<Config>>,
    pub session_manager: Arc<SessionManager>,
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
}

/// Health check response
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub sessions: usize,
}

/// Status response
#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub model: String,
    pub sessions: usize,
    pub uptime: u64,
}

/// Health check handler
async fn health(State(state): State<Arc<HttpState>>) -> Json<HealthResponse> {
    let sessions = state.session_manager.list().len();
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        sessions,
    })
}

/// Status handler
async fn status(State(state): State<Arc<HttpState>>) -> Json<StatusResponse> {
    let config = state.config.read();
    let sessions = state.session_manager.list().len();
    
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: config.agent.model.clone(),
        sessions,
        uptime: 0, // TODO: Track uptime
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
