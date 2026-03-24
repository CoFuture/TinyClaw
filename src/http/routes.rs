//! HTTP routes

use crate::config::{Config, default_config_path};
use crate::gateway::events::{Event, EventEmitter};
use crate::gateway::session::SessionManager;
use crate::gateway::server::ServerState;
use crate::agent::{Agent, SkillRegistry, SessionSkillManager, Scheduler, SessionNotesManager, SessionNoteUpdate, SuggestionManager, MemoryManager, TurnHistoryManager};
use crate::agent::retry::CircuitState;
use crate::metrics::{MetricsCollector, collector::SystemMetrics};
use crate::preferences::{PreferencesManager, UserPreferences, UserPreferencesUpdate};
use crate::ratelimit::RateLimiter;
use crate::types::{SessionHistory, Role};
use axum::{
    extract::{State, Query},
    response::{Json, sse::{Sse, Event as SseEvent}},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use http::StatusCode as HttpStatusCode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use std::convert::Infallible;
use tower_http::services::ServeDir;
use tracing::{info, error};
use std::collections::HashMap;

/// HTTP Server state
pub struct HttpState {
    pub config: Arc<RwLock<Config>>,
    pub session_manager: Arc<SessionManager>,
    pub history_manager: Arc<crate::persistence::HistoryManager>,
    #[allow(dead_code)]
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub start_time: Instant,
    pub metrics: Arc<MetricsCollector>,
    pub rate_limiter: Arc<RateLimiter>,
    pub server_state: ServerState,
    pub skill_registry: Arc<SkillRegistry>,
    pub skill_manager: Arc<SessionSkillManager>,
    pub event_emitter: Arc<EventEmitter>,
    pub scheduler: Arc<Scheduler>,
    pub preferences: Arc<PreferencesManager>,
    pub session_notes: Arc<SessionNotesManager>,
    pub suggestion_manager: Arc<SuggestionManager>,
    pub memory_manager: Arc<MemoryManager>,
    pub turn_history: Arc<TurnHistoryManager>,
    #[allow(dead_code)]
    pub conversation_summary: Arc<RwLock<crate::agent::ConversationSummaryManager>>,
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

/// Active connections response
#[derive(Serialize)]
pub struct ConnectionsResponse {
    pub active_connections: usize,
    pub shutdown_timeout_secs: u64,
}

/// Session export response
#[derive(Serialize)]
pub struct SessionExportResponse {
    pub session_id: String,
    pub exported_at: String,
    pub message_count: usize,
    pub data: SessionHistory,
}

/// Session import request
#[derive(Deserialize)]
pub struct SessionImportRequest {
    pub session_id: String,
    pub data: SessionHistory,
}

/// Session import response
#[derive(Serialize)]
pub struct SessionImportResponse {
    pub success: bool,
    pub session_id: String,
    pub message_count: usize,
    pub error: Option<String>,
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

/// Active connections handler
async fn connections(State(state): State<Arc<HttpState>>) -> Json<ConnectionsResponse> {
    Json(ConnectionsResponse {
        active_connections: state.server_state.active_connections.load(std::sync::atomic::Ordering::SeqCst),
        shutdown_timeout_secs: state.server_state.shutdown_timeout_secs,
    })
}

/// Session export handler - export session history as JSON
async fn session_export(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<SessionExportResponse> {
    if let Some(history) = state.history_manager.get(&session_id) {
        let history = history.read();
        let message_count = history.messages.len();
        Json(SessionExportResponse {
            session_id: session_id.clone(),
            exported_at: Utc::now().to_rfc3339(),
            message_count,
            data: (*history).clone(),
        })
    } else {
        Json(SessionExportResponse {
            session_id,
            exported_at: Utc::now().to_rfc3339(),
            message_count: 0,
            data: SessionHistory::default(),
        })
    }
}

/// Session import handler - import session history from JSON
async fn session_import(
    State(state): State<Arc<HttpState>>,
    Json(request): Json<SessionImportRequest>,
 ) -> (HttpStatusCode, Json<SessionImportResponse>) {
    // Validate that the session_id matches
    if request.data.session_id != request.session_id {
        return (
            HttpStatusCode::BAD_REQUEST,
            Json(SessionImportResponse {
                success: false,
                session_id: request.session_id,
                message_count: 0,
                error: Some("Session ID mismatch between URL and payload".to_string()),
            }),
        );
    }

    // Validate messages have valid roles
    for msg in &request.data.messages {
        if !matches!(msg.role, Role::User | Role::Assistant | Role::System | Role::Tool) {
            return (
                HttpStatusCode::BAD_REQUEST,
                Json(SessionImportResponse {
                    success: false,
                    session_id: request.session_id,
                    message_count: 0,
                    error: Some(format!("Invalid message role: {:?}", msg.role)),
                }),
            );
        }
    }

    // Import the session
    let message_count = request.data.messages.len();
    state.history_manager.import_session(&request.session_id, request.data);

    (HttpStatusCode::OK, Json(SessionImportResponse {
        success: true,
        session_id: request.session_id,
        message_count,
        error: None,
    }))
}

/// Session delete response
#[derive(Serialize)]
pub struct SessionDeleteResponse {
    pub success: bool,
    pub session_id: String,
    pub error: Option<String>,
}

/// Session delete handler
async fn session_delete(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<SessionDeleteResponse>) {
    // Prevent deleting the main session
    if session_id == "main" {
        return (
            HttpStatusCode::BAD_REQUEST,
            Json(SessionDeleteResponse {
                success: false,
                session_id,
                error: Some("Cannot delete the main session".to_string()),
            }),
        );
    }

    // Remove from session manager
    let removed = state.session_manager.remove(&session_id);
    
    if removed.is_none() {
        return (
            HttpStatusCode::NOT_FOUND,
            Json(SessionDeleteResponse {
                success: false,
                session_id,
                error: Some("Session not found".to_string()),
            }),
        );
    }

    // Remove history for this session
    state.history_manager.remove(&session_id);

    info!("HTTP: Deleted session: {}", session_id);

    (
        HttpStatusCode::OK,
        Json(SessionDeleteResponse {
            success: true,
            session_id,
            error: None,
        }),
    )
}

/// Session rename request
#[derive(Deserialize)]
pub struct SessionRenameRequest {
    pub label: Option<String>,
}

/// Session rename response
#[derive(Serialize)]
pub struct SessionRenameResponse {
    pub success: bool,
    pub session_id: String,
    pub label: Option<String>,
    pub error: Option<String>,
}

/// Session rename handler
async fn session_rename(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    Json(request): Json<SessionRenameRequest>,
) -> (HttpStatusCode, Json<SessionRenameResponse>) {
    // Update session label
    let success = state.session_manager.rename(&session_id, request.label.clone());
    
    if !success {
        return (
            HttpStatusCode::NOT_FOUND,
            Json(SessionRenameResponse {
                success: false,
                session_id,
                label: None,
                error: Some("Session not found".to_string()),
            }),
        );
    }

    info!("HTTP: Renamed session {} to {:?}", session_id, request.label);

    (
        HttpStatusCode::OK,
        Json(SessionRenameResponse {
            success: true,
            session_id,
            label: request.label,
            error: None,
        }),
    )
}

/// Config get handler - returns public config only (no secrets)
async fn config_get(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let config = state.config.read();
    let sanitized = serde_json::json!({
        "gateway": {
            "bind": config.gateway.bind,
            "verbose": config.gateway.verbose,
            "dataDir": config.gateway.data_dir,
        },
        "agent": {
            "model": config.agent.model,
            "provider": config.agent.provider,
            "apiBase": config.agent.api_base,
            "workspace": config.agent.workspace,
            // apiKey intentionally omitted - never expose secrets
        },
        "tools": config.tools,
        "models": config.models,
    });
    Json(sanitized)
}

/// Config patch handler
async fn config_patch(
    State(state): State<Arc<HttpState>>,
    Json(new_config): Json<Config>,
 ) -> (HttpStatusCode, Json<Config>) {
    let mut config = state.config.write();
    *config = new_config.clone();
    info!("Configuration updated");
    (HttpStatusCode::OK, Json(config.clone()))
}

/// Config reload response
#[derive(Serialize)]
pub struct ConfigReloadResponse {
    pub success: bool,
    pub message: String,
}

/// Config reload handler - reloads config from disk
async fn config_reload(State(state): State<Arc<HttpState>> ) -> (HttpStatusCode, Json<ConfigReloadResponse>) {
    // Try to find config file
    let config_path = match default_config_path() {
        Some(path) => path,
        None => {
            return (
                HttpStatusCode::INTERNAL_SERVER_ERROR,
                Json(ConfigReloadResponse {
                    success: false,
                    message: "Could not determine config path".to_string(),
                }),
            );
        }
    };

    if !config_path.exists() {
        return (
            HttpStatusCode::NOT_FOUND,
            Json(ConfigReloadResponse {
                success: false,
                message: format!("Config file not found at {:?}", config_path),
            }),
        );
    }

    // Try to load new config
    match crate::config::load_config(&config_path) {
        Ok(new_config) => {
            let mut config = state.config.write();
            *config = new_config;
            info!("Configuration reloaded from {:?}", config_path);
            (
                HttpStatusCode::OK,
                Json(ConfigReloadResponse {
                    success: true,
                    message: format!("Configuration reloaded from {:?}", config_path),
                }),
            )
        }
        Err(e) => {
            error!("Failed to reload config: {}", e);
            (
                HttpStatusCode::INTERNAL_SERVER_ERROR,
                Json(ConfigReloadResponse {
                    success: false,
                    message: format!("Failed to reload config: {}", e),
                }),
            )
        }
    }
}

/// Shutdown handler
async fn shutdown(State(state): State<Arc<HttpState>> ) -> (HttpStatusCode, Json<serde_json::Value>) {
    info!("Shutdown requested via HTTP");
    let _ = state.shutdown_tx.send(());
    (HttpStatusCode::OK, Json(serde_json::json!({ "shuttingDown": true })))
}

/// Sessions list handler
async fn sessions_list(State(state): State<Arc<HttpState>>) -> Json<serde_json::Value> {
    let sessions = state.session_manager.list();
    let now = chrono::Utc::now();
    let session_infos: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            let session = s.read();
            let history = state.history_manager.get(&session.id);
            let msg_count = history.as_ref().map(|h| h.read().messages.len()).unwrap_or(0);
            
            // Get last message preview
            let last_message_preview = history
                .as_ref()
                .and_then(|h| {
                    let msgs = h.read();
                    msgs.messages.iter()
                        .rev()
                        .find(|m| m.role == crate::types::Role::User)
                        .map(|m| {
                            let content = m.content.chars().take(50).collect::<String>();
                            if m.content.len() > 50 {
                                format!("{}...", content)
                            } else {
                                content
                            }
                        })
                });
            
            // Calculate duration since creation (in seconds)
            let duration_secs = (now - session.created_at).num_seconds();
            
            // Check if session is active (last active within 5 minutes)
            let is_active = (now - session.last_active).num_seconds() < 300;
            
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
                "messageCount": msg_count,
                "durationSecs": duration_secs,
                "lastMessagePreview": last_message_preview,
                "isActive": is_active,
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
async fn metrics_handler(State(state): State<Arc<HttpState>>) -> Json<MetricsResponse> {
    // Update circuit breaker state from agent before returning metrics
    let cb_state = state.agent.circuit_breaker_state();
    let cb_state_str = match cb_state {
        CircuitState::Closed => "closed",
        CircuitState::Open => "open",
        CircuitState::HalfOpen => "half_open",
    };
    state.metrics.set_circuit_breaker_state(cb_state_str);

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
        
        // On macOS, use ps command to get RSS in KB
        #[cfg(target_os = "macos")]
        {
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

// ============================================================================
// SSE Event Streaming
// ============================================================================

use std::time::Duration;

/// Query parameters for SSE events endpoint
#[derive(Deserialize)]
pub struct SseQueryParams {
    /// Optional session ID to filter events (only events for this session)
    pub session_id: Option<String>,
}

/// SSE event stream handler - streams real-time events to clients
async fn sse_events(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<SseQueryParams>,
) -> Sse<impl futures_util::Stream<Item = Result<SseEvent, Infallible>>> {
    let mut receiver = state.event_emitter.subscribe();
    let session_filter = params.session_id;
    
    // Create a stream that:
    // 1. Sends an initial "connected" event
    // 2. Forwards all events from the event emitter
    // 3. Sends heartbeat events every 15 seconds to keep connection alive
    let stream = async_stream::stream! {
        // Send initial connected event
        let connected = SseEvent::default()
            .event("connected")
            .data(serde_json::json!({
                "message": "Connected to TinyClaw event stream",
                "filter": session_filter,
            }).to_string());
        yield Ok(connected);
        
        let heartbeat_interval = Duration::from_secs(15);
        let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);
        heartbeat_timer.tick().await; // Skip first immediate tick
        
        loop {
            tokio::select! {
                // Event received from emitter
                event_result = receiver.recv() => {
                    match event_result {
                        Ok(event) => {
                            // Apply session filter if specified
                            let should_emit = if let Some(ref filter) = session_filter {
                                match &event {
                                    Event::TurnStarted { session_id, .. } => session_id == filter,
                                    Event::TurnThinking { session_id, .. } => session_id == filter,
                                    Event::TurnEnded { session_id, .. } => session_id == filter,
                                    Event::TurnUsage { session_id, .. } => session_id == filter,
                                    Event::ContextSummarized { session_id, .. } => session_id == filter,
                                    Event::TurnCancelled { session_id, .. } => session_id == filter,
                                    Event::AssistantText { session_id, .. } => session_id == filter,
                                    Event::AssistantPartial { session_id, .. } => session_id == filter,
                                    Event::AssistantToolUse { session_id, .. } => session_id == filter,
                                    Event::ActionPlanPreview { session_id, .. } => session_id == filter,
                                    Event::ToolResult { session_id, .. } => session_id == filter,
                                    Event::TurnLogUpdated { session_id, .. } => session_id == filter,
                                    Event::TurnLogCompleted { session_id, .. } => session_id == filter,
                                    Event::SessionCreated { session_id, .. } => session_id == filter,
                                    Event::SessionEnded { session_id, .. } => session_id == filter,
                                    Event::Error { session_id, .. } => session_id == filter,
                                    // Status, Heartbeat, and Task events are always broadcast
                                    Event::Status { .. } | Event::Heartbeat { .. } => true,
                                    Event::TaskCreated { .. } | Event::TaskStarted { .. } => true,
                                    Event::TaskProgress { .. } | Event::TaskCompleted { .. } => true,
                                    Event::TaskFailed { .. } | Event::TaskCancelled { .. } => true,
                                    Event::ScheduledTaskCreated { .. } => true,
                                    Event::ScheduledTaskFired { .. } => true,
                                    Event::ScheduledTaskFailed { .. } => true,
                                    Event::ScheduledTaskUpdated { .. } => true,
                                    Event::ScheduledTaskDeleted { .. } => true,
                                    Event::SuggestionGenerated { session_id, .. } => session_id == filter,
                                    Event::SuggestionAccepted { session_id, .. } => session_id == filter,
                                    Event::SuggestionDismissed { session_id, .. } => session_id == filter,
                                    // Action confirmation events - apply session filter
                                    Event::ActionPlanConfirm { session_id, .. } => session_id == filter,
                                    Event::ActionDenied { session_id, .. } => session_id == filter,
                                }
                            } else {
                                // No filter - emit all events
                                true
                            };
                            
                            if should_emit {
                                let event_name = match &event {
                                    Event::TurnStarted { .. } => "turn.started",
                                    Event::TurnThinking { .. } => "turn.thinking",
                                    Event::TurnEnded { .. } => "turn.ended",
                                    Event::TurnUsage { .. } => "turn.usage",
                                    Event::ContextSummarized { .. } => "context.summarized",
                                    Event::TurnCancelled { .. } => "turn.cancelled",
                                    Event::AssistantText { .. } => "assistant.text",
                                    Event::AssistantPartial { .. } => "assistant.partial",
                                    Event::AssistantToolUse { .. } => "assistant.tool_use",
                                    Event::ActionPlanPreview { .. } => "action.plan_preview",
                                    Event::ToolResult { .. } => "tool_result",
                                    Event::TurnLogUpdated { .. } => "turn.log_updated",
                                    Event::TurnLogCompleted { .. } => "turn.log_completed",
                                    Event::SessionCreated { .. } => "session.created",
                                    Event::SessionEnded { .. } => "session.ended",
                                    Event::TaskCreated { .. } => "task.created",
                                    Event::TaskStarted { .. } => "task.started",
                                    Event::TaskProgress { .. } => "task.progress",
                                    Event::TaskCompleted { .. } => "task.completed",
                                    Event::TaskFailed { .. } => "task.failed",
                                    Event::TaskCancelled { .. } => "task.cancelled",
                                    Event::ScheduledTaskCreated { .. } => "scheduled.created",
                                    Event::ScheduledTaskFired { .. } => "scheduled.fired",
                                    Event::ScheduledTaskFailed { .. } => "scheduled.failed",
                                    Event::ScheduledTaskUpdated { .. } => "scheduled.updated",
                                    Event::ScheduledTaskDeleted { .. } => "scheduled.deleted",
                                    Event::SuggestionGenerated { .. } => "suggestion.generated",
                                    Event::SuggestionAccepted { .. } => "suggestion.accepted",
                                    Event::SuggestionDismissed { .. } => "suggestion.dismissed",
                                    Event::ActionPlanConfirm { .. } => "action.plan_confirm",
                                    Event::ActionDenied { .. } => "action.denied",
                                    Event::Error { .. } => "error",
                                    Event::Status { .. } => "status",
                                    Event::Heartbeat { .. } => "heartbeat",
                                };
                                
                                let event = SseEvent::default()
                                    .event(event_name)
                                    .data(serde_json::to_string(&event).unwrap_or_default());
                                yield Ok(event);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Receiver lagged behind, skip this event
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Channel closed, end stream
                            break;
                        }
                    }
                }
                // Heartbeat timer
                _ = heartbeat_timer.tick() => {
                    let heartbeat = SseEvent::default()
                        .event("heartbeat")
                        .data(serde_json::json!({
                            "timestamp": chrono::Utc::now().timestamp(),
                        }).to_string());
                    yield Ok(heartbeat);
                }
            }
        }
    };
    
    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
}

/// Create the router with static files and API routes
pub fn create_router(state: Arc<HttpState>, static_dir: &str) -> Router {
    let metrics_collector = state.metrics.clone();
    
    Router::new()
        .nest_service("/admin", ServeDir::new(static_dir))
        .route("/", get(root_redirect))
        .route("/admin.html", get(root_redirect))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/connections", get(connections))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/ratelimit/{client_id}", get(rate_limit_check))
        .route("/api/config", get(config_get))
        .route("/api/config", axum::routing::patch(config_patch))
        .route("/api/config/reload", axum::routing::post(config_reload))
        .route("/api/shutdown", axum::routing::post(shutdown))
        .route("/api/sessions", get(sessions_list))
        .route("/api/sessions", post(session_create))
        .route("/api/sessions/{id}/messages", get(session_messages))
        .route("/api/sessions/{id}/export", get(session_export))
        .route("/api/sessions/{id}", axum::routing::delete(session_delete))
        .route("/api/sessions/{id}", axum::routing::patch(session_rename))
        .route("/api/sessions/import", post(session_import))
        .route("/api/tools", get(tools_list))
        // Skill management API
        .route("/api/skills", get(skills_list))
        .route("/api/skills", post(skills_create))
        .route("/api/skills/{name}", get(skills_get))
        .route("/api/skills/{name}", axum::routing::put(skills_update))
        .route("/api/skills/{name}", axum::routing::delete(skills_delete))
        // Session skills API
        .route("/api/sessions/{session_id}/skills", get(session_skills_get))
        .route("/api/sessions/{session_id}/skills", axum::routing::post(session_skills_set))
        .route("/api/sessions/{session_id}/skills/{skill_name}", axum::routing::put(session_skills_enable))
        .route("/api/sessions/{session_id}/skills/{skill_name}", axum::routing::delete(session_skills_disable))
        // Scheduled task management API
        .route("/api/scheduled", get(scheduled_list))
        .route("/api/scheduled", post(scheduled_create))
        .route("/api/scheduled/{schedule_id}", get(scheduled_get))
        .route("/api/scheduled/{schedule_id}/pause", axum::routing::post(scheduled_pause))
        .route("/api/scheduled/{schedule_id}/resume", axum::routing::post(scheduled_resume))
        .route("/api/scheduled/{schedule_id}", axum::routing::delete(scheduled_delete))
        .route("/api/scheduled/{schedule_id}/enable", axum::routing::post(scheduled_enable))
        .route("/api/scheduled/{schedule_id}/disable", axum::routing::post(scheduled_disable))
        .route("/api/scheduled/{schedule_id}/fire", axum::routing::post(scheduled_fire_now))
        // User preferences API
        .route("/api/preferences", get(preferences_get))
        .route("/api/preferences", axum::routing::patch(preferences_update))
        // Session notes API
        .route("/api/sessions/{session_id}/notes", get(session_notes_list))
        .route("/api/sessions/{session_id}/notes", post(session_notes_add))
        .route("/api/sessions/{session_id}/notes/{note_id}", axum::routing::put(session_notes_update))
        .route("/api/sessions/{session_id}/notes/{note_id}", axum::routing::delete(session_notes_delete))
        // Session instructions API
        .route("/api/sessions/{session_id}/instructions", get(session_instructions_get))
        .route("/api/sessions/{session_id}/instructions", axum::routing::put(session_instructions_set))
        // Session suggestions API
        .route("/api/sessions/{session_id}/suggestions", get(suggestions_list))
        .route("/api/sessions/{session_id}/suggestions/{suggestion_id}/accept", axum::routing::post(suggestions_accept))
        .route("/api/sessions/{session_id}/suggestions/{suggestion_id}/dismiss", axum::routing::post(suggestions_dismiss))
        // Memory API - long-term fact storage and retrieval
        .route("/api/memory", get(memory_list))
        .route("/api/memory/search", get(memory_search))
        .route("/api/memory", post(memory_add))
        .route("/api/memory/stats", get(memory_stats))
        .route("/api/memory/{fact_id}", axum::routing::delete(memory_delete))
        .route("/api/memory/category/{category}", get(memory_by_category))
        .route("/api/memory/category/{category}", axum::routing::delete(memory_clear_category))
        .route("/api/memory/session/{session_id}", get(memory_for_session))
        // Turn history API - agent turn tracking and analysis
        .route("/api/sessions/{session_id}/turns", get(turns_list))
        .route("/api/sessions/{session_id}/turns/{turn_id}", get(turns_get))
        .route("/api/turns/recent", get(turns_recent))
        .route("/api/turns/stats", get(turns_stats))
        .route("/api/turns/stats/period", get(turns_stats_period))
        .route("/api/turns/export", get(turns_export))
        // Conversation summary API - transient conversation state tracking
        .route("/api/sessions/{session_id}/conversation-summary", get(conversation_summary_get))
        // Summarizer config and history API
        .route("/api/summarizer/config", get(summarizer_config_get))
        .route("/api/summarizer/config", axum::routing::patch(summarizer_config_update))
        .route("/api/summarizer/history", get(summarizer_history_list))
        .route("/api/summarizer/stats", get(summarizer_stats))
        .route("/api/summarizer/session/{session_id}", get(summarizer_history_session))
        // SSE event stream for real-time feedback
        .route("/api/events", get(sse_events))
        .fallback_service(ServeDir::new(static_dir))
        .layer(axum::middleware::from_fn(move |req: http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let metrics = metrics_collector.clone();
            Box::pin(async move {
                let start = std::time::Instant::now();
                let method = req.method().to_string();
                let path = req.uri().path().to_string();

                // Call the next middleware/handler
                let response = next.run(req).await;

                // Record metrics
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                let endpoint = format!("{} {}", method, path);
                
                // Check if response is an error (5xx)
                let is_error = response.status().is_server_error();
                
                metrics.record_request(&endpoint, elapsed_ms, is_error);

                response
            })
        }))
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

/// Session create request
#[derive(Deserialize)]
pub struct SessionCreateRequest {
    pub label: Option<String>,
}

/// Session create response
#[derive(Serialize)]
pub struct SessionCreateResponse {
    pub success: bool,
    pub session_id: String,
    pub label: Option<String>,
    pub kind: String,
    pub error: Option<String>,
}

/// Session create handler - create a new isolated session
async fn session_create(
    State(state): State<Arc<HttpState>>,
    Json(request): Json<SessionCreateRequest>,
) -> (HttpStatusCode, Json<SessionCreateResponse>) {
    use crate::gateway::session::{Session, SessionKind};

    // Create new isolated session
    let mut session = Session::new(SessionKind::Isolated);
    if let Some(ref label) = request.label {
        session = session.with_label(label);
    }

    let session_id = session.id.clone();
    let label = session.label.clone();

    // Register session with session manager
    state.session_manager.create(session);

    // Ensure history manager has an entry for this session
    let _ = state.history_manager.get_or_create(&session_id);

    info!("HTTP: Created session: {}", session_id);

    (
        HttpStatusCode::CREATED,
        Json(SessionCreateResponse {
            success: true,
            session_id,
            label,
            kind: "isolated".to_string(),
            error: None,
        }),
    )
}

/// Tools list response
#[derive(Serialize)]
pub struct ToolsListResponse {
    pub tools: Vec<ToolInfo>,
}

/// Tool info
#[derive(Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tools list handler
async fn tools_list(State(state): State<Arc<HttpState>>) -> Json<ToolsListResponse> {
    let tools = state.agent.list_tools();
    let tool_infos: Vec<ToolInfo> = tools
        .into_iter()
        .map(|t| ToolInfo {
            name: t.name,
            description: t.description,
            input_schema: t.input_schema,
        })
        .collect();

    Json(ToolsListResponse { tools: tool_infos })
}

// ============================================================================
// Skill API Handlers
// ============================================================================

/// Skills list response
#[derive(Serialize)]
pub struct SkillsListResponse {
    pub skills: Vec<SkillInfo>,
}

/// Skill info for API responses
#[derive(Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tool_names: Vec<String>,
    #[serde(default)]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Skills list handler - returns all available skills
async fn skills_list(State(state): State<Arc<HttpState>>) -> Json<SkillsListResponse> {
    let skills = state.skill_registry.list();
    let skill_infos: Vec<SkillInfo> = skills
        .into_iter()
        .map(|s| SkillInfo {
            name: s.name,
            description: s.description,
            instructions: s.instructions,
            tool_names: s.tool_names,
            enabled_by_default: s.enabled_by_default,
            tags: s.tags,
        })
        .collect();

    Json(SkillsListResponse { skills: skill_infos })
}

/// Get a specific skill by name
async fn skills_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    if let Some(skill) = state.skill_registry.get(&name) {
        let info = SkillInfo {
            name: skill.name,
            description: skill.description,
            instructions: skill.instructions,
            tool_names: skill.tool_names,
            enabled_by_default: skill.enabled_by_default,
            tags: skill.tags,
        };
        (HttpStatusCode::OK, Json(serde_json::to_value(info).unwrap()))
    } else {
        (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Skill not found"
        })))
    }
}

/// Create a new skill
async fn skills_create(
    State(state): State<Arc<HttpState>>,
    Json(skill): Json<SkillInfo>,
) -> Result<Json<serde_json::Value>, HttpStatusCode> {
    // Check if skill already exists
    if state.skill_registry.exists(&skill.name) {
        return Err(HttpStatusCode::CONFLICT);
    }

    let new_skill = crate::agent::Skill::new(
        skill.name.clone(),
        skill.description.clone(),
        skill.instructions.clone(),
    )
    .with_tools(skill.tool_names.iter().cloned())
    .with_default_enabled(skill.enabled_by_default);

    state.skill_registry.register(new_skill);

    Ok(Json(serde_json::json!({
        "success": true,
        "name": skill.name
    })))
}

/// Update an existing skill
async fn skills_update(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(skill_info): Json<SkillInfo>,
) -> Result<Json<serde_json::Value>, HttpStatusCode> {
    if !state.skill_registry.exists(&name) {
        return Err(HttpStatusCode::NOT_FOUND);
    }

    let updated_skill = crate::agent::Skill::new(
        skill_info.name.clone(),
        skill_info.description.clone(),
        skill_info.instructions.clone(),
    )
    .with_tools(skill_info.tool_names.iter().cloned())
    .with_default_enabled(skill_info.enabled_by_default)
    .with_tags(skill_info.tags.iter().cloned());

    if state.skill_registry.update(&updated_skill) {
        Ok(Json(serde_json::json!({
            "success": true,
            "name": skill_info.name
        })))
    } else {
        Err(HttpStatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Delete a skill
async fn skills_delete(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    if state.skill_registry.unregister(&name).is_some() {
        // Also remove from all sessions
        state.skill_manager.remove_skill_from_all(&name);
        (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true
        })))
    } else {
        (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Skill not found"
        })))
    }
}

