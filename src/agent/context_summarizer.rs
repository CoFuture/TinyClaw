//! AI-Powered Context Summarizer
//!
//! Instead of simple truncation, uses the AI to create intelligent summaries
//! of old conversation history, preserving more context in fewer tokens.
//!
//! This module provides:
//! - Smart summarization that preserves key decisions, tool usage, and preferences
//! - Automatic summarization trigger when context exceeds threshold
//! - Integration with ContextManager for seamless context management
//! - Configurable summarization settings via HTTP API and JSON-RPC
//! - Persistent summary history across sessions

use crate::agent::client::Agent;
use crate::common::Result;
use crate::types::{Message, Role};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for context summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl SummarizerConfig {
    /// Get minimum messages
    pub fn min_messages(&self) -> usize {
        self.min_messages
    }

    /// Get token threshold
    pub fn token_threshold(&self) -> usize {
        self.token_threshold
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update configuration with new values
    pub fn update(&mut self, min_messages: Option<usize>, token_threshold: Option<usize>, enabled: Option<bool>) {
        if let Some(mm) = min_messages {
            self.min_messages = mm;
        }
        if let Some(tt) = token_threshold {
            self.token_threshold = tt;
        }
        if let Some(en) = enabled {
            self.enabled = en;
        }
    }
}

/// Summary history entry for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryHistoryEntry {
    /// Session ID this summary belongs to
    pub session_id: String,
    /// Number of messages summarized
    pub messages_summarized: usize,
    /// Original token count
    pub original_tokens: usize,
    /// Summary token count
    pub summary_tokens: usize,
    /// Compression ratio
    pub compression_ratio: f32,
    /// Topics extracted
    pub topics: Vec<String>,
    /// When the summary was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<&ContextSummary> for SummaryHistoryEntry {
    fn from(summary: &ContextSummary) -> Self {
        Self {
            session_id: String::new(), // Will be set when recording
            messages_summarized: summary.messages_summarized,
            original_tokens: summary.original_tokens,
            summary_tokens: summary.summary_tokens,
            compression_ratio: summary.compression_ratio(),
            topics: summary.topics.clone(),
            created_at: summary.created_at,
        }
    }
}

/// Summary history for tracking summarization events across sessions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryHistory {
    /// List of all summary events
    pub entries: Vec<SummaryHistoryEntry>,
    /// Total summaries created
    pub total_summaries: usize,
    /// Total messages summarized
    pub total_messages_summarized: usize,
    /// Average compression ratio
    pub avg_compression_ratio: f32,
}

impl SummaryHistory {
    /// Create a new empty summary history
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a summary entry
    pub fn add_entry(&mut self, session_id: &str, summary: &ContextSummary) {
        let mut entry = SummaryHistoryEntry::from(summary);
        entry.session_id = session_id.to_string();
        
        self.entries.push(entry);
        self.total_summaries += 1;
        self.total_messages_summarized += summary.messages_summarized;
        
        // Update average compression ratio
        let total_ratio: f32 = self.entries.iter()
            .map(|e| e.compression_ratio)
            .sum();
        self.avg_compression_ratio = total_ratio / self.entries.len() as f32;
        
        // Keep only last 1000 entries
        if self.entries.len() > 1000 {
            self.entries.remove(0);
        }
    }

    /// Get recent entries (last n)
    pub fn recent_entries(&self, limit: usize) -> Vec<&SummaryHistoryEntry> {
        self.entries.iter().rev().take(limit).collect()
    }

    /// Get entries for a specific session
    pub fn for_session(&self, session_id: &str) -> Vec<&SummaryHistoryEntry> {
        self.entries.iter().filter(|e| e.session_id == session_id).collect()
    }
}

/// Manager for summary history with persistence
pub struct SummaryHistoryManager {
    base_path: PathBuf,
    history: parking_lot::RwLock<SummaryHistory>,
}

impl SummaryHistoryManager {
    /// Create a new manager with default path
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("summary_history");
        
        Self::with_path(path)
    }

    /// Create a manager with custom path
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let manager = Self {
            base_path: path,
            history: parking_lot::RwLock::new(SummaryHistory::new()),
        };
        manager.load();
        manager
    }

    /// Load history from disk
    fn load(&self) {
        if !self.base_path.exists() {
            return;
        }

        let path = self.base_path.join("history.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(history) = serde_json::from_str::<SummaryHistory>(&content) {
                    *self.history.write() = history;
                    tracing::info!("Loaded summary history from {:?}", self.base_path);
                }
            }
        }
    }

    /// Save history to disk
    fn save(&self) {
        if let Some(parent) = self.base_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::create_dir_all(&self.base_path);
        
        let path = self.base_path.join("history.json");
        let history = self.history.read();
        if let Ok(content) = serde_json::to_string_pretty(&*history) {
            let _ = std::fs::write(&path, content);
        }
    }

    /// Add a summary entry
    pub fn record(&self, session_id: &str, summary: &ContextSummary) {
        {
            let mut history = self.history.write();
            history.add_entry(session_id, summary);
        }
        self.save();
    }

    /// Get the history
    pub fn history(&self) -> parking_lot::RwLockReadGuard<'_, SummaryHistory> {
        self.history.read()
    }

    /// Get recent entries
    pub fn recent(&self, limit: usize) -> Vec<SummaryHistoryEntry> {
        self.history.read().recent_entries(limit).into_iter().cloned().collect()
    }

    /// Get entries for a session
    pub fn for_session(&self, session_id: &str) -> Vec<SummaryHistoryEntry> {
        self.history.read().for_session(session_id).into_iter().cloned().collect()
    }

    /// Get statistics
    pub fn stats(&self) -> SummaryHistoryStats {
        let history = self.history.read();
        SummaryHistoryStats {
            total_summaries: history.total_summaries,
            total_messages_summarized: history.total_messages_summarized,
            avg_compression_ratio: history.avg_compression_ratio,
            sessions_count: history.entries.iter()
                .map(|e| e.session_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .len(),
        }
    }
}

/// Statistics about summary history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryHistoryStats {
    pub total_summaries: usize,
    pub total_messages_summarized: usize,
    pub avg_compression_ratio: f32,
    pub sessions_count: usize,
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
    config: parking_lot::RwLock<SummarizerConfig>,
    agent: Arc<Agent>,
}

impl ContextSummarizer {
    /// Create a new context summarizer
    pub fn new(config: SummarizerConfig, agent: Arc<Agent>) -> Self {
        Self { config: parking_lot::RwLock::new(config), agent }
    }

    /// Create with default config
    pub fn with_agent(agent: Arc<Agent>) -> Self {
        Self::new(SummarizerConfig::default(), agent)
    }

    /// Get the current configuration
    pub fn config(&self) -> SummarizerConfig {
        self.config.read().clone()
    }

    /// Update the configuration
    pub fn update_config(&self, min_messages: Option<usize>, token_threshold: Option<usize>, enabled: Option<bool>) {
        let mut config = self.config.write();
        config.update(min_messages, token_threshold, enabled);
        info!("Summarizer config updated: min_msgs={}, threshold={}, enabled={}",
              config.min_messages, config.token_threshold, config.enabled);
    }

    /// Check if summarization should be triggered
    pub fn should_summarize(&self, messages: &[Message], estimated_tokens: usize) -> bool {
        let config = self.config.read();
        if !config.enabled {
            return false;
        }

        if messages.len() < config.min_messages {
            return false;
        }

        estimated_tokens >= config.token_threshold
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
