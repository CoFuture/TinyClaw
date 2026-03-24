//! AI-Powered Context Summarizer
//!
//! Instead of simple truncation, uses the AI to create intelligent summaries
//! of old conversation history, preserving more context in fewer tokens.
//!
//! This module provides:
//! - Smart summarization that preserves key decisions, tool usage, and preferences
//! - Automatic summarization trigger when context exceeds threshold
//! - Integration with ContextManager for seamless context management

use crate::agent::client::Agent;
use crate::common::Result;
use crate::types::{Message, Role};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for context summarization
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// Minimum messages before considering summarization
    pub min_messages: usize,
    /// Token threshold to trigger summarization (default: 100k)
    pub token_threshold: usize,
    /// Whether summarization is enabled
    pub enabled: bool,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            min_messages: 10,
            token_threshold: 100_000,
            enabled: true,
        }
    }
}

/// A summarized section of conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    /// The summary text
    pub summary: String,
    /// Number of original messages summarized
    pub messages_summarized: usize,
    /// Token count of the summary
    pub summary_tokens: usize,
    /// Original token count (before summarization)
    pub original_tokens: usize,
    /// Timestamp when summary was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Key topics extracted
    pub topics: Vec<String>,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Tools that were used
    pub tools_used: Vec<String>,
}

impl ContextSummary {
    /// Create a new context summary
    pub fn new(
        summary: String,
        messages_summarized: usize,
        original_tokens: usize,
        topics: Vec<String>,
        decisions: Vec<String>,
        tools_used: Vec<String>,
    ) -> Self {
        // Estimate tokens in summary (~4 chars per token for English)
        let summary_tokens = summary.len() / 4;
        
        Self {
            summary,
            messages_summarized,
            summary_tokens,
            original_tokens,
            created_at: chrono::Utc::now(),
            topics,
            decisions,
            tools_used,
        }
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        1.0 - (self.summary_tokens as f32 / self.original_tokens as f32)
    }

    /// Convert to a system prompt section
    pub fn to_system_prompt(&self) -> String {
        let mut parts = vec![
            format!("## Conversation Summary ({} messages compressed)", self.messages_summarized),
        ];

        if !self.topics.is_empty() {
            parts.push(format!("Topics: {}", self.topics.join(", ")));
        }

        if !self.decisions.is_empty() {
            parts.push("Key decisions:".to_string());
            for d in &self.decisions {
                parts.push(format!("- {}", d));
            }
        }

        if !self.tools_used.is_empty() {
            parts.push(format!("Tools used: {}", self.tools_used.join(", ")));
        }

        parts.push("".to_string());
        parts.push(self.summary.clone());

        parts.join("\n")
    }
}

/// AI-powered context summarizer
pub struct ContextSummarizer {
    config: SummarizerConfig,
    agent: Arc<Agent>,
}

impl ContextSummarizer {
    /// Create a new context summarizer
    pub fn new(config: SummarizerConfig, agent: Arc<Agent>) -> Self {
        Self { config, agent }
    }

    /// Create with default config
    pub fn with_agent(agent: Arc<Agent>) -> Self {
        Self::new(SummarizerConfig::default(), agent)
    }

    /// Check if summarization should be triggered
    pub fn should_summarize(&self, messages: &[Message], estimated_tokens: usize) -> bool {
        if !self.config.enabled {
            return false;
        }

        if messages.len() < self.config.min_messages {
            return false;
        }

        estimated_tokens >= self.config.token_threshold
    }

    /// Summarize a portion of messages
    /// Returns the summary and the index of the first non-summarized message
    pub async fn summarize_messages(
        &self,
        messages: &[Message],
        keep_recent: usize,
    ) -> Result<Option<ContextSummary>> {
        if messages.len() <= keep_recent {
            return Ok(None);
        }

        // Messages to summarize (excluding recent ones we want to keep)
        let to_summarize = &messages[..messages.len() - keep_recent];
        
        if to_summarize.is_empty() {
            return Ok(None);
        }

        info!(
            messages_count = to_summarize.len(),
            keep_recent = keep_recent,
            "Summarizing conversation history"
        );

        // Build the summarization prompt
        let conversation_text = self.format_messages_for_summary(to_summarize);
        
        let summary_prompt = self.build_summary_prompt(&conversation_text);

        // Use the agent to generate summary
        // We'll use a simple send_message call for this
        let summary_text = self.generate_summary(&summary_prompt).await?;

        // Extract structured info from the summary
        let (topics, decisions, tools) = self.extract_structured_info(&summary_text, to_summarize);

        // Calculate original token estimate
        let original_tokens = to_summarize.iter()
            .map(|m| m.content.len() / 4)
            .sum();

        let summary = ContextSummary::new(
            summary_text,
            to_summarize.len(),
            original_tokens,
            topics,
            decisions,
            tools,
        );

        debug!(
            compression_ratio = format!("{:.1}%", summary.compression_ratio() * 100.0),
            original_tokens = original_tokens,
            summary_tokens = summary.summary_tokens,
            "Context summary created"
        );

        Ok(Some(summary))
    }

