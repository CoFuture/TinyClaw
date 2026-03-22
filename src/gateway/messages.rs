//! Message handlers

use crate::agent::tools::ToolExecutor;
use crate::common::{Error, Result};
use crate::config::Config;
use crate::gateway::events::{Event, EventEmitter};
use crate::persistence::HistoryManager;
use crate::gateway::protocol::{error_codes::*, *};
use crate::gateway::session::SessionManager;
use crate::agent::{Agent, SessionSkillManager, TaskManager, Scheduler};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

lazy_static::lazy_static! {
    static ref TOOL_EXECUTOR: ToolExecutor = ToolExecutor::new();
}

/// A server-generated unique request ID for tracing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Generate a new unique request ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "req:{}", &self.0[..8])
    }
}

/// Message handler context
#[derive(Clone)]
#[allow(clippy::too_many_arguments)]
pub struct HandlerContext {
    pub session_manager: Arc<SessionManager>,
    pub history_manager: Arc<HistoryManager>,
    pub event_emitter: Arc<EventEmitter>,
    pub config: Arc<RwLock<Config>>,
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub skill_manager: Arc<SessionSkillManager>,
    pub task_manager: Arc<TaskManager>,
    pub scheduler: Arc<Scheduler>,
}

impl HandlerContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_manager: Arc<SessionManager>,
        history_manager: Arc<HistoryManager>,
        event_emitter: Arc<EventEmitter>,
        config: Arc<RwLock<Config>>,
        agent: Arc<Agent>,
        shutdown_tx: broadcast::Sender<()>,
        skill_manager: Arc<SessionSkillManager>,
        task_manager: Arc<TaskManager>,
        scheduler: Arc<Scheduler>,
    ) -> Self {
        Self {
            session_manager,
            history_manager,
            event_emitter,
            config,
            agent,
            shutdown_tx,
            skill_manager,
            task_manager,
            scheduler,
        }
    }
}

