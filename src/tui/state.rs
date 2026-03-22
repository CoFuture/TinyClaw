//! TUI application state

use crate::types::{Message, SessionHistory};
use crate::tui::gateway_client::TuiGatewayStatus;
use std::collections::HashMap;

/// Agent activity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentActivityType {
    /// Idle / not doing anything
    #[default]
    Idle,
    /// Agent is thinking/generating
    Thinking,
    /// Agent is executing a tool
    UsingTool,
    /// Waiting for response
    #[allow(dead_code)]
    Waiting,
}

/// Current agent activity state
#[derive(Debug, Clone, Default)]
pub struct AgentActivity {
    /// Activity type
    pub activity_type: AgentActivityType,
    /// Tool name if using a tool
    pub tool_name: Option<String>,
}

/// Completion candidates for tab completion
#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    /// Whether completion is active
    pub active: bool,
    /// Available completion candidates
    pub candidates: Vec<String>,
    /// Current selected candidate index
    pub index: usize,
    /// The prefix being completed
    pub prefix: String,
}

impl CompletionState {
    /// Reset completion state
    pub fn reset(&mut self) {
        self.active = false;
        self.candidates.clear();
        self.index = 0;
        self.prefix.clear();
    }

    /// Activate completion with candidates
    pub fn activate(&mut self, prefix: &str, candidates: Vec<String>) {
        if candidates.is_empty() {
            self.reset();
            return;
        }
        self.active = true;
        self.prefix = prefix.to_string();
        self.candidates = candidates;
        self.index = 0;
    }

    /// Cycle to next candidate (tab)
    pub fn next(&mut self) {
        if !self.candidates.is_empty() {
            self.index = (self.index + 1) % self.candidates.len();
        }
    }

    /// Cycle to previous candidate (shift-tab)
    pub fn prev(&mut self) {
        if !self.candidates.is_empty() {
            self.index = self.index.saturating_sub(1);
            if self.index == 0 && self.candidates.len() > 1 {
                self.index = self.candidates.len() - 1;
            }
        }
    }

    /// Get current completion
    pub fn current(&self) -> Option<&str> {
        self.candidates.get(self.index).map(|s| s.as_str())
    }
}

/// Command category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// Session management commands
    Session,
    /// Connection commands
    Connection,
    /// Navigation commands
    Navigation,
}

/// TUI command metadata
#[derive(Debug, Clone)]
pub struct TuiCommandMeta {
    /// Full command with colon (e.g., ":q")
    pub full_name: &'static str,
    /// Command aliases (e.g., ["quit"])
    pub aliases: &'static [&'static str],
    /// Brief description
    pub description: &'static str,
    /// Command category
    pub category: CommandCategory,
}

impl TuiCommandMeta {
    /// Get all variations of the command (for completion matching)
    pub fn all_variations(&self) -> Vec<String> {
        let mut variations = vec![self.full_name.to_string()];
        for alias in self.aliases {
            variations.push(format!(":{}", alias));
        }
        variations
    }
}

/// Available TUI commands with metadata
pub const TUI_COMMANDS: &[TuiCommandMeta] = &[
    // Session commands
    TuiCommandMeta {
        full_name: ":n",
        aliases: &["new"],
        description: "Create a new session",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":d",
        aliases: &["delete"],
        description: "Delete current session",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":ren",
        aliases: &["rename"],
        description: "Rename current session",
        category: CommandCategory::Session,
    },
    // Connection commands
    TuiCommandMeta {
        full_name: ":rc",
        aliases: &["reconnect"],
        description: "Reconnect to gateway",
        category: CommandCategory::Connection,
    },
    // Navigation commands
    TuiCommandMeta {
        full_name: ":q",
        aliases: &["quit"],
        description: "Quit TinyClaw",
        category: CommandCategory::Navigation,
    },
    TuiCommandMeta {
        full_name: ":h",
        aliases: &["help", "?"],
        description: "Show/hide help",
        category: CommandCategory::Navigation,
    },
];

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
    /// Tab completion state
    pub completion: CompletionState,
    /// Current agent activity state
    pub agent_activity: AgentActivity,
    /// Whether we're in rename mode (waiting for new session name)
    pub rename_mode: bool,
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
            rename_mode: false,
            connected: false,
            loading: false,
            error_message: None,
            retry_count: 0,
            completion: CompletionState::default(),
            agent_activity: AgentActivity::default(),
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

    pub fn set_thinking(&mut self) {
        self.agent_activity = AgentActivity {
            activity_type: AgentActivityType::Thinking,
            tool_name: None,
        };
    }

    pub fn set_using_tool(&mut self, tool_name: &str) {
        self.agent_activity = AgentActivity {
            activity_type: AgentActivityType::UsingTool,
            tool_name: Some(tool_name.to_string()),
        };
    }

    pub fn set_idle(&mut self) {
        self.agent_activity = AgentActivity::default();
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

    /// Get completion candidates for the current input
    pub fn get_completion_candidates(&self) -> Vec<String> {
        let input = &self.input_buffer;
        
        // If input starts with ':', complete command names
        if input.starts_with(':') {
            let prefix = input.to_lowercase();
            let mut candidates: Vec<String> = TUI_COMMANDS
                .iter()
                .flat_map(|cmd| cmd.all_variations())
                .filter(|v| v.to_lowercase().starts_with(&prefix))
                .collect();
            candidates.sort();
            candidates.dedup();
            return candidates;
        }
        
        // For regular input, could add skill names or other completions
        // For now, return session IDs as candidates
        let prefix = input.to_lowercase();
        self.sessions
            .iter()
            .filter(|s| s.to_lowercase().starts_with(&prefix))
            .cloned()
            .collect()
    }
}
