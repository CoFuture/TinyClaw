//! Protocol definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request message from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Request {
    /// Standard request with id, method, params
    Standard(RequestStandard),
    /// Notification (no id, no response)
    Notification(RequestNotification),
}

impl Request {
    pub fn method(&self) -> &str {
        match self {
            Request::Standard(r) => &r.method,
            Request::Notification(r) => &r.method,
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Request::Standard(r) => r.id.as_deref(),
            Request::Notification(_) => None,
        }
    }

    pub fn params(&self) -> &serde_json::Value {
        match self {
            Request::Standard(r) => &r.params,
            Request::Notification(r) => &r.params,
        }
    }
}

/// Standard request with id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestStandard {
    pub id: Option<String>,
    pub method: String,
    pub params: serde_json::Value,
}

/// Notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestNotification {
    pub method: String,
    pub params: serde_json::Value,
}

/// Response message to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// Success response
    Success(ResponseSuccess),
    /// Error response
    Error(ResponseError),
    /// Notification (no id)
    Notification(ResponseNotification),
}

/// Success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSuccess {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub result: serde_json::Value,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub error: ResponseErrorBody,
}

/// Error body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Notification response (no id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseNotification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl ResponseSuccess {
    pub fn new(id: Option<String>, result: serde_json::Value) -> Self {
        Self { id, result }
    }
}

impl ResponseError {
    pub fn new(id: Option<String>, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            error: ResponseErrorBody {
                code: code.into(),
                message: message.into(),
                data: None,
            },
        }
    }

    pub fn with_data(
        id: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id,
            error: ResponseErrorBody {
                code: code.into(),
                message: message.into(),
                data: Some(data),
            },
        }
    }
}

impl From<ResponseSuccess> for Response {
    fn from(r: ResponseSuccess) -> Self {
        Response::Success(r)
    }
}

impl From<ResponseError> for Response {
    fn from(r: ResponseError) -> Self {
        Response::Error(r)
    }
}

/// JSON-RPC 2.0 method names
pub mod methods {
    /// Ping method
    pub const PING: &str = "ping";
    /// Sessions list
    pub const SESSIONS_LIST: &str = "sessions.list";
    /// Sessions send (send message to a session)
    pub const SESSIONS_SEND: &str = "sessions.send";
    /// Agent turn (send message to agent)
    pub const AGENT_TURN: &str = "agent.turn";
    /// Agent spawn (create new agent session)
    pub const AGENT_SPAWN: &str = "agent.spawn";
    /// Sessions history
    pub const SESSIONS_HISTORY: &str = "sessions.history";
    /// Exec command
    pub const EXEC: &str = "exec";
    /// Config get
    pub const CONFIG_GET: &str = "config.get";
    /// Config patch
    pub const CONFIG_PATCH: &str = "config.patch";
    /// Status
    pub const STATUS: &str = "status";
    /// Shutdown
    pub const SHUTDOWN: &str = "shutdown";
}

/// Event types for notifications
pub mod events {
    /// Assistant text event
    pub const ASSISTANT_TEXT: &str = "assistant.text";
    /// Assistant tool use event
    pub const ASSISTANT_TOOL_USE: &str = "assistant.tool_use";
    /// Tool result event
    pub const TOOL_RESULT: &str = "tool_result";
    /// Session ended event
    pub const SESSION_ENDED: &str = "session.ended";
    /// Error event
    pub const ERROR: &str = "error";
}
