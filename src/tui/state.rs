//! TUI application state

use crate::types::{Message, SessionHistory};
use crate::tui::gateway_client::{TuiGatewayStatus, ToolCallPreview};
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
    TuiCommandMeta {
        full_name: ":cancel",
        aliases: &["stop"],
        description: "Cancel ongoing turn",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":note",
        aliases: &["notes"],
        description: "View session notes",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":pin",
        aliases: &[],
        description: "Pin current session notes",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":instr",
        aliases: &["instructions"],
        description: "Edit session instructions",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":sum",
        aliases: &["summary"],
        description: "View summarizer config & stats",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":sumcfg",
        aliases: &[],
        description: "Edit summarizer configuration",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":quality",
        aliases: &["qly"],
        description: "View session quality analysis",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":eval",
        aliases: &["evals"],
        description: "View recent self-evaluations",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":rec",
        aliases: &["recommendations"],
        description: "View skill recommendations",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":safety",
        aliases: &["safetystats"],
        description: "View execution safety status",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":perf",
        aliases: &["performance", "insights"],
        description: "View performance insights",
        category: CommandCategory::Session,
    },
    TuiCommandMeta {
        full_name: ":advisor",
        aliases: &["advice", "suggestions"],
        description: "View context optimization advice",
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
    /// Input history navigation (Up/Down arrows)
    pub input_history: Vec<String>,
    /// Current position in input history (None = not navigating)
    pub input_history_index: Option<usize>,
    /// Saved buffer when starting history navigation
    pub input_history_saved: Option<String>,
    /// AI provider circuit breaker state: "closed", "open", or "half_open"
    pub circuit_breaker_state: String,
    /// Last context summarization info for display (e.g., "📝 10 msgs → 200 tokens (10%)")
    pub last_summary_info: Option<String>,
    /// Whether we're in search mode
    pub search_mode: bool,
    /// Current search query
    pub search_query: String,
    /// Search result indices in current messages
    pub search_results: Vec<usize>,
    /// Current highlighted search result index (in search_results)
    pub search_index: Option<usize>,
    /// Whether we're in notes viewing mode
    pub notes_mode: bool,
    /// Session ID for current notes view
    pub notes_session_id: Option<String>,
    /// Cached notes content for display
    pub notes_content: Option<String>,
    /// Whether we're in instructions editing mode
    pub instructions_mode: bool,
    /// Session ID for current instructions editing
    pub instructions_session_id: Option<String>,
    /// Current instructions being edited (None = loading/fetching)
    pub current_instructions: Option<String>,
    /// Whether we're in action confirmation mode (waiting for user to confirm/deny)
    pub confirm_mode: bool,
    /// Session ID for pending action confirmation
    pub confirm_session_id: Option<String>,
    /// Plan ID to confirm or deny
    pub confirm_plan_id: Option<String>,
    /// Tools in the pending action plan
    pub confirm_tools: Vec<ToolCallPreview>,
    /// Whether we're currently streaming text from the agent
    pub is_streaming: bool,
    /// Accumulated streaming text (partial response being received)
    pub partial_text: String,
    /// Session ID of current streaming (for multi-session awareness)
    pub streaming_session_id: Option<String>,
    /// Whether a message has already been created for the current turn via AssistantText
    /// (to avoid duplicate messages when TurnEnded also creates one)
    pub streaming_message_created: bool,
    /// Total input tokens used (cumulative across all sessions)
    pub token_input_total: u64,
    /// Total output tokens used (cumulative across all sessions)
    pub token_output_total: u64,
    /// Token usage by session (session_id -> (input, output))
    pub token_usage_by_session: HashMap<String, (u64, u64)>,
    /// Whether we're in summarizer viewing mode
    pub summarizer_mode: bool,
    /// Cached summarizer config for display (JSON string)
    pub summarizer_config: Option<String>,
    /// Cached summarizer stats for display (JSON string)
    pub summarizer_stats: Option<String>,
    /// Cached summarizer history for display (JSON string)
    pub summarizer_history: Option<String>,
    /// Whether we're in summarizer config editing mode
    pub sumcfg_mode: bool,
    /// Whether we're in session quality viewing mode
    pub quality_mode: bool,
    /// Cached session quality data for display
    pub quality_data: Option<SessionQualityDisplay>,
    /// Whether we're in self-evaluation viewing mode
    pub eval_mode: bool,
    /// Cached self-evaluation data for display
    pub eval_data: Option<Vec<SelfEvaluationDisplay>>,
    /// Whether we're in skill recommendations viewing mode
    pub recommendations_mode: bool,
    /// Session ID for current recommendations view
    pub recommendations_session_id: Option<String>,
    /// Cached skill recommendations for display
    pub recommendations_data: Option<Vec<SkillRecommendationDisplay>>,
    /// Whether we're in execution safety viewing mode
    pub safety_mode: bool,
    /// Session ID for current safety view
    pub safety_session_id: Option<String>,
    /// Cached safety stats for display (JSON string)
    pub safety_stats: Option<String>,
    /// Cached safety state for display (JSON string)
    pub safety_state: Option<String>,
    /// Last safety warning info for display
    pub last_safety_warning: Option<String>,
    /// Whether execution is currently halted due to safety limit
    pub safety_halted: bool,
    /// Whether we're in performance insights viewing mode
    pub perf_mode: bool,
    /// Cached performance insights data
    pub perf_data: Option<crate::tui::gateway_client::PerformanceInsightsDisplay>,
    /// Whether we're in context health viewing mode
    pub context_health_mode: bool,
    /// Cached context health data
    pub context_health_data: Option<ContextHealthDisplay>,
}

