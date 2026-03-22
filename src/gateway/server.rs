//! WebSocket server

use crate::common::Result;
use crate::config::Config;
use crate::gateway::messages::handle_request;
use crate::gateway::protocol::*;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Maximum concurrent requests per connection
const MAX_CONCURRENT_REQUESTS: usize = 10;

/// RAII guard that decrements active connection count on drop
struct ConnGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Shared server state for graceful shutdown
#[derive(Clone)]
pub struct ServerState {
    /// Number of currently active connections
    pub active_connections: Arc<AtomicUsize>,
    /// Shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,
}

impl ServerState {
    pub fn new(shutdown_timeout_secs: u64) -> Self {
        Self {
            active_connections: Arc::new(AtomicUsize::new(0)),
            shutdown_timeout_secs,
        }
    }
}

/// Start the WebSocket server
pub async fn start_server(
    config: Arc<RwLock<Config>>,
    ctx: crate::gateway::messages::HandlerContext,
    shutdown_rx: broadcast::Receiver<()>,
    server_state: ServerState,
) -> Result<()> {
    let addr = config.read().gateway.bind.clone();
    let listener = TcpListener::bind(&addr).await?;

    info!("Gateway listening on ws://{}", addr);

    let mut shutdown_rx = shutdown_rx;

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let ctx = crate::gateway::messages::HandlerContext::new(
                            ctx.session_manager.clone(),
                            ctx.history_manager.clone(),
                            ctx.event_emitter.clone(),
                            ctx.config.clone(),
                            ctx.agent.clone(),
                            ctx.shutdown_tx.clone(),
                            ctx.skill_manager.clone(),
                            ctx.task_manager.clone(),
                            ctx.scheduler.clone(),
                            ctx.suggestion_engines.clone(),
                            ctx.preferences.clone(),
                        );
                        let server_state = server_state.clone();
                        tokio::spawn(handle_connection(stream, ctx, server_state));
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, draining connections...");
                // Wait for active connections to drain
                let timeout = Duration::from_secs(server_state.shutdown_timeout_secs);
                let deadline = tokio::time::Instant::now() + timeout;
                while server_state.active_connections.load(Ordering::SeqCst) > 0 {
                    if tokio::time::Instant::now() >= deadline {
                        warn!(
                            "Shutdown timeout reached, {} connections still active",
                            server_state.active_connections.load(Ordering::SeqCst)
                        );
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                info!("All connections drained, server shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Internal message for the connection handler
enum ConnectionMessage {
    /// A text message from the client
    Text(String),
    /// Ping received
    Ping(Vec<u8>),
    /// Connection closed
    Close,
    /// WebSocket error
    Error(()),
}

/// Handle a single WebSocket connection with message queue
async fn handle_connection(
    stream: TcpStream,
    ctx: crate::gateway::messages::HandlerContext,
    server_state: ServerState,
) {
    let addr = stream.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap());
    info!("New connection from: {}", addr);

    server_state.active_connections.fetch_add(1, Ordering::SeqCst);
    let _conn_guard = ConnGuard {
        counter: server_state.active_connections.clone(),
    };

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    // Create a channel for message processing
    let (msg_tx, mut msg_rx) = mpsc::channel::<ConnectionMessage>(MAX_CONCURRENT_REQUESTS);

    // Spawn the response writer task
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match msg {
                ConnectionMessage::Text(text) => {
                    if let Err(e) = write.send(Message::Text(text.into())).await {
                        error!("Failed to send response: {}", e);
                        break;
                    }
                }
                ConnectionMessage::Ping(data) => {
                    if let Err(e) = write.send(Message::Pong(data.into())).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                ConnectionMessage::Close => {
                    let _ = write.close().await;
                    break;
                }
                ConnectionMessage::Error(_) => {
                    // These don't produce responses
                }
            }
        }
    });

    // Process messages from the client
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received: {}", text);

                // Parse JSON-RPC request
                let request: std::result::Result<Request, _> = serde_json::from_str(&text);

                match request {
                    Ok(request) => {
                        let ctx = ctx.clone();
                        let msg_tx = msg_tx.clone();
                        
                        // Handle request asynchronously
                        tokio::spawn(async move {
                            let response = handle_request(&ctx, request).await;
                            
                            if let Some(response) = response {
                                let response_json = serde_json::to_string(&response).unwrap();
                                let _ = msg_tx.send(ConnectionMessage::Text(response_json)).await;
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to parse request: {}", e);
                        let error_response = ResponseError::new(
                            None,
                            "PARSE_ERROR",
                            format!("Invalid JSON: {}", e),
                        );
                        let response_json = serde_json::to_string(&error_response).unwrap();
                        let _ = msg_tx.send(ConnectionMessage::Text(response_json)).await;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Connection closed by client");
                let _ = msg_tx.send(ConnectionMessage::Close).await;
                break;
            }
            Ok(Message::Ping(data)) => {
                // Respond with Pong
                let _ = msg_tx.send(ConnectionMessage::Ping(data.to_vec())).await;
            }
            Ok(Message::Pong(_)) => {
                // Ignore pong
            }
            Ok(Message::Binary(_data)) => {
                warn!("Received binary data, ignoring");
            }
            Ok(Message::Frame(_)) => {
                // Ignore frame
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                let _ = msg_tx.send(ConnectionMessage::Error(())).await;
                break;
            }
        }
    }

    // Wait for writer to finish
    let _ = writer_handle.await;

    info!("Connection closed: {}", addr);
}