/// Handle an incoming request
pub async fn handle_request(
    ctx: &HandlerContext,
    request: Request,
) -> Option<Response> {
    let request_id = RequestId::new();
    let jsonrpc_id = request.id().map(String::from);
    let method = request.method().to_string();
    let params = request.params().clone();

    info!(
        request_id = %request_id,
        method = %method,
        jsonrpc_id = %jsonrpc_id.as_deref().unwrap_or("none"),
        "Incoming JSON-RPC request"
    );

    let result = match method.as_str() {
        methods::PING => handle_ping(jsonrpc_id.clone()).await,
        methods::SESSIONS_LIST => handle_sessions_list(ctx, jsonrpc_id.clone(), params).await,
        methods::SESSIONS_SEND => handle_sessions_send(ctx, request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::SESSIONS_HISTORY => handle_sessions_history(ctx, jsonrpc_id.clone(), params).await,
        methods::SESSIONS_DELETE => handle_sessions_delete(ctx, jsonrpc_id.clone(), params).await,
        methods::SESSION_RENAME => handle_session_rename(ctx, jsonrpc_id.clone(), params).await,
        methods::SESSION_CANCEL => handle_session_cancel(ctx, jsonrpc_id.clone(), params).await,
        methods::AGENT_TURN => handle_agent_turn(ctx, request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::AGENT_SPAWN => handle_agent_spawn(ctx, jsonrpc_id.clone(), params).await,
        methods::EXEC => handle_exec(request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::TOOLS_LIST => handle_tools_list(jsonrpc_id.clone()).await,
        methods::TOOL_EXECUTE => handle_tool_execute(request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::STATUS => handle_status(ctx, jsonrpc_id.clone()).await,
        methods::SHUTDOWN => handle_shutdown(ctx, jsonrpc_id.clone()).await,
        methods::AGENT_CIRCUIT_BREAKER => handle_agent_circuit_breaker(ctx, jsonrpc_id.clone()).await,
        methods::TASK_CREATE => handle_task_create(ctx, jsonrpc_id.clone(), params).await,
        methods::TASK_LIST => handle_task_list(ctx, jsonrpc_id.clone(), params).await,
        methods::TASK_GET => handle_task_get(ctx, jsonrpc_id.clone(), params).await,
        methods::TASK_START => handle_task_start(ctx, jsonrpc_id.clone(), params).await,
        methods::TASK_CANCEL => handle_task_cancel(ctx, jsonrpc_id.clone(), params).await,
        methods::TASK_REMOVE => handle_task_remove(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_CREATE => handle_scheduled_create(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_LIST => handle_scheduled_list(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_GET => handle_scheduled_get(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_PAUSE => handle_scheduled_pause(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_RESUME => handle_scheduled_resume(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_DELETE => handle_scheduled_delete(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_ENABLE => handle_scheduled_enable(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_DISABLE => handle_scheduled_disable(ctx, jsonrpc_id.clone(), params).await,
        methods::SCHEDULED_FIRE_NOW => handle_scheduled_fire_now(ctx, jsonrpc_id.clone(), params).await,
        
        _ => Err(Error::Protocol(format!("Unknown method: {}", method))),
    };

    match result {
        Ok(value) => {
            info!(request_id = %request_id, method = %method, "JSON-RPC request succeeded");
            Some(ResponseSuccess::new(jsonrpc_id, value).into())
        }
        Err(e) => {
            error!(request_id = %request_id, method = %method, error = %e, "JSON-RPC request failed");
            let err_response = map_error_to_response(jsonrpc_id, &e);
            Some(err_response.into())
        }
    }
}

/// Map internal Error to JSON-RPC ResponseError with proper codes and recovery suggestions
fn map_error_to_response(id: Option<String>, error: &Error) -> ResponseError {
    match error {
        Error::SessionNotFound(session_id) => {
            ResponseError::with_recovery(
                id,
                SESSION_NOT_FOUND,
                format!("Session not found: {}", session_id),
                "Check the session ID or create a new session using sessions.create",
            )
        }
        Error::Agent(msg) => {
            // Check for common agent errors and provide specific recovery
            let (code, recovery) = if msg.contains("API key") || msg.contains("api key") {
                (
                    AUTH_ERROR,
                    "Check your API key configuration. Set the ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable.",
                )
            } else if msg.contains("rate limit") {
                (
                    RATE_LIMIT_ERROR,
                    "Rate limit exceeded. Wait a moment before retrying or increase the rate limit in config.",
                )
            } else if msg.contains("timeout") || msg.contains("Timeout") {
                (
                    TIMEOUT_ERROR,
                    "Request timed out. Try again with a shorter request or check network connectivity.",
                )
            } else if msg.contains("model") {
                (
                    AGENT_ERROR,
                    "Model configuration issue. Check the model name in your config and ensure it's available.",
                )
            } else {
                (
                    AGENT_ERROR,
                    "An error occurred with the AI agent. Check logs for details and try again.",
                )
            };
            ResponseError::with_recovery(id, code, msg.clone(), recovery)
        }
        Error::Tool(msg) => {
            ResponseError::with_recovery(
                id,
                TOOL_ERROR,
                format!("Tool execution failed: {}", msg),
                "Check the tool name and parameters. Use tools.list to see available tools.",
            )
        }
        Error::Config(msg) => {
            ResponseError::with_recovery(
                id,
                CONFIG_ERROR,
                format!("Configuration error: {}", msg),
                "Check your config.json file and ensure all required fields are present and valid.",
            )
        }
        Error::Network(msg) => {
            let recovery = if msg.contains("connection") || msg.contains("connect") {
                "Check your network connection and ensure the API endpoint is reachable."
            } else if msg.contains("dns") {
                "DNS resolution failed. Check your network settings and API base URL."
            } else {
                "A network error occurred. Check your connection and try again."
            };
            ResponseError::with_recovery(id, NETWORK_ERROR, msg.clone(), recovery)
        }
        Error::Protocol(msg) => {
            ResponseError::with_recovery(
                id,
                PROTOCOL_ERROR,
                format!("Protocol error: {}", msg),
                "Ensure the request follows the JSON-RPC 2.0 specification with correct parameter types.",
            )
        }
        Error::Auth(msg) => {
            ResponseError::with_recovery(
                id,
                AUTH_ERROR,
                format!("Authentication error: {}", msg),
                "Check your API keys are correctly configured in the environment or config file.",
            )
        }
        Error::Timeout => {
            ResponseError::with_recovery(
                id,
                TIMEOUT_ERROR,
                "Request timed out",
                "The request took too long to complete. Try again or increase the timeout setting.",
            )
        }
        Error::Cancelled => {
            ResponseError::with_recovery(
                id,
                INTERNAL_ERROR,
                "Request was cancelled",
                "The request was cancelled. This may be due to a shutdown. Try again if needed.",
            )
        }
        Error::Io(msg) => {
            ResponseError::with_recovery(
                id,
                INTERNAL_ERROR,
                format!("IO error: {}", msg),
                "A file system error occurred. Check disk space and file permissions.",
            )
        }
        Error::Json(msg) => {
            ResponseError::with_recovery(
                id,
                INVALID_REQUEST,
                format!("JSON parsing error: {}", msg),
                "The request body contains invalid JSON. Check the request format.",
            )
        }
        Error::WebSocket(msg) => {
            ResponseError::with_recovery(
                id,
                WS_ERROR,
                format!("WebSocket error: {}", msg),
                "A WebSocket error occurred. Check network connectivity and try reconnecting.",
            )
        }
        Error::Plugin(msg) => {
            ResponseError::with_recovery(
                id,
                INTERNAL_ERROR,
                format!("Plugin error: {}", msg),
                "A plugin error occurred. Check plugin configuration and logs.",
            )
        }
        Error::Other(msg) => {
            // Check for common patterns to give better recovery
            let recovery = if msg.contains("not configured") {
                "Required configuration is missing. Check config.json or environment variables."
            } else {
                "An unexpected error occurred. Check logs for details."
            };
            ResponseError::with_recovery(id, INTERNAL_ERROR, msg.clone(), recovery)
        }
    }
}

/// Generate system prompt supplement from active skills for a session
fn generate_skill_prompt(ctx: &HandlerContext, session_key: &str) -> Option<String> {
    let active_skills = ctx.skill_manager.get_active_skills(session_key);
    if active_skills.is_empty() {
        return None;
    }

    let mut prompt = String::from("\n\n## Active Skills\n\n");
    prompt.push_str("The following skills are available for this conversation:\n\n");

    for skill_name in &active_skills {
        if let Some(skill) = ctx.skill_manager.get_skill(skill_name) {
            prompt.push_str(&format!("### {}\n", skill.name));
            prompt.push_str(&format!("{}\n\n", skill.description));
            prompt.push_str(&format!("Instructions: {}\n", skill.instructions));
            if !skill.tool_names.is_empty() {
                prompt.push_str(&format!("Tools: {}\n\n", skill.tool_names.join(", ")));
            }
        }
    }

    Some(prompt)
}

/// Handle ping
async fn handle_ping(_id: Option<String>) -> Result<serde_json::Value> {
    Ok(serde_json::json!({ "pong": true }))
}

/// Handle sessions.list
async fn handle_sessions_list(
    ctx: &HandlerContext,
    _id: Option<String>,
    _params: serde_json::Value,
) -> Result<serde_json::Value> {
    let sessions = ctx.session_manager.list();
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

    Ok(serde_json::json!({ "sessions": session_infos }))
}

/// Handle sessions.history
async fn handle_sessions_history(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    if let Some(history) = ctx.history_manager.get(session_key) {
        let history = history.read();
        Ok(serde_json::json!({
            "sessionId": history.session_id,
            "messages": history.messages,
        }))
    } else {
        Ok(serde_json::json!({
            "sessionId": session_key,
            "messages": [],
        }))
    }
}

/// Handle sessions.delete
async fn handle_sessions_delete(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("sessionKey required".to_string()))?
        .to_string();

    // Prevent deleting the main session
    if session_key == "main" {
        return Err(Error::Protocol("Cannot delete the main session".to_string()));
    }

    // Remove from session manager
    let removed = ctx.session_manager.remove(&session_key);
    
    if removed.is_none() {
        return Err(Error::SessionNotFound(session_key.to_string()));
    }

    // Remove history for this session
    ctx.history_manager.remove(&session_key);

    info!(session_id = %session_key, "Deleted session");

    Ok(serde_json::json!({
        "deleted": true,
        "sessionId": session_key,
    }))
}

/// Handle session.rename
async fn handle_session_rename(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("sessionKey required".to_string()))?
        .to_string();

    // Label is optional - if null or omitted, clears the label
    let new_label = params
        .get("label")
        .and_then(|v| v.as_str().map(String::from))
        .or_else(|| {
            params.get("label")
                .and_then(|v| v.as_str().map(String::from))
        });

    // Update session label
    let success = ctx.session_manager.rename(&session_key, new_label.clone());
    
    if !success {
        return Err(Error::SessionNotFound(session_key.to_string()));
    }

    info!(session_id = %session_key, ?new_label, "Renamed session");

    Ok(serde_json::json!({
        "success": true,
        "sessionId": session_key,
        "label": new_label,
    }))
}

/// Handle session.cancel - cancel an ongoing agent turn
async fn handle_session_cancel(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    // Try to cancel the turn
    let was_active = ctx.agent.cancel_turn(session_key);
    
    if was_active {
        // Emit cancellation event
        ctx.event_emitter.emit(Event::TurnCancelled {
            session_id: session_key.to_string(),
        });
        info!(session_id = %session_key, "Cancelled ongoing turn");
    } else {
        debug!(session_id = %session_key, "No active turn to cancel");
    }

    Ok(serde_json::json!({
        "success": true,
        "sessionId": session_key,
        "cancelled": was_active,
    }))
}

/// Handle sessions.send
async fn handle_sessions_send(
    ctx: &HandlerContext,
    request_id: RequestId,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("sessionKey required".to_string()))?;
    
    let message = params
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("message required".to_string()))?;

    debug!("[{}] Session {} received message: {}", request_id, session_key, message);
    
    // Add to history
    ctx.history_manager.add_message(
        session_key,
        crate::types::Message::user(message),
    );
    
    // Forward to agent
    ctx.agent.send_message(session_key, message, None).await?;

    Ok(serde_json::json!({ "sent": true }))
}

/// Handle agent.turn (send message to agent and get response)
async fn handle_agent_turn(
    ctx: &HandlerContext,
    request_id: RequestId,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let message = params
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("message required".to_string()))?;

    let session_key = params
        .get("sessionKey")
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    debug!("[{}] Agent turn: session={}", request_id, session_key);

    // Emit turn started event
    ctx.event_emitter.emit(Event::TurnStarted {
        session_id: session_key.to_string(),
        message: message.to_string(),
    });

    // Add user message to history
    ctx.history_manager.add_message(
        session_key,
        crate::types::Message::user(message),
    );

    // Generate skill prompt for this session
    let skill_prompt = generate_skill_prompt(ctx, session_key);
    
    // Emit thinking event
    ctx.event_emitter.emit(Event::TurnThinking {
        session_id: session_key.to_string(),
    });

    // Use streaming mode to get partial text updates
    let session_id_clone = session_key.to_string();
    let event_emitter_clone = ctx.event_emitter.clone();
    
    let response: Result<String> = ctx.agent.send_message_streaming(
        session_key,
        message,
        &[],
        skill_prompt.as_deref(),
        move |chunk| {
            event_emitter_clone.emit(Event::AssistantPartial {
                session_id: session_id_clone.clone(),
                text: chunk,
            });
        },
    ).await;

    // Handle cancellation specially
    if response.is_err() {
        let err = response.as_ref().err().unwrap();
        if matches!(err, crate::common::Error::Cancelled) {
            // Emit cancellation event
            ctx.event_emitter.emit(Event::TurnCancelled {
                session_id: session_key.to_string(),
            });
            return Err(Error::Cancelled);
        }
    }

    let response = response?;

    // Add assistant response to history
    ctx.history_manager.add_message(
        session_key,
        crate::types::Message::assistant(&response),
    );

    // Emit assistant text event
    ctx.event_emitter.emit(Event::AssistantText {
        session_id: session_key.to_string(),
        text: response.clone(),
    });

    // Emit turn ended event
    ctx.event_emitter.emit(Event::TurnEnded {
        session_id: session_key.to_string(),
        response: response.clone(),
    });

    Ok(serde_json::json!({
        "text": response
    }))
}

/// Handle agent.spawn (create a new isolated session)
async fn handle_agent_spawn(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    use crate::gateway::session::{Session, SessionKind};

    // Extract optional label from params
    let label = params
        .get("label")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Create new isolated session
    let mut session = Session::new(SessionKind::Isolated);
    if let Some(ref l) = label {
        session = session.with_label(l);
    }

    let session_id = session.id.clone();

    // Register session with session manager
    ctx.session_manager.create(session);

    // Ensure history manager has an entry for this session
    let _ = ctx.history_manager.get_or_create(&session_id);

    info!(session_id = %session_id, kind = "isolated", "Created new session");

    Ok(serde_json::json!({
        "session_id": session_id,
        "label": label,
        "kind": "isolated"
    }))
}

/// Handle exec
async fn handle_exec(
    request_id: RequestId,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("command required".to_string()))?;

    let timeout = params
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(30000);

    info!(request_id = %request_id, command = %command, timeout_ms = %timeout, "Executing command");

    // Execute command using tokio
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await?;

    Ok(serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
        "exitCode": output.status.code().unwrap_or(-1),
    }))
}

/// Handle tools.list
async fn handle_tools_list(_id: Option<String>) -> Result<serde_json::Value> {
    let tools = TOOL_EXECUTOR.list_tools();
    Ok(serde_json::json!({ "tools": tools }))
}

/// Handle tools.execute
async fn handle_tool_execute(
    request_id: RequestId,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("tool name required".to_string()))?;

    let tool_input = params.get("input").cloned().unwrap_or(serde_json::Value::Null);

    debug!("[{}] Tool execute: {}", request_id, tool_name);

    let result = TOOL_EXECUTOR.execute(tool_name, tool_input).await;

    Ok(serde_json::json!({
        "success": result.success,
        "output": result.output,
        "error": result.error
    }))
}

/// Handle status
async fn handle_status(
    ctx: &HandlerContext,
    _id: Option<String>,
) -> Result<serde_json::Value> {
    let config = ctx.config.read();
    let agent_config = &config.agent;
    
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "model": agent_config.model,
        "sessions": ctx.session_manager.list().len(),
    }))
}

