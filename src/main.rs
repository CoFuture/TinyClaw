//! TinyClaw - A minimal implementation of OpenClaw in Rust
//! 
//! This is the entry point for the TinyClaw Gateway.

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{error, info};

mod agent;
mod common;
mod config;
mod gateway;
mod http;

use common::logging;
use config::{load_config, Config};
use gateway::messages::HandlerContext;
use gateway::server;
use gateway::session::SessionManager;
use http::routes::{create_router, HttpState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let log_dir = logging::default_log_dir();
    if let Err(e) = logging::init_logging(log_dir) {
        eprintln!("Failed to initialize logging: {}", e);
    }

    info!("TinyClaw v{} starting...", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = load_or_default_config();

    // Create shared components
    let config = Arc::new(RwLock::new(config));
    let session_manager = Arc::new(SessionManager::new());
    let agent = Arc::new(agent::Agent::new(Arc::new(RwLock::new(
        config.read().agent.clone(),
    ))));

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    // Create handler context for WebSocket
    let _ws_ctx = HandlerContext::new(
        session_manager.clone(),
        config.clone(),
        agent.clone(),
        shutdown_tx.clone(),
    );

    // Create HTTP state
    let http_state = Arc::new(HttpState {
        config: config.clone(),
        session_manager: session_manager.clone(),
        agent: agent.clone(),
        shutdown_tx: shutdown_tx.clone(),
    });

    // Create the main session
    session_manager.get_or_create_main();
    info!("Main session created");

    // Spawn WebSocket server
    let server_config = config.clone();
    let ws_ctx_clone = HandlerContext::new(
        session_manager.clone(),
        config.clone(),
        agent.clone(),
        shutdown_tx.clone(),
    );
    
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = server::start_server(server_config, ws_ctx_clone, shutdown_rx).await {
            error!("WebSocket server error: {}", e);
        }
    });

    // Spawn HTTP server
    let http_port = 8080u16;
    let http_addr = format!("0.0.0.0:{}", http_port);
    
    let http_state_clone = http_state.clone();
    let http_handle = tokio::spawn(async move {
        let router = create_router(http_state_clone);
        let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
        info!("HTTP server listening on http://{}", http_addr);
        axum::serve(listener, router).await.unwrap();
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        result = ws_handle => {
            if let Err(e) = result {
                error!("WebSocket server task failed: {}", e);
            }
        }
        result = http_handle => {
            if let Err(e) = result {
                error!("HTTP server task failed: {}", e);
            }
        }
    }

    info!("TinyClaw shutdown complete");

    Ok(())
}

/// Load configuration or use defaults
fn load_or_default_config() -> Config {
    // Try to load from default path
    if let Some(config_path) = config::default_config_path() {
        if config_path.exists() {
            if let Ok(config) = load_config(&config_path) {
                info!("Loaded config from: {:?}", config_path);
                return config;
            }
        }
    }

    // Use default config
    info!("Using default configuration");
    Config::default()
}