/// Session skills response
#[derive(Serialize)]
pub struct SessionSkillsResponse {
    pub session_id: String,
    pub active_skills: Vec<String>,
    pub available_skills: Vec<String>,
}

/// Get active skills for a session
async fn session_skills_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<SessionSkillsResponse> {
    let active = state.skill_manager.get_active_skills(&session_id);
    let available: Vec<String> = state.skill_registry.list()
        .into_iter()
        .map(|s| s.name)
        .collect();

    Json(SessionSkillsResponse {
        session_id,
        active_skills: active,
        available_skills: available,
    })
}

/// Enable a skill for a session
async fn session_skills_enable(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, skill_name)): axum::extract::Path<(String, String)>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    if state.skill_manager.enable_skill(&session_id, &skill_name) {
        (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "skill": skill_name,
            "session": session_id
        })))
    } else {
        (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Skill not found"
        })))
    }
}

/// Disable a skill for a session
async fn session_skills_disable(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, skill_name)): axum::extract::Path<(String, String)>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    state.skill_manager.disable_skill(&session_id, &skill_name);
    (HttpStatusCode::OK, Json(serde_json::json!({
        "success": true,
        "skill": skill_name,
        "session": session_id
    })))
}

/// Set all active skills for a session
async fn session_skills_set(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    Json(skills): Json<Vec<String>>,
) -> Json<serde_json::Value> {
    state.skill_manager.set_active_skills(&session_id, skills.clone());
    Json(serde_json::json!({
        "success": true,
        "session": session_id,
        "active_skills": skills
    }))
}