/// Handle agent.circuit_breaker - get AI provider circuit breaker state
async fn handle_agent_circuit_breaker(
    ctx: &HandlerContext,
    _id: Option<String>,
) -> Result<serde_json::Value> {
    use crate::agent::retry::CircuitState;
    
    let state = ctx.agent.circuit_breaker_state();
    let state_str = match state {
        CircuitState::Closed => "closed",
        CircuitState::Open => "open",
        CircuitState::HalfOpen => "half_open",
    };
    
    Ok(serde_json::json!({
        "state": state_str,
    }))
}

/// Handle task.create - create a new background task
async fn handle_task_create(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let description = params
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("description required".to_string()))?;

    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    let task_handle = ctx.task_manager.create_task(description, session_id).await;
    let task = task_handle.read();
    
    let summary = crate::agent::TaskSummary::from(&*task);

    info!(task_id = %task.id, description = %description, "Created task");

    Ok(serde_json::json!({
        "task": {
            "id": task.id,
            "description": task.description,
            "sessionId": task.session_id,
            "state": task.state.as_str(),
            "createdAt": task.created_at.to_rfc3339(),
        },
        "summary": summary,
    }))
}

/// Handle task.list - list all tasks
async fn handle_task_list(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let state_filter = params
        .get("state")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "pending" => Some(crate::agent::TaskState::Pending),
            "running" => Some(crate::agent::TaskState::Running),
            "completed" => Some(crate::agent::TaskState::Completed),
            "failed" => Some(crate::agent::TaskState::Failed),
            "cancelled" => Some(crate::agent::TaskState::Cancelled),
            _ => None,
        });

    let tasks = ctx.task_manager.list_tasks(state_filter).await;
    let counts = ctx.task_manager.task_counts().await;

    Ok(serde_json::json!({
        "tasks": tasks,
        "counts": {
            "pending": counts.get(&crate::agent::TaskState::Pending).unwrap_or(&0),
            "running": counts.get(&crate::agent::TaskState::Running).unwrap_or(&0),
            "completed": counts.get(&crate::agent::TaskState::Completed).unwrap_or(&0),
            "failed": counts.get(&crate::agent::TaskState::Failed).unwrap_or(&0),
            "cancelled": counts.get(&crate::agent::TaskState::Cancelled).unwrap_or(&0),
        },
    }))
}

