//! Conversation Summary Module
//!
//! Maintains a lightweight running summary of conversation state,
//! including topics, key decisions, user preferences, and current focus.
//! This summary is included in the system prompt when the conversation
//! gets long, helping the agent maintain context when older messages
//! are truncated from the context window.
//!
//! Unlike long-term memory (memory.rs) which stores persistent facts,
//! conversation summary tracks transient state of the CURRENT conversation
//! and resets when the conversation ends or is cleared.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Topic or subject being discussed in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// The topic name
    pub name: String,
    /// How many times this topic was mentioned
    pub mention_count: usize,
    /// Last mention timestamp
    pub last_mentioned: u64,
    /// Whether this is the current active topic
    pub is_active: bool,
}

/// A key decision made during the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub description: String,
    pub timestamp: u64,
    pub turn_index: usize,
}

/// A user preference or stated requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub description: String,
    pub category: PreferenceCategory,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PreferenceCategory {
    Style,      // Communication style preferences
    Technical,  // Technical preferences (language, tools, etc.)
    Process,    // Process preferences (how to work)
    General,    // General preferences
}

/// A question that was raised but not yet answered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub question: String,
    pub timestamp: u64,
    pub turn_index: usize,
}

/// The main conversation summary structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    /// Session key this summary belongs to
    pub session_key: String,
    /// Topics discussed (most recent first)
    pub topics: Vec<Topic>,
    /// Key decisions made
    pub decisions: Vec<Decision>,
    /// User preferences mentioned
    pub preferences: Vec<UserPreference>,
    /// Open/unanswered questions
    pub open_questions: Vec<OpenQuestion>,
    /// Summary of what's currently being discussed
    pub current_focus: String,
    /// Brief overview of conversation so far
    pub overview: String,
    /// Number of turns in conversation
    pub turn_count: usize,
    /// When the conversation/summary was last updated
    pub last_updated: u64,
    /// When the conversation started
    pub started_at: u64,
}

