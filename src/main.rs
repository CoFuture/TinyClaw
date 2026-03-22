//! TinyClaw - A minimal implementation of OpenClaw in Rust
//! 
//! This is the entry point for the TinyClaw Gateway.

use tiny_claw::chat;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

mod agent;
mod common;
mod config;
mod gateway;
mod http;
mod metrics;
mod persistence;
mod ratelimit;
mod tui;
mod types;

use common::logging;
use config::{load_config, Config};
use gateway::events::EventEmitter;
use gateway::messages::HandlerContext;
use gateway::server;
use gateway::session::SessionManager;
use http::routes::{create_router, HttpState};
use metrics::MetricsCollector;
use persistence::HistoryManager;
use ratelimit::RateLimiter;
use agent::{SkillRegistry, SessionSkillManager, TaskManager, Scheduler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Record start time
    let start_time = Instant::now();
    
    // Initialize logging
    let log_dir = logging::default_log_dir();
    if let Err(e) = logging::init_logging(log_dir) {
        eprintln!("Failed to initialize logging: {}", e);
    }

    // Check for --tui flag
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--tui".to_string()) || args.contains(&"-t".to_string()) {
        run_tui_mode()?;
        return Ok(());
    }

    // Check for --chat flag (interactive CLI chat client)
    if args.contains(&"--chat".to_string()) || args.contains(&"-c".to_string()) {
        let url = args.iter()
            .position(|a| a == "--url" || a == "-u")
            .and_then(|i| args.get(i + 1))
            .map(String::from)
            .unwrap_or_else(|| "ws://127.0.0.1:18790".to_string());
        
        println!("🪶 TinyClaw Chat Client connecting to {}", url);
        let rt = tokio::runtime::Runtime::new()?;
        if let Err(e) = rt.block_on(async { chat::run_chat(&url).await }) {
            eprintln!("Chat error: {}", e);
        }
        return Ok(());
    }

    info!("TinyClaw v{} starting...", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = load_or_default_config();

    // Create shared components
    let config = Arc::new(RwLock::new(config));
    let session_manager = Arc::new(SessionManager::new());

    // Create history manager with optional SQLite persistence
    let history_manager: Arc<HistoryManager> = if config.read().persistence.enabled {
        let data_dir = config.read().gateway.data_dir.clone();
        let persistence_path = config.read().persistence.path.clone();
        let db_path = if persistence_path.starts_with('/') {
            std::path::PathBuf::from(persistence_path)
        } else {
            let base = data_dir
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("tiny_claw")
                });
            base.join(&persistence_path)
        };
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match HistoryManager::new_with_persistence(&db_path) {
            Ok(manager) => {
                info!("SQLite persistence enabled: {:?}", db_path);
                Arc::new(manager)
            }
            Err(e) => {
                warn!("Failed to enable SQLite persistence, using in-memory: {}", e);
                Arc::new(HistoryManager::new())
            }
        }
    } else {
        info!("Persistence disabled, using in-memory history");
        Arc::new(HistoryManager::new())
    };

    let event_emitter = Arc::new(EventEmitter::new());
    let agent = Arc::new(agent::Agent::new(Arc::new(RwLock::new(
        config.read().agent.clone(),
    ))));
    
    // Create metrics collector and rate limiter
    let metrics = Arc::new(MetricsCollector::new());
    let rate_limiter = Arc::new(RateLimiter::new());

    // Create skill registry with persistence (if configured)
    let skill_registry: Arc<SkillRegistry> = if !config.read().persistence.skills_path.is_empty() {
        let data_dir = config.read().gateway.data_dir.clone();
        let skills_path = config.read().persistence.skills_path.clone();
        let persist_path = if skills_path.starts_with('/') {
            std::path::PathBuf::from(&skills_path)
        } else {
            let base = data_dir
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("tiny_claw")
                });
            base.join(&skills_path)
        };
        // Ensure parent directory exists
        if let Some(parent) = persist_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let persist_path_str = persist_path.to_string_lossy().to_string();
        info!("Enabling skill persistence at: {:?}", persist_path);
        SkillRegistry::new_with_persistence(&persist_path_str)
    } else {
        info!("Skill persistence disabled, using in-memory skills");
        SkillRegistry::new()
    };
    let skill_manager = Arc::new(SessionSkillManager::new(skill_registry.clone()));

    // Create task manager with event emitter and agent
    let task_manager = Arc::new(
        TaskManager::new()
            .with_event_emitter(event_emitter.clone())
            .with_agent(agent.clone())
    );

    // Create scheduler with event emitter and task manager
    let scheduler = Arc::new(
        Scheduler::new()
            .with_event_emitter(event_emitter.clone())
            .with_task_manager(task_manager.clone())
    );
    scheduler.start();
    info!("Scheduled task scheduler started");

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    // Create server state for graceful shutdown
    let server_state = server::ServerState::new(config.read().shutdown.timeout_secs);

    // Create the main session and enable default skills
    let main_session = session_manager.get_or_create_main();
    let main_session_id = main_session.read().id.clone();
    skill_manager.enable_defaults_for_session(&main_session_id);
    info!("Main session created with default skills");

    // Create HTTP state with start time
    let http_state = Arc::new(HttpState {
        config: config.clone(),
        session_manager: session_manager.clone(),
        history_manager: history_manager.clone(),
        agent: agent.clone(),
        shutdown_tx: shutdown_tx.clone(),
        start_time,
        metrics: metrics.clone(),
        rate_limiter: rate_limiter.clone(),
        server_state: server_state.clone(),
        skill_registry: skill_registry.clone(),
        skill_manager: skill_manager.clone(),
        event_emitter: event_emitter.clone(),
        scheduler: scheduler.clone(),
    });

    // Spawn WebSocket server
    let server_config = config.clone();
    let ws_ctx_clone = HandlerContext::new(
        session_manager.clone(),
        history_manager.clone(),
        event_emitter.clone(),
        config.clone(),
        agent.clone(),
        shutdown_tx.clone(),
        skill_manager.clone(), // skill_manager was already cloned into http_state, clone again for WS
        task_manager.clone(), // TaskManager for background task execution
        scheduler.clone(), // Scheduler for scheduled task triggering
    );
    
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = server::start_server(server_config, ws_ctx_clone, shutdown_rx, server_state).await {
            error!("WebSocket server error: {}", e);
        }
    });

    // Spawn HTTP server
    let http_port = 8080u16;
    let http_addr = format!("0.0.0.0:{}", http_port);
    
    // Get static files directory (examples folder in project root)
    let static_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    
    let http_state_clone = http_state.clone();
    let http_handle = tokio::spawn(async move {
        let router = create_router(http_state_clone, static_dir.to_str().unwrap_or("examples"));
        let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
        info!("HTTP server listening on http://{}", http_addr);
        info!("Admin UI available at http://{}/admin.html", http_addr);
        axum::serve(listener, router).await.unwrap();
    });

    // Wait for shutdown signal
    let shutdown_time = tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            Some(Instant::now())
        }
        result = ws_handle => {
            if let Err(e) = result {
                error!("WebSocket server task failed: {}", e);
            }
            None
        }
        result = http_handle => {
            if let Err(e) = result {
                error!("HTTP server task failed: {}", e);
            }
            None
        }
    };

    // Graceful shutdown: flush persistence
    info!("Flushing session history to storage...");
    history_manager.shutdown_persistence();

    if let Some(start) = shutdown_time {
        let elapsed = start.elapsed();
        info!("Shutdown completed in {:?}", elapsed);
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

/// Run TUI mode
fn run_tui_mode() -> Result<(), Box<dyn std::error::Error>> {
    use ratatui::Terminal;
    use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    use crossterm::execute;

    let version = env!("CARGO_PKG_VERSION").to_string();
    
    // Setup terminal
    std::panic::set_hook(Box::new(|_| {
        // Restore terminal on panic
        let _ = execute!(std::io::stderr(), LeaveAlternateScreen);
    }));

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Run TUI
    if let Err(e) = tui::run_tui(&mut terminal, version) {
        eprintln!("TUI error: {}", e);
    }

    // Restore terminal
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    
    Ok(())
}