/// Handle task.get - get task details
async fn handle_task_get(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let task_id = params
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("taskId required".to_string()))?;

    let task_handle = ctx.task_manager.get_task(task_id).await
        .ok_or_else(|| Error::Protocol(format!("Task not found: {}", task_id)))?;

    let task = task_handle.read();

    Ok(serde_json::json!({
        "task": {
            "id": task.id,
            "description": task.description,
            "sessionId": task.session_id,
            "state": task.state.as_str(),
            "steps": task.steps,
            "result": task.result,
            "error": task.error,
            "createdAt": task.created_at.to_rfc3339(),
            "startedAt": task.started_at.map(|t| t.to_rfc3339()),
            "completedAt": task.completed_at.map(|t| t.to_rfc3339()),
            "progressPercent": task.progress_percent(),
            "metadata": task.metadata,
        }
    }))
}

/// Handle task.start - start a pending task
async fn handle_task_start(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let task_id = params
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("taskId required".to_string()))?;

    ctx.task_manager.start_task(task_id).await?;

    info!(task_id = %task_id, "Started task");

    Ok(serde_json::json!({
        "success": true,
        "taskId": task_id,
    }))
}

/// Handle task.cancel - cancel a running task
async fn handle_task_cancel(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let task_id = params
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("taskId required".to_string()))?;

    let cancelled = ctx.task_manager.cancel_task(task_id).await?;

    info!(task_id = %task_id, cancelled = %cancelled, "Cancelled task");

    Ok(serde_json::json!({
        "success": true,
        "taskId": task_id,
        "cancelled": cancelled,
    }))
}

