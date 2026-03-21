//! Context Manager Module
//!
//! Manages conversation context for AI models including:
//! - Token estimation
//! - Context formatting for different providers
//! - Context truncation when approaching limits

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

/// Context manager for formatting and truncating conversation history
#[derive(Debug, Clone)]
pub struct ContextManager {
    options: ContextOptions,
}

impl ContextManager {
    pub fn new(options: ContextOptions) -> Self {
        Self { options }
    }

    /// Estimate tokens from text (rough approximation: 4 chars ≈ 1 token)
    pub fn estimate_tokens(text: &str) -> usize {
        // Basic estimation: ~4 characters per token for English
        // This is a rough approximation; actual tokenizers vary
        text.chars().count() / 4
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

    /// Truncate messages to fit within token budget
    /// Uses a simple strategy: keep recent messages, drop oldest from the middle
    pub fn truncate_to_fit(&self, messages: &[Message]) -> Vec<Message> {
        let max_input = self.options.max_context_tokens - self.options.reserved_output_tokens;
        
        // First pass: check if we need truncation
        let total_tokens = Self::estimate_messages_tokens(messages);
        if total_tokens <= max_input {
            return messages.to_vec();
        }

        // Strategy: Keep system messages, user messages at the start, and recent assistant messages
        // Drop oldest messages from the middle
        
        let mut result = Vec::new();
        
        // Keep system messages
        let system_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role == Role::System)
            .cloned()
            .collect();
        
        // Get user/assistant messages
        let conv_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role != Role::System)
            .cloned()
            .collect();

        // Start from the most recent and work backwards
        // Keep at least the last N messages
        let keep_recent = 10;
        let mut tokens_so_far = 0;
        
        // Add recent messages first (they're most important)
        for msg in conv_msgs.iter().rev().take(keep_recent) {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_so_far + msg_tokens <= max_input {
                result.insert(0, msg.clone());
                tokens_so_far += msg_tokens;
            } else {
                break;
            }
        }

        // Add older messages if we have room
        for msg in conv_msgs.iter().rev().skip(keep_recent) {
            let msg_tokens = Self::estimate_message_tokens(msg);
            if tokens_so_far + msg_tokens <= max_input {
                result.insert(0, msg.clone());
                tokens_so_far += msg_tokens;
            }
            // If no room, just skip this message (drop it)
        }

        // Prepend system messages
        result = system_msgs.into_iter().chain(result).collect();

        result
    }

    /// Check if context needs truncation
    pub fn needs_truncation(&self, messages: &[Message]) -> bool {
        let max_input = self.options.max_context_tokens - self.options.reserved_output_tokens;
        Self::estimate_messages_tokens(messages) > max_input
    }
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
}