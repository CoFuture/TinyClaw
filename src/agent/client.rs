//! AI Model client with multi-provider support

use crate::agent::retry::{with_retry, RetrySettings};
use crate::agent::tools::{Tool, ToolExecutor};
use crate::common::{Error, Result};
use crate::config::{AgentConfig, ModelProvider};
use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, info};

/// Anthropic API request
#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Anthropic API message
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
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
#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
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
#[derive(Debug, Serialize, Clone)]
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

/// Ollama streaming SSE response line
#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    /// Text chunk
    response: String,
    /// Whether this is the final chunk
    done: bool,
}

/// Agent client for AI model interaction
pub struct Agent {
    config: Arc<RwLock<AgentConfig>>,
    retry_config: RetrySettings,
    http_client: reqwest::Client,
    tool_executor: Arc<ToolExecutor>,
    /// Active turn cancellation channels (session_id -> sender)
    turn_cancellations: RwLock<HashMap<String, broadcast::Sender<()>>>,
}

/// History message for passing conversation context to API methods
#[derive(Debug, Clone)]
struct HistoryMessage {
    role: String,
    content: String,
}

impl From<&(String, String)> for HistoryMessage {
    fn from(tuple: &(String, String)) -> Self {
        Self {
            role: tuple.0.clone(),
            content: tuple.1.clone(),
        }
    }
}

impl From<&crate::types::Message> for HistoryMessage {
    fn from(msg: &crate::types::Message) -> Self {
        match msg.role {
            crate::types::Role::User => Self {
                role: "user".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::Assistant => Self {
                role: "assistant".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::System => Self {
                role: "system".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::Tool => {
                // Tool messages: encode tool_call_id and tool_name in content
                // Format: "[TOOL:tool_name:id] content"
                let name = msg.tool_name.as_deref().unwrap_or("unknown");
                let id = msg.tool_call_id.as_deref().unwrap_or("unknown");
                let encoded = format!("[TOOL:{}:{}] {}", name, id, msg.content);
                Self {
                    role: "user".to_string(), // Tool results as user role
                    content: encoded,
                }
            }
        }
    }
}

impl HistoryMessage {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "role": self.role,
            "content": self.content
        })
    }
}

impl Agent {
    pub fn new(config: Arc<RwLock<AgentConfig>>) -> Self {
        Self {
            config,
            retry_config: RetrySettings::default(),
            http_client: reqwest::Client::new(),
            tool_executor: Arc::new(ToolExecutor::new()),
            turn_cancellations: RwLock::new(HashMap::new()),
        }
    }

    /// Create Agent with custom retry settings
    #[allow(dead_code)]
    pub fn with_retry(config: Arc<RwLock<AgentConfig>>, retry: RetrySettings) -> Self {
        Self {
            config,
            retry_config: retry,
            http_client: reqwest::Client::new(),
            tool_executor: Arc::new(ToolExecutor::new()),
            turn_cancellations: RwLock::new(HashMap::new()),
        }
    }

    /// Start a new turn cancellation channel for a session.
    /// Returns a receiver that will be notified when cancel is called.
    /// If a turn is already active, returns None.
    pub fn start_turn_cancellation(&self, session_key: &str) -> Option<broadcast::Receiver<()>> {
        let mut cancellations = self.turn_cancellations.write();
        // Check if a turn is already active
        if cancellations.contains_key(session_key) {
            return None;
        }
        let (tx, rx) = broadcast::channel(1);
        cancellations.insert(session_key.to_string(), tx);
        Some(rx)
    }