/// Handle task.remove - remove a completed task
async fn handle_task_remove(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let task_id = params
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("taskId required".to_string()))?;

    let removed = ctx.task_manager.remove_task(task_id).await;

    if removed.is_some() {
        info!(task_id = %task_id, "Removed task");
    }

    Ok(serde_json::json!({
        "success": removed.is_some(),
        "taskId": task_id,
    }))
}

/// Handle scheduled.create - create a new scheduled task
async fn handle_scheduled_create(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("name required".to_string()))?;

    let schedule_type = params
        .get("scheduleType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleType required (cron or interval)".to_string()))?;

    let task_description = params
        .get("taskDescription")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("taskDescription required".to_string()))?;

    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    let schedule_id = match schedule_type {
        "cron" => {
            let cron_expression = params
                .get("cronExpression")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Protocol("cronExpression required for cron schedule".to_string()))?;
            
            let handle = ctx.scheduler.add_cron(name, cron_expression, task_description, session_id)
                .map_err(Error::Protocol)?;
            let id = handle.read().id.clone();
            id
        }
        "interval" => {
            let interval_seconds = params
                .get("intervalSeconds")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Error::Protocol("intervalSeconds required for interval schedule".to_string()))?;
            
            let handle = ctx.scheduler.add_interval(name, interval_seconds, task_description, session_id);
            let id = handle.read().id.clone();
            id
        }
        _ => return Err(Error::Protocol(
            format!("Invalid scheduleType: {}. Must be 'cron' or 'interval'", schedule_type)
        )),
    };

    let schedule = ctx.scheduler.get(&schedule_id)
        .ok_or_else(|| Error::Protocol(format!("Failed to get created schedule: {}", schedule_id)))?;
    let st = schedule.read();
    let summary = crate::agent::ScheduledTaskSummary::from(&*st);

    info!(schedule_id = %schedule_id, name = %name, schedule_type = %schedule_type, "Created scheduled task");

    Ok(serde_json::json!({
        "schedule": {
            "id": schedule_id,
            "name": st.name,
            "scheduleType": st.schedule_type.as_str(),
            "cronExpression": st.cron_expression,
            "intervalSeconds": st.interval_seconds,
            "taskDescription": st.task_description,
            "sessionId": st.session_id,
            "enabled": st.enabled,
            "paused": st.paused,
            "nextRunAt": st.next_run_at.map(|t| t.to_rfc3339()),
            "lastRunAt": st.last_run_at.map(|t| t.to_rfc3339()),
            "runCount": st.run_count,
            "createdAt": st.created_at.to_rfc3339(),
        },
        "summary": summary,
    }))
}

