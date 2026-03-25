//! Agent Execution Safety System
//!
//! Prevents runaway agent execution loops by monitoring consecutive tool-call
//! turns and providing automatic intervention when safety thresholds are reached.
//!
//! This module provides:
//! - Consecutive turn tracking per session
//! - Configurable safety thresholds and actions
//! - Warning events before halting
//! - Automatic context summarization trigger
//! - Per-session state persistence

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// Action to take when safety limit is reached
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SafetyAction {
    /// Just emit a warning, continue execution
    #[default]
    Warn,
    /// Trigger context summarization to reduce load
    Summarize,
    /// Halt execution and wait for user confirmation
    Halt,
    /// Stop execution immediately
    Stop,
}

/// Configuration for execution safety monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSafetyConfig {
    /// Maximum consecutive tool-call turns before triggering safety action
    pub max_consecutive_turns: usize,
    /// Warning threshold (percentage of max) to emit early warning
    pub warning_threshold_pct: u8,
    /// Action to take when limit is reached
    pub safety_action: SafetyAction,
    /// Whether safety monitoring is enabled
    pub enabled: bool,
}

impl Default for ExecutionSafetyConfig {
    fn default() -> Self {
        Self {
            max_consecutive_turns: 20,
            warning_threshold_pct: 75, // Warn at 75% (15 turns)
            safety_action: SafetyAction::Warn,
            enabled: true,
        }
    }
}

impl ExecutionSafetyConfig {
    /// Get warning threshold (turn count)
    pub fn warning_turn_count(&self) -> usize {
        (self.max_consecutive_turns * self.warning_threshold_pct as usize) / 100
    }

    /// Check if warnings are enabled
    pub fn is_warning_enabled(&self) -> bool {
        self.warning_threshold_pct > 0
    }
}

/// Current state of execution safety for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSafetyState {
    /// Session ID
    pub session_id: String,
    /// Current consecutive tool-call turn count
    pub consecutive_tool_turns: usize,
    /// Total tool-call turns in this session
    pub total_tool_turns: usize,
    /// Whether execution is currently halted
    pub is_halted: bool,
    /// Whether a warning was issued
    pub warning_issued: bool,
    /// When the consecutive count was last reset
    pub last_reset_at: DateTime<Utc>,
    /// When the consecutive count started accumulating
    pub streak_started_at: Option<DateTime<Utc>>,
    /// Number of safety events triggered this session
    pub safety_events_count: u32,
}

impl ExecutionSafetyState {
    /// Create new initial state for a session
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            consecutive_tool_turns: 0,
            total_tool_turns: 0,
            is_halted: false,
            warning_issued: false,
            last_reset_at: Utc::now(),
            streak_started_at: None,
            safety_events_count: 0,
        }
    }

    /// Record a tool-call turn (increments counters)
    pub fn record_tool_turn(&mut self) {
        self.consecutive_tool_turns += 1;
        self.total_tool_turns += 1;
        if self.streak_started_at.is_none() {
            self.streak_started_at = Some(Utc::now());
        }
    }

    /// Record a user message (resets consecutive counter)
    pub fn record_user_message(&mut self) {
        self.consecutive_tool_turns = 0;
        self.warning_issued = false;
        self.streak_started_at = None;
        self.last_reset_at = Utc::now();
    }

    /// Reset halt state (user acknowledged)
    pub fn reset_halt(&mut self) {
        self.is_halted = false;
        self.consecutive_tool_turns = 0;
        self.warning_issued = false;
        self.streak_started_at = None;
        self.last_reset_at = Utc::now();
    }

    /// Mark that a safety event was triggered
    pub fn record_safety_event(&mut self) {
        self.safety_events_count += 1;
        self.is_halted = true;
    }

    /// Get streak duration in seconds
    pub fn streak_duration_secs(&self) -> i64 {
        match self.streak_started_at {
            Some(start) => (Utc::now() - start).num_seconds(),
            None => 0,
        }
    }
}

/// Aggregated safety statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSafetyStats {
    /// Total safety events across all sessions
    pub total_safety_events: u32,
    /// Total warnings issued
    pub total_warnings: u32,
    /// Total halts
    pub total_halts: u32,
    /// Sessions currently being monitored
    pub active_sessions: usize,
    /// Sessions currently halted
    pub halted_sessions: usize,
}

/// Manager for execution safety across all sessions
pub struct ExecutionSafetyManager {
    /// Per-session safety states
    states: RwLock<HashMap<String, ExecutionSafetyState>>,
    /// Configuration (shared across sessions)
    config: RwLock<ExecutionSafetyConfig>,
    /// Path for persistence
    persistence_path: PathBuf,
}

impl ExecutionSafetyManager {
    /// Create a new manager with default config
    pub fn new(config_dir: PathBuf) -> Self {
        let persistence_path = config_dir.join("execution_safety.json");
        let mut manager = Self {
            states: RwLock::new(HashMap::new()),
            config: RwLock::new(ExecutionSafetyConfig::default()),
            persistence_path,
        };
        manager.load();
        manager
    }

