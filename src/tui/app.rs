//! TUI Application

use crate::tui::gateway_client::{TuiGatewayClient, TuiGatewayEvent, TuiGatewayStatus};
use crate::tui::persistence::TuiPersistence;
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
    /// TUI local persistence (optional - gracefully degrades if unavailable)
    persistence: Option<TuiPersistence>,
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

        // Initialize TUI persistence (optional - gracefully degrades)
        let persistence = match TuiPersistence::new() {
            Ok(p) => {
                info!("TUI persistence initialized");
                // Load any existing session histories from disk
                let persisted_histories = p.load_all();
                if !persisted_histories.is_empty() {
                    info!("Recovered {} session histories from disk", persisted_histories.len());
                    for history in persisted_histories {
                        let session_id = history.session_id.clone();
                        state.session_histories.insert(session_id.clone(), history);
                        if !state.sessions.contains(&session_id) {
                            state.sessions.push(session_id);
                        }
                    }
                }
                Some(p)
            }
            Err(e) => {
                info!("TUI persistence unavailable ({}), messages won't be persisted locally", e);
                None
            }
        };

        Self {
            state,
            version,
            gateway_client: Arc::new(TokioRwLock::new(TuiGatewayClient::default())),
            gateway_handle: None,
            persistence,
        }
    }

    /// Save the current session's history to persistence
    fn save_current_history(&self) {
        if let (Some(session_id), Some(ref persist)) = (&self.state.current_session_id, &self.persistence) {
            if let Some(history) = self.state.session_histories.get(session_id) {
                persist.save_history(history);
            }
        }
    }

    /// Save a specific session's history to persistence
    fn save_session_history(&self, session_id: &str) {
        if let Some(ref persist) = self.persistence {
            if let Some(history) = self.state.session_histories.get(session_id) {
                persist.save_history(history);
            }
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
                        self.save_current_history();
                    }
                }
            }
            TuiGatewayEvent::TurnStarted { session_id, message } => {
                info!("Turn started: {} - {}", session_id, message);
                self.state.set_thinking();
                self.state.set_loading(true);
                // Add user message to history if not already added locally
                if let Some(current_sid) = &self.state.current_session_id {
                    if current_sid == &session_id {
                        if let Some(history) = self.state.session_histories.get_mut(current_sid) {
                            // Check if message already exists (may have been added locally on send)
                            let msg_exists = history.messages.iter().any(|m| {
                                matches!(m.role, crate::types::Role::User) && m.content == message
                            });
                            if !msg_exists {
                                use crate::types::{Message, Role};
                                history.add_message(Message {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    role: Role::User,
                                    content: message.clone(),
                                    timestamp: chrono::Utc::now(),
                                    tool_call_id: None,
                                    tool_name: None,
                                });
                                self.save_current_history();
                            }
                        }
                    }
                }
            }
            TuiGatewayEvent::TurnThinking { session_id } => {
                debug!("Turn thinking: {}", session_id);
                self.state.set_thinking();
                self.state.set_loading(true);
            }
            TuiGatewayEvent::ToolStart { tool, .. } => {
                info!("Tool started: {}", tool);
                self.state.set_using_tool(&tool);
                // Add tool call message to history
                if let Some(session_id) = &self.state.current_session_id {
                    if let Some(history) = self.state.session_histories.get_mut(session_id) {
                        use crate::types::{Message, Role};
                        history.add_message(Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: Role::Assistant,
                            content: format!("[Calling tool: {}]", tool),
                            timestamp: chrono::Utc::now(),
                            tool_call_id: None,
                            tool_name: Some(tool.clone()),
                        });
                        self.save_current_history();
                    }
                }
            }
            TuiGatewayEvent::ToolResult { tool, output } => {
                info!("Tool result: {} = {}", tool, output);
                // Add tool result to history
                if let Some(session_id) = &self.state.current_session_id {
                    if let Some(history) = self.state.session_histories.get_mut(session_id) {
                        use crate::types::{Message, Role};
                        // Truncate long outputs for display
                        let display_output = if output.len() > 200 {
                            format!("{}...[truncated]", &output[..200])
                        } else {
                            output.clone()
                        };
                        history.add_message(Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: Role::Tool,
                            content: display_output,
                            timestamp: chrono::Utc::now(),
                            tool_call_id: None,
                            tool_name: Some(tool.clone()),
                        });
                        self.save_current_history();
                    }
                }
                self.state.set_idle();
            }
            TuiGatewayEvent::TurnEnded(response) => {
                info!("Turn ended: {}", response);
                self.state.set_loading(false);
                self.state.set_idle();
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
                        self.save_current_history();
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
                // Save merged history to persistence
                self.save_session_history(&session_id);
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
                // Remove from persistence
                if let Some(ref persist) = self.persistence {
                    persist.delete_session(&session_id);
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
                self.state.set_current_session(session_id.clone());
                // Save new session to persistence
                self.save_session_history(&session_id);
            }
            TuiGatewayEvent::SessionRenamed { session_id, label } => {
                info!("Session renamed: {} -> {:?}", session_id, label);
                // Refresh the sessions list to get updated labels
                let client = self.gateway_client.clone();
                tokio::spawn(async move {
                    let client = client.read().await;
                    if let Err(e) = client.list_sessions().await {
                        error!("Failed to refresh sessions list after rename: {}", e);
                    }
                });
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
                    if self.state.active_panel == 2 {
                        // Tab completion in input panel
                        if self.state.completion.active {
                            // Cycle to next completion
                            self.state.completion.next();
                            // Update input with current completion
                            self.state.input_buffer = self.state.completion.current()
                                .unwrap_or(&self.state.input_buffer).to_string();
                        } else {
                            // Start completion
                            let candidates = self.state.get_completion_candidates();
                            if !candidates.is_empty() {
                                self.state.completion.activate(&self.state.input_buffer, candidates);
                                self.state.input_buffer = self.state.completion.current()
                                    .unwrap_or(&self.state.input_buffer).to_string();
                            }
                        }
                    } else {
                        // Switch panel
                        self.state.active_panel = (self.state.active_panel + 1) % 3;
                    }
                    return Ok(true);
                }
                KeyCode::BackTab => {
                    // Shift+Tab - previous completion
                    if self.state.active_panel == 2 && self.state.completion.active {
                        self.state.completion.prev();
                        self.state.input_buffer = self.state.completion.current()
                            .unwrap_or(&self.state.input_buffer).to_string();
                    }
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
                    // Handle rename mode
                    if self.state.rename_mode {
                        if let Some(ref session_id) = self.state.current_session_id {
                            let new_label = self.state.input_buffer.trim().to_string();
                            if !new_label.is_empty() {
                                let client = self.gateway_client.clone();
                                let sid = session_id.clone();
                                tokio::spawn(async move {
                                    let client = client.read().await;
                                    if let Err(e) = client.rename_session(&sid, &new_label).await {
                                        error!("Failed to rename session: {}", e);
                                    }
                                });
                            }
                        }
                        self.state.rename_mode = false;
                        self.state.input_buffer.clear();
                        return Ok(true);
                    }
                    
                    if self.state.current_session_id.is_some() && !self.state.input_buffer.is_empty() {
                        let content = self.state.input_buffer.clone();
                        let client = self.gateway_client.clone();
                        let session_id = self.state.current_session_id.clone();
                        
                        // Add user message to state BEFORE sending (so it persists even if send fails)
                        if let Some(sid) = &session_id {
                            if let Some(history) = self.state.session_histories.get_mut(sid) {
                                use crate::types::{Message, Role};
                                history.add_message(Message {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    role: Role::User,
                                    content: content.clone(),
                                    timestamp: chrono::Utc::now(),
                                    tool_call_id: None,
                                    tool_name: None,
                                });
                                // Save to persistence
                                self.save_current_history();
                            }
                        }
                        
                        // Spawn async task to send message
                        tokio::spawn(async move {
                            let client = client.read().await;
                            if let Some(sid) = &session_id {
                                if let Err(e) = client.send_message(sid, content).await {
                                    error!("Failed to send message: {}", e);
                                }
                            }
                        });
                        
                        self.state.input_buffer.clear();
                        self.state.set_thinking();
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
                                // Check for :ren (rename) - need to peek next chars
                                // First check if 'e' is already in the event buffer
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(e_key)) = event::read() {
                                        if let KeyCode::Char('e') = e_key.code {
                                            // Got :re, check for :ren
                                            if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                if let Ok(crossterm::event::Event::Key(n_key)) = event::read() {
                                                    if let KeyCode::Char('n') = n_key.code {
                                                        // Got :ren - enter rename mode
                                                        if self.state.current_session_id.is_some() {
                                                            self.state.rename_mode = true;
                                                            self.state.input_buffer = String::new();
                                                            return Ok(true);
                                                        }
                                                        return Ok(true);
                                                    }
                                                }
                                            }
                                            // Not :ren, add 're' to buffer
                                            if self.state.active_panel == 2 {
                                                self.state.input_buffer.push('r');
                                                self.state.input_buffer.push('e');
                                                self.state.completion.reset();
                                            }
                                            return Ok(true);
                                        }
                                    }
                                }
                                // No more chars buffered, treat as single 'r' -> reconnect
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
                            KeyCode::Char('c') => {
                                // Check for :rc (reconnect)
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(c_key)) = event::read() {
                                        if let KeyCode::Char('c') = c_key.code {
                                            // Got :rc - reconnect
                                            self.state.set_error(None);
                                            self.state.set_connected(false);
                                            return Ok(true);
                                        }
                                        // Was :c but not :rc - add 'c' to buffer
                                        if self.state.active_panel == 2 {
                                            self.state.input_buffer.push('c');
                                            self.state.completion.reset();
                                        }
                                        return Ok(true);
                                    }
                                }
                                // No more chars buffered, treat as single 'c'
                                if self.state.active_panel == 2 {
                                    self.state.input_buffer.push('c');
                                    self.state.completion.reset();
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
                    self.state.completion.reset();
                    return Ok(true);
                }
                KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    // Ctrl+C - cancel current operation
                    self.state.input_buffer.clear();
                    self.state.completion.reset();
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
                        self.state.completion.reset();
                    }
                    return Ok(true);
                }
                KeyCode::Char(c) => {
                    if self.state.active_panel == 2 {
                        self.state.input_buffer.push(c);
                        self.state.completion.reset();
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
                        self.state.completion.reset();
                    }
                    return Ok(true);
                }
                KeyCode::Esc => {
                    if self.state.show_help {
                        self.state.show_help = false;
                    } else if self.state.rename_mode {
                        self.state.rename_mode = false;
                        self.state.input_buffer.clear();
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
        use ratatui::{style::{Color, Style}, text::{Line, Span}};
        
        let connection_str = if self.state.connected { 
            "● Connected" 
        } else { 
            "○ Disconnected" 
        };
        let connection_color = if self.state.connected { Color::Green } else { Color::Red };
        
        let session_str = self.state.current_session_id.as_deref()
            .map(|s| if s == "main" { "main" } else { s })
            .unwrap_or("none");
        
        // Build title line with styled spans
        let spans: Vec<Span> = vec![
            Span::raw(" TinyClaw v"),
            Span::raw(&self.version),
            Span::raw(" | "),
            Span::styled(connection_str, Style::default().fg(connection_color)),
            Span::raw(" | Session: "),
            Span::styled(session_str, Style::default().fg(Color::Cyan)),
        ];
        
        let paragraph = Paragraph::new(Line::from(spans))
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
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::Span;
        use crate::tui::state::{CommandCategory, TuiCommandMeta, TUI_COMMANDS};

        let size = f.area();
        
        // Build structured help content using vec! macro
        let help_content: Vec<Line> = {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" TinyClaw v", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(self.version.as_str()),
                    Span::raw(" Help "),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("  Tab "),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Switch panel "),
                    Span::styled("|", Style::default().fg(Color::DarkGray)),
                    Span::raw(" ↑/↓ - Navigate/scroll"),
                ]),
                Line::from(vec![
                    Span::raw("  Enter "),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Send message "),
                    Span::styled("|", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Backspace - Delete"),
                ]),
                Line::from(vec![
                    Span::raw("  Ctrl+D "),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Clear input "),
                    Span::styled("|", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Ctrl+C - Cancel"),
                ]),
                Line::from(vec![
                    Span::raw("  Esc "),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Close/dismiss "),
                    Span::styled("|", Style::default().fg(Color::DarkGray)),
                    Span::raw(" Shift+Tab - Previous completion"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Commands", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
            ];
            
            // Group commands by category
            let session_cmds: Vec<&TuiCommandMeta> = TUI_COMMANDS
                .iter()
                .filter(|c| c.category == CommandCategory::Session)
                .collect();
            let conn_cmds: Vec<&TuiCommandMeta> = TUI_COMMANDS
                .iter()
                .filter(|c| c.category == CommandCategory::Connection)
                .collect();
            let nav_cmds: Vec<&TuiCommandMeta> = TUI_COMMANDS
                .iter()
                .filter(|c| c.category == CommandCategory::Navigation)
                .collect();
            
            // Session commands
            lines.push(Line::from(vec![
                Span::styled("  Session:", Style::default().fg(Color::Cyan)),
            ]));
            for cmd in session_cmds {
                let aliases_str = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!("/{}", cmd.aliases.join("/"))
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("{}{}", cmd.full_name, aliases_str), Style::default().fg(Color::Green)),
                    Span::raw(" - "),
                    Span::raw(cmd.description),
                ]));
            }
            
            // Connection commands
            lines.push(Line::from(vec![
                Span::styled("  Connection:", Style::default().fg(Color::Cyan)),
            ]));
            for cmd in conn_cmds {
                let aliases_str = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!("/{}", cmd.aliases.join("/"))
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("{}{}", cmd.full_name, aliases_str), Style::default().fg(Color::Green)),
                    Span::raw(" - "),
                    Span::raw(cmd.description),
                ]));
            }
            
            // Navigation commands
            lines.push(Line::from(vec![
                Span::styled("  Navigation:", Style::default().fg(Color::Cyan)),
            ]));
            for cmd in nav_cmds {
                let aliases_str = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!("/{}", cmd.aliases.join("/"))
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("{}{}", cmd.full_name, aliases_str), Style::default().fg(Color::Green)),
                    Span::raw(" - "),
                    Span::raw(cmd.description),
                ]));
            }
            
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(" Press Esc or any key to close ", Style::default().fg(Color::DarkGray)),
            ]));
            
            lines
        };
        
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
