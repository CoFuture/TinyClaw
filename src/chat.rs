//! Simple CLI Chat Client
//! 
//! Interactive chat client that connects to a running TinyClaw gateway
//! via WebSocket and provides a line-based chat interface.

use crate::gateway::protocol::{methods, Request, RequestStandard, Response};
use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncBufReadExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::connect_async;
use tracing::error;

/// Chat client state
pub struct ChatClient {
    /// Current session ID
    session_id: String,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Send channel for outgoing messages
    send_tx: mpsc::Sender<String>,
}

impl ChatClient {
    /// Create a new chat client
    pub fn new(send_tx: mpsc::Sender<String>) -> Self {
        Self {
            session_id: "main".to_string(),
            running: Arc::new(AtomicBool::new(true)),
            send_tx,
        }
    }

    /// Set current session
    pub fn set_session(&mut self, session_id: String) {
        self.session_id = session_id;
    }

    /// Get current session
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Stop the client
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Send a chat message to the agent
    pub async fn send_message(&self, content: &str) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("chat-{}-{}", self.session_id, uuid::Uuid::new_v4())),
            method: methods::AGENT_TURN.to_string(),
            params: serde_json::json!({
                "sessionKey": self.session_id,
                "message": content,
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List sessions via WebSocket
    pub async fn list_sessions(&self) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some("chat-list".to_string()),
            method: methods::SESSIONS_LIST.to_string(),
            params: serde_json::json!({}),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Create a new session via WebSocket
    pub async fn create_session(&self, label: Option<&str>) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("chat-spawn-{}", uuid::Uuid::new_v4())),
            method: methods::AGENT_SPAWN.to_string(),
            params: serde_json::json!({
                "label": label,
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a session via WebSocket
    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let request = Request::Standard(RequestStandard {
            id: Some(format!("chat-delete-{}", session_id)),
            method: methods::SESSIONS_DELETE.to_string(),
            params: serde_json::json!({
                "sessionKey": session_id
            }),
        });

        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.send_tx.send(json).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Parse a chat command (starts with :)
fn parse_command(line: &str) -> Option<(String, Vec<String>)> {
    if !line.starts_with(':') {
        return None;
    }
    let rest = line.trim_start_matches(':').trim();
    let parts: Vec<String> = rest.split_whitespace().map(String::from).collect();
    if parts.is_empty() {
        return None;
    }
    let cmd = parts[0].clone();
    let args = parts[1..].to_vec();
    Some((cmd, args))
}

/// Run the interactive chat client
pub async fn run_chat(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🪶 TinyClaw Chat Client");
    println!("=======================");
    println!("Connecting to {}...", url);
    
    // Connect to gateway
    let (ws_stream, _) = connect_async(url).await?;
    println!("✅ Connected to gateway!\n");
    
    let (mut ws_send, ws_recv) = ws_stream.split();
    
    // Create channels
    let (send_tx, mut send_rx) = mpsc::channel::<String>(32);
    let (event_tx, mut event_rx) = broadcast::channel::<crate::tui::TuiGatewayEvent>(100);
    
    // Create chat client
    let mut client = ChatClient::new(send_tx.clone());
    
    // Spawn combined read/write task
    let write_handle = tokio::spawn(async move {
        // Read task part
        let mut stream = ws_recv;
        let event_tx_clone = event_tx.clone();
        
        // Write task part - select between sending and receiving
        loop {
            tokio::select! {
                // Outgoing message
                msg = send_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if ws_send.send(TungsteniteMessage::Text(msg.into())).await.is_err() {
                                let _ = event_tx_clone.send(crate::tui::TuiGatewayEvent::Disconnected);
                                break;
                            }
                        }
                        None => break,
                    }
                }
                // Incoming message
                msg = stream.next() => {
                    match msg {
                        Some(Ok(TungsteniteMessage::Text(text))) => {
                            if let Ok(response) = serde_json::from_str::<Response>(text.as_ref()) {
                                ChatClient::handle_response(&event_tx_clone, response);
                            }
                        }
                        Some(Ok(TungsteniteMessage::Close(_))) => {
                            let _ = event_tx_clone.send(crate::tui::TuiGatewayEvent::Disconnected);
                            break;
                        }
                        Some(Ok(TungsteniteMessage::Ping(data))) => {
                            let _ = ws_send.send(TungsteniteMessage::Pong(data)).await;
                        }
                        Some(Ok(TungsteniteMessage::Pong(_))) => {}
                        Some(Ok(TungsteniteMessage::Binary(_))) | Some(Ok(TungsteniteMessage::Frame(_))) => {}
                        Some(Err(e)) => {
                            let _ = event_tx_clone.send(crate::tui::TuiGatewayEvent::ConnectionError(e.to_string()));
                            break;
                        }
                        None => break,
                    }
                }
            }
        }
        let _ = event_tx_clone.send(crate::tui::TuiGatewayEvent::Disconnected);
    });
    
    // Send initial ping
    let ping_request = Request::Standard(RequestStandard {
        id: Some("chat-ping".to_string()),
        method: methods::PING.to_string(),
        params: serde_json::json!({}),
    });
    if let Ok(json) = serde_json::to_string(&ping_request) {
        let _ = send_tx.send(json).await;
    }
    
    // Print help
    println!("Type your message and press Enter to chat.");
    println!("Commands:");
    println!("  :sessions  - List all sessions");
    println!("  :new [label] - Create a new session");
    println!("  :switch <id> - Switch to a session");
    println!("  :delete <id> - Delete a session");
    println!("  :quit     - Quit chat");
    println!();
    print!("[{}] > ", client.session_id());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    
    // Main loop - read from stdin
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    
    // Pending assistant response
    let mut pending_response = String::new();
    let mut _waiting_for_response = false;
    
    loop {
        tokio::select! {
            // Read from stdin
            result = stdin.read_line(&mut line) => {
                if result.is_err() || line.is_empty() {
                    continue;
                }
                let input = line.trim().to_string();
                line.clear();
                
                if input.is_empty() {
                    print!("[{}] > ", client.session_id());
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    continue;
                }
                
                // Check for command
                if let Some((cmd, args)) = parse_command(&input) {
                    match cmd.as_str() {
                        "quit" | "q" => {
                            println!("👋 Goodbye!");
                            client.stop();
                            break;
                        }
                        "sessions" | "s" => {
                            if client.list_sessions().await.is_ok() {
                                _waiting_for_response = false;
                            }
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                        "new" | "n" => {
                            let label = args.first().map(|s| s.as_str());
                            if client.create_session(label).await.is_ok() {
                                _waiting_for_response = false;
                            }
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                        "switch" | "sw" => {
                            if let Some(sid) = args.first() {
                                client.set_session(sid.clone());
                                println!("📍 Switched to session: {}", sid);
                            } else {
                                println!("Usage: :switch <session_id>");
                            }
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                        "delete" | "d" => {
                            if let Some(sid) = args.first() {
                                if client.delete_session(sid).await.is_ok() {
                                    _waiting_for_response = false;
                                }
                            } else {
                                println!("Usage: :delete <session_id>");
                            }
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                        "help" | "h" => {
                            println!("Commands:");
                            println!("  :sessions  - List all sessions");
                            println!("  :new [label] - Create a new session");
                            println!("  :switch <id> - Switch to a session");
                            println!("  :delete <id> - Delete a session");
                            println!("  :quit     - Quit chat");
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                        _ => {
                            println!("Unknown command: {}. Try :help", cmd);
                            print!("[{}] > ", client.session_id());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            continue;
                        }
                    }
                }
                
                // Regular message
                print!("[{}] > ", client.session_id());
                let _ = std::io::Write::flush(&mut std::io::stdout());
                
                if let Err(e) = client.send_message(&input).await {
                    error!("Failed to send message: {}", e);
                } else {
                    _waiting_for_response = true;
                    pending_response.clear();
                }
            }
            
            // Handle gateway events
            event_result = event_rx.recv() => {
                match event_result {
                    Ok(event) => {
                        match event {
                            crate::tui::TuiGatewayEvent::AssistantText(text) => {
                                pending_response.push_str(&text);
                                print!("\r\033[K"); // Clear current line
                                println!("\n📨 {}", pending_response);
                                _waiting_for_response = false;
                                pending_response.clear();
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::TurnEnded(text) => {
                                if pending_response.is_empty() {
                                    pending_response = text;
                                }
                                print!("\r\033[K");
                                println!("\n📨 {}", pending_response);
                                _waiting_for_response = false;
                                pending_response.clear();
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::ToolStart { tool, .. } => {
                                print!("\r\033[K");
                                println!("\n🔧 [Calling tool: {}]", tool);
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::SessionsList(sessions) => {
                                print!("\r\033[K");
                                println!("\n📋 Sessions:");
                                for s in sessions {
                                    let mark = if s.id == client.session_id() { " ←" } else { "" };
                                    let id_short = s.id.get(0..s.id.len().min(8)).unwrap_or(&s.id);
                                    println!("  • {} ({}){}", id_short, s.kind, mark);
                                }
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::SessionCreated { session_id, label } => {
                                print!("\r\033[K");
                                let sid_short = session_id.get(0..session_id.len().min(8)).unwrap_or(&session_id);
                                println!("\n✅ Created session: {} ({})", sid_short, label.as_deref().unwrap_or("-"));
                                client.set_session(session_id);
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::Error(err) => {
                                print!("\r\033[K");
                                println!("\n❌ Error: {}", err);
                                _waiting_for_response = false;
                                pending_response.clear();
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            crate::tui::TuiGatewayEvent::Disconnected => {
                                print!("\r\033[K");
                                println!("\n⚠️ Disconnected from gateway");
                                client.stop();
                                break;
                            }
                            crate::tui::TuiGatewayEvent::ConnectionError(e) => {
                                print!("\r\033[K");
                                println!("\n⚠️ Connection error: {}", e);
                            }
                            crate::tui::TuiGatewayEvent::ToolResult { tool, output } => {
                                print!("\r\033[K");
                                let truncated = if output.len() > 200 { format!("{}...", &output[..200]) } else { output };
                                println!("\n🔧 [{}] → {}", tool, truncated);
                                print!("[{}] > ", client.session_id());
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            _ => {}
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            
            // Check if still running
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {
                if !client.is_running() {
                    break;
                }
            }
        }
    }
    
    // Cleanup
    drop(send_tx);  // Signal task to end
    let _ = write_handle.await;
    
    Ok(())
}

impl ChatClient {
    /// Handle a gateway response
    fn handle_response(event_tx: &broadcast::Sender<crate::tui::TuiGatewayEvent>, response: Response) {
        match response {
            Response::Success(resp) => {
                if let Some(result_obj) = resp.result.as_object() {
                    if let Some(sessions) = result_obj.get("sessions") {
                        if let Some(sessions_arr) = sessions.as_array() {
                            let session_infos: Vec<crate::tui::SessionInfo> = sessions_arr
                                .iter()
                                .filter_map(|s| {
                                    let obj = s.as_object()?;
                                    let id = obj.get("id")?.as_str()?.to_string();
                                    let label = obj.get("label").and_then(|v| v.as_str()).map(String::from);
                                    let kind = obj.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                    Some(crate::tui::SessionInfo { id, label, kind })
                                })
                                .collect();
                            let _ = event_tx.send(crate::tui::TuiGatewayEvent::SessionsList(session_infos));
                            return;
                        }
                    }
                    if let Some(session_id) = result_obj.get("session_id") {
                        let label = result_obj.get("label").and_then(|v| v.as_str()).map(String::from);
                        let _ = event_tx.send(crate::tui::TuiGatewayEvent::SessionCreated {
                            session_id: session_id.to_string(),
                            label,
                        });
                        return;
                    }
                    if let Some(text) = result_obj.get("text") {
                        let _ = event_tx.send(crate::tui::TuiGatewayEvent::AssistantText(text.to_string()));
                    }
                    if let Some(response_text) = result_obj.get("response") {
                        let _ = event_tx.send(crate::tui::TuiGatewayEvent::TurnEnded(response_text.to_string()));
                    }
                    if result_obj.get("deleted") == Some(&serde_json::json!(true)) {
                        if let Some(session_id) = result_obj.get("sessionId") {
                            let _ = event_tx.send(crate::tui::TuiGatewayEvent::SessionDeleted {
                                session_id: session_id.to_string(),
                            });
                        }
                    }
                } else if resp.result.is_string() {
                    let _ = event_tx.send(crate::tui::TuiGatewayEvent::Pong);
                }
            }
            Response::Error(resp) => {
                let _ = event_tx.send(crate::tui::TuiGatewayEvent::Error(resp.error.message));
            }
            Response::Notification(resp) => {
                if let Some(params) = resp.params {
                    match resp.method.as_str() {
                        "assistant.text" => {
                            if let Some(text) = params.get("text") {
                                let _ = event_tx.send(crate::tui::TuiGatewayEvent::AssistantText(text.to_string()));
                            }
                        }
                        "assistant.tool_use" => {
                            let tool = params.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let input = params.get("input").cloned().unwrap_or_default();
                            let _ = event_tx.send(crate::tui::TuiGatewayEvent::ToolStart { tool, _input: input });
                        }
                        "tool_result" => {
                            let output = params.get("output").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let tool = params.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let _ = event_tx.send(crate::tui::TuiGatewayEvent::ToolResult { tool, output });
                        }
                        "turn.ended" => {
                            if let Some(response) = params.get("response") {
                                let _ = event_tx.send(crate::tui::TuiGatewayEvent::TurnEnded(response.to_string()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