/// Handle scheduled.list - list all scheduled tasks
async fn handle_scheduled_list(
    ctx: &HandlerContext,
    _id: Option<String>,
    _params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedules = ctx.scheduler.list();

    Ok(serde_json::json!({
        "schedules": schedules,
        "count": schedules.len(),
    }))
}

/// Handle scheduled.get - get a scheduled task by ID
async fn handle_scheduled_get(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    let schedule = ctx.scheduler.get(schedule_id)
        .ok_or_else(|| Error::Protocol(format!("Schedule not found: {}", schedule_id)))?;

    let st = schedule.read();

    Ok(serde_json::json!({
        "schedule": {
            "id": st.id,
            "name": st.name,
            "scheduleType": st.schedule_type.as_str(),
            "cronExpression": st.cron_expression,
            "intervalSeconds": st.interval_seconds,
            "taskDescription": st.task_description,
            "sessionId": st.session_id,
            "enabled": st.enabled,
            "paused": st.paused,
            "nextRunAt": st.next_run_at.map(|t| t.to_rfc3339()),
            "lastRunAt": st.last_run_at.map(|t| t.to_rfc3339()),
            "runCount": st.run_count,
            "lastTaskId": st.last_task_id,
            "createdAt": st.created_at.to_rfc3339(),
            "updatedAt": st.updated_at.to_rfc3339(),
        }
    }))
}