// Scheduled task management handlers

use crate::agent::scheduled_task::{ScheduledTaskSummary, ScheduleType};

/// List all scheduled tasks
async fn scheduled_list(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let schedules = state.scheduler.list();
    Json(serde_json::json!({
        "schedules": schedules
    }))
}

/// Create a new scheduled task
async fn scheduled_create(
    State(state): State<Arc<HttpState>>,
    Json(params): Json<serde_json::Value>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "name is required"}))),
    };
    let schedule_type = match params.get("schedule_type").and_then(|v| v.as_str()) {
        Some("cron") => ScheduleType::Cron,
        Some("interval") => ScheduleType::Interval,
        _ => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "schedule_type must be 'cron' or 'interval'"}))),
    };
    let task_description = match params.get("task_description").and_then(|v| v.as_str()) {
        Some(d) => d.to_string(),
        None => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "task_description is required"}))),
    };
    let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "session_id is required"}))),
    };

    let result = match schedule_type {
        ScheduleType::Cron => {
            let cron_expr = match params.get("cron_expression").and_then(|v| v.as_str()) {
                Some(e) => e.to_string(),
                None => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "cron_expression is required for cron schedule"}))),
            };
            state.scheduler.add_cron(name, cron_expr, task_description, session_id)
        }
        ScheduleType::Interval => {
            let interval = match params.get("interval_seconds").and_then(|v| v.as_u64()) {
                Some(i) => i,
                None => return (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "interval_seconds is required for interval schedule"}))),
            };
            Ok(state.scheduler.add_interval(name, interval, task_description, session_id))
        }
    };

    match result {
        Ok(handle) => {
            let st = handle.read();
            let summary = ScheduledTaskSummary::from(&*st);
            (HttpStatusCode::CREATED, Json(serde_json::json!({
                "schedule": summary
            })))
        }
        Err(e) => (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))),
    }
}