    /// Get or create state for a session
    pub fn get_or_create_state(&self, session_id: &str) -> ExecutionSafetyState {
        let states = self.states.read();
        if let Some(state) = states.get(session_id) {
            return state.clone();
        }
        drop(states);
        
        let mut states = self.states.write();
        let state = ExecutionSafetyState::new(session_id.to_string());
        states.insert(session_id.to_string(), state.clone());
        self.save();
        state
    }

    /// Get state for a session (if exists)
    pub fn get_state(&self, session_id: &str) -> Option<ExecutionSafetyState> {
        self.states.read().get(session_id).cloned()
    }

    /// Update state for a session
    pub fn update_state(&self, session_id: &str, state: ExecutionSafetyState) {
        let mut states = self.states.write();
        states.insert(session_id.to_string(), state);
        drop(states);
        self.save();
    }

    /// Record a tool-call turn for a session
    /// Returns SafetyCheckResult indicating what action to take
    pub fn record_tool_turn(&self, session_id: &str) -> SafetyCheckResult {
        let config = self.config.read().clone();
        if !config.enabled {
            return SafetyCheckResult::Proceed;
        }

        let mut states = self.states.write();
        let state = states.entry(session_id.to_string())
            .or_insert_with(|| ExecutionSafetyState::new(session_id.to_string()));
        
        state.record_tool_turn();
        let result = self.check_thresholds(state, &config);
        
        // Update state based on result
        match &result {
            SafetyCheckResult::Warning => {
                state.warning_issued = true;
            }
            SafetyCheckResult::SafetyEvent(action) => {
                state.record_safety_event();
                info!(session_id = %session_id, action = ?action, 
                      consecutive_turns = state.consecutive_tool_turns,
                      "Execution safety event triggered");
            }
            _ => {}
        }
        
        drop(states);
        self.save();
        
        result
    }

    /// Record a user message for a session (resets consecutive counter)
    pub fn record_user_message(&self, session_id: &str) {
        let mut states = self.states.write();
        if let Some(state) = states.get_mut(session_id) {
            state.record_user_message();
            debug!(session_id = %session_id, "Execution safety streak reset by user message");
        }
        drop(states);
        self.save();
    }

    /// Reset halt state for a session
    pub fn reset_halt(&self, session_id: &str) -> bool {
        let mut states = self.states.write();
        if let Some(state) = states.get_mut(session_id) {
            state.reset_halt();
            info!(session_id = %session_id, "Execution safety halt reset");
            drop(states);
            self.save();
            return true;
        }
        false
    }

    /// Check thresholds against current state
    fn check_thresholds(&self, state: &ExecutionSafetyState, config: &ExecutionSafetyConfig) -> SafetyCheckResult {
        // Check halt state first
        if state.is_halted {
            return SafetyCheckResult::Halted;
        }

        // Check if we should issue a warning
        if config.is_warning_enabled() {
            let warning_threshold = config.warning_turn_count();
            if state.consecutive_tool_turns >= warning_threshold && !state.warning_issued {
                return SafetyCheckResult::Warning;
            }
        }

        // Check if we should trigger safety action
        if state.consecutive_tool_turns >= config.max_consecutive_turns {
            return SafetyCheckResult::SafetyEvent(config.safety_action);
        }

        SafetyCheckResult::Proceed
    }

    /// Get current configuration
    pub fn get_config(&self) -> ExecutionSafetyConfig {
        self.config.read().clone()
    }

    /// Update configuration
    pub fn update_config(&self, config: ExecutionSafetyConfig) {
        *self.config.write() = config.clone();
        info!("Execution safety config updated: {:?}", config);
        self.save();
    }

    /// Update specific config fields
    pub fn update_config_fields(
        &self,
        max_consecutive_turns: Option<usize>,
        warning_threshold_pct: Option<u8>,
        safety_action: Option<SafetyAction>,
        enabled: Option<bool>,
    ) {
        let mut config = self.config.write();
        if let Some(v) = max_consecutive_turns {
            config.max_consecutive_turns = v;
        }
        if let Some(v) = warning_threshold_pct {
            config.warning_threshold_pct = v;
        }
        if let Some(v) = safety_action {
            config.safety_action = v;
        }
        if let Some(v) = enabled {
            config.enabled = v;
        }
        info!("Execution safety config updated: {:?}", *config);
        drop(config);
        self.save();
    }

    /// Get aggregated stats across all sessions
    pub fn get_stats(&self) -> ExecutionSafetyStats {
        let states = self.states.read();
        let total_safety_events: u32 = states.values().map(|s| s.safety_events_count).sum();
        let total_warnings: u32 = states.values().filter(|s| s.warning_issued).count() as u32;
        let halted_sessions = states.values().filter(|s| s.is_halted).count();
        
        ExecutionSafetyStats {
            total_safety_events,
            total_warnings,
            total_halts: halted_sessions as u32,
            active_sessions: states.len(),
            halted_sessions,
        }
    }

