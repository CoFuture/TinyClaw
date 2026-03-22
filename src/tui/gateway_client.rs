//! TUI Gateway Client - WebSocket client for connecting to the TinyClaw gateway

use crate::gateway::protocol::{methods, Request, RequestStandard, Response};
use crate::types::SessionHistory;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info};

/// Gateway client for TUI
pub struct TuiGatewayClient {
    /// WebSocket connection URL
    url: String,
    /// Sender for outgoing messages to the gateway
    send_tx: mpsc::Sender<String>,
    /// Receiver for incoming messages from the gateway
    recv_rx: Option<mpsc::Receiver<String>>,
    /// Channel for broadcasting received events to TUI components
    event_tx: broadcast::Sender<TuiGatewayEvent>,
    /// Connection status
    status: TuiGatewayStatus,
    /// Current session ID (for future multi-session support)
    #[allow(dead_code)]
    session_id: Option<String>,
}

/// Gateway event types for TUI consumption
#[derive(Debug, Clone)]
pub enum TuiGatewayEvent {
    /// Connected to gateway
    Connected,
    /// Disconnected from gateway
    Disconnected,
    /// Error occurred
    Error(String),
    /// Text response from assistant
    AssistantText(String),
    /// Tool call started
    ToolStart { tool: String, _input: serde_json::Value },
    /// Tool result received
    #[allow(dead_code)]
    ToolResult { tool: String, output: String },
    /// Final response received (turn ended)
    TurnEnded(String),
    /// Connection error
    ConnectionError(String),
    /// Ping response
    Pong,
    /// Sessions list received
    SessionsList(Vec<SessionInfo>),
    /// New session created
    SessionCreated { session_id: String, label: Option<String> },
    /// Session history loaded
    SessionHistoryLoaded { session_id: String, history: SessionHistory },
    /// Session deleted
    SessionDeleted { session_id: String },
}

/// Session info from gateway
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    #[allow(dead_code)]
    pub label: Option<String>,
    #[allow(dead_code)]
    pub kind: String,
}

/// Connection status
#[derive(Debug, Clone, PartialEq)]
pub enum TuiGatewayStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl TuiGatewayClient {
    /// Create a new gateway client
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        let (event_tx, _) = broadcast::channel(100);
        let (send_tx, send_rx) = mpsc::channel(32);

