//! Agent Suggestion Engine Module
//!
//! Proactively suggests relevant actions to the user based on conversation context.
//! The suggestion engine analyzes conversation history and emits suggestions after each turn.

use crate::types::Message;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A proactive suggestion to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Unique suggestion ID
    pub id: String,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Short title for the suggestion
    pub title: String,
    /// Detailed description explaining the suggestion
    pub description: String,
    /// Action label shown to user (e.g., "Do it", "Learn more")
    pub action_label: String,
    /// Action data passed when user accepts (e.g., command to execute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_data: Option<serde_json::Value>,
    /// Confidence score (0.0-1.0) - higher means more confident
    pub confidence: f32,
    /// When this suggestion was generated
    pub created_at: DateTime<Utc>,
    /// Keywords that triggered this suggestion (for debugging/transparency)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<Vec<String>>,
}

impl Suggestion {
    /// Create a new suggestion
    pub fn new(
        suggestion_type: SuggestionType,
        title: impl Into<String>,
        description: impl Into<String>,
        action_label: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            suggestion_type,
            title: title.into(),
            description: description.into(),
            action_label: action_label.into(),
            action_data: None,
            confidence: confidence.clamp(0.0, 1.0),
            created_at: Utc::now(),
            triggered_by: None,
        }
    }

    /// Create with action data
    #[allow(dead_code)]
    pub fn with_action(mut self, action_data: serde_json::Value) -> Self {
        self.action_data = Some(action_data);
        self
    }

    /// Create with triggered by keywords
    pub fn with_triggered_by(mut self, keywords: Vec<String>) -> Self {
        self.triggered_by = Some(keywords);
        self
    }
}

/// Type of suggestion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionType {
    /// Follow-up question or clarification
    #[serde(rename = "follow_up")]
    FollowUp,
    /// Suggested action to take
    #[serde(rename = "action")]
    Action,
    /// Information lookup
    #[serde(rename = "information")]
    Information,
    /// Reminder or schedule suggestion
    #[serde(rename = "reminder")]
    Reminder,
    /// Task creation suggestion
    #[serde(rename = "task")]
    Task,
}

/// Suggestion engine - analyzes conversation and generates proactive suggestions
#[derive(Clone)]
pub struct SuggestionEngine {
    /// Recent keywords seen in conversation (for pattern detection)
    recent_keywords: HashMap<String, usize>,
    /// Action patterns seen (tool names used) - reserved for future use
    #[allow(dead_code)]
    action_patterns: HashMap<String, u32>,
    /// Total turns analyzed
    turn_count: u32,
}

impl SuggestionEngine {
    /// Create a new suggestion engine
    pub fn new() -> Self {
        Self {
            recent_keywords: HashMap::new(),
            action_patterns: HashMap::new(),
            turn_count: 0,
        }
    }

    /// Analyze conversation history and generate suggestions
    /// Called after each agent turn completes
    pub fn generate_suggestions(&mut self, history: &[Message], last_response: &str) -> Vec<Suggestion> {
        self.turn_count += 1;
        let mut suggestions = Vec::new();

        // Extract keywords from conversation
        self.extract_keywords(history, last_response);

        // Generate suggestions based on patterns
        let last_user = self.get_last_user_message(history);
        let last_user_lower = last_user.to_lowercase();

        // 1. Follow-up suggestions based on content
        if let Some(follow_up) = self.generate_follow_up(&last_user_lower, history) {
            suggestions.push(follow_up);
        }

        // 2. Action suggestions based on keywords
        if let Some(action) = self.generate_action_suggestion(&last_user_lower, history) {
            suggestions.push(action);
        }

        // 3. Information suggestions
        if let Some(info) = self.generate_information_suggestion(&last_user_lower, last_response) {
            suggestions.push(info);
        }

        // 4. Reminder suggestions based on time-related keywords
        if let Some(reminder) = self.generate_reminder_suggestion(&last_user_lower) {
            suggestions.push(reminder);
        }

        // 5. Task suggestions based on task-related keywords
        if let Some(task) = self.generate_task_suggestion(&last_user_lower) {
            suggestions.push(task);
        }

        // Limit to top 3 suggestions by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(3);

        suggestions
    }

