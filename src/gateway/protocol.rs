//! Protocol definitions

use serde::{Deserialize, Serialize};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<ErrorRecovery>,
}

/// Error recovery suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecovery {
    /// What the user can do to recover
    pub suggestion: String,
    /// Link to documentation (optional)
    pub doc_url: Option<String>,
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
    /// Create a new error response
    pub fn new(id: Option<String>, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            error: ResponseErrorBody {
                code: code.into(),
                message: message.into(),
                data: None,
                recovery: None,
            },
        }
    }

    #[allow(dead_code)]
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
                recovery: None,
            },
        }
    }

    /// Create an error with recovery suggestion
    pub fn with_recovery(
        id: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            id,
            error: ResponseErrorBody {
                code: code.into(),
                message: message.into(),
                data: None,
                recovery: Some(ErrorRecovery {
                    suggestion: suggestion.into(),
                    doc_url: None,
                }),
            },
        }
    }

}

/// JSON-RPC 2.0 error codes
#[allow(dead_code)]
pub mod error_codes {
    /// Parse error - Invalid JSON
    pub const PARSE_ERROR: &str = "PARSE_ERROR";
    /// Invalid request - Request is not valid
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    /// Method not found - Unknown method
    pub const METHOD_NOT_FOUND: &str = "METHOD_NOT_FOUND";
    /// Invalid params - Parameters are invalid
    pub const INVALID_PARAMS: &str = "INVALID_PARAMS";
    /// Internal error - Server-side error
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

    // Application-specific error codes (1000-1999)
    /// Session not found
    pub const SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
    /// Agent error - AI provider issues
    pub const AGENT_ERROR: &str = "AGENT_ERROR";
    /// Tool execution failed
    pub const TOOL_ERROR: &str = "TOOL_ERROR";
    /// Network error - Connectivity issues
    pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
    /// Configuration error
    pub const CONFIG_ERROR: &str = "CONFIG_ERROR";
    /// Authentication error
    pub const AUTH_ERROR: &str = "AUTH_ERROR";
    /// Protocol error - Invalid protocol usage
    pub const PROTOCOL_ERROR: &str = "PROTOCOL_ERROR";
    /// Timeout error
    pub const TIMEOUT_ERROR: &str = "TIMEOUT_ERROR";
    /// Rate limit exceeded
    pub const RATE_LIMIT_ERROR: &str = "RATE_LIMIT_ERROR";
    /// User denied - action plan was not confirmed
    pub const USER_DENIED_ERROR: &str = "USER_DENIED_ERROR";
    /// WebSocket connection error
    pub const WS_ERROR: &str = "WS_ERROR";
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
#[allow(dead_code)]
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
    /// Sessions delete
    pub const SESSIONS_DELETE: &str = "sessions.delete";
    /// Sessions rename
    pub const SESSION_RENAME: &str = "session.rename";
    /// Session cancel (cancel ongoing turn)
    pub const SESSION_CANCEL: &str = "session.cancel";
    /// Session confirm action (confirm or deny pending action plan)
    pub const SESSION_CONFIRM_ACTION: &str = "session.confirm_action";
    /// Session instructions get
    pub const SESSION_INSTRUCTIONS_GET: &str = "session.instructions.get";
    /// Session instructions set
    pub const SESSION_INSTRUCTIONS_SET: &str = "session.instructions.set";
    /// Exec command
    pub const EXEC: &str = "exec";
    /// Tool execute
    pub const TOOL_EXECUTE: &str = "tools.execute";
    /// Tool list
    pub const TOOLS_LIST: &str = "tools.list";
    /// Config get
    pub const CONFIG_GET: &str = "config.get";
    /// Config patch
    pub const CONFIG_PATCH: &str = "config.patch";
    /// Status
    pub const STATUS: &str = "status";
    /// Shutdown
    pub const SHUTDOWN: &str = "shutdown";
    /// Agent circuit breaker state
    pub const AGENT_CIRCUIT_BREAKER: &str = "agent.circuit_breaker";
    /// Task create
    pub const TASK_CREATE: &str = "task.create";
    /// Task list
    pub const TASK_LIST: &str = "task.list";
    /// Task get
    pub const TASK_GET: &str = "task.get";
    /// Task start
    pub const TASK_START: &str = "task.start";
    /// Task cancel
    pub const TASK_CANCEL: &str = "task.cancel";
    /// Task remove
    pub const TASK_REMOVE: &str = "task.remove";
    /// Scheduled task create
    pub const SCHEDULED_CREATE: &str = "scheduled.create";
    /// Scheduled task list
    pub const SCHEDULED_LIST: &str = "scheduled.list";
    /// Scheduled task get
    pub const SCHEDULED_GET: &str = "scheduled.get";
    /// Scheduled task pause
    pub const SCHEDULED_PAUSE: &str = "scheduled.pause";
    /// Scheduled task resume
    pub const SCHEDULED_RESUME: &str = "scheduled.resume";
    /// Scheduled task delete
    pub const SCHEDULED_DELETE: &str = "scheduled.delete";
    /// Scheduled task enable
    pub const SCHEDULED_ENABLE: &str = "scheduled.enable";
    /// Scheduled task disable
    pub const SCHEDULED_DISABLE: &str = "scheduled.disable";
    /// Scheduled task fire now (manual trigger)
    pub const SCHEDULED_FIRE_NOW: &str = "scheduled.fire_now";
    /// Session notes list
    pub const SESSION_NOTES_LIST: &str = "session.notes.list";
    /// Session notes add
    pub const SESSION_NOTES_ADD: &str = "session.notes.add";
    /// Session notes update
    pub const SESSION_NOTES_UPDATE: &str = "session.notes.update";
    /// Session notes delete
    pub const SESSION_NOTES_DELETE: &str = "session.notes.delete";
    /// Session suggestions list
    pub const SESSION_SUGGESTIONS_LIST: &str = "session.suggestions.list";
    /// Session suggestion accept
    pub const SESSION_SUGGESTIONS_ACCEPT: &str = "session.suggestions.accept";
    /// Session suggestion dismiss
    pub const SESSION_SUGGESTIONS_DISMISS: &str = "session.suggestions.dismiss";
}

