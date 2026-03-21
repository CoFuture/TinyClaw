//! Message handlers

use crate::agent::tools::ToolExecutor;
use crate::common::{Error, Result};
use crate::config::Config;
use crate::gateway::events::{Event, EventEmitter};
use crate::persistence::HistoryManager;
use crate::gateway::protocol::{error_codes::*, *};
use crate::gateway::session::SessionManager;
use crate::agent::{Agent, SessionSkillManager};
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
pub struct HandlerContext {
    pub session_manager: Arc<SessionManager>,
    pub history_manager: Arc<HistoryManager>,
    pub event_emitter: Arc<EventEmitter>,
    pub config: Arc<RwLock<Config>>,
    pub agent: Arc<Agent>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub skill_manager: Arc<SessionSkillManager>,
}

impl HandlerContext {
    pub fn new(
        session_manager: Arc<SessionManager>,
        history_manager: Arc<HistoryManager>,
        event_emitter: Arc<EventEmitter>,
        config: Arc<RwLock<Config>>,
        agent: Arc<Agent>,
        shutdown_tx: broadcast::Sender<()>,
        skill_manager: Arc<SessionSkillManager>,
    ) -> Self {
        Self {
            session_manager,
            history_manager,
            event_emitter,
            config,
            agent,
            shutdown_tx,
            skill_manager,
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
        "[{}] --> {} request: method={}",
        request_id,
        method,
        jsonrpc_id.as_deref().unwrap_or("none")
    );

    let result = match method.as_str() {
        methods::PING => handle_ping(jsonrpc_id.clone()).await,
        methods::SESSIONS_LIST => handle_sessions_list(ctx, jsonrpc_id.clone(), params).await,
        methods::SESSIONS_SEND => handle_sessions_send(ctx, request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::SESSIONS_HISTORY => handle_sessions_history(ctx, jsonrpc_id.clone(), params).await,
        methods::AGENT_TURN => handle_agent_turn(ctx, request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::EXEC => handle_exec(request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::TOOLS_LIST => handle_tools_list(jsonrpc_id.clone()).await,
        methods::TOOL_EXECUTE => handle_tool_execute(request_id.clone(), jsonrpc_id.clone(), params).await,
        methods::STATUS => handle_status(ctx, jsonrpc_id.clone()).await,
        methods::SHUTDOWN => handle_shutdown(ctx, jsonrpc_id.clone()).await,
        
        _ => Err(Error::Protocol(format!("Unknown method: {}", method))),
    };

    match result {
        Ok(value) => {
            info!("[{}] <-- {} success", request_id, method);
            Some(ResponseSuccess::new(jsonrpc_id, value).into())
        }
        Err(e) => {
            error!("[{}] <-- {} error: {}", request_id, method, e);
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

    // Send to agent and get response
    let response: String = ctx.agent.send_message(session_key, message, skill_prompt.as_deref()).await?;

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

    info!("[{}] Exec: {} (timeout={}ms)", request_id, command, timeout);

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

/// Handle shutdown
async fn handle_shutdown(
    ctx: &HandlerContext,
    _id: Option<String>,
) -> Result<serde_json::Value> {
    info!("Shutting down");
    let _ = ctx.shutdown_tx.send(());
    Ok(serde_json::json!({ "shuttingDown": true }))
}
