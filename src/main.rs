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

use common::logging;
use config::{load_config, Config};
use gateway::messages::HandlerContext;
use gateway::server;
use gateway::session::SessionManager;

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

    // Create handler context
    let _ctx = HandlerContext::new(
        session_manager.clone(),
        config.clone(),
        agent.clone(),
        shutdown_tx.clone(),
    );

    // Create the main session
    session_manager.get_or_create_main();
    info!("Main session created");

    // Spawn server
    let server_config = config.clone();
    let server_ctx = HandlerContext::new(
        session_manager,
        config.clone(),
        agent,
        shutdown_tx,
    );
    
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server::start_server(server_config, server_ctx, shutdown_rx).await {
            error!("Server error: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        result = server_handle => {
            if let Err(e) = result {
                error!("Server task failed: {}", e);
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