/// Context health data for TUI display
#[derive(Debug, Clone)]
pub struct ContextHealthDisplay {
    pub health_level: String,
    pub health_score: u8,
    pub utilization_pct: f32,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub truncation_count: usize,
    pub summarization_count: usize,
    pub total_turns: usize,
    pub peak_utilization_pct: f32,
    pub recommendations_count: usize,
}

/// Session quality data for TUI display
#[derive(Debug, Clone)]
pub struct SessionQualityDisplay {
    pub session_id: String,
    pub quality_score: f64,
    pub turn_count: u32,
    pub task_completion_rate: f64,
    pub tool_success_rate: f64,
    pub rating: u8,
    pub issue_count: usize,
    pub suggestions: Vec<String>,
}

/// Self-evaluation data for TUI display
#[derive(Debug, Clone)]
pub struct SelfEvaluationDisplay {
    pub turn_id: String,
    pub session_id: String,
    pub overall_score: f64,
    pub dimension_scores: Vec<(String, f64)>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
}

/// Skill recommendation data for TUI display
#[derive(Debug, Clone)]
pub struct SkillRecommendationDisplay {
    pub id: String,
    pub skill_name: String,
    pub description: String,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub triggered_keywords: Vec<String>,
    pub already_enabled: bool,
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
            input_history: Vec::new(),
            input_history_index: None,
            input_history_saved: None,
            circuit_breaker_state: "closed".to_string(),
            last_summary_info: None,
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_index: None,
            notes_mode: false,
            notes_session_id: None,
            notes_content: None,
            instructions_mode: false,
            instructions_session_id: None,
            current_instructions: None,
            confirm_mode: false,
            confirm_session_id: None,
            confirm_plan_id: None,
            confirm_tools: Vec::new(),
            is_streaming: false,
            partial_text: String::new(),
            streaming_session_id: None,
            streaming_message_created: false,
            token_input_total: 0,
            token_output_total: 0,
            token_usage_by_session: HashMap::new(),
            summarizer_mode: false,
            summarizer_config: None,
            summarizer_stats: None,
            summarizer_history: None,
            sumcfg_mode: false,
            quality_mode: false,
            quality_data: None,
            eval_mode: false,
            eval_data: None,
            recommendations_mode: false,
            recommendations_session_id: None,
            recommendations_data: None,
            safety_mode: false,
            safety_session_id: None,
            safety_stats: None,
            safety_state: None,
            last_safety_warning: None,
            safety_halted: false,
            perf_mode: false,
            perf_data: None,
            context_health_mode: false,
            context_health_data: None,
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

    /// Add current input buffer to input history
    pub fn add_to_input_history(&mut self) {
        let text = self.input_buffer.trim();
        if text.is_empty() {
            return;
        }
        // Don't add duplicates at the end
        if self.input_history.last().map(|s| s.as_str()) != Some(text) {
            self.input_history.push(text.to_string());
        }
        // Limit history size to 100 entries
        if self.input_history.len() > 100 {
            self.input_history.remove(0);
        }
    }

