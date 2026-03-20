//! WebSocket server

use crate::common::Result;
use crate::config::Config;
use crate::gateway::messages::handle_request;
use crate::gateway::protocol::*;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Start the WebSocket server
pub async fn start_server(
    config: Arc<RwLock<Config>>,
    ctx: crate::gateway::messages::HandlerContext,
    shutdown_rx: broadcast::Receiver<()>,
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
                            ctx.config.clone(),
                            ctx.agent.clone(),
                            ctx.shutdown_tx.clone(),
                        );
                        tokio::spawn(handle_connection(stream, ctx));
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Server shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single WebSocket connection
async fn handle_connection(stream: TcpStream, ctx: crate::gateway::messages::HandlerContext) {
    let addr = stream.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap());
    info!("New connection from: {}", addr);

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received: {}", text);

                // Parse JSON-RPC request
                let request: std::result::Result<Request, _> = serde_json::from_str(&text);

                match request {
                    Ok(request) => {
                        // Handle the request
                        if let Some(response) = handle_request(&ctx, request).await {
                            // Send response
                            let response_json = serde_json::to_string(&response).unwrap();
                            if let Err(e) = write.send(Message::Text(response_json.into())).await {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse request: {}", e);
                        let error_response = ResponseError::new(
                            None,
                            "PARSE_ERROR",
                            format!("Invalid JSON: {}", e),
                        );
                        let response_json = serde_json::to_string(&error_response).unwrap();
                        if let Err(e) = write.send(Message::Text(response_json.into())).await {
                            error!("Failed to send error: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Connection closed by client");
                break;
            }
            Ok(Message::Ping(data)) => {
                // Automatically respond with Pong
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
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
                break;
            }
        }
    }

    info!("Connection closed: {}", addr);
}