    /// Extract and track keywords from conversation
    fn extract_keywords(&mut self, history: &[Message], last_response: &str) {
        let mut all_text: String = history.iter()
            .map(|m| m.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        all_text.push(' ');
        all_text.push_str(&last_response.to_lowercase());

        // Key patterns to look for
        let patterns = [
            "file", "directory", "folder", "path", "code", "function",
            "error", "bug", "issue", "problem", "fix", "test",
            "search", "find", "look up", "query", "information",
            "remind", "schedule", "meeting", "appointment", "deadline",
            "task", "todo", "checklist", "project", "plan",
            "weather", "news", "stock", "price", "score",
            "git", "commit", "branch", "merge", "pull", "push",
            "config", "setting", "option", "preference",
            "email", "calendar", "document", "report",
        ];

        for pattern in patterns {
            let count = all_text.matches(pattern).count();
            if count > 0 {
                *self.recent_keywords.entry(pattern.to_string()).or_insert(0) += count;
            }
        }
    }

    /// Get the last user message
    fn get_last_user_message<'a>(&self, history: &'a [Message]) -> &'a str {
        for msg in history.iter().rev() {
            if msg.role == crate::types::Role::User {
                return &msg.content;
            }
        }
        ""
    }

    /// Generate a follow-up suggestion
    fn generate_follow_up(&self, last_user: &str, history: &[Message]) -> Option<Suggestion> {
        // Check if user asked about something that could use more context
        let follow_up_triggers = [
            ("error", "Would you like me to explain the error in more detail?"),
            ("bug", "Should I help you investigate and fix this bug?"),
            ("code", "Would you like me to analyze the code structure?"),
            ("file", "Would you like me to show you the full file?"),
            ("function", "Should I check for similar functions in the codebase?"),
            ("explain", "Would you like me to explain how this works?"),
            ("how", "Would you like a step-by-step explanation?"),
            ("why", "Shall I explain the reasoning behind this?"),
        ];

        for (keyword, question) in follow_up_triggers {
            if last_user.contains(keyword) {
                let confidence = if last_user.contains("explain") || last_user.contains("how") || last_user.contains("why") {
                    0.5 // Lower confidence when already asking for explanation
                } else {
                    0.7
                };

                return Some(Suggestion::new(
                    SuggestionType::FollowUp,
                    format!("Follow up on: {}", keyword),
                    question.to_string(),
                    "Ask me more",
                    confidence,
                ).with_triggered_by(vec![keyword.to_string()]));
            }
        }

        // If conversation is long, suggest a summary
        if history.len() > 10 && self.turn_count > 3 {
            return Some(Suggestion::new(
                SuggestionType::Information,
                "Long conversation",
                "This conversation has many messages. Would you like me to summarize what we've discussed?",
                "Summarize",
                0.6,
            ).with_triggered_by(vec!["conversation_length".to_string()]));
        }

        None
    }

    /// Generate an action suggestion based on detected patterns
    fn generate_action_suggestion(&self, last_user: &str, history: &[Message]) -> Option<Suggestion> {
        // Check recent keyword patterns
        let action_triggers = [
            ("file", "read_file", "Read the file content", "Would you like me to read this file?"),
            ("directory", "list_dir", "List directory contents", "Should I list the directory contents?"),
            ("git", "git_status", "Check git status", "Would you like me to check git status?"),
            ("config", "read_file", "Read config file", "Should I look at the configuration file?"),
            ("test", "exec", "Run tests", "Would you like me to run the tests?"),
        ];

        for (keyword, _action, title, desc) in action_triggers {
            if last_user.contains(keyword) && self.recent_keywords.get(keyword).copied().unwrap_or(0) >= 1 {
                return Some(Suggestion::new(
                    SuggestionType::Action,
                    title,
                    desc.to_string(),
                    "Do it",
                    0.65,
                ).with_triggered_by(vec![keyword.to_string()]));
            }
        }

        // If user mentioned files but we haven't seen tool results
        let has_tool_results = history.iter().any(|m| m.role == crate::types::Role::Tool);
        if !has_tool_results && (last_user.contains("file") || last_user.contains("directory")) {
            return Some(Suggestion::new(
                SuggestionType::Action,
                "Explore files",
                "I can help you read files or list directory contents. Just ask!",
                "Show me how",
                0.5,
            ).with_triggered_by(vec!["file_exploration".to_string()]));
        }

        None
    }

    /// Generate an information lookup suggestion
    fn generate_information_suggestion(&self, last_user: &str, last_response: &str) -> Option<Suggestion> {
        let info_triggers = [
            ("weather", "What's the weather like?"),
            ("news", "Latest news?"),
            ("stock", "Stock price?"),
            ("score", "Sports score?"),
        ];

        for (keyword, question) in info_triggers {
            if last_user.contains(keyword) {
                return Some(Suggestion::new(
                    SuggestionType::Information,
                    format!("Look up {}", keyword),
                    question.to_string(),
                    "Search",
                    0.75,
                ).with_triggered_by(vec![keyword.to_string()]));
            }
        }

        // If assistant mentioned something incomplete
        if last_response.contains("I don't know") || last_response.contains("I couldn't find") {
            return Some(Suggestion::new(
                SuggestionType::Information,
                "Try a different search",
                "I couldn't find an answer. Would you like me to try a different approach or search more broadly?",
                "Try again",
                0.6,
            ).with_triggered_by(vec!["search_failed".to_string()]));
        }

        None
    }

    /// Generate a reminder suggestion based on time-related keywords
    fn generate_reminder_suggestion(&self, last_user: &str) -> Option<Suggestion> {
        let reminder_triggers = [
            ("meeting", "schedule a reminder for your meeting"),
            ("appointment", "set a reminder"),
            ("deadline", "set a deadline reminder"),
            ("remind", "create a reminder"),
            ("later", "set a reminder for later"),
        ];

        for (keyword, action) in reminder_triggers {
            if last_user.contains(keyword) {
                return Some(Suggestion::new(
                    SuggestionType::Reminder,
                    "Set a reminder",
                    format!("Would you like me to {}?", action),
                    "Create reminder",
                    0.7,
                ).with_triggered_by(vec![keyword.to_string()]));
            }
        }

        None
    }

    /// Generate a task suggestion based on task-related keywords
    fn generate_task_suggestion(&self, last_user: &str) -> Option<Suggestion> {
        let task_triggers = [
            ("task", "create a task"),
            ("todo", "add to todo list"),
            ("checklist", "create a checklist"),
            ("project", "break down into tasks"),
            ("plan", "create an action plan"),
        ];

        for (keyword, action) in task_triggers {
            if last_user.contains(keyword) {
                return Some(Suggestion::new(
                    SuggestionType::Task,
                    "Create a task",
                    format!("Would you like me to {}?", action),
                    "Create task",
                    0.7,
                ).with_triggered_by(vec![keyword.to_string()]));
            }
        }

        None
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of suggestions for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SuggestionSummary {
    pub session_id: String,
    pub suggestions: Vec<Suggestion>,
    pub generated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    fn create_test_history() -> Vec<Message> {
        vec![
            Message {
                id: "1".to_string(),
                role: Role::User,
                content: "Can you help me with this file?".to_string(),
                timestamp: Utc::now(),
                tool_call_id: None,
                tool_name: None,
            },
            Message {
                id: "2".to_string(),
                role: Role::Assistant,
                content: "Sure! Which file would you like help with?".to_string(),
                timestamp: Utc::now(),
                tool_call_id: None,
                tool_name: None,
            },
        ]
    }

    #[test]
    fn test_suggestion_new() {
        let suggestion = Suggestion::new(
            SuggestionType::Action,
            "Test title",
            "Test description",
            "Do it",
            0.8,
        );
        assert_eq!(suggestion.title, "Test title");
        assert_eq!(suggestion.confidence, 0.8);
        assert!(!suggestion.id.is_empty());
    }

    #[test]
    fn test_suggestion_confidence_clamp() {
        let suggestion = Suggestion::new(
            SuggestionType::FollowUp,
            "Test",
            "Test",
            "Do it",
            1.5, // Above 1.0
        );
        assert_eq!(suggestion.confidence, 1.0);
    }

    #[test]
    fn test_suggestion_with_action() {
        let suggestion = Suggestion::new(
            SuggestionType::Action,
            "Read file",
            "Read the file",
            "Read",
            0.8,
        ).with_action(serde_json::json!({"path": "/tmp/test.txt"}));

        assert!(suggestion.action_data.is_some());
    }

    #[test]
    fn test_suggestion_engine_new() {
        let engine = SuggestionEngine::new();
        assert_eq!(engine.turn_count, 0);
        assert!(engine.recent_keywords.is_empty());
    }

    #[test]
    fn test_generate_follow_up_on_error() {
        let engine = SuggestionEngine::new();
        let history = create_test_history();

        let suggestion = engine.generate_follow_up("there's an error in my code", &history);
        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.suggestion_type, SuggestionType::FollowUp);
        assert!(s.title.contains("error"));
    }

    #[test]
    fn test_generate_follow_up_on_bug() {
        let engine = SuggestionEngine::new();
        let history = create_test_history();

        let suggestion = engine.generate_follow_up("I found a bug", &history);
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap().suggestion_type, SuggestionType::FollowUp);
    }

    #[test]
    fn test_generate_reminder_suggestion() {
        let engine = SuggestionEngine::new();

        let suggestion = engine.generate_reminder_suggestion("I have a meeting tomorrow");
        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.suggestion_type, SuggestionType::Reminder);
        assert!(s.description.contains("reminder"));
    }

    #[test]
    fn test_generate_task_suggestion() {
        let engine = SuggestionEngine::new();

        let suggestion = engine.generate_task_suggestion("I need to create a task for this");
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap().suggestion_type, SuggestionType::Task);
    }

    #[test]
    fn test_generate_suggestions_limit() {
        let mut engine = SuggestionEngine::new();
        let history = create_test_history();

        // Force multiple suggestions by having multiple keywords
        let suggestions = engine.generate_suggestions(&history, "");
        assert!(suggestions.len() <= 3); // Limited to top 3
    }

    #[test]
    fn test_get_last_user_message() {
        let engine = SuggestionEngine::new();
        let history = create_test_history();

        let last = engine.get_last_user_message(&history);
        assert_eq!(last, "Can you help me with this file?");
    }

    #[test]
    fn test_extract_keywords() {
        let mut engine = SuggestionEngine::new();
        let history = create_test_history();

        engine.extract_keywords(&history, "There's an error in the code file");

        assert!(*engine.recent_keywords.get("error").unwrap_or(&0) > 0);
        assert!(*engine.recent_keywords.get("code").unwrap_or(&0) > 0);
        assert!(*engine.recent_keywords.get("file").unwrap_or(&0) > 0);
    }
}
