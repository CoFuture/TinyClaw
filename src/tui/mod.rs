//! TUI - Terminal User Interface module

pub mod app;
pub mod ui;

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::Config;
use crate::gateway::session::SessionManager;

/// Run the TUI application
pub async fn run_tui(
    config: Arc<RwLock<Config>>,
    session_manager: Arc<SessionManager>,
) -> Result<()> {
    app::run(config, session_manager).await?;
    Ok(())
}