/// Get a scheduled task by ID
async fn scheduled_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.get(&schedule_id) {
        Some(handle) => {
            let st = handle.read();
            let summary = ScheduledTaskSummary::from(&*st);
            (HttpStatusCode::OK, Json(serde_json::json!({
                "schedule": summary
            })))
        }
        None => (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Schedule not found"
        }))),
    }
}

/// Pause a scheduled task
async fn scheduled_pause(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.pause(&schedule_id) {
        Ok(()) => (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "schedule_id": schedule_id
        }))),
        Err(e) => (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Resume a paused scheduled task
async fn scheduled_resume(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.resume(&schedule_id) {
        Ok(()) => (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "schedule_id": schedule_id
        }))),
        Err(e) => (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Delete a scheduled task
async fn scheduled_delete(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    let removed = state.scheduler.delete(&schedule_id);
    (HttpStatusCode::OK, Json(serde_json::json!({
        "success": removed.is_some(),
        "schedule_id": schedule_id
    })))
}

/// Enable a scheduled task
async fn scheduled_enable(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.enable(&schedule_id) {
        Ok(()) => (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "schedule_id": schedule_id
        }))),
        Err(e) => (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Disable a scheduled task
async fn scheduled_disable(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.disable(&schedule_id) {
        Ok(()) => (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "schedule_id": schedule_id
        }))),
        Err(e) => (HttpStatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Fire a scheduled task immediately
async fn scheduled_fire_now(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(schedule_id): axum::extract::Path<String>,
) -> (HttpStatusCode, Json<serde_json::Value>) {
    match state.scheduler.fire_now(&schedule_id).await {
        Ok(()) => (HttpStatusCode::OK, Json(serde_json::json!({
            "success": true,
            "schedule_id": schedule_id
        }))),
        Err(e) => (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": e
        }))),
    }
}

