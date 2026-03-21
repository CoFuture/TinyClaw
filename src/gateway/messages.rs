//! Message handlers

use crate::agent::tools::ToolExecutor;
use crate::common::{Error, Result};
use crate::config::Config;
use crate::gateway::events::{Event, EventEmitter};
use crate::persistence::HistoryManager;
use crate::gateway::protocol::*;
use crate::gateway::session::SessionManager;
use crate::agent::Agent;
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
}

impl HandlerContext {
    pub fn new(
        session_manager: Arc<SessionManager>,
        history_manager: Arc<HistoryManager>,
        event_emitter: Arc<EventEmitter>,
        config: Arc<RwLock<Config>>,
        agent: Arc<Agent>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        Self {
            session_manager,
            history_manager,
            event_emitter,
            config,
            agent,
            shutdown_tx,
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
            let err_response = ResponseError::new(jsonrpc_id, "METHOD_NOT_FOUND", e.to_string());
            Some(err_response.into())
        }
    }
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
    ctx.agent.send_message(session_key, message).await?;

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

    // Add user message to history
    ctx.history_manager.add_message(
        session_key,
        crate::types::Message::user(message),
    );

    // Send to agent and get response
    let response: String = ctx.agent.send_message(session_key, message).await?;

    // Add assistant response to history
    ctx.history_manager.add_message(
        session_key,
        crate::types::Message::assistant(&response),
    );

    // Emit event
    ctx.event_emitter.emit(Event::AssistantText {
        session_id: session_key.to_string(),
        text: response.clone(),
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
