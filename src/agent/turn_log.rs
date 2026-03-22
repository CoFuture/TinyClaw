//! Turn Execution Log Module
//!
//! Captures a detailed record of every action the agent takes during a turn.
//! Provides transparency into the agent's decision-making and tool usage.
//!
//! Each turn produces a log containing:
//! - Tool executions (name, input, output, success, duration)
//! - Thinking steps (if enabled)
//! - Final response

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Action types that can appear in a turn log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum TurnAction {
    /// Agent was thinking/reasoning
    Thinking {
        duration_ms: u64,
    },
    /// A tool was executed
    Tool {
        name: String,
        input: serde_json::Value,
        output_preview: String,
        success: bool,
        duration_ms: u64,
    },
    /// Agent produced a text response
    Response {
        preview: String,
        duration_ms: u64,
    },
}

/// A single entry in the turn log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnLogEntry {
    /// Timestamp relative to turn start (ms)
    pub offset_ms: u64,
    /// The action that was taken
    pub action: TurnAction,
}

#[allow(dead_code)]
impl TurnAction {
    /// Human-readable summary of this action
    pub fn summary(&self) -> String {
        match self {
            TurnAction::Thinking { duration_ms } => {
                format!("Thinking... ({}ms)", duration_ms)
            }
            TurnAction::Tool { name, success, duration_ms, .. } => {
                let status = if *success { "✓" } else { "✗" };
                format!("{} {} ({}ms)", status, name, duration_ms)
            }
            TurnAction::Response { preview, .. } => {
                let preview = if preview.len() > 60 {
                    format!("{}...", &preview[..60])
                } else {
                    preview.clone()
                };
                format!("Response: \"{}\"", preview)
            }
        }
    }
}

/// Turn execution log - records all actions taken during a single turn
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TurnLog {
    /// Entries in chronological order
    entries: Vec<TurnLogEntry>,
    /// When the turn started (milliseconds since an arbitrary point)
    start_ms: u64,
}

#[allow(dead_code)]
impl TurnLog {
    /// Start a new turn log
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            start_ms: Instant::now().elapsed().as_millis() as u64,
        }
    }

    /// Record a thinking step
    pub fn record_thinking(&mut self) {
        let offset = self.offset_ms();
        self.entries.push(TurnLogEntry {
            offset_ms: offset,
            action: TurnAction::Thinking { duration_ms: 0 },
        });
    }

    /// Record a tool execution
    pub fn record_tool(
        &mut self,
        name: &str,
        input: serde_json::Value,
        output: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let offset = self.offset_ms();
        // Truncate output preview to 200 chars
        let output_preview = if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output.to_string()
        };

        self.entries.push(TurnLogEntry {
            offset_ms: offset,
            action: TurnAction::Tool {
                name: name.to_string(),
                input,
                output_preview,
                success,
                duration_ms,
            },
        });
    }

    /// Record a final response
    pub fn record_response(&mut self, response: &str, duration_ms: u64) {
        let offset = self.offset_ms();
        // Truncate preview
        let preview = if response.len() > 120 {
            format!("{}...", &response[..120])
        } else {
            response.to_string()
        };

        self.entries.push(TurnLogEntry {
            offset_ms: offset,
            action: TurnAction::Response {
                preview,
                duration_ms,
            },
        });
    }

    /// Get all entries
    pub fn entries(&self) -> &[TurnLogEntry] {
        &self.entries
    }

    /// Get total duration of this turn
    pub fn total_duration_ms(&self) -> u64 {
        self.entries
            .iter()
            .map(|e| match &e.action {
                TurnAction::Thinking { duration_ms } => *duration_ms,
                TurnAction::Tool { duration_ms, .. } => *duration_ms,
                TurnAction::Response { duration_ms, .. } => *duration_ms,
            })
            .sum()
    }

    /// Get the number of tool executions in this turn
    pub fn tool_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.action, TurnAction::Tool { .. }))
            .count()
    }

    /// Get successful tool count
    pub fn successful_tool_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.action, TurnAction::Tool { success: true, .. }))
            .count()
    }

    fn offset_ms(&self) -> u64 {
        let elapsed = Instant::now().elapsed().as_millis() as u64;
        elapsed.saturating_sub(self.start_ms)
    }
}

/// Summary of a completed turn log (for display in lists)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnLogSummary {
    /// Number of tool calls made
    pub tool_count: usize,
    /// Number of successful tools
    pub successful_tools: usize,
    /// Total duration in ms
    pub duration_ms: u64,
    /// List of tool names called
    pub tools_used: Vec<String>,
    /// Whether the turn completed normally
    pub completed: bool,
}

#[allow(dead_code)]
impl TurnLog {
    /// Get a summary of this turn log
    pub fn summary(&self) -> TurnLogSummary {
        let tools: Vec<_> = self
            .entries
            .iter()
            .filter_map(|e| match &e.action {
                TurnAction::Tool { name, success, .. } if *success => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();

        TurnLogSummary {
            tool_count: self.tool_count(),
            successful_tools: self.successful_tool_count(),
            duration_ms: self.total_duration_ms(),
            tools_used: tools,
            completed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_log_tool() {
        let mut log = TurnLog::new();
        log.record_tool(
            "read_file",
            serde_json::json!({"path": "/tmp/test.txt"}),
            "file contents here",
            true,
            45,
        );

        assert_eq!(log.tool_count(), 1);
        assert_eq!(log.successful_tool_count(), 1);
        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        match &entries[0].action {
            TurnAction::Tool { name, success, .. } => {
                assert_eq!(name, "read_file");
                assert!(success);
            }
            _ => panic!("expected Tool action"),
        }
    }

    #[test]
    fn test_turn_log_response() {
        let mut log = TurnLog::new();
        log.record_response("Hello, how can I help you?", 120);

        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        match &entries[0].action {
            TurnAction::Response { preview, .. } => {
                assert_eq!(preview, "Hello, how can I help you?");
            }
            _ => panic!("expected Response action"),
        }
    }

    #[test]
    fn test_turn_log_summary() {
        let mut log = TurnLog::new();
        log.record_tool("exec", serde_json::json!({}), "output", true, 30);
        log.record_tool("read", serde_json::json!({}), "err", false, 10);

        let summary = log.summary();
        assert_eq!(summary.tool_count, 2);
        assert_eq!(summary.successful_tools, 1);
        assert_eq!(summary.tools_used, vec!["exec"]);
    }
}