    /// Navigate up in input history (Up arrow)
    /// Returns true if navigation happened
    pub fn input_history_up(&mut self) -> bool {
        if self.input_history.is_empty() {
            return false;
        }
        // Save current buffer if not already navigating
        if self.input_history_index.is_none() {
            if !self.input_buffer.is_empty() {
                self.input_history_saved = Some(self.input_buffer.clone());
            }
            self.input_history_index = Some(self.input_history.len().saturating_sub(1));
        } else {
            // Move to previous entry
            let idx = self.input_history_index.unwrap();
            if idx > 0 {
                self.input_history_index = Some(idx - 1);
            }
        }
        if let Some(idx) = self.input_history_index {
            if let Some(history_entry) = self.input_history.get(idx) {
                self.input_buffer = history_entry.clone();
            }
        }
        true
    }

    /// Navigate down in input history (Down arrow)
    /// Returns true if navigation happened
    pub fn input_history_down(&mut self) -> bool {
        if self.input_history.is_empty() || self.input_history_index.is_none() {
            return false;
        }
        let idx = self.input_history_index.unwrap();
        if idx >= self.input_history.len().saturating_sub(1) {
            // At the end - restore saved buffer and exit navigation
            self.input_history_index = None;
            self.input_buffer = self.input_history_saved.take().unwrap_or_default();
        } else {
            // Move to next entry
            self.input_history_index = Some(idx + 1);
            if let Some(history_entry) = self.input_history.get(self.input_history_index.unwrap()) {
                self.input_buffer = history_entry.clone();
            }
        }
        true
    }

    /// Check if currently navigating input history
    pub fn is_navigating_history(&self) -> bool {
        self.input_history_index.is_some()
    }

    /// Get input history position display string (e.g., "3/10" or None)
    pub fn input_history_position(&self) -> Option<String> {
        self.input_history_index.map(|idx| {
            format!("{}/{}", idx + 1, self.input_history.len())
        })
    }

    /// Reset input history navigation state
    pub fn reset_input_history_navigation(&mut self) {
        self.input_history_index = None;
        self.input_history_saved = None;
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

    // ========================================================================
    // Search functionality
    // ========================================================================

    /// Enter search mode
    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_index = None;
    }

