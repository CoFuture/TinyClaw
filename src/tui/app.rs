//! TUI Application

use crate::tui::gateway_client::{TuiGatewayClient, TuiGatewayEvent, TuiGatewayStatus};
use crate::tui::state::AppState;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

/// Main TUI application
pub struct TuiApp {
    state: AppState,
    version: String,
    gateway_client: Arc<TokioRwLock<TuiGatewayClient>>,
    gateway_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Simple wrapper for RwLock in async context
use tokio::sync::RwLock as TokioRwLock;

/// Message types for TUI internal communication
enum TuiMessage {
    GatewayEvent(TuiGatewayEvent),
}

impl TuiApp {
    pub fn new(version: String) -> Self {
        let mut state = AppState::new();
        state.add_session("main".to_string());
        state.set_current_session("main".to_string());

        Self {
            state,
            version,
            gateway_client: Arc::new(TokioRwLock::new(TuiGatewayClient::default())),
            gateway_handle: None,
        }
    }

    /// Run the TUI application with gateway integration
    pub async fn run_async<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), io::Error> {
        // Connect to gateway
        self.connect_to_gateway().await;

        // Draw initial frame
        terminal.draw(|f| self.draw(f))?;

        // Create event channel for TUI input handling
        let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<TuiMessage>(32);
        let msg_tx_clone = msg_tx.clone();

        // Spawn gateway event listener
        let event_tx = msg_tx_clone.clone();
        let client_clone = self.gateway_client.clone();
        let gateway_event_handle = tokio::spawn(async move {
            loop {
                let client = client_clone.read().await;
                let mut receiver = client.subscribe();
                drop(client);

                match receiver.recv().await {
                    Ok(event) => {
                        if event_tx.send(TuiMessage::GatewayEvent(event)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        info!("TUI event receiver lagged by {} messages, skipping", n);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        // Main event loop
        loop {
            // Handle terminal input (non-blocking)
            if let Ok(false) = self.handle_terminal_events() {
                break;
            }
            terminal.draw(|f| self.draw(f))?;
            
            // Handle messages from gateway
            if let Ok(msg) = msg_rx.try_recv() {
                match msg {
                    TuiMessage::GatewayEvent(event) => {
                        self.handle_gateway_event(event).await;
                        terminal.draw(|f| self.draw(f))?;
                    }
                }
            }
            
            // Small delay to prevent busy loop
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Cleanup
        gateway_event_handle.abort();
        if let Some(handle) = self.gateway_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Connect to the gateway
    async fn connect_to_gateway(&mut self) {
        let url = "ws://127.0.0.1:18790".to_string();
        info!("TUI gateway URL: {}", url);
        
        let mut client = TuiGatewayClient::new(url);
        
        // Try to connect with timeout
        match tokio::time::timeout(Duration::from_secs(5), client.connect()).await {
            Ok(Ok(())) => {
                self.state.set_connected(true);
                self.state.gateway_status = TuiGatewayStatus::Connected;
                info!("TUI connected to gateway");
                
                // Request sessions list after successful connection
                if let Err(e) = client.list_sessions().await {
                    error!("Failed to request sessions list: {}", e);
                }
            }
            Ok(Err(e)) => {
                error!("Failed to connect to gateway: {}", e);
                self.state.set_error(Some(format!("Gateway connection failed: {}. Run with --gateway flag.", e)));
                self.state.gateway_status = TuiGatewayStatus::Error(e.to_string());
            }
            Err(_) => {
                error!("Gateway connection timed out");
                self.state.set_error(Some("Gateway connection timed out. Is the gateway running?".to_string()));
                self.state.gateway_status = TuiGatewayStatus::Error("timeout".to_string());
            }
        }

        // Store the connected client
        *self.gateway_client.write().await = client;
    }

    /// Handle gateway events
    async fn handle_gateway_event(&mut self, event: TuiGatewayEvent) {
        match event {
            TuiGatewayEvent::Connected => {
                info!("TUI received connected event");
                self.state.set_connected(true);
                self.state.gateway_status = TuiGatewayStatus::Connected;
                self.state.set_error(None);
                // Request sessions list when connected
                let client = self.gateway_client.clone();
                tokio::spawn(async move {
                    let client = client.read().await;
                    if let Err(e) = client.list_sessions().await {
                        error!("Failed to request sessions list: {}", e);
                    }
                });
            }
            TuiGatewayEvent::Disconnected => {
                info!("TUI received disconnected event");
                self.state.set_connected(false);
                self.state.gateway_status = TuiGatewayStatus::Disconnected;
            }
            TuiGatewayEvent::ConnectionError(e) => {
                error!("Gateway connection error: {}", e);
                self.state.set_error(Some(e.clone()));
                self.state.gateway_status = TuiGatewayStatus::Error(e);
            }
            TuiGatewayEvent::Error(msg) => {
                error!("Gateway error: {}", msg);
                self.state.set_error(Some(msg));
                self.state.set_loading(false);
            }
            TuiGatewayEvent::AssistantText(text) => {
                info!("Assistant text: {}", text);
                // Add assistant message to current session
                if let Some(session_id) = &self.state.current_session_id {
                    if let Some(history) = self.state.session_histories.get_mut(session_id) {
                        use crate::types::{Message, Role};
                        let clean_text = text.trim_matches('"').to_string();
                        history.add_message(Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: Role::Assistant,
                            content: clean_text,
                            timestamp: chrono::Utc::now(),
                            tool_call_id: None,
                            tool_name: None,
                        });
                    }
                }
            }
            TuiGatewayEvent::ToolStart { tool, input } => {
                info!("Tool call: {} with {:?}", tool, input);
                // Could display in a status area
                if let Some(session_id) = &self.state.current_session_id {
                    if let Some(history) = self.state.session_histories.get_mut(session_id) {
                        use crate::types::{Message, Role};
                        history.add_message(Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: Role::Assistant,
                            content: format!("[Calling tool: {}]", tool),
                            timestamp: chrono::Utc::now(),
                            tool_call_id: None,
                            tool_name: Some(tool),
                        });
                    }
                }
            }
            TuiGatewayEvent::ToolResult { tool: _, output } => {
                info!("Tool result: {}", output);
            }
            TuiGatewayEvent::TurnEnded(response) => {
                info!("Turn ended: {}", response);
                self.state.set_loading(false);
                // Add final response
                if let Some(session_id) = &self.state.current_session_id {
                    if let Some(history) = self.state.session_histories.get_mut(session_id) {
                        use crate::types::{Message, Role};
                        let clean = response.trim_matches('"').to_string();
                        history.add_message(Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: Role::Assistant,
                            content: clean,
                            timestamp: chrono::Utc::now(),
                            tool_call_id: None,
                            tool_name: None,
                        });
                    }
                }
            }
            TuiGatewayEvent::Pong => {
                debug!("Received pong");
            }
            TuiGatewayEvent::SessionsList(sessions) => {
                info!("Received sessions list: {} sessions", sessions.len());
                // Update sessions in state
                self.state.sessions.clear();
                self.state.session_histories.clear();
                for session in sessions {
                    let session_id = session.id.clone();
                    self.state.sessions.push(session_id.clone());
                    self.state.session_histories.insert(
                        session_id.clone(),
                        crate::types::SessionHistory::new(session_id.clone()),
                    );
                }
                // Set current session to first one if available and not set
                if self.state.current_session_id.is_none() {
                    if let Some(first) = self.state.sessions.first() {
                        self.state.set_current_session(first.clone());
                    }
                }
                // Fetch history for all sessions
                let client = self.gateway_client.clone();
                let session_ids = self.state.sessions.clone();
                tokio::spawn(async move {
                    let client = client.read().await;
                    for session_id in session_ids {
                        if let Err(e) = client.get_history(&session_id).await {
                            error!("Failed to fetch history for {}: {}", session_id, e);
                        }
                    }
                });
            }
            TuiGatewayEvent::SessionHistoryLoaded { session_id, history } => {
                info!("Loaded history for session {}: {} messages", session_id, history.messages.len());
                // Update session history in state
                if let Some(existing) = self.state.session_histories.get_mut(&session_id) {
                    // Merge messages - only add messages that don't already exist
                    let existing_ids: std::collections::HashSet<_> = 
                        existing.messages.iter().map(|m| m.id.clone()).collect();
                    for msg in history.messages {
                        if !existing_ids.contains(&msg.id) {
                            existing.add_message(msg);
                        }
                    }
                } else {
                    // Session exists in list but no history yet
                    self.state.session_histories.insert(session_id.clone(), history);
                }
            }
            TuiGatewayEvent::SessionDeleted { session_id } => {
                info!("Session deleted: {}", session_id);
                // Remove from sessions list
                self.state.sessions.retain(|s| s != &session_id);
                self.state.session_histories.remove(&session_id);
                // If current session was deleted, switch to another
                if self.state.current_session_id.as_ref() == Some(&session_id) {
                    self.state.current_session_id = self.state.sessions.first().cloned();
                    self.state.scroll_offset = 0;
                }
            }
            TuiGatewayEvent::SessionCreated { session_id, label } => {
                info!("New session created: {} ({:?})", session_id, label);
                // Add the new session to state
                if !self.state.sessions.contains(&session_id) {
                    self.state.sessions.push(session_id.clone());
                    self.state.session_histories.insert(
                        session_id.clone(),
                        crate::types::SessionHistory::new(session_id.clone()),
                    );
                }
                // Switch to the new session
                self.state.set_current_session(session_id);
            }
        }
    }

    /// Handle terminal input events (non-blocking check)
    fn handle_terminal_events(&mut self) -> Result<bool, io::Error> {
        use crossterm::event::{self, KeyCode, KeyEventKind};

        if !event::poll(std::time::Duration::from_millis(10))? {
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
                        let content = self.state.input_buffer.clone();
                        let client = self.gateway_client.clone();
                        let session_id = self.state.current_session_id.clone();
                        let mut history_map = self.state.session_histories.clone();
                        
                        // Spawn async task to send message
                        tokio::spawn(async move {
                            let client = client.read().await;
                            if let Some(sid) = &session_id {
                                // Add user message to history
                                if let Some(history) = history_map.get_mut(sid) {
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
                                
                                if let Err(e) = client.send_message(sid, content).await {
                                    error!("Failed to send message: {}", e);
                                }
                            }
                        });
                        
                        self.state.input_buffer.clear();
                        self.state.set_loading(true);
                    }
                    return Ok(true);
                }
                KeyCode::Char(':') => {
                    // Read next char for commands
                    if let Ok(crossterm::event::Event::Key(key)) = event::read() {
                        match key.code {
                            KeyCode::Char('q') => return Ok(false), // Quit
                            KeyCode::Char('h') | KeyCode::Char('?') => {
                                self.state.show_help = !self.state.show_help;
                            }
                            KeyCode::Char('r') => {
                                // Reconnect
                                self.state.set_error(None);
                                self.state.set_connected(false);
                            }
                            KeyCode::Char('n') => {
                                // Create new session
                                let client = self.gateway_client.clone();
                                tokio::spawn(async move {
                                    let client = client.read().await;
                                    if let Err(e) = client.create_session().await {
                                        error!("Failed to create session: {}", e);
                                    }
                                });
                            }
                            KeyCode::Char('d') => {
                                // Delete current session (if not main)
                                if let Some(ref session_id) = self.state.current_session_id {
                                    if session_id != "main" {
                                        let client = self.gateway_client.clone();
                                        let sid = session_id.clone();
                                        tokio::spawn(async move {
                                            let client = client.read().await;
                                            if let Err(e) = client.delete_session(&sid).await {
                                                error!("Failed to delete session: {}", e);
                                            }
                                        });
                                    } else {
                                        self.state.set_error(Some("Cannot delete main session".to_string()));
                                    }
                                }
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
                KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    // Ctrl+C - cancel current operation
                    self.state.input_buffer.clear();
                    self.state.set_loading(false);
                    return Ok(true);
                }
                KeyCode::Char('c') => {
                    if self.state.active_panel == 0 {
                        // Number key for session selection
                        if let Some(idx) = 'c'.to_digit(10) {
                            let idx = idx as usize;
                            if idx < self.state.sessions.len() {
                                let session_id = self.state.sessions[idx].clone();
                                self.state.set_current_session(session_id);
                            }
                        }
                    } else if self.state.active_panel == 2 {
                        self.state.input_buffer.push('c');
                    }
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

        // Main content area - split into sessions (25%) and messages (75%)
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

        // Draw error overlay if there's an error
        if self.state.error_message.is_some() && !self.state.show_help {
            self.draw_error_overlay(f);
        }
    }

    fn draw_title_bar(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let connection_status = if self.state.connected { "●" } else { "○" };
        let loading_indicator = if self.state.loading { " [loading...]" } else { "" };
        let title = format!(
            " TinyClaw v{} {} | Session: {:?}{} ",
            self.version,
            connection_status,
            self.state.current_session_id.as_deref().unwrap_or("None"),
            loading_indicator
        );
        
        let paragraph = Paragraph::new(title.as_str())
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn draw_error_overlay(&self, f: &mut ratatui::Frame<'_>) {
        use ratatui::widgets::Clear;

        let size = f.area();
        
        let error_msg = self.state.error_message.as_deref().unwrap_or("Unknown error");
        let error_content: Vec<Line> = vec![
            Line::from(" ⚠ Connection Issue "),
            Line::from(""),
            Line::from(error_msg),
            Line::from(""),
            Line::from(" Press :r to retry or Esc to dismiss "),
        ];

        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL);

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
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(inner_rect)[1];

        let paragraph = Paragraph::new(error_content)
            .block(block)
            .alignment(Alignment::Center);

        // Clear the area first
        f.render_widget(Clear, box_rect);
        f.render_widget(paragraph, box_rect);
    }

    fn draw_help_overlay(&self, f: &mut ratatui::Frame<'_>) {
        use ratatui::widgets::Clear;

        let size = f.area();
        
        let help_content: Vec<Line> = vec![
            Line::from(" TinyClaw TUI Help "),
            Line::from(""),
            Line::from(" Tab - Switch panel"),
            Line::from(" ↑/↓ - Navigate messages / scroll"),
            Line::from(" Enter - Send message"),
            Line::from(" Backspace - Delete character"),
            Line::from(" Ctrl+D - Clear input"),
            Line::from(" Ctrl+C - Cancel operation"),
            Line::from(" :q - Quit"),
            Line::from(" :r - Reconnect gateway"),
            Line::from(" :n - Create new session"),
            Line::from(" :d - Delete current session"),
            Line::from(" :h - Toggle this help"),
            Line::from(""),
            Line::from(" Press any key to close "),
        ];

        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL);

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

        let paragraph = Paragraph::new(help_content)
            .block(block)
            .alignment(Alignment::Center);

        // Clear the area first
        f.render_widget(Clear, box_rect);
        f.render_widget(paragraph, box_rect);
    }
}

/// Run the TUI application (blocking)
pub fn run_tui(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, version: String) -> Result<(), io::Error> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = TuiApp::new(version);
    rt.block_on(app.run_async(terminal))
}

/// Run the TUI application with an existing tokio runtime
#[allow(dead_code)]
pub async fn run_tui_with_runtime<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    version: String,
) -> Result<(), io::Error> {
    let mut app = TuiApp::new(version);
    app.run_async(terminal).await
}
