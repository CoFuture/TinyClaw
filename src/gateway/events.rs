//! Event system module

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::agent::turn_log::{TurnLogEntry, TurnLogSummary};
use crate::agent::task::TaskSummary;

/// Event types for real-time streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    /// Turn started (agent beginning to process a message)
    #[serde(rename = "turn.started")]
    TurnStarted {
        session_id: String,
        message: String,
    },
    
    /// Agent is thinking
    #[serde(rename = "turn.thinking")]
    TurnThinking {
        session_id: String,
    },
    
    /// Turn completed
    #[serde(rename = "turn.ended")]
    TurnEnded {
        session_id: String,
        response: String,
    },
    
    /// Turn was cancelled
    #[serde(rename = "turn.cancelled")]
    TurnCancelled {
        session_id: String,
    },
    
    /// Assistant sent text
    #[serde(rename = "assistant.text")]
    AssistantText {
        session_id: String,
        text: String,
    },
    
    /// Assistant is sending partial/streaming text (incremental update)
    #[serde(rename = "assistant.partial")]
    AssistantPartial {
        session_id: String,
        text: String,
    },
    
    /// Assistant used a tool
    #[serde(rename = "assistant.tool_use")]
    AssistantToolUse {
        session_id: String,
        tool: String,
        input: serde_json::Value,
    },
    
    /// Tool result
    #[serde(rename = "tool_result")]
    ToolResult {
        session_id: String,
        tool_call_id: String,
        output: String,
    },

    /// Turn execution log was updated (new action recorded)
    #[serde(rename = "turn.log_updated")]
    TurnLogUpdated {
        session_id: String,
        entry: TurnLogEntry,
    },

    /// Turn execution log was completed
    #[serde(rename = "turn.log_completed")]
    TurnLogCompleted {
        session_id: String,
        summary: TurnLogSummary,
    },

    /// Session created
    #[serde(rename = "session.created")]
    SessionCreated {
        session_id: String,
        kind: String,
    },

    /// Session ended
    #[serde(rename = "session.ended")]
    SessionEnded {
        session_id: String,
    },

    /// Task created
    #[serde(rename = "task.created")]
    TaskCreated {
        task_id: String,
        summary: TaskSummary,
    },

    /// Task started
    #[serde(rename = "task.started")]
    TaskStarted {
        task_id: String,
    },

    /// Task progress update
    #[serde(rename = "task.progress")]
    TaskProgress {
        task_id: String,
        step: usize,
        total_steps: usize,
        message: String,
    },

    /// Task completed successfully
    #[serde(rename = "task.completed")]
    TaskCompleted {
        task_id: String,
        result: String,
    },

    /// Task failed
    #[serde(rename = "task.failed")]
    TaskFailed {
        task_id: String,
        error: String,
    },

    /// Task cancelled
    #[serde(rename = "task.cancelled")]
    TaskCancelled {
        task_id: String,
    },
    
    /// Error occurred
    #[serde(rename = "error")]
    Error {
        session_id: String,
        message: String,
    },
    
    /// Status update
    #[serde(rename = "status")]
    Status {
        message: String,
    },
    
    /// Heartbeat to keep connections alive
    #[serde(rename = "heartbeat")]
    Heartbeat {
        timestamp: i64,
    },
}

/// Event emitter for broadcasting events
pub struct EventEmitter {
    sender: broadcast::Sender<Event>,
}

impl EventEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    /// Emit an event
    pub fn emit(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get subscriber count
    #[allow(dead_code)]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