// ============================================================================
// User Preferences API Handlers
// ============================================================================

/// Get user preferences
async fn preferences_get(
    State(state): State<Arc<HttpState>>,
) -> Json<UserPreferences> {
    Json(state.preferences.get())
}

/// Update user preferences (partial update)
async fn preferences_update(
    State(state): State<Arc<HttpState>>,
    Json(update): Json<UserPreferencesUpdate>,
) -> Json<UserPreferences> {
    state.preferences.update(update);
    Json(state.preferences.get())
}

// ============================================================================
// Session Notes API Handlers
// ============================================================================

/// List all notes for a session
async fn session_notes_list(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let notes = state.session_notes.list_summaries(&session_id);
    Json(serde_json::json!({
        "sessionId": session_id,
        "notes": notes,
        "count": notes.len(),
    }))
}

/// Add a note to a session
async fn session_notes_add(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (HttpStatusCode, Json<serde_json::Value>)> {
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "content required"}))))?;

    let note = state.session_notes.add(&session_id, content);

    Ok(Json(serde_json::json!({
        "note": note,
        "success": true,
    })))
}

/// Update a note
async fn session_notes_update(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, note_id)): axum::extract::Path<(String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (HttpStatusCode, Json<serde_json::Value>)> {
    let update = SessionNoteUpdate {
        content: payload.get("content").and_then(|v| v.as_str()).map(String::from),
        pinned: payload.get("pinned").and_then(|v| v.as_bool()),
        tags: payload.get("tags").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter().filter_map(|x| x.as_str().map(String::from)).collect()
            })
        }),
    };

    match state.session_notes.update(&session_id, &note_id, update) {
        Some(note) => Ok(Json(serde_json::json!({
            "note": note,
            "success": true,
        }))),
        None => Err((HttpStatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Note not found"})))),
    }
}

/// Delete a note
async fn session_notes_delete(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, note_id)): axum::extract::Path<(String, String)>,
) -> Json<serde_json::Value> {
    let deleted = state.session_notes.delete(&session_id, &note_id);
    Json(serde_json::json!({
        "success": deleted,
        "sessionId": session_id,
        "noteId": note_id,
    }))
}

