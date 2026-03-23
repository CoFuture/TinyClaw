//! Context Manager Module
//!
//! Manages conversation context for AI models including:
//! - Token estimation (language-aware)
//! - Message importance scoring
//! - Smart context truncation (priority-based)
//! - Context formatting for different providers

use crate::types::{Message, Role};
use serde::{Deserialize, Serialize};

/// Token estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TokenEstimate {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

/// Context formatting options
#[derive(Debug, Clone)]
pub struct ContextOptions {
    /// Maximum tokens for input context
    pub max_context_tokens: usize,
    /// Reserved tokens for output
    pub reserved_output_tokens: usize,
    /// System prompt to prepend
    pub system_prompt: Option<String>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        // Default to Claude's context limit (200k tokens)
        Self {
            max_context_tokens: 180_000,
            reserved_output_tokens: 4000,
            system_prompt: None,
        }
    }
}

/// Importance level for message prioritization during truncation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageImportance {
    /// Low importance - can be dropped first
    Low = 0,
    /// Medium importance - kept if there's room
    Medium = 1,
    /// High importance - decisions, preferences, key info - should be preserved
    High = 2,
    /// Critical importance - tool results, must be kept
    Critical = 3,
}

/// Context manager for formatting and truncating conversation history
#[derive(Debug, Clone)]
pub struct ContextManager {
    options: ContextOptions,
}

impl ContextManager {
    pub fn new(options: ContextOptions) -> Self {
        Self { options }
    }

    /// Estimate tokens from text with language and content awareness
    /// 
    /// Uses a more accurate estimation considering:
    /// - Chinese/Asian characters (~1.5-2 chars per token vs ~4 for English)
    /// - Code content (different ratio)
    /// - Repetitive content (compresses well)
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        
        let _char_count = text.chars().count();
        
        // Check for code blocks (they have different token ratios)
        let is_code = text.contains("```") || text.contains("function ") || 
                      text.contains("def ") || text.contains("class ") ||
                      text.contains("const ") || text.contains("let ") ||
                      text.contains("var ") || text.contains("import ");
        
        // Check for Chinese/Japanese/Korean characters (need more tokens per char)
        let cjk_count = text.chars().filter(|c| {
            let code = *c as u32;
            // CJK Unified Ideographs and related ranges
            (0x4E00..=0x9FFF).contains(&code) || // CJK Unified Ideographs
            (0x3000..=0x303F).contains(&code) || // CJK Symbols
            (0xFF00..=0xFFEF).contains(&code) || // Halfwidth and Fullwidth Forms
            (0x3040..=0x309F).contains(&code) || // Hiragana
            (0x30A0..=0x30FF).contains(&code)    // Katakana
        }).count();
        
        // Count non-ASCII characters that aren't CJK
        let non_ascii_non_cjk = text.chars()
            .filter(|c| {
                let code = *c as u32;
                !c.is_ascii() && 
                !(0x4E00..=0x9FFF).contains(&code) &&
                !(0x3000..=0x303F).contains(&code) &&
                !(0xFF00..=0xFFEF).contains(&code) &&
                !(0x3040..=0x309F).contains(&code) &&
                !(0x30A0..=0x30FF).contains(&code)
            })
            .count();
        
        let ascii_count = text.chars().filter(|c| c.is_ascii()).count();
        
        // Calculate weighted estimate
        // ASCII: ~4 chars per token (English-like)
        // CJK: ~1.5-2 chars per token (more tokens per char)
        // Other: ~2-3 chars per token
        let ascii_tokens = ascii_count / 4;
        let cjk_tokens = (cjk_count * 2) / 3; // ~1.5 chars per token
        let other_tokens = non_ascii_non_cjk / 2;
        
        let base_tokens = ascii_tokens + cjk_tokens + other_tokens;
        