/// Event types for notifications
#[allow(dead_code)]
pub mod events {
    /// Assistant text event
    pub const ASSISTANT_TEXT: &str = "assistant.text";
    /// Assistant tool use event
    pub const ASSISTANT_TOOL_USE: &str = "assistant.tool_use";
    /// Action plan preview event
    pub const ACTION_PLAN_PREVIEW: &str = "action.plan_preview";
    /// Tool result event
    pub const TOOL_RESULT: &str = "tool_result";
    /// Session ended event
    pub const SESSION_ENDED: &str = "session.ended";
    /// Error event
    pub const ERROR: &str = "error";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_standard_deserialization() {
        let json = r#"{"id": "1", "method": "ping", "params": {}}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        
        assert_eq!(request.method(), "ping");
        assert_eq!(request.id(), Some("1"));
    }

    #[test]
    fn test_request_notification_deserialization() {
        let json = r#"{"method": "ping", "params": {}}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        
        assert_eq!(request.method(), "ping");
        assert_eq!(request.id(), None);
    }

    #[test]
    fn test_request_serialization() {
        let request = Request::Standard(RequestStandard {
            id: Some("1".to_string()),
            method: "ping".to_string(),
            params: serde_json::json!({}),
        });
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("ping"));
    }

    #[test]
    fn test_response_success() {
        let response = ResponseSuccess::new(Some("1".to_string()), serde_json::json!({"status": "ok"}));
        let json = serde_json::to_string(&response).unwrap();
        
        assert!(json.contains("result"));
        assert!(json.contains("ok"));
    }

    #[test]
    fn test_response_error() {
        let response = ResponseError::new(Some("1".to_string()), "ERROR_CODE", "Error message");
        let json = serde_json::to_string(&response).unwrap();
        
        assert!(json.contains("error"));
        assert!(json.contains("ERROR_CODE"));
        assert!(json.contains("Error message"));
    }

    #[test]
    fn test_response_error_with_data() {
        let response = ResponseError::with_data(
            Some("1".to_string()),
            "ERROR_CODE",
            "Error message",
            serde_json::json!({"key": "value"}),
        );
        let json = serde_json::to_string(&response).unwrap();
        
        assert!(json.contains("data"));
    }

    #[test]
    fn test_response_success_without_id() {
        let response = ResponseSuccess::new(None, serde_json::json!("pong"));
        let json = serde_json::to_string(&response).unwrap();
        
        assert!(!json.contains("id"));
    }

    #[test]
    fn test_response_notification() {
        let response = ResponseNotification {
            method: "ping".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        
        assert!(json.contains("ping"));
    }

    #[test]
    fn test_methods_constants() {
        assert_eq!(methods::PING, "ping");
        assert_eq!(methods::SESSIONS_LIST, "sessions.list");
        assert_eq!(methods::SESSIONS_SEND, "sessions.send");
        assert_eq!(methods::AGENT_TURN, "agent.turn");
        assert_eq!(methods::AGENT_SPAWN, "agent.spawn");
        assert_eq!(methods::SESSIONS_HISTORY, "sessions.history");
        assert_eq!(methods::EXEC, "exec");
        assert_eq!(methods::TOOL_EXECUTE, "tools.execute");
        assert_eq!(methods::TOOLS_LIST, "tools.list");
        assert_eq!(methods::CONFIG_GET, "config.get");
        assert_eq!(methods::CONFIG_PATCH, "config.patch");
        assert_eq!(methods::STATUS, "status");
        assert_eq!(methods::SHUTDOWN, "shutdown");
    }

    #[test]
    fn test_events_constants() {
        assert_eq!(events::ASSISTANT_TEXT, "assistant.text");
        assert_eq!(events::ASSISTANT_TOOL_USE, "assistant.tool_use");
        assert_eq!(events::TOOL_RESULT, "tool_result");
        assert_eq!(events::SESSION_ENDED, "session.ended");
        assert_eq!(events::ERROR, "error");
    }
}