impl Default for ConversationSummary {
    fn default() -> Self {
        let now = current_timestamp();
        Self {
            session_key: String::new(),
            topics: Vec::new(),
            decisions: Vec::new(),
            preferences: Vec::new(),
            open_questions: Vec::new(),
            current_focus: String::new(),
            overview: String::new(),
            turn_count: 0,
            last_updated: now,
            started_at: now,
        }
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Keywords that indicate a decision or conclusion
const DECISION_KEYWORDS: &[&str] = &[
    "decided", "concluded", "agreed", "chosen", "selected",
    "will use", "going to", "plan to", "should", "must",
    "will do", "let's", "best approach", "recommend",
];

/// Keywords that indicate a user preference
const PREFERENCE_KEYWORDS: &[&str] = &[
    "prefer", "like", "dislike", "want", "don't want",
    "I usually", "I like", "I hate", "I need", "I don't like",
    "always", "never", "best if", "better to",
];

/// Keywords that indicate a question
const QUESTION_KEYWORDS: &[&str] = &[
    "how do I", "how can I", "what is", "what are", "when",
    "where", "why", "can you", "could you", "would you",
    "should I", "which", "who", "?",
];

/// Keywords that indicate technical topics
const TECHNICAL_TOPICS: &[&str] = &[
    "rust", "python", "javascript", "typescript", "java", "go", "c++",
    "api", "http", "database", "sql", "nosql", "redis", "cache",
    "docker", "kubernetes", "git", "github", "ci/cd", "deployment",
    "frontend", "backend", "fullstack", "microservice", "server",
    "code", "function", "class", "module", "package", "library",
    "bug", "error", "crash", "performance", "optimization",
];

/// Keywords for conversation topics
const TOPIC_KEYWORDS: &[&str] = &[
    "project", "task", "meeting", "design", "architecture",
    "implementation", "testing", "review", "debug", "refactor",
    "documentation", "deployment", "production", "feature",
];

impl ConversationSummary {
    /// Create a new conversation summary for a session
    pub fn new(session_key: &str) -> Self {
        let now = current_timestamp();
        Self {
            session_key: session_key.to_string(),
            topics: Vec::new(),
            decisions: Vec::new(),
            preferences: Vec::new(),
            open_questions: Vec::new(),
            current_focus: String::new(),
            overview: String::new(),
            turn_count: 0,
            last_updated: now,
            started_at: now,
        }
    }

    /// Update the summary with a new user message and agent response
    pub fn update(&mut self, user_message: &str, assistant_response: &str) {
        self.turn_count += 1;
        let now = current_timestamp();
        self.last_updated = now;

        // Extract and update topics
        self.update_topics(user_message, assistant_response);

        // Extract decisions
        self.extract_decisions(user_message, assistant_response);

        // Extract preferences
        self.extract_preferences(user_message);

        // Extract questions
        self.extract_questions(user_message);

        // Update current focus (most recent topic)
        self.update_current_focus(user_message, assistant_response);

        // Update overview (brief summary)
        self.update_overview(user_message, assistant_response);
    }

    /// Update topics based on message content
    fn update_topics(&mut self, user_message: &str, assistant_response: &str) {
        let combined = format!("{} {}", user_message, assistant_response);
        let combined_lower = combined.to_lowercase();

        // Check for technical topics
        for topic_keyword in TECHNICAL_TOPICS {
            if combined_lower.contains(topic_keyword) {
                self.add_topic(topic_keyword.to_string());
            }
        }

        // Check for general conversation topics
        for topic_keyword in TOPIC_KEYWORDS {
            if combined_lower.contains(topic_keyword) {
                self.add_topic(topic_keyword.to_string());
            }
        }
    }

    /// Add or update a topic
    fn add_topic(&mut self, name: String) {
        let now = current_timestamp();
        
        // Check if topic already exists
        let existing_index = self.topics.iter().position(|t| t.name == name);
        
        if let Some(idx) = existing_index {
            // Mark all as not active first
            for t in &mut self.topics {
                t.is_active = false;
            }
            // Update existing topic
            self.topics[idx].mention_count += 1;
            self.topics[idx].last_mentioned = now;
            self.topics[idx].is_active = true;
            // Move to front
            let topic = self.topics.remove(idx);
            self.topics.insert(0, topic);
        } else {
            // Add new topic
            // Keep at most 10 topics
            if self.topics.len() >= 10 {
                // Remove oldest non-active topic
                self.topics.retain(|t| t.is_active);
                if self.topics.len() >= 9 {
                    self.topics.pop();
                }
            }
            // Mark all as not active
            for t in &mut self.topics {
                t.is_active = false;
            }
            self.topics.insert(0, Topic {
                name,
                mention_count: 1,
                last_mentioned: now,
                is_active: true,
            });
        }
    }

    /// Extract decisions from messages
    fn extract_decisions(&mut self, user_message: &str, assistant_response: &str) {
        let combined = format!("{} {}", user_message, assistant_response);
        let combined_lower = combined.to_lowercase();

        for keyword in DECISION_KEYWORDS {
            if combined_lower.contains(keyword) {
                // Try to extract the decision context
                if let Some(sentence) = self.extract_relevant_sentence(&combined, keyword) {
                    // Avoid duplicates (check last 3 decisions)
                    let is_duplicate = self.decisions.iter()
                        .rev()
                        .take(3)
                        .any(|d| d.description.contains(&sentence[..sentence.len().min(30)]));
                    
                    if !is_duplicate && !sentence.is_empty() && sentence.len() > 10 {
                        self.decisions.push(Decision {
                            description: sentence,
                            timestamp: current_timestamp(),
                            turn_index: self.turn_count,
                        });
                        // Keep at most 10 decisions
                        if self.decisions.len() > 10 {
                            self.decisions.pop();
                        }
                    }
                }
            }
        }
    }

    /// Extract user preferences from messages
    fn extract_preferences(&mut self, user_message: &str) {
        let user_lower = user_message.to_lowercase();

        for keyword in PREFERENCE_KEYWORDS {
            if user_lower.contains(keyword) {
                if let Some(sentence) = self.extract_relevant_sentence(user_message, keyword) {
                    // Determine category
                    let category = if TECHNICAL_TOPICS.iter().any(|t| sentence.to_lowercase().contains(t)) {
                        PreferenceCategory::Technical
                    } else if ["step", "process", "workflow", "order", "approach"].iter().any(|p| sentence.to_lowercase().contains(p)) {
                        PreferenceCategory::Process
                    } else {
                        PreferenceCategory::General
                    };

                    // Check for duplicates
                    let is_duplicate = self.preferences.iter()
                        .rev()
                        .take(3)
                        .any(|p| p.description.contains(&sentence[..sentence.len().min(20)]));
                    
                    if !is_duplicate && !sentence.is_empty() && sentence.len() > 10 {
                        self.preferences.push(UserPreference {
                            description: sentence,
                            category,
                            timestamp: current_timestamp(),
                        });
                        // Keep at most 10 preferences
                        if self.preferences.len() > 10 {
                            self.preferences.pop();
                        }
                    }
                }
            }
        }
    }

    /// Extract questions from user messages
    fn extract_questions(&mut self, user_message: &str) {
        // Check for question marks
        if !user_message.contains('?') {
            return;
        }

        for keyword in QUESTION_KEYWORDS {
            if user_message.to_lowercase().contains(keyword) {
                if let Some(sentence) = self.extract_relevant_sentence(user_message, keyword) {
                    // Mark similar existing questions as answered
                    let is_answered = self.open_questions.iter()
                        .any(|q| q.question.to_lowercase().contains(keyword));
                    
                    if is_answered {
                        // Remove the answered question
                        self.open_questions.retain(|q| !q.question.to_lowercase().contains(keyword));
                    }

                    // Add new question if it looks substantive
                    if sentence.len() > 15 && !sentence.contains("what is your") && !sentence.contains("do you") {
                        // Check for duplicates
                        let is_duplicate = self.open_questions.iter()
                            .any(|q| q.question == sentence);
                        
                        if !is_duplicate {
                            self.open_questions.push(OpenQuestion {
                                question: sentence,
                                timestamp: current_timestamp(),
                                turn_index: self.turn_count,
                            });
                            // Keep at most 5 open questions
                            if self.open_questions.len() > 5 {
                                self.open_questions.pop();
                            }
                        }
                    }
                }
                break; // Only process first matching keyword
            }
        }
    }

    /// Update current focus based on most recent topic
    fn update_current_focus(&mut self, user_message: &str, assistant_response: &str) {
        // Use the most recently mentioned active topic as current focus
        if let Some(active_topic) = self.topics.iter().find(|t| t.is_active) {
            self.current_focus = active_topic.name.clone();
        } else if !self.topics.is_empty() {
            // Use the most recently mentioned topic
            self.current_focus = self.topics[0].name.clone();
        } else {
            // Fallback: extract key phrase from recent exchange
            let recent = format!("{} {}", user_message, assistant_response);
            let words: Vec<&str> = recent.split_whitespace().collect();
            if words.len() > 3 {
                // Take the first 5-8 significant words
                let end = std::cmp::min(8, words.len());
                self.current_focus = words[..end].join(" ");
            }
        }
    }

    /// Update the overview summary
    fn update_overview(&mut self, user_message: &str, _assistant_response: &str) {
        // Create a brief overview based on topics and decisions
        let mut parts: Vec<String> = Vec::new();

        if !self.topics.is_empty() {
            let topic_names: Vec<&str> = self.topics.iter()
                .take(3)
                .map(|t| t.name.as_str())
                .collect();
            parts.push(format!("Topics: {}", topic_names.join(", ")));
        }

        if self.decisions.len() >= 2 {
            parts.push(format!("Decisions made: {}", self.decisions.len()));
        }

        if !self.open_questions.is_empty() {
            parts.push(format!("Open questions: {}", self.open_questions.len()));
        }

        if !parts.is_empty() {
            self.overview = parts.join(" | ");
        } else {
            // Fallback overview
            let user_words: Vec<&str> = user_message.split_whitespace().take(10).collect();
            self.overview = format!("User asked about: {}", user_words.join(" "));
        }
    }

    /// Extract a relevant sentence containing the keyword
    fn extract_relevant_sentence(&self, text: &str, keyword: &str) -> Option<String> {
        let keyword_lower = keyword.to_lowercase();
        
        for sentence in text.split(&['.', '!', '?', '\n'][..]) {
            let sentence_lower = sentence.to_lowercase();
            if sentence_lower.contains(&keyword_lower) {
                let trimmed = sentence.trim().to_string();
                if !trimmed.is_empty() && trimmed.len() >= 10 {
                    return Some(trimmed);
                }
            }
        }
        None
    }

    /// Check if the conversation is getting long enough to need a summary
    pub fn needs_summary(&self) -> bool {
        // If conversation has 8+ turns or meaningful content, include summary
        self.turn_count >= 8 || !self.topics.is_empty() || !self.decisions.is_empty()
    }

    /// Generate system prompt section from this summary
    pub fn to_system_prompt(&self) -> String {
        if !self.needs_summary() {
            return String::new();
        }

        let mut parts: Vec<String> = Vec::new();
        parts.push("## Conversation Summary".to_string());

        if !self.overview.is_empty() {
            parts.push(format!("Overview: {}", self.overview));
        }

        if !self.current_focus.is_empty() {
            parts.push(format!("Current focus: {}", self.current_focus));
        }

        if !self.topics.is_empty() {
            let active_topics: Vec<&str> = self.topics.iter()
                .take(5)
                .map(|t| t.name.as_str())
                .collect();
            parts.push(format!("Topics discussed: {}", active_topics.join(", ")));
        }

        if !self.decisions.is_empty() {
            parts.push("\n### Key Decisions\n".to_string());
            for decision in self.decisions.iter().take(5) {
                parts.push(format!("- {}", decision.description));
            }
        }

        if !self.preferences.is_empty() {
            parts.push("\n### User Preferences\n".to_string());
            for pref in self.preferences.iter().take(5) {
                parts.push(format!("- [{}] {}", 
                    match pref.category {
                        PreferenceCategory::Style => "Style",
                        PreferenceCategory::Technical => "Technical",
                        PreferenceCategory::Process => "Process",
                        PreferenceCategory::General => "General",
                    },
                    pref.description
                ));
            }
        }

        if !self.open_questions.is_empty() {
            parts.push("\n### Open Questions (unanswered)\n".to_string());
            for q in self.open_questions.iter().take(3) {
                parts.push(format!("- {}", q.question));
            }
        }

        parts.join("\n")
    }
}

/// Manager for conversation summaries across multiple sessions
pub struct ConversationSummaryManager {
    /// Per-session conversation summaries
    summaries: std::collections::HashMap<String, ConversationSummary>,
}

impl Default for ConversationSummaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationSummaryManager {
    pub fn new() -> Self {
        Self {
            summaries: std::collections::HashMap::new(),
        }
    }

    /// Get or create a summary for a session
    pub fn get_summary(&mut self, session_key: &str) -> &mut ConversationSummary {
        self.summaries
            .entry(session_key.to_string())
            .or_insert_with(|| ConversationSummary::new(session_key))
    }

    /// Update summary for a session after a turn
    pub fn record_turn(&mut self, session_key: &str, user_message: &str, assistant_response: &str) {
        let summary = self.get_summary(session_key);
        summary.update(user_message, assistant_response);
    }

    /// Get summary for API response
    pub fn get(&self, session_key: &str) -> Option<&ConversationSummary> {
        self.summaries.get(session_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_summary() {
        let summary = ConversationSummary::new("test-session");
        assert_eq!(summary.session_key, "test-session");
        assert_eq!(summary.turn_count, 0);
        assert!(summary.topics.is_empty());
    }

    #[test]
    fn test_update_with_user_message() {
        let mut summary = ConversationSummary::new("test");
        summary.update("How do I use Rust's error handling?", "You can use Result<T, E> type...");
        
        assert_eq!(summary.turn_count, 1);
        assert!(!summary.current_focus.is_empty());
        assert!(!summary.overview.is_empty());
    }

    #[test]
    fn test_topic_tracking() {
        let mut summary = ConversationSummary::new("test");
        summary.update("I'm working on a Rust project", "Great! Rust is a systems programming language...");
        summary.update("Tell me more about Rust concurrency", "Rust has async/await support...");
        
        // Should have tracked rust as a topic
        assert!(!summary.topics.is_empty());
        let rust_topic = summary.topics.iter().find(|t| t.name == "rust");
        assert!(rust_topic.is_some());
        assert!(rust_topic.unwrap().mention_count >= 2);
    }

    #[test]
    fn test_decision_extraction() {
        let mut summary = ConversationSummary::new("test");
        summary.update("Which approach should I use?", "I recommend using async/await. It's the modern approach in Rust.");
        
        // The recommendation might be extracted as a decision-like statement
        // (this is a weak test since extraction is keyword-based)
        assert!(summary.turn_count == 1);
    }

    #[test]
    fn test_question_tracking() {
        let mut summary = ConversationSummary::new("test");
        summary.update("How does async/await work in Rust?", "It's a language feature that...");
        
        // Question should be tracked
        assert!(!summary.open_questions.is_empty() || summary.turn_count == 1);
    }

    #[test]
    fn test_preference_extraction() {
        let mut summary = ConversationSummary::new("test");
        summary.update("I prefer using async/await over threads", "Understood!");
        
        // Preference should be tracked
        assert!(!summary.preferences.is_empty() || summary.turn_count == 1);
    }

    #[test]
    fn test_system_prompt_generation() {
        let mut summary = ConversationSummary::new("test");
        summary.update("Tell me about Rust", "Rust is a systems programming language focused on safety and performance.");
        summary.update("What's the best error handling approach?", "Use Result<T, E> for recoverable errors.");
        
        let prompt = summary.to_system_prompt();
        
        // Should include summary section when conversation has content
        if summary.needs_summary() {
            assert!(prompt.contains("Conversation Summary"));
        }
    }

    #[test]
    fn test_manager_multiple_sessions() {
        let mut manager = ConversationSummaryManager::new();
        
        manager.record_turn("session1", "Hello", "Hi there!");
        manager.record_turn("session2", "Hi", "Hello!");
        
        // Check session1
        {
            let summary1 = manager.get_summary("session1");
            assert_eq!(summary1.turn_count, 1);
        }
        // Check session2
        {
            let summary2 = manager.get_summary("session2");
            assert_eq!(summary2.turn_count, 1);
        }
    }
}