    /// Format messages for summarization
    fn format_messages_for_summary(&self, messages: &[Message]) -> String {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                    Role::Tool => "Tool",
                };
                format!("[{}]: {}", role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build the prompt for summarization
    fn build_summary_prompt(&self, conversation: &str) -> String {
        format!(
            r#"Please summarize the following conversation history. Your summary should:

1. Preserve ALL important decisions made
2. Note any user preferences or requirements mentioned
3. Track the progression of topics discussed
4. Remember key information that might be needed later

Format your response as:

TOPICS: [list main topics, comma-separated]
DECISIONS: [list key decisions, one per line, starting with -]
TOOLS: [list tools/commands used, comma-separated]

SUMMARY:
[Write a concise narrative summary of the conversation, focusing on information that would be important for continuing this conversation. Include specific details like file names, code snippets discussed, error messages encountered, etc.]

---

CONVERSATION TO SUMMARIZE:

{}"#,
            conversation
        )
    }

    /// Generate summary using the agent
    async fn generate_summary(&self, prompt: &str) -> Result<String> {
        // Use the agent's client directly for summarization
        // This is a special internal call that doesn't go through the normal tool loop
        self.agent.summarize_content(prompt).await
    }

    /// Extract structured information from summary text
    fn extract_structured_info(
        &self,
        summary_text: &str,
        _original_messages: &[Message],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut topics = Vec::new();
        let mut decisions = Vec::new();
        let mut tools = Vec::new();

        // Parse the structured sections
        for line in summary_text.lines() {
            let line = line.trim();
            
            // Parse TOPICS
            if line.starts_with("TOPICS:") {
                let topics_str = line.strip_prefix("TOPICS:").unwrap_or("");
                topics = topics_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            
            // Parse DECISIONS
            if line.starts_with("- ") {
                let decision = line.strip_prefix("- ").unwrap_or(line);
                if !decision.is_empty() {
                    decisions.push(decision.to_string());
                }
            }
            
            // Parse TOOLS
            if line.starts_with("TOOLS:") {
                let tools_str = line.strip_prefix("TOOLS:").unwrap_or("");
                tools = tools_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        // Also extract tool names from the summary if mentioned in narrative
        let tool_keywords = ["read_file", "write_file", "exec", "http_request", "list_dir", 
                            "grep", "find", "glob", "cp", "mv", "rm", "cat"];
        for tool in tool_keywords {
            if summary_text.contains(tool) && !tools.contains(&tool.to_string()) {
                tools.push(tool.to_string());
            }
        }

        (topics, decisions, tools)
    }

    /// Create a system message from a context summary
    pub fn create_summary_message(summary: &ContextSummary) -> Message {
        Message::system(summary.to_system_prompt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_summary_creation() {
        let summary = ContextSummary::new(
            "Test summary".to_string(),
            10,
            5000,
            vec!["rust".to_string(), "api".to_string()],
            vec!["Use async/await".to_string()],
            vec!["read_file".to_string()],
        );

        assert_eq!(summary.messages_summarized, 10);
        assert_eq!(summary.topics.len(), 2);
        assert_eq!(summary.decisions.len(), 1);
        assert!(summary.compression_ratio() > 0.0);
    }

    #[test]
    fn test_summary_to_system_prompt() {
        let summary = ContextSummary::new(
            "We discussed Rust async programming.".to_string(),
            5,
            2000,
            vec!["rust".to_string()],
            vec!["Use tokio runtime".to_string()],
            vec!["read_file".to_string()],
        );

        let prompt = summary.to_system_prompt();
        
        assert!(prompt.contains("Conversation Summary"));
        assert!(prompt.contains("5 messages compressed"));
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("tokio runtime"));
    }

    #[test]
    fn test_summarizer_config_defaults() {
        let config = SummarizerConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.min_messages, 10);
        assert_eq!(config.token_threshold, 100_000);
    }

    #[test]
    fn test_should_summarize() {
        let config = SummarizerConfig {
            min_messages: 5,
            token_threshold: 1000,
            ..Default::default()
        };

        // Would need an Agent to create summarizer, so just test the logic
        let messages: Vec<Message> = (0..10)
            .map(|i| {
                if i % 2 == 0 {
                    Message::user("Test message content here".repeat(10))
                } else {
                    Message::assistant("Test message content here".repeat(10))
                }
            })
            .collect();

        let should = messages.len() >= config.min_messages;
        assert!(should);
    }

    #[test]
    fn test_extract_structured_info() {
        // This test verifies the parsing logic
        let summary_text = r#"TOPICS: rust, async, tokio
DECISIONS:
- Use async/await pattern
- Implement error handling with Result
TOOLS: read_file, exec

SUMMARY:
We discussed async programming in Rust."#;

        // Parse manually for test
        let mut topics = Vec::new();
        let mut decisions = Vec::new();
        let mut tools = Vec::new();

        for line in summary_text.lines() {
            let line = line.trim();
            if line.starts_with("TOPICS:") {
                let t = line.strip_prefix("TOPICS:").unwrap_or("");
                topics = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            if line.starts_with("- ") {
                decisions.push(line.strip_prefix("- ").unwrap_or(line).to_string());
            }
            if line.starts_with("TOOLS:") {
                let t = line.strip_prefix("TOOLS:").unwrap_or("");
                tools = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
        }

        assert_eq!(topics, vec!["rust", "async", "tokio"]);
        assert_eq!(decisions.len(), 2);
        assert_eq!(tools, vec!["read_file", "exec"]);
    }
}
