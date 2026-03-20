//! Event system module

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    /// Assistant sent text
    #[serde(rename = "assistant.text")]
    AssistantText {
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