        // Adjust for code (code often has repetitive keywords that compress)
        if is_code {
            // Code tends to compress - estimate more conservatively
            (base_tokens * 3) / 4
        } else {
            base_tokens
        }
    }

    /// Estimate tokens from a message
    pub fn estimate_message_tokens(msg: &Message) -> usize {
        let base = Self::estimate_tokens(&msg.content);
        // Add overhead for role and other fields
        let role_overhead = 10; // ~"[role]\n" 
        base + role_overhead
    }

    /// Estimate total tokens for a list of messages
    pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
        messages.iter().map(Self::estimate_message_tokens).sum()
    }
    
    /// Score a message's importance for context preservation
    /// Higher importance = should be kept during truncation
    fn score_message_importance(msg: &Message) -> MessageImportance {
        let content_lower = msg.content.to_lowercase();
        let content = &msg.content;
        
        // Tool results are critical - they provide essential context
        if msg.role == Role::Tool {
            // But filter out very long tool outputs that might be noise
            if content.len() > 5000 {
                return MessageImportance::Medium;
            }
            return MessageImportance::Critical;
        }
        
        // Check for high-importance indicators
        let high_importance_patterns = [
            // Decision indicators
            "decided", "concluded", "agreed", "chosen", "selected",
            "will use", "going to", "plan to", "best approach",
            // Preference indicators
            "i prefer", "i like", "i want", "i need", "i don't like",
            "i usually", "best if", "better to", "don't want",
            // Action items
            "need to", "should", "must", "will do", "let's",
            // Important user info
            "my name is", "i am ", "i'm ", "call me",
            // Technical decisions
            "we'll use", "use the", "implement", "architecture",
            "design", "approach is",
        ];
        
        let has_high = high_importance_patterns.iter()
            .any(|p| content_lower.contains(p));
        
        if has_high {
            return MessageImportance::High;
        }
        
        // Check for medium importance - substantive content
        let medium_importance_patterns = [
            // Questions (preserve for context)
            "how do i", "how can i", "what is", "what are", "when",
            "where", "why", "can you", "could you", "would you",
            "should i", "which", "who", "?",
            // Technical content
            "code", "file", "function", "api", "error", "bug",
            "project", "test", "run", "build", "compile",
            // Responses with substance (assistant messages with actual content)
        ];
        
        // Assistant messages with substantial content are medium importance
        if msg.role == Role::Assistant && content.len() > 100 {
            // Check if it contains substantive patterns
            let has_medium = medium_importance_patterns.iter()
                .any(|p| content_lower.contains(p));
            if has_medium || content.contains("```") {
                return MessageImportance::Medium;
            }
        }
        
        // Short acknowledgments, greetings, etc. are low importance
        if content.len() < 20 {
            return MessageImportance::Low;
        }
        
        MessageImportance::Medium
    }

    /// Format messages for Anthropic API
    /// Anthropic format: system prompt, then messages with roles
    #[allow(dead_code)]
    pub fn format_for_anthropic(&self, messages: &[Message]) -> (Vec<AnthropicMessage>, Option<String>) {
        let mut system = self.options.system_prompt.clone();
        
        // Anthropic: system is separate, messages are just user/assistant
        let formatted: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != Role::System && m.role != Role::Tool) // Filter out system and tool messages
            .filter_map(|m| {
                let role = match m.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    _ => return None, // Should not happen due to filter above
                };
                Some(AnthropicMessage {
                    role,
                    content: m.content.clone(),
                })
            })
            .collect();

        // If there's a system message in the history, prepend it to our system prompt
        if let Some(first_sys) = messages.iter().find(|m| m.role == Role::System) {
            if system.is_some() {
                system = Some(format!("{}\n\n---\n\n{}", first_sys.content, system.as_ref().unwrap()));
            } else {
                system = Some(first_sys.content.clone());
            }
        }

        (formatted, system)
    }

    /// Format messages for OpenAI API
    /// OpenAI format: messages array with role field
    #[allow(dead_code)]
    pub fn format_for_openai(&self, messages: &[Message]) -> Vec<OpenAIMessage> {
        let mut result = Vec::new();

        // Add system prompt if configured
        if let Some(ref sys) = self.options.system_prompt {
            result.push(OpenAIMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }

        // Add conversation messages
        for m in messages {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => continue, // Already handled above
                Role::Tool => {
                    // For OpenAI, tool results come as assistant messages with tool_calls
                    "assistant"
                }
            };

            result.push(OpenAIMessage {
                role: role.to_string(),
                content: m.content.clone(),
            });
        }

        result
    }

    /// Format messages for Ollama API (similar to OpenAI)
    #[allow(dead_code)]
    pub fn format_for_ollama(&self, messages: &[Message]) -> Vec<OpenAIMessage> {
        // Ollama uses the same format as OpenAI
        self.format_for_openai(messages)
    }

    /// Truncate messages to fit within token budget using priority-based strategy
    /// 
    /// Priority strategy:
    /// 1. Always keep system messages
    /// 2. Always keep the most recent message (current context)
    /// 3. Keep high/critical importance messages (decisions, preferences, tool results)
    /// 4. Keep recent messages (recency bias)
    /// 5. Fill remaining budget with medium, then low importance
    pub fn truncate_to_fit(&self, messages: &[Message]) -> Vec<Message> {
        let max_input = self.options.max_context_tokens - self.options.reserved_output_tokens;
        
        // First pass: check if we need truncation
        let total_tokens = Self::estimate_messages_tokens(messages);
        if total_tokens <= max_input {
            return messages.to_vec();
        }

        // Separate system messages (always keep)
        let system_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role == Role::System)
            .cloned()
            .collect();
        
        // Get conversation messages (excluding system)
        let conv_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role != Role::System)
            .cloned()
            .collect();

        // Calculate system message overhead
        let system_overhead = Self::estimate_messages_tokens(&system_msgs);
        let available_budget = max_input.saturating_sub(system_overhead);
        
        if available_budget == 0 {
            // Even system messages don't fit - return just system
            return system_msgs;
        }

        // Score all conversation messages by importance
        let scored_msgs: Vec<(&Message, MessageImportance)> = conv_msgs
            .iter()
            .map(|m| (m, Self::score_message_importance(m)))
            .collect();

        // Strategy: Build result by priority tiers
        let mut result: Vec<Message> = Vec::new();
        let mut tokens_used = 0;
        
        // Phase 1: Always keep the most recent message (current context anchor)
        if let Some(last_msg) = conv_msgs.last() {
            let msg_tokens = Self::estimate_message_tokens(last_msg);
            if msg_tokens <= available_budget {
                result.push(last_msg.clone());
                tokens_used += msg_tokens;
            }
        }
        
        // Phase 2: Keep high and critical importance messages
        // Process in reverse order (older first) so we can insert at front
        let high_priority: Vec<Message> = scored_msgs
            .iter()
            .filter(|(m, imp)| *imp >= MessageImportance::High && !is_same_message(m, conv_msgs.last()))
            .map(|(m, _)| (*m).clone())
            .collect();
        
        // We want to keep recent high-priority items and older ones if space allows
        for msg in high_priority.iter().rev() {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_used + msg_tokens <= max_input {
                result.insert(0, msg.clone());
                tokens_used += msg_tokens;
            }
        }
        
        // Phase 3: Keep recent medium importance (fill remaining budget)
        // Take from most recent backwards
        let recent_medium: Vec<&Message> = scored_msgs
            .iter()
            .filter(|(m, imp)| *imp == MessageImportance::Medium && !is_in_result(m, &result))
            .map(|(m, _)| *m)
            .collect();
        
        for msg in recent_medium.iter().rev() {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_used + msg_tokens <= max_input {
                result.insert(0, (*msg).clone());
                tokens_used += msg_tokens;
            }
        }
        
        // Phase 4: If still room, take older medium importance
        let older_medium: Vec<&Message> = scored_msgs
            .iter()
            .filter(|(m, imp)| *imp == MessageImportance::Medium && !is_in_result(m, &result))
            .map(|(m, _)| *m)
            .collect();
        
        for msg in older_medium.iter() {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_used + msg_tokens <= max_input {
                result.insert(0, (*msg).clone());
                tokens_used += msg_tokens;
            }
        }
        
        // Phase 5: Low importance only if we have extra room
        let low_priority: Vec<&Message> = scored_msgs
            .iter()
            .filter(|(m, imp)| *imp == MessageImportance::Low && !is_in_result(m, &result))
            .map(|(m, _)| *m)
            .collect();
        
        // Take most recent low priority first
        for msg in low_priority.iter().rev() {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_used + msg_tokens <= max_input {
                result.insert(0, (*msg).clone());
                tokens_used += msg_tokens;
            }
        }

        // Prepend system messages
        let final_result: Vec<Message> = system_msgs.into_iter()
            .chain(result)
            .collect();

        final_result
    }

    /// Check if context needs truncation
    pub fn needs_truncation(&self, messages: &[Message]) -> bool {
        let max_input = self.options.max_context_tokens - self.options.reserved_output_tokens;
        Self::estimate_messages_tokens(messages) > max_input
    }
}

