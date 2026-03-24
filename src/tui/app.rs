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
                // Request circuit breaker state
                if let Err(e) = client.get_circuit_breaker().await {
                    error!("Failed to request circuit breaker state: {}", e);
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
                // Request sessions list and circuit breaker state when connected
                let client = self.gateway_client.clone();
                let client2 = client.clone();
                tokio::spawn(async move {
                    let client = client.read().await;
                    if let Err(e) = client.get_circuit_breaker().await {
                        error!("Failed to request circuit breaker state: {}", e);
                    }
                });
                tokio::spawn(async move {
                    let client = client2.read().await;
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
                        // Only add if not empty (empty text during streaming means no new content)
                        if !clean_text.is_empty() {
                            history.add_message(Message {
                                id: uuid::Uuid::new_v4().to_string(),
                                role: Role::Assistant,
                                content: clean_text,
                                timestamp: chrono::Utc::now(),
                                tool_call_id: None,
                                tool_name: None,
                            });
                            self.save_current_history();
                            // Mark that a message was created (for streaming deduplication)
                            if self.state.is_streaming {
                                self.state.mark_streaming_message_created();
                            }
                        }
                    }
                }
            }
            TuiGatewayEvent::StreamingText { session_id, text } => {
                debug!("Streaming text for {}: {}", session_id, text);
                // Only handle if this is for the current session
                if self.state.current_session_id.as_deref() == Some(&session_id) {
                    // Start streaming if not already
                    if !self.state.is_streaming {
                        self.state.start_streaming(&session_id);
                    }
                    // Accumulate the partial text
                    self.state.append_streaming_text(&text);
                }
            }
            TuiGatewayEvent::TurnStarted { session_id, message } => {
                info!("Turn started: {} - {}", session_id, message);
                self.state.set_thinking();
                self.state.set_loading(true);
                // Reset streaming state for the new turn
                self.state.reset_streaming_state();
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
                
                // Determine what text to use for the message
                let message_text = if self.state.is_streaming && !self.state.streaming_message_created {
                    // Streaming was active but no message created yet (Ollama streaming path)
                    // Use accumulated partial text, or fallback to response if empty
                    let partial = self.state.end_streaming().unwrap_or_default();
                    if !partial.is_empty() {
                        partial
                    } else {
                        response.trim_matches('"').to_string()
                    }
                } else if self.state.is_streaming && self.state.streaming_message_created {
                    // Message already created via AssistantText (non-streaming path)
                    // Don't create duplicate - just clear streaming state
                    self.state.end_streaming();
                    String::new()
                } else {
                    // Not streaming - use response (shouldn't normally happen but handle it)
                    response.trim_matches('"').to_string()
                };
                
                // Add message only if we have text and no duplicate
                if !message_text.is_empty() {
                    if let Some(session_id) = &self.state.current_session_id {
                        if let Some(history) = self.state.session_histories.get_mut(session_id) {
                            use crate::types::{Message, Role};
                            history.add_message(Message {
                                id: uuid::Uuid::new_v4().to_string(),
                                role: Role::Assistant,
                                content: message_text,
                                timestamp: chrono::Utc::now(),
                                tool_call_id: None,
                                tool_name: None,
                            });
                            self.save_current_history();
                        }
                    }
                }
            }
            TuiGatewayEvent::TurnCancelled { session_id } => {
                info!("Turn cancelled for session: {}", session_id);
                self.state.set_loading(false);
                self.state.set_idle();
                self.state.cancel_streaming();
                self.state.set_error(Some("Turn cancelled".to_string()));
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
            TuiGatewayEvent::CircuitBreakerState(state) => {
                self.state.circuit_breaker_state = state;
            }
            TuiGatewayEvent::SessionNotesLoaded { session_id, notes } => {
                if self.state.notes_mode && self.state.notes_session_id.as_ref() == Some(&session_id) {
                    let content = format_notes_display(&notes);
                    self.state.notes_content = Some(content);
                }
            }
            TuiGatewayEvent::SessionInstructionsLoaded { session_id, instructions } => {
                if self.state.instructions_mode && self.state.instructions_session_id.as_ref() == Some(&session_id) {
                    self.state.current_instructions = instructions.clone();
                    self.state.input_buffer = instructions.unwrap_or_default();
                }
            }
            TuiGatewayEvent::ActionPlanConfirm { session_id, plan_id, tools } => {
                info!("Action plan confirmation requested: {} with {} tools", plan_id, tools.len());
                self.state.confirm_mode = true;
                self.state.confirm_session_id = Some(session_id);
                self.state.confirm_plan_id = Some(plan_id);
                self.state.confirm_tools = tools;
                self.state.set_loading(false);
            }
            TuiGatewayEvent::ActionDenied { session_id, .. } => {
                info!("Action plan denied for session: {}", session_id);
                self.state.confirm_mode = false;
                self.state.confirm_session_id = None;
                self.state.confirm_plan_id = None;
                self.state.confirm_tools.clear();
            }
            TuiGatewayEvent::TurnUsage { session_id, input_tokens, output_tokens, total_tokens } => {
                debug!("Token usage for {}: in={}, out={}, total={}", session_id, input_tokens, output_tokens, total_tokens);
                self.state.update_token_usage(&session_id, input_tokens, output_tokens);
            }
            TuiGatewayEvent::ContextSummarized { session_id, messages_summarized, summary_tokens, compression_ratio, .. } => {
                // Format summary info for display: "📝 10 msgs → 200 tokens (10%)"
                let info = format!(
                    "📝 {} msgs → {} tokens ({:.0}%)",
                    messages_summarized,
                    summary_tokens,
                    compression_ratio * 100.0
                );
                self.state.last_summary_info = Some(info.clone());
                debug!("Context summarized for {}: {}", session_id, info);
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
                    if self.state.active_panel == 2 {
                        // Input panel: navigate input history
                        self.state.input_history_up();
                    } else {
                        // Message panel: scroll up
                        if self.state.scroll_offset > 0 {
                            self.state.scroll_offset -= 1;
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Down => {
                    if self.state.active_panel == 2 {
                        // Input panel: navigate input history
                        self.state.input_history_down();
                    } else {
                        // Message panel: scroll down
                        let msg_count = self.state.get_current_messages().len();
                        let max_scroll = msg_count.saturating_sub(1);
                        if self.state.scroll_offset < max_scroll {
                            self.state.scroll_offset += 1;
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Enter => {
                    // Handle search mode
                    if self.state.search_mode {
                        // Perform search with current query
                        if self.state.input_buffer.starts_with('/') {
                            let query = self.state.input_buffer.trim_start_matches('/').to_string();
                            self.state.search(&query);
                        }
                        return Ok(true);
                    }
                    
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
                    
                    // Handle instructions mode
                    if self.state.instructions_mode {
                        if let Some(ref session_id) = self.state.instructions_session_id {
                            let instructions = self.state.input_buffer.trim().to_string();
                            let instructions_opt = if instructions.is_empty() { None } else { Some(instructions) };
                            let client = self.gateway_client.clone();
                            let sid = session_id.clone();
                            tokio::spawn(async move {
                                let client = client.read().await;
                                if let Err(e) = client.set_session_instructions(&sid, instructions_opt.as_deref()).await {
                                    error!("Failed to set session instructions: {}", e);
                                }
                            });
                        }
                        self.state.instructions_mode = false;
                        self.state.instructions_session_id = None;
                        self.state.input_buffer.clear();
                        self.state.current_instructions = None;
                        return Ok(true);
                    }
                    
                    // Handle action confirmation mode (Enter confirms, Esc denies)
                    if self.state.confirm_mode {
                        if let (Some(ref session_id), Some(ref plan_id)) = 
                            (self.state.confirm_session_id.clone(), self.state.confirm_plan_id.clone()) 
                        {
                            // Check if user typed :deny or :n
                            let input_lower = self.state.input_buffer.to_lowercase();
                            let confirmed = !input_lower.starts_with(":deny") && !input_lower.starts_with(":n");
                            
                            let client = self.gateway_client.clone();
                            let sid = session_id.clone();
                            let pid = plan_id.clone();
                            tokio::spawn(async move {
                                let client = client.read().await;
                                if let Err(e) = client.confirm_action(&sid, &pid, confirmed).await {
                                    error!("Failed to send action confirmation: {}", e);
                                }
                            });
                        }
                        self.state.confirm_mode = false;
                        self.state.confirm_session_id = None;
                        self.state.confirm_plan_id = None;
                        self.state.confirm_tools.clear();
                        self.state.input_buffer.clear();
                        return Ok(true);
                    }
                    
                    // Handle :confirm and :y commands (confirm from normal mode)
                    let input_lower = self.state.input_buffer.to_lowercase();
                    if (input_lower.starts_with(":confirm") || input_lower == ":y") 
                        && self.state.confirm_session_id.is_some() 
                        && self.state.confirm_plan_id.is_some() 
                    {
                        if let (Some(ref session_id), Some(ref plan_id)) = 
                            (self.state.confirm_session_id.clone(), self.state.confirm_plan_id.clone()) 
                        {
                            let client = self.gateway_client.clone();
                            let sid = session_id.clone();
                            let pid = plan_id.clone();
                            tokio::spawn(async move {
                                let client = client.read().await;
                                if let Err(e) = client.confirm_action(&sid, &pid, true).await {
                                    error!("Failed to confirm action: {}", e);
                                }
                            });
                        }
                        self.state.confirm_mode = false;
                        self.state.confirm_session_id = None;
                        self.state.confirm_plan_id = None;
                        self.state.confirm_tools.clear();
                        self.state.input_buffer.clear();
                        return Ok(true);
                    }
                    
                    // Handle :deny and :n commands (deny from normal mode)
                    if (input_lower.starts_with(":deny") || input_lower == ":n")
                        && self.state.confirm_session_id.is_some() 
                        && self.state.confirm_plan_id.is_some() 
                    {
                        if let (Some(ref session_id), Some(ref plan_id)) = 
                            (self.state.confirm_session_id.clone(), self.state.confirm_plan_id.clone()) 
                        {
                            let client = self.gateway_client.clone();
                            let sid = session_id.clone();
                            let pid = plan_id.clone();
                            tokio::spawn(async move {
                                let client = client.read().await;
                                if let Err(e) = client.confirm_action(&sid, &pid, false).await {
                                    error!("Failed to deny action: {}", e);
                                }
                            });
                        }
                        self.state.confirm_mode = false;
                        self.state.confirm_session_id = None;
                        self.state.confirm_plan_id = None;
                        self.state.confirm_tools.clear();
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
                        
                        // Add to input history for future navigation
                        self.state.add_to_input_history();
                        self.state.reset_input_history_navigation();
                        
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
                            KeyCode::Char('c') => {
                                // Check for :cancel
                                // First check if 'a' is already buffered
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(a_key)) = event::read() {
                                        if let KeyCode::Char('a') = a_key.code {
                                            // Got :ca, check for :can
                                            if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                if let Ok(crossterm::event::Event::Key(n_key)) = event::read() {
                                                    if let KeyCode::Char('n') = n_key.code {
                                                        // Got :can, check for :canc
                                                        if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                            if let Ok(crossterm::event::Event::Key(c_key)) = event::read() {
                                                                if let KeyCode::Char('c') = c_key.code {
                                                                    // Got :canc - check for :cance
                                                                    if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                                        if let Ok(crossterm::event::Event::Key(e_key)) = event::read() {
                                                                            if let KeyCode::Char('e') = e_key.code {
                                                                                // Got :cance - it's :cancel!
                                                                                let client = self.gateway_client.clone();
                                                                                let session_id = self.state.current_session_id.clone();
                                                                                if let Some(sid) = session_id {
                                                                                    tokio::spawn(async move {
                                                                                        let client = client.read().await;
                                                                                        if let Err(e) = client.cancel_turn(&sid).await {
                                                                                            error!("Failed to cancel turn: {}", e);
                                                                                        }
                                                                                    });
                                                                                }
                                                                                self.state.input_buffer = String::new();
                                                                                return Ok(true);
                                                                            }
                                                                        }
                                                                    }
                                                                    // Not :cance, add 'canc' to buffer
                                                                    if self.state.active_panel == 2 {
                                                                        self.state.input_buffer.push('c');
                                                                        self.state.input_buffer.push('a');
                                                                        self.state.input_buffer.push('n');
                                                                        self.state.input_buffer.push('c');
                                                                        self.state.completion.reset();
                                                                    }
                                                                    return Ok(true);
                                                                }
                                                            }
                                                        }
                                                        // Not :canc, add 'can' to buffer
                                                        if self.state.active_panel == 2 {
                                                            self.state.input_buffer.push('c');
                                                            self.state.input_buffer.push('a');
                                                            self.state.input_buffer.push('n');
                                                            self.state.completion.reset();
                                                        }
                                                        return Ok(true);
                                                    }
                                                }
                                            }
                                            // Not :can, add 'ca' to buffer
                                            if self.state.active_panel == 2 {
                                                self.state.input_buffer.push('c');
                                                self.state.input_buffer.push('a');
                                                self.state.completion.reset();
                                            }
                                            return Ok(true);
                                        }
                                    }
                                }
                                // No more chars buffered, treat as single 'c'
                                // Check if it's :rc (reconnect) before treating as single 'c'
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(c_key)) = event::read() {
                                        if let KeyCode::Char('c') = c_key.code {
                                            // Got :rc - reconnect
                                            self.state.set_error(None);
                                            self.state.set_connected(false);
                                            return Ok(true);
                                        }
                                        // Was ':c' followed by something else - add 'c' to buffer
                                        // (the other char was already consumed above)
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
                            KeyCode::Char('n') => {
                                // Check for :note (n-o-t-e) first
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(o_key)) = event::read() {
                                        if let KeyCode::Char('o') = o_key.code {
                                            // Got :no, check for :not
                                            if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                if let Ok(crossterm::event::Event::Key(t_key)) = event::read() {
                                                    if let KeyCode::Char('t') = t_key.code {
                                                        // Got :not, check for :note
                                                        if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                            if let Ok(crossterm::event::Event::Key(e_key)) = event::read() {
                                                                if let KeyCode::Char('e') = e_key.code {
                                                                    // Got :note - toggle notes view
                                                                    if let Some(ref sid) = self.state.current_session_id {
                                                                        self.state.notes_mode = !self.state.notes_mode;
                                                                        if self.state.notes_mode {
                                                                            self.state.notes_session_id = Some(sid.clone());
                                                                            // Request notes list
                                                                            let client = self.gateway_client.clone();
                                                                            let sid_clone = sid.clone();
                                                                            tokio::spawn(async move {
                                                                                let client = client.read().await;
                                                                                if let Err(e) = client.list_session_notes(&sid_clone).await {
                                                                                    error!("Failed to load session notes: {}", e);
                                                                                }
                                                                            });
                                                                        }
                                                                    }
                                                                    self.state.input_buffer = String::new();
                                                                    return Ok(true);
                                                                }
                                                            }
                                                        }
                                                        // Not :note, add 'not' to buffer
                                                        if self.state.active_panel == 2 {
                                                            self.state.input_buffer.push('n');
                                                            self.state.input_buffer.push('o');
                                                            self.state.input_buffer.push('t');
                                                            self.state.completion.reset();
                                                        }
                                                        return Ok(true);
                                                    }
                                                }
                                            }
                                            // Not :not, add 'no' to buffer
                                            if self.state.active_panel == 2 {
                                                self.state.input_buffer.push('n');
                                                self.state.input_buffer.push('o');
                                                self.state.completion.reset();
                                            }
                                            return Ok(true);
                                        }
                                    }
                                }
                                // Not :note, treat as :n (new session)
                                let client = self.gateway_client.clone();
                                tokio::spawn(async move {
                                    let client = client.read().await;
                                    if let Err(e) = client.create_session().await {
                                        error!("Failed to create session: {}", e);
                                    }
                                });
                            }
                            KeyCode::Char('p') => {
                                // Check for :pin (p-i-n)
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(i_key)) = event::read() {
                                        if let KeyCode::Char('i') = i_key.code {
                                            // Got :pi, check for :pin
                                            if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                if let Ok(crossterm::event::Event::Key(n_key)) = event::read() {
                                                    if let KeyCode::Char('n') = n_key.code {
                                                        // Got :pin - toggle notes view
                                                        if let Some(ref sid) = self.state.current_session_id {
                                                            self.state.notes_mode = !self.state.notes_mode;
                                                            if self.state.notes_mode {
                                                                self.state.notes_session_id = Some(sid.clone());
                                                                let client = self.gateway_client.clone();
                                                                let sid_clone = sid.clone();
                                                                tokio::spawn(async move {
                                                                    let client = client.read().await;
                                                                    if let Err(e) = client.list_session_notes(&sid_clone).await {
                                                                        error!("Failed to load session notes: {}", e);
                                                                    }
                                                                });
                                                            }
                                                        }
                                                        self.state.input_buffer = String::new();
                                                        return Ok(true);
                                                    }
                                                }
                                            }
                                            // Not :pin, add 'pi' to buffer
                                            if self.state.active_panel == 2 {
                                                self.state.input_buffer.push('p');
                                                self.state.input_buffer.push('i');
                                                self.state.completion.reset();
                                            }
                                            return Ok(true);
                                        }
                                    }
                                }
                                // Single 'p' - just add to buffer
                                if self.state.active_panel == 2 {
                                    self.state.input_buffer.push('p');
                                    self.state.completion.reset();
                                }
                            }
                            KeyCode::Char('i') => {
                                // Check for :instr (i-n-s-t-r)
                                // First check for 'n' after 'i'
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(n_key)) = event::read() {
                                        if let KeyCode::Char('n') = n_key.code {
                                            // Got :in, check for :ins
                                            if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                if let Ok(crossterm::event::Event::Key(s_key)) = event::read() {
                                                    if let KeyCode::Char('s') = s_key.code {
                                                        // Got :ins, check for :inst
                                                        if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                            if let Ok(crossterm::event::Event::Key(t_key)) = event::read() {
                                                                if let KeyCode::Char('t') = t_key.code {
                                                                    // Got :inst, check for :instr
                                                                    if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                                                        if let Ok(crossterm::event::Event::Key(r_key)) = event::read() {
                                                                            if let KeyCode::Char('r') = r_key.code {
                                                                                // Got :instr - toggle instructions mode
                                                                                if let Some(ref sid) = self.state.current_session_id {
                                                                                    self.state.instructions_mode = !self.state.instructions_mode;
                                                                                    if self.state.instructions_mode {
                                                                                        self.state.instructions_session_id = Some(sid.clone());
                                                                                        self.state.input_buffer.clear();
                                                                                        // Request current instructions
                                                                                        let client = self.gateway_client.clone();
                                                                                        let sid_clone = sid.clone();
                                                                                        tokio::spawn(async move {
                                                                                            let client = client.read().await;
                                                                                            if let Err(e) = client.get_session_instructions(&sid_clone).await {
                                                                                                error!("Failed to load session instructions: {}", e);
                                                                                            }
                                                                                        });
                                                                                    }
                                                                                }
                                                                                self.state.input_buffer = String::new();
                                                                                return Ok(true);
                                                                            }
                                                                        }
                                                                    }
                                                                    // Not :instr, add 'inst' to buffer
                                                                    if self.state.active_panel == 2 {
                                                                        self.state.input_buffer.push('i');
                                                                        self.state.input_buffer.push('n');
                                                                        self.state.input_buffer.push('s');
                                                                        self.state.input_buffer.push('t');
                                                                        self.state.completion.reset();
                                                                    }
                                                                    return Ok(true);
                                                                }
                                                            }
                                                        }
                                                        // Not :inst, add 'ins' to buffer
                                                        if self.state.active_panel == 2 {
                                                            self.state.input_buffer.push('i');
                                                            self.state.input_buffer.push('n');
                                                            self.state.input_buffer.push('s');
                                                            self.state.completion.reset();
                                                        }
                                                        return Ok(true);
                                                    }
                                                }
                                            }
                                            // Not :ins, add 'in' to buffer
                                            if self.state.active_panel == 2 {
                                                self.state.input_buffer.push('i');
                                                self.state.input_buffer.push('n');
                                                self.state.completion.reset();
                                            }
                                            return Ok(true);
                                        }
                                    }
                                }
                                // Single 'i' - just add to buffer
                                if self.state.active_panel == 2 {
                                    self.state.input_buffer.push('i');
                                    self.state.completion.reset();
                                }
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
                            KeyCode::Char('f') => {
                                // Enter search mode
                                self.state.enter_search_mode();
                                self.state.input_buffer.clear();
                                self.state.input_buffer.push('/');
                            }
                            KeyCode::Char('g') => {
                                // Scroll to bottom - gg in vim style (first 'g')
                                // Store that we got first 'g', wait for second 'g'
                                if event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                                    if let Ok(crossterm::event::Event::Key(g_key)) = event::read() {
                                        if let KeyCode::Char('g') = g_key.code {
                                            // Got 'gg' - scroll to bottom
                                            self.state.scroll_to_bottom();
                                            return Ok(true);
                                        }
                                        // Not 'gg', add 'g' to buffer
                                        if self.state.active_panel == 2 {
                                            self.state.input_buffer.push('g');
                                            self.state.completion.reset();
                                        }
                                        return Ok(true);
                                    }
                                }
                                // Single 'g' - just scroll to bottom for now
                                self.state.scroll_to_bottom();
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
                    self.state.reset_input_history_navigation();
                    return Ok(true);
                }
                KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    // Ctrl+C - cancel current operation
                    self.state.input_buffer.clear();
                    self.state.completion.reset();
                    self.state.set_loading(false);
                    self.state.reset_input_history_navigation();
                    return Ok(true);
                }
                KeyCode::Char('f') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    // Ctrl+F - enter search mode
                    self.state.enter_search_mode();
                    self.state.input_buffer.clear();
                    self.state.input_buffer.push('/');
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
                    // Handle search mode
                    if self.state.search_mode {
                        // In search mode, typing updates the search query
                        if c == '/' {
                            // '/' is already in buffer from entering search mode
                            self.state.input_buffer.push(c);
                        } else {
                            self.state.input_buffer.push(c);
                            // Update search results as user types (after the '/')
                            if self.state.input_buffer.starts_with('/') {
                                let query = self.state.input_buffer.trim_start_matches('/').to_string();
                                self.state.search(&query);
                            }
                        }
                        return Ok(true);
                    }
                    
                    // Handle search navigation when not in input panel
                    if self.state.search_mode {
                        if c == 'n' {
                            // Next search result
                            self.state.search_next();
                            return Ok(true);
                        } else if c == 'N' || c == 'p' {
                            // Previous search result (Shift+N or p)
                            self.state.search_prev();
                            return Ok(true);
                        }
                    }
                    
                    if self.state.active_panel == 2 {
                        // If navigating history, reset and start fresh
                        if self.state.is_navigating_history() {
                            self.state.reset_input_history_navigation();
                            self.state.input_buffer.clear();
                        }
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
                    // Handle search mode backspace
                    if self.state.search_mode {
                        if !self.state.input_buffer.is_empty() {
                            self.state.input_buffer.pop();
                            // Update search results
                            if self.state.input_buffer.starts_with('/') {
                                let query = self.state.input_buffer.trim_start_matches('/').to_string();
                                self.state.search(&query);
                            } else {
                                self.state.search_results.clear();
                                self.state.search_index = None;
                            }
                        }
                        return Ok(true);
                    }
                    
                    if self.state.active_panel == 2 {
                        // If navigating history, reset
                        if self.state.is_navigating_history() {
                            self.state.reset_input_history_navigation();
                            self.state.input_buffer.clear();
                        }
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
                    } else if self.state.is_navigating_history() {
                        // Cancel history navigation - restore saved buffer
                        self.state.input_buffer = self.state.input_history_saved.take().unwrap_or_default();
                        self.state.reset_input_history_navigation();
                    } else if self.state.search_mode {
                        self.state.exit_search_mode();
                    } else if self.state.notes_mode {
                        self.state.notes_mode = false;
                        self.state.notes_content = None;
                    } else if self.state.instructions_mode {
                        self.state.instructions_mode = false;
                        self.state.instructions_session_id = None;
                        self.state.input_buffer.clear();
                        self.state.current_instructions = None;
                    } else if self.state.confirm_mode {
                        // Esc in confirm mode = deny
                        if let (Some(ref session_id), Some(ref plan_id)) = 
                            (self.state.confirm_session_id.clone(), self.state.confirm_plan_id.clone()) 
                        {
                            let client = self.gateway_client.clone();
                            let sid = session_id.clone();
                            let pid = plan_id.clone();
                            tokio::spawn(async move {
                                let client = client.read().await;
                                if let Err(e) = client.confirm_action(&sid, &pid, false).await {
                                    error!("Failed to deny action: {}", e);
                                }
                            });
                        }
                        self.state.confirm_mode = false;
                        self.state.confirm_session_id = None;
                        self.state.confirm_plan_id = None;
                        self.state.confirm_tools.clear();
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
        if self.state.confirm_mode {
            crate::tui::components::draw_confirm_panel(f, msg_chunks[0], &self.state);
        } else if self.state.instructions_mode {
            crate::tui::components::draw_instructions_panel(f, msg_chunks[0], &self.state);
        } else if self.state.notes_mode {
            crate::tui::components::draw_notes_panel(f, msg_chunks[0], &self.state);
        } else {
            crate::tui::components::draw_messages_panel(f, msg_chunks[0], &self.state);
        }
        crate::tui::components::draw_input_panel(f, msg_chunks[1], &self.state);
        crate::tui::components::draw_help_bar(f, chunks[2], &self.state);

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
        use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}};
        
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
        let mut spans: Vec<Span> = vec![
            Span::raw(" TinyClaw v"),
            Span::raw(&self.version),
            Span::raw(" | "),
            Span::styled(connection_str, Style::default().fg(connection_color)),
            Span::raw(" | Session: "),
            Span::styled(session_str, Style::default().fg(Color::Cyan)),
        ];
        
        // Add thinking indicator if agent is processing
        if self.state.loading {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled("⚙ Thinking...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        }
        
        // Add AI circuit breaker indicator
        let cb_state = &self.state.circuit_breaker_state;
        let cb_indicator = match cb_state.as_str() {
            "open" => ("● AI Unavailable", Color::Red),
            "half_open" => ("◐ AI Recovering", Color::Yellow),
            _ => ("● AI OK", Color::Green),
        };
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(cb_indicator.0, Style::default().fg(cb_indicator.1)));
        
        // Add context summarization indicator if available
        if let Some(ref summary_info) = self.state.last_summary_info {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(summary_info.clone(), Style::default().fg(Color::Magenta)));
        }
        
        let paragraph = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn draw_error_overlay(&self, f: &mut ratatui::Frame<'_>) {
        use ratatui::widgets::Clear;
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::Span;

        let size = f.area();
        
        let error_msg = self.state.error_message.as_deref().unwrap_or("Unknown error");
        
        // Build error content with color styling
        let error_content: Vec<Line> = vec![
            Line::from(vec![
                Span::styled(" ⚠ Error ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(error_msg, Style::default().fg(Color::LightRed)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Press :r to retry or Esc to dismiss ", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let block = Block::default()
            .title(vec![
                Span::styled(" Error ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ])
            .title_style(Style::default().fg(Color::Red))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

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

/// Format session notes for display in TUI
fn format_notes_display(notes: &[crate::tui::gateway_client::SessionNoteInfo]) -> String {
    if notes.is_empty() {
        return "No notes for this session.\n\nUse :note to reload or :note again to exit.".to_string();
    }
    
    let mut result = String::new();
    result.push_str(&format!("{} Session Notes ({} notes)\n", "═".repeat(30), notes.len()));
    result.push_str(&"─".repeat(50));
    result.push('\n');
    
    for note in notes {
        result.push_str(&format!("{} {}\n", if note.pinned { "📌" } else { "  " }, note.id));
        if !note.content.is_empty() {
            let preview = if note.content.len() > 200 {
                format!("{}...", &note.content[..200])
            } else {
                note.content.clone()
            };
            result.push_str(&format!("  {}\n", preview.replace('\n', "\n  ")));
        }
        if !note.tags.is_empty() {
            result.push_str(&format!("  Tags: {}\n", note.tags.join(", ")));
        }
        result.push_str(&"─".repeat(50));
        result.push('\n');
    }
    
    result.push_str("\nPress :note or :pin to exit notes view.");
    result
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