    /// Remove state for a session (session ended)
    pub fn remove_session(&self, session_id: &str) {
        let mut states = self.states.write();
        states.remove(session_id);
        drop(states);
        self.save();
    }

    /// Save state to disk
    fn save(&self) {
        use std::fs;
        
        let data = PersistenceData {
            config: self.config.read().clone(),
            states: self.states.read().clone(),
            saved_at: Utc::now(),
        };
        
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            if let Some(parent) = self.persistence_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&self.persistence_path, json);
        }
    }

    /// Load state from disk
    fn load(&mut self) {
        use std::fs;
        
        if !self.persistence_path.exists() {
            return;
        }
        
        if let Ok(json) = fs::read_to_string(&self.persistence_path) {
            if let Ok(data) = serde_json::from_str::<PersistenceData>(&json) {
                *self.config.write() = data.config;
                *self.states.write() = data.states;
                info!("Loaded execution safety state from disk");
            }
        }
    }
}

/// Result of safety check after recording a turn
#[derive(Debug, Clone)]
pub enum SafetyCheckResult {
    /// Continue execution normally
    Proceed,
    /// Issue a warning but continue
    Warning,
    /// Execution is halted, waiting for user
    Halted,
    /// Safety event triggered, take configured action
    SafetyEvent(SafetyAction),
}

impl SafetyCheckResult {
    /// Check if execution should continue
    pub fn should_continue(&self) -> bool {
        matches!(self, SafetyCheckResult::Proceed | SafetyCheckResult::Warning)
    }
    
    /// Check if we need user confirmation
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, SafetyCheckResult::Halted)
    }
}

/// Data structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistenceData {
    config: ExecutionSafetyConfig,
    states: HashMap<String, ExecutionSafetyState>,
    saved_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_config_default() {
        let config = ExecutionSafetyConfig::default();
        assert_eq!(config.max_consecutive_turns, 20);
        assert_eq!(config.warning_threshold_pct, 75);
        assert!(config.enabled);
    }

    #[test]
    fn test_safety_config_warning_threshold() {
        let config = ExecutionSafetyConfig {
            max_consecutive_turns: 100,
            warning_threshold_pct: 50,
            safety_action: SafetyAction::Warn,
            enabled: true,
        };
        assert_eq!(config.warning_turn_count(), 50);
    }

    #[test]
    fn test_execution_safety_state_new() {
        let state = ExecutionSafetyState::new("session1".to_string());
        assert_eq!(state.session_id, "session1");
        assert_eq!(state.consecutive_tool_turns, 0);
        assert!(!state.is_halted);
    }

    #[test]
    fn test_record_tool_turn() {
        let mut state = ExecutionSafetyState::new("session1".to_string());
        state.record_tool_turn();
        assert_eq!(state.consecutive_tool_turns, 1);
        assert_eq!(state.total_tool_turns, 1);
        assert!(state.streak_started_at.is_some());
    }

    #[test]
    fn test_record_user_message_resets() {
        let mut state = ExecutionSafetyState::new("session1".to_string());
        state.record_tool_turn();
        state.record_tool_turn();
        assert_eq!(state.consecutive_tool_turns, 2);
        
        state.record_user_message();
        assert_eq!(state.consecutive_tool_turns, 0);
        assert!(state.streak_started_at.is_none());
    }

    #[test]
    fn test_safety_check_result_proceed() {
        assert!(SafetyCheckResult::Proceed.should_continue());
        assert!(!SafetyCheckResult::Proceed.needs_confirmation());
    }

    #[test]
    fn test_safety_check_result_warning() {
        assert!(SafetyCheckResult::Warning.should_continue());
        assert!(!SafetyCheckResult::Warning.needs_confirmation());
    }

    #[test]
    fn test_safety_check_result_halted() {
        assert!(!SafetyCheckResult::Halted.should_continue());
        assert!(SafetyCheckResult::Halted.needs_confirmation());
    }

    #[test]
    fn test_manager_state_tracking() {
        let temp_dir = std::env::temp_dir().join("test_safety");
        let manager = ExecutionSafetyManager::new(temp_dir.clone());
        
        // Record tool turns
        let result1 = manager.record_tool_turn("session1");
        assert!(matches!(result1, SafetyCheckResult::Proceed));
        
        let result2 = manager.record_tool_turn("session1");
        assert!(matches!(result2, SafetyCheckResult::Proceed));
        
        // Record user message resets
        manager.record_user_message("session1");
        
        let state = manager.get_state("session1").unwrap();
        assert_eq!(state.consecutive_tool_turns, 0);
        
        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