    /// Exit search mode
    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_index = None;
    }

    /// Update search query and find matching messages
    pub fn search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.search_results.clear();
        self.search_index = None;

        if query.is_empty() {
            return;
        }

        let query_lower = query.to_lowercase();
        let messages = self.get_current_messages();
        // Collect indices first to avoid borrow conflict
        let matching_indices: Vec<usize> = messages.iter()
            .enumerate()
            .filter(|(_, msg)| msg.content.to_lowercase().contains(&query_lower))
            .map(|(idx, _)| idx)
            .collect();
        
        self.search_results = matching_indices;

        if !self.search_results.is_empty() {
            self.search_index = Some(0);
            // Scroll to first result
            self.scroll_offset = self.search_results[0].saturating_sub(2);
        }
    }

    /// Navigate to next search result
    pub fn search_next(&mut self) -> bool {
        if self.search_results.is_empty() {
            return false;
        }

        let idx = self.search_index.unwrap_or(0);
        let next_idx = (idx + 1) % self.search_results.len();
        self.search_index = Some(next_idx);
        self.scroll_offset = self.search_results[next_idx].saturating_sub(2);
        true
    }

    /// Navigate to previous search result
    pub fn search_prev(&mut self) -> bool {
        if self.search_results.is_empty() {
            return false;
        }

        let idx = self.search_index.unwrap_or(0);
        let prev_idx = if idx == 0 {
            self.search_results.len() - 1
        } else {
            idx - 1
        };
        self.search_index = Some(prev_idx);
        self.scroll_offset = self.search_results[prev_idx].saturating_sub(2);
        true
    }

    /// Get search status string (e.g., "3/10" or None)
    #[allow(dead_code)]
    pub fn search_status(&self) -> Option<String> {
        if !self.search_mode || self.search_results.is_empty() {
            return None;
        }
        let idx = self.search_index.unwrap_or(0) + 1;
        Some(format!("{}/{}", idx, self.search_results.len()))
    }

    /// Check if a message is a search result (for highlighting)
    #[allow(dead_code)]
    pub fn is_search_result(&self, msg_idx: usize) -> bool {
        self.search_results.contains(&msg_idx)
    }

    /// Check if a message is the currently selected search result
    #[allow(dead_code)]
    pub fn is_selected_search_result(&self, msg_idx: usize) -> bool {
        if let Some(idx) = self.search_index {
            self.search_results.get(idx) == Some(&msg_idx)
        } else {
            false
        }
    }

    // ========================================================================
    // Navigation helpers
    // ========================================================================

    /// Scroll to the bottom of messages
    pub fn scroll_to_bottom(&mut self) {
        let msg_count = self.get_current_messages().len();
        if msg_count > 1 {
            self.scroll_offset = msg_count.saturating_sub(1);
        }
    }

    /// Get whether search is active and has results
    #[allow(dead_code)]
    pub fn has_search_results(&self) -> bool {
        !self.search_results.is_empty()
    }

    // ========================================================================
    // Streaming text helpers
    // ========================================================================

    /// Start streaming text for a session
    pub fn start_streaming(&mut self, session_id: &str) {
        self.is_streaming = true;
        self.partial_text.clear();
        self.streaming_session_id = Some(session_id.to_string());
        self.streaming_message_created = false;
    }

    /// Append streaming text fragment
    pub fn append_streaming_text(&mut self, text: &str) {
        if self.is_streaming {
            self.partial_text.push_str(text);
        }
    }

    /// End streaming and finalize the accumulated text as an assistant message
    /// Returns the final text if streaming was active
    pub fn end_streaming(&mut self) -> Option<String> {
        if self.is_streaming {
            let text = self.partial_text.clone();
            self.is_streaming = false;
            self.partial_text.clear();
            self.streaming_session_id = None;
            Some(text)
        } else {
            None
        }
    }

    /// Mark that a message was created for the current streaming turn
    /// (called when AssistantText creates a message during a streaming turn)
    pub fn mark_streaming_message_created(&mut self) {
        self.streaming_message_created = true;
    }

    /// Reset all streaming state (called on turn start/cancel)
    pub fn reset_streaming_state(&mut self) {
        self.is_streaming = false;
        self.partial_text.clear();
        self.streaming_session_id = None;
        self.streaming_message_created = false;
    }

    /// Cancel streaming (e.g., on turn cancelled)
    pub fn cancel_streaming(&mut self) {
        self.is_streaming = false;
        self.partial_text.clear();
        self.streaming_session_id = None;
    }

    // ========================================================================
    // Token usage tracking
    // ========================================================================

    /// Update token usage from a turn.usage event
    pub fn update_token_usage(&mut self, session_id: &str, input_tokens: u32, output_tokens: u32) {
        // Update totals
        self.token_input_total += input_tokens as u64;
        self.token_output_total += output_tokens as u64;

        // Update per-session tracking
        let entry = self.token_usage_by_session.entry(session_id.to_string()).or_insert((0, 0));
        entry.0 += input_tokens as u64;
        entry.1 += output_tokens as u64;
    }

    /// Get total tokens used
    #[allow(dead_code)]
    pub fn total_tokens(&self) -> u64 {
        self.token_input_total + self.token_output_total
    }

    /// Get token usage for a specific session
    #[allow(dead_code)]
    pub fn session_tokens(&self, session_id: &str) -> (u64, u64) {
        self.token_usage_by_session.get(session_id).copied().unwrap_or((0, 0))
    }

    /// Format token count for display (e.g., "1.2K", "3.5M")
    pub fn format_token_count(count: u64) -> String {
        if count >= 1_000_000 {
            format!("{:.1}M", count as f64 / 1_000_000.0)
        } else if count >= 1_000 {
            format!("{:.1}K", count as f64 / 1_000.0)
        } else {
            count.to_string()
        }
    }

    /// Get formatted token usage string (e.g., "In: 1.2K | Out: 500")
    pub fn formatted_token_usage(&self) -> String {
        format!(
            "In: {} | Out: {}",
            Self::format_token_count(self.token_input_total),
            Self::format_token_count(self.token_output_total)
        )
    }

}