/// Handle scheduled.pause - pause a scheduled task
async fn handle_scheduled_pause(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    ctx.scheduler.pause(schedule_id)
        .map_err(Error::Protocol)?;

    info!(schedule_id = %schedule_id, "Paused scheduled task");

    Ok(serde_json::json!({
        "success": true,
        "scheduleId": schedule_id,
    }))
}

/// Handle scheduled.resume - resume a paused scheduled task
async fn handle_scheduled_resume(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    ctx.scheduler.resume(schedule_id)
        .map_err(Error::Protocol)?;

    info!(schedule_id = %schedule_id, "Resumed scheduled task");

    Ok(serde_json::json!({
        "success": true,
        "scheduleId": schedule_id,
    }))
}

/// Handle scheduled.delete - delete a scheduled task
async fn handle_scheduled_delete(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    let removed = ctx.scheduler.delete(schedule_id);

    if removed.is_some() {
        info!(schedule_id = %schedule_id, "Deleted scheduled task");
    }

    Ok(serde_json::json!({
        "success": removed.is_some(),
        "scheduleId": schedule_id,
    }))
}

/// Handle scheduled.enable - enable a disabled scheduled task
async fn handle_scheduled_enable(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    ctx.scheduler.enable(schedule_id)
        .map_err(Error::Protocol)?;

    info!(schedule_id = %schedule_id, "Enabled scheduled task");

    Ok(serde_json::json!({
        "success": true,
        "scheduleId": schedule_id,
    }))
}

/// Handle scheduled.disable - disable a scheduled task
async fn handle_scheduled_disable(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    ctx.scheduler.disable(schedule_id)
        .map_err(Error::Protocol)?;

    info!(schedule_id = %schedule_id, "Disabled scheduled task");

    Ok(serde_json::json!({
        "success": true,
        "scheduleId": schedule_id,
    }))
}

/// Handle scheduled.fire_now - manually trigger a scheduled task
async fn handle_scheduled_fire_now(
    ctx: &HandlerContext,
    _id: Option<String>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let schedule_id = params
        .get("scheduleId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Protocol("scheduleId required".to_string()))?;

    ctx.scheduler.fire_now(schedule_id).await
        .map_err(Error::Protocol)?;

    info!(schedule_id = %schedule_id, "Manually fired scheduled task");

    Ok(serde_json::json!({
        "success": true,
        "scheduleId": schedule_id,
    }))
}

/// Handle shutdown
async fn handle_shutdown(
    ctx: &HandlerContext,
    _id: Option<String>,
) -> Result<serde_json::Value> {
    info!("Gateway shutdown requested");
    let _ = ctx.shutdown_tx.send(());
    Ok(serde_json::json!({ "shuttingDown": true }))
}
