//! AI Model client with multi-provider support

use crate::common::{Error, Result};
use crate::config::{AgentConfig, ModelProvider};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Anthropic API request
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Anthropic API message
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

/// Anthropic API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    #[serde(rename = "usage")]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    #[serde(rename = "input_tokens")]
    input_tokens: u32,
    #[serde(rename = "output_tokens")]
    output_tokens: u32,
}

/// OpenAI API request
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// OpenAI API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    completion_tokens: u32,
}

/// Ollama API request
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Ollama API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    response: String,
}

/// Agent client for AI model interaction
pub struct Agent {
    config: Arc<RwLock<AgentConfig>>,
    http_client: reqwest::Client,
}

impl Agent {
    pub fn new(config: Arc<RwLock<AgentConfig>>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Send a message and get a response
    pub async fn send_message(&self, _session_key: &str, message: &str) -> Result<String> {
        let config = self.config.read().clone();

        // Determine provider based on model name or explicit config
        let provider = config.provider.clone().unwrap_or_else(|| {
            if config.model.starts_with("claude-") || config.model.starts_with("anthropic/") {
                ModelProvider::Anthropic
            } else if config.model.starts_with("gpt-") || config.model.starts_with("openai/") {
                ModelProvider::OpenAI
            } else {
                ModelProvider::Ollama
            }
        });

        match provider {
            ModelProvider::Anthropic => self.send_anthropic(&config, message).await,
            ModelProvider::OpenAI => self.send_openai(&config, message).await,
            ModelProvider::Ollama => self.send_ollama(&config, message).await,
        }
    }

    /// Send message to Anthropic API
    async fn send_anthropic(&self, config: &AgentConfig, message: &str) -> Result<String> {
        // Check if API key is set
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to Anthropic {}: {}", model, message);

        // Build request
        let request = AnthropicRequest {
            model,
            max_tokens: 1024,
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            system: Some("You are TinyClaw, a helpful AI assistant.".to_string()),
        };

        // Send request
        let response = self
            .http_client
            .post(format!("{}/v1/messages", api_base))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        // Parse response
        let response: AnthropicResponse = response.json().await?;

        // Extract text from response
        let text = response
            .content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("");

        debug!("Anthropic response: {}", text);

        Ok(text)
    }

    /// Send message to OpenAI API
    async fn send_openai(&self, config: &AgentConfig, message: &str) -> Result<String> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to OpenAI {}: {}", model, message);

        let request = OpenAIRequest {
            model,
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(1024),
        };

        let response = self
            .http_client
            .post(format!("{}/v1/chat/completions", api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OpenAIResponse = response.json().await?;

        let text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        debug!("OpenAI response: {}", text);

        Ok(text)
    }

    /// Send message to Ollama API
    async fn send_ollama(&self, config: &AgentConfig, message: &str) -> Result<String> {
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // For Ollama, default to localhost if no base URL
        let base_url = if api_base.is_empty() || api_base == "https://api.anthropic.com" {
            "http://localhost:11434".to_string()
        } else {
            api_base
        };

        info!("Sending message to Ollama {}: {}", model, message);

        let request = OllamaRequest {
            model,
            prompt: message.to_string(),
            stream: false,
        };

        let response = self
            .http_client
            .post(format!("{}/api/generate", base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OllamaResponse = response.json().await?;

        debug!("Ollama response: {}", response.response);

        Ok(response.response)
    }

    /// Send a message with conversation history
    #[allow(dead_code)]
    pub async fn send_message_with_history(
        &self,
        _session_key: &str,
        message: &str,
        _history: &[(String, String)],
    ) -> Result<String> {
        // For now, just send the message without history
        // History management will be added in later iterations
        self.send_message(_session_key, message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelConfig;

    #[test]
    fn test_model_provider_display_name() {
        assert_eq!(ModelProvider::Anthropic.display_name(), "Anthropic");
        assert_eq!(ModelProvider::OpenAI.display_name(), "OpenAI");
        assert_eq!(ModelProvider::Ollama.display_name(), "Ollama");
    }

    #[test]
    fn test_agent_default_config() {
        let agent_config = AgentConfig::default();
        assert_eq!(agent_config.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(agent_config.api_base, "https://api.anthropic.com");
        assert!(agent_config.provider.is_none());
    }

    #[test]
    fn test_model_config_default() {
        let model_config = ModelConfig::default();
        assert_eq!(model_config.provider, ModelProvider::Anthropic);
        assert_eq!(model_config.max_tokens, 1024);
    }
}
