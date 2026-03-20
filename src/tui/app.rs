//! TUI Application

use std::sync::Arc;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parking_lot::RwLock;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::time::Duration;
use crate::config::Config;
use crate::gateway::session::SessionManager;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application state
pub struct App {
    pub config: Arc<RwLock<Config>>,
    pub session_manager: Arc<SessionManager>,
    pub current_tab: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Arc<RwLock<Config>>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            config,
            session_manager,
            current_tab: 0,
            should_quit: false,
        }
    }

    /// Handle key events
    pub fn handle_key(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.current_tab = (self.current_tab + 1) % 3;
            }
            KeyCode::Char('1') => self.current_tab = 0,
            KeyCode::Char('2') => self.current_tab = 1,
            KeyCode::Char('3') => self.current_tab = 2,
            _ => {}
        }
    }

    /// Update (refresh) the state
    pub fn update(&mut self) {
        // Could refresh data here if needed
    }
}

/// Run the TUI
pub async fn run(
    config: Arc<RwLock<Config>>,
    session_manager: Arc<SessionManager>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config, session_manager);

    // Main loop
    loop {
        terminal.draw(|f| super::ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        app.update();

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