/// Get session instructions
async fn session_instructions_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let instructions = state.session_manager.get_instructions(&session_id);
    Json(serde_json::json!({
        "sessionId": session_id,
        "instructions": instructions,
    }))
}

/// Set session instructions
async fn session_instructions_set(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let instructions = payload
        .get("instructions")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let success = state.session_manager.set_instructions(&session_id, instructions.clone());
    Json(serde_json::json!({
        "success": success,
        "sessionId": session_id,
        "instructions": instructions,
    }))
}

// ============================================================
// Memory API Endpoints
// ============================================================

/// List all memory facts
async fn memory_list(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let facts = state.memory_manager.list_all();
    Json(serde_json::json!({
        "facts": facts,
        "count": facts.len(),
    }))
}

/// Search memory facts
async fn memory_search(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(|v| v.as_str()).unwrap_or("");
    let facts = state.memory_manager.search(query);
    let summaries: Vec<_> = facts.iter().map(crate::agent::memory::MemoryFactSummary::from).collect();
    Json(serde_json::json!({
        "query": query,
        "facts": summaries,
        "count": summaries.len(),
    }))
}

/// Add a new memory fact
async fn memory_add(
    State(state): State<Arc<HttpState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (HttpStatusCode, Json<serde_json::Value>)> {
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (HttpStatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "content required"}))))?;
    
    let category_str = payload
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let category = crate::agent::memory::FactCategory::from_str(category_str);
    
    let importance = payload
        .get("importance")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;
    
    let source_session = payload
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("http_api")
        .to_string();
    
    let fact = crate::agent::memory::MemoryFact::new(
        content.to_string(),
        category,
        source_session,
        importance,
    );
    
    state.memory_manager.add_fact(fact.clone());
    
    Ok(Json(serde_json::json!({
        "success": true,
        "fact": crate::agent::memory::MemoryFactSummary::from(&fact),
    })))
}