    /// Cancel an ongoing turn for a session.
    /// Returns true if a turn was cancelled, false if no turn was active.
    pub fn cancel_turn(&self, session_key: &str) -> bool {
        let mut cancellations = self.turn_cancellations.write();
        if let Some(tx) = cancellations.remove(session_key) {
            // Drop the sender to notify the receiver
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    /// Check if a turn is currently active for a session.
    pub fn is_turn_active(&self, session_key: &str) -> bool {
        let cancellations = self.turn_cancellations.read();
        cancellations.contains_key(session_key)
    }

    /// Clean up cancellation channel after turn ends.
    fn cleanup_turn_cancellation(&self, session_key: &str) {
        let mut cancellations = self.turn_cancellations.write();
        cancellations.remove(session_key);
    }

    /// Update retry settings
    #[allow(dead_code)]
    pub fn set_retry_settings(&mut self, retry: RetrySettings) {
        self.retry_config = retry;
    }

    /// Get list of available tools
    #[allow(dead_code)]
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_executor.list_tools()
    }

    /// Send a message with conversation history and get a response (with tool calling loop)
///
/// # Arguments
/// * `session_key` - Session identifier (unused, kept for API compatibility)
/// * `message` - User message to send
/// * `history` - Conversation history to prepend (role, content tuples)
/// * `skill_prompt` - Optional skill instructions to inject into system prompt
pub async fn send_message_with_history(
    &self,
    _session_key: &str,
    message: &str,
    history: &[(String, String)],
    skill_prompt: Option<&str>,
) -> Result<String> {
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

        // Build messages: prepend history, then add current user message
        let mut messages: Vec<serde_json::Value> = history
            .iter()
            .map(HistoryMessage::from)
            .map(|h| h.to_json())
            .collect();
        
        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        // Maximum tool call iterations
        let max_turns = 10;
        let tools = self.tool_executor.list_tools();

        for turn in 0..max_turns {
            debug!("Tool loop turn {}", turn + 1);

            match provider {
                ModelProvider::Anthropic => {
                    let response = self.send_anthropic_with_tools(&config, &messages, &tools, skill_prompt).await?;
                    
                    // Extract text content
                    let text_content: Option<String> = response["content"]
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .filter_map(|c| c.get("text").and_then(|t| t.as_str().map(String::from)))
                                .next()
                        });

                    // Check for tool_use in response
                    let tool_calls = response["content"]
                        .as_array()
                        .map(|arr| {
                            arr.iter().filter_map(|c| c.get("tool_use").cloned()).collect::<Vec<_>>()
                        })
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);

                    if tool_calls {
                        // Get tool_use blocks
                        let tool_use_blocks = response["content"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|c| c.get("tool_use").is_some())
                            .filter_map(|c| c.get("tool_use").cloned())
                            .collect::<Vec<_>>();

                        // Add assistant message with tool use
                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": tool_use_blocks
                        }));