/// Helper: Check if two messages are the same (by content and role)
fn is_same_message(a: &Message, b: Option<&Message>) -> bool {
    match b {
        None => false,
        Some(b_msg) => a.role == b_msg.role && a.content == b_msg.content,
    }
}

/// Helper: Check if a message is already in the result
fn is_in_result(msg: &Message, result: &[Message]) -> bool {
    result.iter().any(|m| m.role == msg.role && m.content == msg.content)
}

/// Anthropic API message format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI API message format  
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        // ~4 chars per token
        let text = "Hello, world!";
        let tokens = ContextManager::estimate_tokens(text);
        assert!(tokens >= 2 && tokens <= 4, "Expected ~3 tokens, got {}", tokens);
    }

    #[test]
    fn test_token_estimation_longer() {
        let text = "This is a longer piece of text that should contain more tokens when estimated.";
        let tokens = ContextManager::estimate_tokens(text);
        assert!(tokens > 10, "Expected > 10 tokens, got {}", tokens);
    }

    #[test]
    fn test_truncate_to_fit() {
        let options = ContextOptions {
            max_context_tokens: 100, // Very small for testing
            reserved_output_tokens: 20,
            system_prompt: None,
        };
        let manager = ContextManager::new(options);

        let messages = vec![
            Message::user("Message 1"),
            Message::assistant("Response 1"),
            Message::user("Message 2"),
            Message::assistant("Response 2"),
            Message::user("Message 3"),
            Message::assistant("Response 3"),
        ];

        let truncated = manager.truncate_to_fit(&messages);
        
        // Should have fewer messages due to token limit
        assert!(truncated.len() < messages.len() || 
                ContextManager::estimate_messages_tokens(&truncated) <= 80);
    }

    #[test]
    fn test_format_for_anthropic() {
        let manager = ContextManager::new(ContextOptions::default());
        
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let (formatted, system) = manager.format_for_anthropic(&messages);
        
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0].role, "user");
        assert_eq!(formatted[1].role, "assistant");
        assert!(system.is_none()); // No system prompt set
    }

    #[test]
    fn test_format_for_openai() {
        let manager = ContextManager::new(ContextOptions::default());
        
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let formatted = manager.format_for_openai(&messages);
        
        // Should have 2 messages (user/assistant)
        assert_eq!(formatted.len(), 2);
    }

    #[test]
    fn test_format_for_openai_with_system() {
        let options = ContextOptions {
            system_prompt: Some("You are a helpful assistant.".to_string()),
            ..Default::default()
        };
        let manager = ContextManager::new(options);
        
        let messages = vec![
            Message::user("Hello"),
        ];

        let formatted = manager.format_for_openai(&messages);
        
        // Should have 2 messages (system + user)
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0].role, "system");
    }

    #[test]
    fn test_needs_truncation() {
        let options = ContextOptions {
            max_context_tokens: 100,
            reserved_output_tokens: 20,
            system_prompt: None,
        };
        let manager = ContextManager::new(options);

        // Short messages should not need truncation
        let short = vec![Message::user("Hi")];
        assert!(!manager.needs_truncation(&short));

        // Very long messages might need truncation
        let long_text = "a".repeat(500);
        let long = vec![Message::user(long_text)];
        assert!(manager.needs_truncation(&long));
    }
    
    #[test]
    fn test_token_estimation_chinese() {
        // Chinese characters should use more tokens per character
        let chinese = "你好世界";
        let tokens = ContextManager::estimate_tokens(chinese);
        // 4 Chinese chars * ~2/3 token per char = ~2-3 tokens
        assert!(tokens >= 2 && tokens <= 4, "Expected ~3 tokens for Chinese, got {}", tokens);
    }
    
    #[test]
    fn test_token_estimation_code() {
        // Code content should have different estimation
        let code = "function test() { return 42; }";
        let tokens = ContextManager::estimate_tokens(code);
        // Code tends to compress better, should be reasonable
        assert!(tokens >= 2 && tokens <= 10, "Expected reasonable tokens for code, got {}", tokens);
    }
    
    #[test]
    fn test_message_importance_high() {
        // Decision message should be high importance
        let msg = Message::user("I decided to use Rust for this project");
        let importance = ContextManager::score_message_importance(&msg);
        assert!(importance >= MessageImportance::High, "Decision should be high importance");
    }
    
    #[test]
    fn test_message_importance_preference() {
        // Preference message should be high importance
        let msg = Message::user("I prefer using async/await over threads");
        let importance = ContextManager::score_message_importance(&msg);
        assert!(importance >= MessageImportance::High, "Preference should be high importance");
    }
    
    #[test]
    fn test_message_importance_tool_result() {
        // Tool results are critical
        let mut msg = Message::assistant("");
        msg.role = Role::Tool;
        msg.content = "File content here...".to_string();
        let importance = ContextManager::score_message_importance(&msg);
        assert_eq!(importance, MessageImportance::Critical, "Tool results should be critical");
    }
    
    #[test]
    fn test_message_importance_low() {
        // Short acknowledgment should be low importance
        let msg = Message::assistant("OK");
        let importance = ContextManager::score_message_importance(&msg);
        assert_eq!(importance, MessageImportance::Low, "Short acknowledgment should be low importance");
    }
    
    #[test]
    fn test_truncate_preserves_important() {
        let options = ContextOptions {
            max_context_tokens: 200, // Small budget
            reserved_output_tokens: 40,
            system_prompt: None,
        };
        let manager = ContextManager::new(options);

        // Create messages where some are important
        let messages = vec![
            Message::user("Hello"),  // Low importance
            Message::assistant("Hi"),  // Low importance
            Message::user("I decided to use PostgreSQL for the database"),  // High importance - decision
            Message::assistant("Good choice!"),
            Message::user("What about caching?"),  // Medium importance
            Message::assistant("You should consider Redis for caching. It's fast and reliable."),  // Medium importance
            Message::user("I prefer Redis over Memcached"),  // High importance - preference
            Message::assistant("Understood. Redis is a solid choice."),
        ];

        let truncated = manager.truncate_to_fit(&messages);
        
        // Should keep the last message (current context)
        assert!(!truncated.is_empty(), "Should keep at least one message");
        
        // Important messages with decisions/preferences should be preserved
        let has_decision = truncated.iter().any(|m| m.content.contains("PostgreSQL"));
        let has_preference = truncated.iter().any(|m| m.content.contains("prefer Redis"));
        
        // At least one of the important messages should be preserved
        assert!(has_decision || has_preference, "Should preserve important messages");
    }
}