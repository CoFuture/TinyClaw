//! AI Model client

use crate::common::{Error, Result};
use crate::config::AgentConfig;
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
#[derive(Debug, Serialize, Deserialize)]
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

        // Check if API key is set
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to {}: {}", model, message);

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

        debug!("Response: {}", text);

        Ok(text)
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