/// Delete a memory fact by ID
async fn memory_delete(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(fact_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let deleted = state.memory_manager.delete_fact(&fact_id);
    Json(serde_json::json!({
        "success": deleted,
        "factId": fact_id,
    }))
}

/// Get facts by category
async fn memory_by_category(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(category): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let cat = crate::agent::memory::FactCategory::from_str(&category);
    let facts = state.memory_manager.get_by_category(&cat);
    let summaries: Vec<_> = facts.iter().map(crate::agent::memory::MemoryFactSummary::from).collect();
    Json(serde_json::json!({
        "category": category,
        "facts": summaries,
        "count": summaries.len(),
    }))
}

/// Clear all facts in a category
async fn memory_clear_category(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(category): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let cat = crate::agent::memory::FactCategory::from_str(&category);
    state.memory_manager.clear_category(&cat);
    Json(serde_json::json!({
        "success": true,
        "category": category,
        "message": "Category cleared",
    }))
}

/// Get facts relevant to a session
async fn memory_for_session(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let facts = state.memory_manager.get_for_session(&session_id, 20);
    let summaries: Vec<_> = facts.iter().map(crate::agent::memory::MemoryFactSummary::from).collect();
    Json(serde_json::json!({
        "sessionId": session_id,
        "facts": summaries,
        "count": summaries.len(),
    }))
}

/// Get memory stats
async fn memory_stats(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    use crate::agent::memory::FactCategory;
    
    let total = state.memory_manager.count();
    let all = state.memory_manager.list_all();
    let decay_stats = state.memory_manager.get_decay_stats();
    
    let mut by_category: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut importance_buckets: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    importance_buckets.insert("high".to_string(), 0); // >= 0.7
    importance_buckets.insert("medium".to_string(), 0); // 0.4-0.7
    importance_buckets.insert("low".to_string(), 0); // < 0.4
    
    for fact in &all {
        *by_category.entry(fact.category.clone()).or_insert(0) += 1;
        
        if fact.importance >= 0.7 {
            *importance_buckets.get_mut("high").unwrap() += 1;
        } else if fact.importance >= 0.4 {
            *importance_buckets.get_mut("medium").unwrap() += 1;
        } else {
            *importance_buckets.get_mut("low").unwrap() += 1;
        }
    }
    
    // Per-category importance breakdown
    let mut category_details: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    for cat_str in by_category.keys() {
        let cat = FactCategory::from_str(cat_str);
        let facts = state.memory_manager.get_by_category(&cat);
        let high = facts.iter().filter(|f| f.importance >= 0.7).count();
        let medium = facts.iter().filter(|f| f.importance >= 0.4 && f.importance < 0.7).count();
        let low = facts.iter().filter(|f| f.importance < 0.4).count();
        category_details.insert(cat_str.clone(), serde_json::json!({
            "high": high,
            "medium": medium,
            "low": low,
            "total": facts.len(),
        }));
    }
    
    Json(serde_json::json!({
        "totalFacts": total,
        "byCategory": by_category,
        "importanceDistribution": importance_buckets,
        "categoryDetails": category_details,
        "decay": {
            "decayCycles": decay_stats.decay_cycles,
            "factsDecayed": decay_stats.facts_decayed,
            "lastDecayAt": decay_stats.last_decay_at,
        },
    }))
}

// ============================================================
// Turn History API Endpoints
// ============================================================

/// List turns for a session
async fn turns_list(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let turns = state.turn_history.get_turns(&session_id);
    Json(serde_json::json!({
        "sessionId": session_id,
        "turns": turns,
        "count": turns.len(),
    }))
}

/// Get a specific turn
async fn turns_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, turn_id)): axum::extract::Path<(String, String)>,
) -> Json<serde_json::Value> {
    match state.turn_history.get_turn(&session_id, &turn_id) {
        Some(turn) => Json(serde_json::json!({
            "success": true,
            "turn": turn,
        })),
        None => Json(serde_json::json!({
            "success": false,
            "error": "Turn not found",
        })),
    }
}

/// Get recent turns across all sessions
async fn turns_recent(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let turns = state.turn_history.get_recent_turns(20);
    Json(serde_json::json!({
        "turns": turns,
        "count": turns.len(),
    }))
}

/// Get turn statistics
async fn turns_stats(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let stats = state.turn_history.get_stats();
    Json(serde_json::json!({
        "stats": stats,
    }))
}

/// Get turn statistics grouped by time period
async fn turns_stats_period(
    State(state): State<Arc<HttpState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    use crate::agent::turn_history::StatsPeriod;
    
    let period_str = params.get("period").map(|s| s.as_str()).unwrap_or("daily");
    let limit: usize = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    
    let period = match period_str {
        "hourly" | "hour" => StatsPeriod::Hourly,
        "weekly" | "week" => StatsPeriod::Weekly,
        _ => StatsPeriod::Daily,
    };
    
    // Limit to reasonable range
    let limit = limit.min(168); // Max 168 hours (1 week of hourly)
    
    let stats = state.turn_history.get_stats_by_period(period, limit);
    Json(serde_json::json!({
        "period": period_str,
        "limit": limit,
        "stats": stats,
    }))
}

