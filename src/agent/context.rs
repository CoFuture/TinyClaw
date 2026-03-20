//! Agent Context Module
//!
//! Manages the context for agent execution including:
//! - Session information
//! - Tool registry
//! - Conversation history
//! - Execution state

use crate::gateway::history::{HistoryManager, Message};
use crate::gateway::session::SessionManager;
use crate::agent::client::Agent;
use crate::agent::tools::ToolExecutor;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Execution state of the agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionState {
    /// Agent is thinking
    Thinking,
    /// Agent is using a tool
    UsingTool { tool: String },
    /// Agent is responding
    Responding,
    /// Agent has finished
    Finished,
    /// Agent encountered an error
    Error,
}

/// Agent context for a single conversation
pub struct AgentContext {
    /// Session ID
    pub session_id: String,
    /// Current execution state
    pub state: RwLock<ExecutionState>,
    /// History manager reference
    pub history_manager: Arc<HistoryManager>,
    /// Session manager reference
    pub session_manager: Arc<SessionManager>,
    /// Tool executor reference
    pub tool_executor: Arc<ToolExecutor>,
    /// Agent client reference
    pub agent: Arc<Agent>,
    /// Maximum turns in a loop
    pub max_turns: usize,
    /// Current turn count
    pub turn_count: RwLock<usize>,
}

impl AgentContext {
    /// Create a new agent context
    pub fn new(
        session_id: String,
        history_manager: Arc<HistoryManager>,
        session_manager: Arc<SessionManager>,
        tool_executor: Arc<ToolExecutor>,
        agent: Arc<Agent>,
    ) -> Self {
        Self {
            session_id,
            state: RwLock::new(ExecutionState::Thinking),
            history_manager,
            session_manager,
            tool_executor,
            agent,
            max_turns: 10,
            turn_count: RwLock::new(0),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> ExecutionState {
        self.state.read().clone()
    }

    /// Set state
    pub fn set_state(&self, new_state: ExecutionState) {
        *self.state.write() = new_state;
    }

    /// Increment turn count
    pub fn increment_turn(&self) {
        let mut count = self.turn_count.write();
        *count += 1;
    }

    /// Check if max turns reached
    pub fn max_turns_reached(&self) -> bool {
        *self.turn_count.read() >= self.max_turns
    }

    /// Reset turn count
    pub fn reset_turns(&self) {
        *self.turn_count.write() = 0;
    }

    /// Add user message to history
    pub fn add_user_message(&self, content: &str) {
        self.history_manager.add_message(
            &self.session_id,
            Message::user(content),
        );
    }

    /// Add assistant message to history
    pub fn add_assistant_message(&self, content: &str) {
        self.history_manager.add_message(
            &self.session_id,
            Message::assistant(content),
        );
    }

    /// Get conversation history
    pub fn get_history(&self) -> Vec<Message> {
        if let Some(history) = self.history_manager.get(&self.session_id) {
            history.read().get_messages().to_vec()
        } else {
            Vec::new()
        }
    }
}