                        // Execute tools and add results
                        for tool_block in &tool_use_blocks {
                            let tool_name = tool_block["name"].as_str().unwrap_or("");
                            let tool_input = tool_block["input"].clone();
                            let result = self.tool_executor.execute(tool_name, tool_input).await;
                            let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "content": result_json,
                                "tool_use_id": tool_block["id"]
                            }));
                        }
                    } else {
                        // No tool calls, return the text response
                        return Ok(text_content.unwrap_or_default());
                    }
                }
                ModelProvider::OpenAI => {
                    let response = self.send_openai_with_tools(&config, &messages, &tools, skill_prompt).await?;
                    
                    // Check for tool_calls in response
                    let has_tool_calls = response["choices"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("tool_calls"))
                        .is_some();

                    if has_tool_calls {
                        let choices = response["choices"].as_array().unwrap();
                        let message_obj = &choices[0]["message"];
                        
                        // Add assistant message
                        messages.push(message_obj.clone());

                        let tool_calls = message_obj["tool_calls"].as_array().unwrap();
                        
                        // Execute tools and add results
                        for tool_call in tool_calls {
                            let tool_name = tool_call["function"]["name"].as_str().unwrap_or("");
                            let args: serde_json::Value = serde_json::from_str(
                                tool_call["function"]["arguments"].as_str().unwrap_or("{}")
                            ).unwrap_or(serde_json::json!({}));
                            let result = self.tool_executor.execute(tool_name, args).await;
                            let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "content": result_json,
                                "tool_call_id": tool_call["id"]
                            }));
                        }
                    } else {
                        // No tool calls, return the text response
                        let text = response["choices"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|c| c.get("message"))
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                            .map(String::from)
                            .unwrap_or_default();
                        return Ok(text);
                    }
                }
                ModelProvider::Ollama => {
                    // Ollama doesn't support tools, just return direct response
                    return self.send_ollama(&config, message).await;
                }
            }
        }

        Ok("Max tool call iterations reached".to_string())
    }

    /// Send message to Anthropic API with tools
    async fn send_anthropic_with_tools(
        &self,
        config: &AgentConfig,
        messages: &[serde_json::Value],
        tools: &[Tool],
        skill_prompt: Option<&str>,
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // Build base system prompt
        let base_system = "You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You have access to tools to help you answer questions. You should introduce yourself as TinyClaw powered by GLM-5 when asked.";
        let system = if let Some(skills) = skill_prompt {
            format!("{}\n\n{}", base_system, skills)
        } else {
            base_system.to_string()
        };

        // Convert tools to Anthropic format
        let anthropic_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema
                })
            })
            .collect();

        // Build request with tools
        let request = serde_json::json!({
            "model": model,
            "max_tokens": 1024,
            "messages": messages,
            "system": system,
            "tools": anthropic_tools
        });

        // Use retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = with_retry(&self.retry_config, || {
            let http_client = http_client.clone();
            let api_base = api_base.clone();
            let api_key = api_key.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/messages", api_base))
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        // Parse response as JSON value
        let response: serde_json::Value = result.json().await?;

        debug!("Anthropic tool response received");

        Ok(response)
    }

    /// Send message to OpenAI API with tools
    async fn send_openai_with_tools(
        &self,
        config: &AgentConfig,
        messages: &[serde_json::Value],
        tools: &[Tool],
        skill_prompt: Option<&str>,
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // Build system message with skill context
        let base_system = "You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You have access to tools to help you answer questions. You should introduce yourself as TinyClaw powered by GLM-5 when asked.";
        let system_content = if let Some(skills) = skill_prompt {
            format!("{}\n\n{}", base_system, skills)
        } else {
            base_system.to_string()
        };

        // Build messages with system prompt prepended
        let mut all_messages = vec![serde_json::json!({
            "role": "system",
            "content": system_content
        })];
        all_messages.extend(messages.iter().cloned());

        // Convert tools to OpenAI format
        let openai_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    }
                })
            })
            .collect();

        // Build request with tools
        let request = serde_json::json!({
            "model": model,
            "messages": all_messages,
            "max_tokens": 1024,
            "tools": openai_tools
        });

        // Determine API version path based on model
        // GLM models use v4, others use v1
        let api_version = if model.starts_with("glm") {
            "v4"
        } else {
            "v1"
        };

        let api_base_clone = api_base.clone();
        let api_version_clone = api_version.to_string();
        let api_key_clone = api_key.clone();

        // Use retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = with_retry(&self.retry_config, || {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_version_clone = api_version_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/{}/chat/completions", api_base_clone, api_version_clone))
                    .header("Authorization", format!("Bearer {}", api_key_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: serde_json::Value = result.json().await?;

        debug!("OpenAI tool response received");

        Ok(response)
    }

    /// Send message to Anthropic API
    #[allow(dead_code)]
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
            system: Some("You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You should introduce yourself as TinyClaw powered by GLM-5 when asked.".to_string()),
        };

        // Use retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let api_base_clone = api_base.clone();
        let api_key_clone = api_key.clone();
        let response = with_retry(&self.retry_config, || {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/messages", api_base_clone))
                    .header("x-api-key", api_key_clone)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

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
    #[allow(dead_code)]
    async fn send_openai(&self, config: &AgentConfig, message: &str) -> Result<String> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to OpenAI {}: {}", model, message);

        let request = OpenAIRequest {
            model: model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(1024),
        };

        // Determine API version path based on model
        let api_version = if model.starts_with("glm") {
            "v4"
        } else {
            "v1"
        };

        let api_base_clone = api_base.clone();
        let api_version_clone = api_version.to_string();
        let api_key_clone = api_key.clone();

        // Use retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = with_retry(&self.retry_config, || {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_version_clone = api_version_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/{}/chat/completions", api_base_clone, api_version_clone))
                    .header("Authorization", format!("Bearer {}", api_key_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OpenAIResponse = result.json().await?;

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

        let base_url_clone = base_url.clone();

        // Use retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = with_retry(&self.retry_config, || {
            let http_client = http_client.clone();
            let base_url_clone = base_url_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/api/generate", base_url_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OllamaResponse = result.json().await?;

        debug!("Ollama response: {}", response.response);

        Ok(response.response)
    }

    /// Send message to Ollama API with streaming support.
    /// Calls the callback for each text chunk as it arrives via SSE.
    async fn send_ollama_streaming<F>(&self, config: &AgentConfig, message: &str, mut on_chunk: F, mut cancel_rx: broadcast::Receiver<()>) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // For Ollama, default to localhost if no base URL
        let base_url = if api_base.is_empty() || api_base == "https://api.anthropic.com" {
            "http://localhost:11434".to_string()
        } else {
            api_base
        };

        info!("Sending streaming message to Ollama {}: {}", model, message);

        let request = serde_json::json!({
            "model": model,
            "prompt": message,
            "stream": true
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/generate", base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Ollama streaming error: {} - {}", status, body);
            return Err(Error::Agent(format!("Ollama error: {} - {}", status, body)));
        }

        // Read SSE lines and process each chunk
        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = Vec::new();
        let check_interval = Duration::from_millis(100);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        // Spawn a task to watch for cancellation
        let cancelled_flag = cancelled_clone;
        tokio::spawn(async move {
            let _ = cancel_rx.recv().await;
            cancelled_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        
        while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            // Use timeout to periodically check for cancellation
            let result = timeout(check_interval, stream.next()).await;
            
            match result {
                Ok(Some(chunk_result)) => {
                    let chunk = match chunk_result {
                        Ok(c) => c,
                        Err(e) => {
                            debug!("Stream error: {}", e);
                            break;
                        }
                    };
                    buffer.extend_from_slice(&chunk);
                    
                    // Process complete lines (each SSE line starts with "data: ")
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line_str = String::from_utf8_lossy(&line);
                        let line_str = line_str.trim();
                        
                        if let Some(json_str) = line_str.strip_prefix("data: ") {
                            if let Ok(stream_resp) = serde_json::from_str::<OllamaStreamResponse>(json_str) {
                                if !stream_resp.response.is_empty() {
                                    full_response.push_str(&stream_resp.response);
                                    on_chunk(stream_resp.response);
                                }
                                if stream_resp.done {
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended
                    break;
                }
                Err(_) => {
                    // Timeout - check if cancelled and continue
                    continue;
                }
            }
        }

        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            debug!("Ollama streaming cancelled: {} chars accumulated", full_response.len());
            return Err(Error::Cancelled);
        }

        debug!("Ollama streaming complete: {} chars", full_response.len());
        Ok(full_response)
    }

    /// Send a message with streaming partial text support.
    /// Calls the callback for each text chunk as it arrives (for streaming providers like Ollama).
    /// 
    /// # Arguments
    /// * `session_key` - Session identifier
    /// * `message` - User message to send
    /// * `history` - Conversation history
    /// * `skill_prompt` - Optional skill instructions
    /// * `on_partial` - Callback invoked with each text chunk as it arrives
    pub async fn send_message_streaming(
        &self,
        session_key: &str,
        message: &str,
        history: &[(String, String)],
        skill_prompt: Option<&str>,
        on_partial: impl FnMut(String) + Send + 'static,
    ) -> Result<String> {
        // Start cancellation tracking for this turn
        let cancel_rx = match self.start_turn_cancellation(session_key) {
            Some(rx) => rx,
            None => {
                return Err(Error::Agent(format!(
                    "A turn is already in progress for session '{}'",
                    session_key
                )));
            }
        };

        let config = self.config.read().clone();

        // Determine provider
        let provider = config.provider.clone().unwrap_or_else(|| {
            if config.model.starts_with("claude-") || config.model.starts_with("anthropic/") {
                ModelProvider::Anthropic
            } else if config.model.starts_with("gpt-") || config.model.starts_with("openai/") {
                ModelProvider::OpenAI
            } else {
                ModelProvider::Ollama
            }
        });

        // For non-streaming providers (Anthropic, OpenAI), fall back to regular method
        // Ollama supports native streaming
        let result = if provider == ModelProvider::Ollama {
            // Build prompt from history + current message
            let mut prompt = String::new();
            for (role, content) in history {
                prompt.push_str(&format!("{}: {}\n", role, content));
            }
            prompt.push_str(&format!("user: {}\nassistant:", message));
            
            self.send_ollama_streaming(&config, &prompt, on_partial, cancel_rx).await
        } else {
            // Fall back to non-streaming for other providers
            // Release the cancellation channel since we won't use it for non-streaming
            self.cleanup_turn_cancellation(session_key);
            self.send_message_with_history(session_key, message, history, skill_prompt).await
        };

        // Clean up cancellation channel after turn ends
        self.cleanup_turn_cancellation(session_key);
        result
    }

    /// Send a message and get a response (with tool calling loop)
    /// For backward compatibility: uses empty history. Use send_message_with_history for context.
    ///
    /// # Arguments
    /// * `session_key` - Session identifier (unused, kept for API compatibility)
    /// * `message` - User message to send
    /// * `skill_prompt` - Optional skill instructions to inject into system prompt
    pub async fn send_message(&self, session_key: &str, message: &str, skill_prompt: Option<&str>) -> Result<String> {
        self.send_message_with_history(session_key, message, &[], skill_prompt).await
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