        Self {
            url,
            send_tx,
            recv_rx: Some(send_rx),
            event_tx,
            status: TuiGatewayStatus::Disconnected,
            session_id: None,
        }
    }

    /// Get the event receiver for subscribing to gateway events
    pub fn subscribe(&self) -> broadcast::Receiver<TuiGatewayEvent> {
        self.event_tx.subscribe()
    }

    /// Get current connection status
    #[allow(dead_code)]
    pub fn status(&self) -> &TuiGatewayStatus {
        &self.status
    }

    /// Get current session ID
    #[allow(dead_code)]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Connect to the gateway
    pub async fn connect(&mut self) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        self.status = TuiGatewayStatus::Connecting;
        info!("TUI connecting to gateway at {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await?;
        info!("TUI connected to gateway");

        let (mut ws_send, ws_recv) = ws_stream.split();

        // Create channels for communication between read and write tasks
        let (ping_tx, mut ping_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
        let (close_tx, mut close_rx) = tokio::sync::mpsc::channel::<()>(1);

        // Spawn task to handle outgoing messages
        let mut send_rx = self.recv_rx.take().unwrap();
        let write_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = send_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                // ignore write errors as they indicate connection issues
                                let _ = ws_send.send(TungsteniteMessage::Text(msg.into())).await;
                            }
                            None => break,
                        }
                    }
                    ping_data = ping_rx.recv() => {
                        match ping_data {
                            Some(data) => {
                                let _ = ws_send.send(TungsteniteMessage::Pong(data.into())).await;
                            }
                            None => break,
                        }
                    }
                    _ = close_rx.recv() => {
                        let _ = ws_send.close().await;
                        break;
                    }
                }
            }
        });

        // Spawn task to handle incoming messages
        let event_tx = self.event_tx.clone();
        let read_handle = tokio::spawn(async move {
            let mut stream = ws_recv;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(TungsteniteMessage::Text(text)) => {
                        let text_str = text.to_string();
                        debug!("TUI received: {}", text_str);
                        
                        // Parse and emit events
                        if let Ok(response) = serde_json::from_str::<Response>(&text_str) {
                            Self::handle_response(&event_tx, response);
                        }
                    }
                    Ok(TungsteniteMessage::Close(_)) => {
                        let _ = event_tx.send(TuiGatewayEvent::Disconnected);
                        let _ = close_tx.send(()).await;
                        break;
                    }
                    Ok(TungsteniteMessage::Ping(data)) => {
                        let _ = event_tx.send(TuiGatewayEvent::Pong);
                        let _ = ping_tx.send(data.to_vec()).await;
                    }
                    Ok(TungsteniteMessage::Pong(_)) => {}
                    Ok(TungsteniteMessage::Binary(_)) | Ok(TungsteniteMessage::Frame(_)) => {}
                    Err(e) => {
                        let _ = event_tx.send(TuiGatewayEvent::ConnectionError(e.to_string()));
                        let _ = close_tx.send(()).await;
                        break;
                    }
                }
            }
            let _ = event_tx.send(TuiGatewayEvent::Disconnected);
            let _ = close_tx.send(()).await;
        });

        // Wait for connection to be established (send ping)
        if let Err(e) = self.send_ping().await {
            error!("Failed to send ping: {}", e);
            // Don't fail connection for ping error, just log it
        }
        
        self.status = TuiGatewayStatus::Connected;
        let _ = self.event_tx.send(TuiGatewayEvent::Connected);
        
        // Keep handles alive until dropped
        let _ = write_handle.await;
        let _ = read_handle.await;

        Ok(())
    }

    /// Handle a response from the gateway
    fn handle_response(event_tx: &broadcast::Sender<TuiGatewayEvent>, response: Response) {
        match response {
            Response::Success(resp) => {
                // Handle different result types
                if let Some(result_obj) = resp.result.as_object() {
                    // Check if this is a sessions.list response
                    if let Some(sessions) = result_obj.get("sessions") {
                        if let Some(sessions_arr) = sessions.as_array() {
                            let session_infos: Vec<SessionInfo> = sessions_arr
                                .iter()
                                .filter_map(|s| {
                                    let obj = s.as_object()?;
                                    let id = obj.get("id")?.as_str()?.to_string();
                                    let label = obj.get("label").and_then(|v| v.as_str()).map(String::from);
                                    let kind = obj.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                    Some(SessionInfo { id, label, kind })
                                })
                                .collect();
                            let _ = event_tx.send(TuiGatewayEvent::SessionsList(session_infos));
                            return;
                        }
                    }
                    // Check if this is an agent.spawn response
                    if let Some(session_id) = result_obj.get("session_id") {
                        let label = result_obj.get("label").and_then(|v| v.as_str()).map(String::from);
                        let _ = event_tx.send(TuiGatewayEvent::SessionCreated {
                            session_id: session_id.to_string(),
                            label,
                        });
                        return;
                    }
                    // Check if this is a turn response
                    if let Some(text) = result_obj.get("text") {
                        let _ = event_tx.send(TuiGatewayEvent::AssistantText(text.to_string()));
                    }
                    if let Some(response_text) = result_obj.get("response") {
                        let _ = event_tx.send(TuiGatewayEvent::TurnEnded(response_text.to_string()));
                    }
                    // Check if this is a sessions.history response
                    if let Some(session_id) = result_obj.get("sessionId") {
                        if let Some(messages) = result_obj.get("messages") {
                            if let Ok(history) = serde_json::from_value::<SessionHistory>(
                                serde_json::json!({
                                    "session_id": session_id,
                                    "messages": messages
                                })
                            ) {
                                let _ = event_tx.send(TuiGatewayEvent::SessionHistoryLoaded {
                                    session_id: session_id.to_string(),
                                    history,
                                });
                            }
                        }
                    }
                    // Check if this is a sessions.delete response
                    if result_obj.get("deleted") == Some(&serde_json::json!(true)) {
                        if let Some(session_id) = result_obj.get("sessionId") {
                            let _ = event_tx.send(TuiGatewayEvent::SessionDeleted {
                                session_id: session_id.to_string(),
                            });
                        }
                    }
                } else if resp.result.is_string() {
                    // Pong response
                    let _ = event_tx.send(TuiGatewayEvent::Pong);
                }
            }
            Response::Error(resp) => {
                let _ = event_tx.send(TuiGatewayEvent::Error(resp.error.message));
            }
            Response::Notification(resp) => {
                // Handle notification events
                match resp.method.as_str() {
                    "assistant.text" => {
                        if let Some(params) = resp.params {
                            if let Some(text) = params.get("text") {
                                let _ = event_tx.send(TuiGatewayEvent::AssistantText(text.to_string()));
                            }
                        }
                    }
                    "assistant.tool_use" => {
                        if let Some(params) = resp.params {
                            let tool = params.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let input = params.get("input").cloned().unwrap_or_default();
                            let _ = event_tx.send(TuiGatewayEvent::ToolStart { tool, _input: input });
                        }
                    }
                    "tool_result" => {
                        if let Some(params) = resp.params {
                            let output = params.get("output").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let tool = params.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let _ = event_tx.send(TuiGatewayEvent::ToolResult { tool, output });
                        }
                    }
                    _ => {
                        debug!("Unknown notification method: {}", resp.method);
                    }
                }
            }
        }
    }

    /// Send a ping to verify connection
    pub async fn send_ping(&self) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some("tui-ping".to_string()),
            method: methods::PING.to_string(),
            params: serde_json::json!({}),
        });
        
        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Send a message to the agent (agent.turn)
    pub async fn send_message(&self, session_id: &str, content: String) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("tui-{}-{}", session_id, uuid::Uuid::new_v4())),
            method: methods::AGENT_TURN.to_string(),
            params: serde_json::json!({
                "session_id": session_id,
                "message": {
                    "role": "user",
                    "content": content
                }
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List available sessions - sends request and returns immediately
    /// Sessions are delivered via the event channel as TuiGatewayEvent::SessionsList
    pub async fn list_sessions(&self) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some("tui-list-sessions".to_string()),
            method: methods::SESSIONS_LIST.to_string(),
            params: serde_json::json!({}),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Create a new session - sends request and returns immediately
    /// Session creation result is delivered via the event channel as TuiGatewayEvent::SessionCreated
    pub async fn create_session(&self) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("tui-create-session-{}", uuid::Uuid::new_v4())),
            method: methods::AGENT_SPAWN.to_string(),
            params: serde_json::json!({}),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get session history - sends request and returns immediately
    /// History is delivered via the event channel as TuiGatewayEvent::SessionHistoryLoaded
    pub async fn get_history(&self, session_id: &str) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("tui-history-{}", session_id)),
            method: methods::SESSIONS_HISTORY.to_string(),
            params: serde_json::json!({
                "sessionKey": session_id
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a session - sends request and returns immediately
    /// Result is delivered via the event channel as TuiGatewayEvent::SessionDeleted
    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("tui-delete-{}", session_id)),
            method: methods::SESSIONS_DELETE.to_string(),
            params: serde_json::json!({
                "sessionKey": session_id
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Check if connected
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.status == TuiGatewayStatus::Connected
    }
}

impl Default for TuiGatewayClient {
    fn default() -> Self {
        Self::new("ws://127.0.0.1:18790")
    }
}