/// Export all turn history as JSON for download/backup
async fn turns_export(
    State(state): State<Arc<HttpState>>,
) -> impl axum::response::IntoResponse {
    use axum::http::header;

    let all_turns = state.turn_history.get_all_sessions_turns();
    let turn_count: usize = all_turns.values().map(|t| t.len()).sum();
    
    let body = serde_json::json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "version": "1.0",
        "turn_count": turn_count,
        "sessions": all_turns,
    });

    let json = serde_json::to_string_pretty(&body).unwrap_or_default();
    
    axum::response::Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

// ============================================================
// Conversation Summary API Endpoints
// ============================================================

/// Get conversation summary for a session
async fn conversation_summary_get(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let summary_manager = state.conversation_summary.read();
    match summary_manager.get(&session_id) {
        Some(summary) => {
            let needs_summary = summary.needs_summary();
            Json(serde_json::json!({
                "success": true,
                "sessionId": session_id,
                "summary": {
                    "sessionKey": summary.session_key,
                    "topics": summary.topics,
                    "decisions": summary.decisions,
                    "preferences": summary.preferences,
                    "openQuestions": summary.open_questions,
                    "currentFocus": summary.current_focus,
                    "overview": summary.overview,
                    "turnCount": summary.turn_count,
                    "needsSummary": needs_summary,
                    "lastUpdated": summary.last_updated,
                    "startedAt": summary.started_at,
                },
                "systemPrompt": if needs_summary { summary.to_system_prompt() } else { String::new() },
            }))
        }
        None => {
            Json(serde_json::json!({
                "success": false,
                "error": "No conversation summary found for this session",
            }))
        }
    }
}

// ============================================================
// Suggestions API Endpoints
// ============================================================

/// List suggestions for a session
async fn suggestions_list(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let suggestions = state.suggestion_manager.list_summaries(&session_id);
    Json(serde_json::json!({
        "sessionId": session_id,
        "suggestions": suggestions,
        "count": suggestions.len(),
    }))
}

/// Accept a suggestion
async fn suggestions_accept(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, suggestion_id)): axum::extract::Path<(String, String)>,
) -> Json<serde_json::Value> {
    match state.suggestion_manager.accept(&session_id, &suggestion_id) {
        Some(suggestion) => {
            // Emit suggestion accepted event
            state.event_emitter.emit(Event::SuggestionAccepted {
                session_id: session_id.clone(),
                suggestion_id: suggestion_id.clone(),
                suggestion_type: format!("{:?}", suggestion.suggestion_type),
            });
            Json(serde_json::json!({
                "success": true,
                "sessionId": session_id,
                "suggestionId": suggestion_id,
                "suggestion": suggestion,
            }))
        }
        None => Json(serde_json::json!({
            "success": false,
            "error": "Suggestion not found",
            "sessionId": session_id,
            "suggestionId": suggestion_id,
        })),
    }
}

/// Dismiss a suggestion
async fn suggestions_dismiss(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path((session_id, suggestion_id)): axum::extract::Path<(String, String)>,
) -> Json<serde_json::Value> {
    let dismissed = state.suggestion_manager.dismiss(&session_id, &suggestion_id);
    if dismissed {
        state.event_emitter.emit(Event::SuggestionDismissed {
            session_id: session_id.clone(),
            suggestion_id: suggestion_id.clone(),
        });
    }
    Json(serde_json::json!({
        "success": dismissed,
        "sessionId": session_id,
        "suggestionId": suggestion_id,
    }))
}

// ============================================================================
// Summarizer Configuration and History API
// ============================================================================

/// Get current summarizer configuration
async fn summarizer_config_get(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let config = state.agent.get_summarizer_config();
    Json(serde_json::json!({
        "config": {
            "minMessages": config.min_messages,
            "tokenThreshold": config.token_threshold,
            "enabled": config.enabled,
        }
    }))
}

/// Update summarizer configuration
async fn summarizer_config_update(
    State(state): State<Arc<HttpState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let min_messages = payload.get("minMessages").and_then(|v| v.as_u64()).map(|v| v as usize);
    let token_threshold = payload.get("tokenThreshold").and_then(|v| v.as_u64()).map(|v| v as usize);
    let enabled = payload.get("enabled").and_then(|v| v.as_bool());

    let updated = state.agent.update_summarizer_config(min_messages, token_threshold, enabled);
    Json(serde_json::json!({
        "config": {
            "minMessages": updated.min_messages,
            "tokenThreshold": updated.token_threshold,
            "enabled": updated.enabled,
        }
    }))
}

/// Get summary history statistics
async fn summarizer_stats(
    State(state): State<Arc<HttpState>>,
) -> Json<serde_json::Value> {
    let stats = state.agent.get_summary_stats();
    Json(serde_json::json!({
        "stats": {
            "totalSummaries": stats.total_summaries,
            "totalMessagesSummarized": stats.total_messages_summarized,
            "avgCompressionRatio": stats.avg_compression_ratio,
            "sessionsCount": stats.sessions_count,
        }
    }))
}

/// Get recent summary history
async fn summarizer_history_list(
    State(state): State<Arc<HttpState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit: usize = params.get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    
    let history = state.agent.get_summary_history(limit);
    let entries: Vec<_> = history.into_iter().map(|e| serde_json::json!({
        "sessionId": e.session_id,
        "messagesSummarized": e.messages_summarized,
        "originalTokens": e.original_tokens,
        "summaryTokens": e.summary_tokens,
        "compressionRatio": e.compression_ratio,
        "topics": e.topics,
        "createdAt": e.created_at.to_rfc3339(),
    })).collect();
    
    Json(serde_json::json!({
        "entries": entries,
        "count": entries.len(),
    }))
}

/// Get summary history for a specific session
async fn summarizer_history_session(
    State(state): State<Arc<HttpState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let history = state.agent.get_session_summary_history(&session_id);
    let entries: Vec<_> = history.into_iter().map(|e| serde_json::json!({
        "sessionId": e.session_id,
        "messagesSummarized": e.messages_summarized,
        "originalTokens": e.original_tokens,
        "summaryTokens": e.summary_tokens,
        "compressionRatio": e.compression_ratio,
        "topics": e.topics,
        "createdAt": e.created_at.to_rfc3339(),
    })).collect();
    
    Json(serde_json::json!({
        "sessionId": session_id,
        "entries": entries,
        "count": entries.len(),
    }))
}
