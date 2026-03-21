//! TUI application state

use crate::types::{Message, SessionHistory};
use crate::tui::gateway_client::TuiGatewayStatus;
use std::collections::HashMap;

/// Application state for TUI
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current session ID
    pub current_session_id: Option<String>,
    /// Session list
    pub sessions: Vec<String>,
    /// Session histories
    pub session_histories: HashMap<String, SessionHistory>,
    /// Input buffer
    pub input_buffer: String,
    /// Scroll offset for message view
    pub scroll_offset: usize,
    /// Whether to show help
    pub show_help: bool,
    /// Current panel (0=sessions, 1=messages, 2=input)
    pub active_panel: usize,
    /// Gateway connection status
    pub gateway_status: TuiGatewayStatus,
    /// Whether gateway is connected
    pub connected: bool,
    /// Loading indicator for pending agent response
    pub loading: bool,
    /// Error message to display
    pub error_message: Option<String>,
    /// Connection retry count
    pub retry_count: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_session_id: None,
            sessions: Vec::new(),
            session_histories: HashMap::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            show_help: false,
            active_panel: 1,
            gateway_status: TuiGatewayStatus::Disconnected,
            connected: false,
            loading: false,
            error_message: None,
            retry_count: 0,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_current_session(&mut self, session_id: String) {
        self.current_session_id = Some(session_id.clone());
        self.scroll_offset = 0;
    }

    pub fn get_current_history(&self) -> Option<&SessionHistory> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.session_histories.get(id))
    }

    pub fn get_current_messages(&self) -> Vec<&Message> {
        self.get_current_history()
            .map(|h| h.messages.iter().collect())
            .unwrap_or_default()
    }

    pub fn add_session(&mut self, session_id: String) {
        if !self.sessions.contains(&session_id) {
            self.sessions.push(session_id.clone());
            self.session_histories.insert(
                session_id.clone(),
                SessionHistory::new(session_id.clone()),
            );
        }
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn set_error(&mut self, msg: Option<String>) {
        self.error_message = msg;
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
        if connected {
            self.retry_count = 0;
            self.error_message = None;
        }
    }

    #[allow(dead_code)]
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}
