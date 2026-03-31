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
mod preferences;
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
use preferences::PreferencesManager;
use ratelimit::RateLimiter;
use agent::{SkillRegistry, SessionSkillManager, TaskManager, Scheduler, SessionNotesManager, MemoryManager, ToolPatternLearner};

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

    // Create execution safety manager for preventing runaway tool loops
    let execution_safety_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tiny_claw")
        .join("execution_safety");
    let safety_manager = Arc::new(crate::agent::ExecutionSafetyManager::new(execution_safety_dir));
    info!("Execution safety manager initialized");

    let event_emitter = Arc::new(EventEmitter::new());
    let agent = Arc::new(agent::Agent::new(Arc::new(RwLock::new(
        config.read().agent.clone(),
    ))).with_event_emitter(event_emitter.clone()).with_safety_manager(safety_manager.clone()));
    
    // Create turn history manager with persistence
    let turn_history_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tiny_claw")
        .join("turn_history");
    let turn_history = match agent::TurnHistoryManager::new_with_persistence(&turn_history_dir) {
        Ok(mgr) => {
            info!("Turn history loaded from: {:?}", turn_history_dir);
            Arc::new(mgr)
        }
        Err(e) => {
            tracing::warn!("Failed to load turn history: {}, using in-memory", e);
            Arc::new(agent::TurnHistoryManager::new())
        }
    };
    info!("Turn history manager initialized");
    
    // Create self-evaluation manager with persistence
    let self_eval_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tiny_claw")
        .join("self_evaluation");
    let self_evaluation_manager = if let Err(e) = std::fs::create_dir_all(&self_eval_dir) {
        tracing::warn!("Failed to create self-evaluation dir: {}, using in-memory", e);
        Arc::new(agent::SelfEvaluationManager::new())
    } else {
        Arc::new(agent::SelfEvaluationManager::with_persistence(self_eval_dir))
    };
    info!("Self-evaluation manager initialized");
    
    // Create session quality manager for tracking session quality metrics
    let session_quality_manager = Arc::new(agent::SessionQualityManager::new());
    info!("Session quality manager initialized");
    
    // Create context health monitor for tracking context utilization and health
    let context_health_monitor = Arc::new(agent::ContextHealthMonitor::default());
    info!("Context health monitor initialized");
    
    // Create conversation summary manager for tracking conversation state
    let conversation_summary_manager = Arc::new(parking_lot::RwLock::new(
        crate::agent::ConversationSummaryManager::new()
    ));
    info!("Conversation summary manager initialized");
    
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

    // Create preferences manager with persistence
    let preferences_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tiny_claw")
        .join("preferences.json");
    // Ensure parent directory exists
    if let Some(parent) = preferences_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let preferences_manager = Arc::new(PreferencesManager::new_with_persistence(&preferences_path));
    info!("User preferences loaded from: {:?}", preferences_path);

    // Create session notes manager
    let session_notes_manager = Arc::new(SessionNotesManager::new());
    info!("Session notes manager initialized");

    // Create suggestion manager for interactive suggestions
    let suggestion_manager = Arc::new(crate::agent::SuggestionManager::new());
    info!("Suggestion manager initialized");

    // Create memory manager for long-term fact storage
    let memory_manager = Arc::new(MemoryManager::new());
    info!("Memory manager initialized");

    // Create tool pattern learner for learning from turn history
    let tool_pattern_learner = Arc::new(RwLock::new(ToolPatternLearner::new()));
    info!("Tool pattern learner initialized");

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
        preferences: preferences_manager.clone(),
        session_notes: session_notes_manager.clone(),
        suggestion_manager: suggestion_manager.clone(),
        memory_manager: memory_manager.clone(),
        turn_history: turn_history.clone(),
        tool_pattern_learner: tool_pattern_learner.clone(),
        conversation_summary: conversation_summary_manager.clone(),
        self_evaluation_manager: self_evaluation_manager.clone(),
        session_quality_manager: session_quality_manager.clone(),
        context_health_monitor: context_health_monitor.clone(),
    });

    // Spawn WebSocket server
    let server_config = config.clone();
    let suggestion_engines: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, crate::agent::suggestion::SuggestionEngine>>> =
        std::sync::Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
    let suggestion_engines_ws = suggestion_engines.clone();
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
        suggestion_engines_ws, // Suggestion engines for proactive suggestions
        preferences_manager.clone(), // User preferences manager
        session_notes_manager.clone(), // Session notes manager
        suggestion_manager.clone(), // Suggestion manager for interactive suggestions
        memory_manager.clone(), // Memory manager for long-term fact storage
        turn_history.clone(), // Turn history manager
        conversation_summary_manager.clone(), // Conversation summary manager
        self_evaluation_manager.clone(), // Self-evaluation manager
        session_quality_manager.clone(), // Session quality manager
        context_health_monitor.clone(), // Context health monitor
        tool_pattern_learner.clone(), // Tool pattern learner for learning from turns
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
