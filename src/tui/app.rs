//! TUI Application

use crate::tui::state::AppState;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Line,
    Terminal,
};
use std::io;
use tracing::{error, info};

/// Main TUI application
pub struct TuiApp {
    state: AppState,
    version: String,
}

impl TuiApp {
    pub fn new(version: String) -> Self {
        let mut state = AppState::new();
        state.add_session("main".to_string());
        state.set_current_session("main".to_string());

        Self { state, version }
    }

    pub fn run<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), io::Error> {
        // Draw initial frame
        terminal.draw(|f| self.draw(f))?;

        loop {
            // Handle input
            match self.handle_events() {
                Ok(false) => break, // Quit signal
                Ok(true) => {}      // Continue
                Err(e) => {
                    error!("Error handling input: {}", e);
                }
            }

            // Draw frame
            terminal.draw(|f| self.draw(f))?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<bool, io::Error> {
        use crossterm::event::{self, KeyCode, KeyEventKind};

        if !event::poll(std::time::Duration::from_millis(100))? {
            return Ok(true); // No event, continue
        }

        let event = event::read()?;

        if let crossterm::event::Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(true);
            }

            match key.code {
                KeyCode::Tab => {
                    self.state.active_panel = (self.state.active_panel + 1) % 3;
                    return Ok(true);
                }
                KeyCode::Up => {
                    if self.state.scroll_offset > 0 {
                        self.state.scroll_offset -= 1;
                    }
                    return Ok(true);
                }
                KeyCode::Down => {
                    let msg_count = self.state.get_current_messages().len();
                    let max_scroll = msg_count.saturating_sub(1);
                    if self.state.scroll_offset < max_scroll {
                        self.state.scroll_offset += 1;
                    }
                    return Ok(true);
                }
                KeyCode::Enter => {
                    if self.state.current_session_id.is_some() && !self.state.input_buffer.is_empty() {
                        self.submit_message();
                    }
                    return Ok(true);
                }
                KeyCode::Char(':') => {
                    // Read next char for commands
                    if let crossterm::event::Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char('q') => return Ok(false), // Quit
                            KeyCode::Char('h') | KeyCode::Char('?') => {
                                self.state.show_help = !self.state.show_help;
                            }
                            _ => {}
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    // Ctrl+D - clear input
                    self.state.input_buffer.clear();
                    return Ok(true);
                }
                KeyCode::Char(c) => {
                    if self.state.active_panel == 2 {
                        self.state.input_buffer.push(c);
                    } else if self.state.active_panel == 0 {
                        // Session selection with number keys
                        if let Some(idx) = c.to_digit(10) {
                            let idx = idx as usize;
                            if idx < self.state.sessions.len() {
                                let session_id = self.state.sessions[idx].clone();
                                self.state.set_current_session(session_id);
                            }
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Backspace => {
                    if self.state.active_panel == 2 {
                        self.state.input_buffer.pop();
                    }
                    return Ok(true);
                }
                KeyCode::Esc => {
                    if self.state.show_help {
                        self.state.show_help = false;
                    } else {
                        return Ok(false); // Quit
                    }
                    return Ok(true);
                }
                _ => {}
            }
        }

        Ok(true)
    }

    fn submit_message(&mut self) {
        let content = self.state.input_buffer.clone();
        self.state.input_buffer.clear();

        if let Some(session_id) = &self.state.current_session_id {
            info!("Submitting message to session '{}': {}", session_id, content);
            
            // Add user message to history
            if let Some(history) = self.state.session_histories.get_mut(session_id) {
                use crate::types::{Message, Role};
                history.add_message(Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: Role::User,
                    content: content.clone(),
                    timestamp: chrono::Utc::now(),
                    tool_call_id: None,
                    tool_name: None,
                });
            }

            // TODO: In a full implementation, this would send to the agent
            // For now, we'll just show the message was added
            info!("Message added to session '{}' (agent response pending via WebSocket)", session_id);
        }
    }

    fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        let size = f.area();
        
        // Layout: title bar (1 line), main content (remaining), help bar (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(size);

        // Title bar
        self.draw_title_bar(f, chunks[0]);

        // Main content area - split into sessions (30%) and messages (70%)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(75),
            ])
            .split(chunks[1]);

        // Input area at bottom of messages panel
        let msg_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(main_chunks[1]);

        // Draw panels
        crate::tui::components::draw_sessions_panel(f, main_chunks[0], &self.state);
        crate::tui::components::draw_messages_panel(f, msg_chunks[0], &self.state);
        crate::tui::components::draw_input_panel(f, msg_chunks[1], &self.state);
        crate::tui::components::draw_help_bar(f, chunks[2]);

        // Draw help overlay if shown
        if self.state.show_help {
            self.draw_help_overlay(f);
        }
    }

    fn draw_title_bar(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let title = format!(" TinyClaw v{} ", self.version);
        
        let paragraph = ratatui::widgets::Paragraph::new(title.as_str())
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn draw_help_overlay(&self, f: &mut ratatui::Frame<'_>) {
        use ratatui::widgets::{Block, Clear};

        let size = f.area();
        
        let help_content: Vec<Line> = vec![
            Line::from(" TinyClaw TUI Help "),
            Line::from(""),
            Line::from(" Tab - Switch panel"),
            Line::from(" ↑/↓ - Navigate messages / scroll"),
            Line::from(" Enter - Send message"),
            Line::from(" Backspace - Delete character"),
            Line::from(" Ctrl+D - Clear input"),
            Line::from(" :q - Quit"),
            Line::from(" :h - Toggle this help"),
            Line::from(""),
            Line::from(" Press any key to close "),
        ];

        let block = Block::default()
            .title(" Help ")
            .borders(ratatui::widgets::Borders::ALL);

        let inner_rect = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(size)[1];

        let box_rect = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(inner_rect)[1];

        let paragraph = ratatui::widgets::Paragraph::new(help_content)
            .block(block)
            .alignment(Alignment::Center);

        // Clear the area first
        f.render_widget(Clear, box_rect);
        f.render_widget(paragraph, box_rect);
    }
}

/// Run the TUI application
pub fn run_tui(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, version: String) -> Result<(), io::Error> {
    let mut app = TuiApp::new(version);
    app.run(terminal)
}
