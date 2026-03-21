//! AI Model client with multi-provider support

use crate::agent::tools::{Tool, ToolExecutor, ToolResult};
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
    tool_executor: Arc<ToolExecutor>,
}

impl Agent {
    pub fn new(config: Arc<RwLock<AgentConfig>>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            tool_executor: Arc::new(ToolExecutor::new()),
        }
    }

    /// Get list of available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_executor.list_tools()
    }

    /// Send a message and get a response (with tool calling loop)
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

        // Build messages with initial user message
        let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "user",
            "content": message
        })];

        // Maximum tool call iterations
        let max_turns = 10;
        let tools = self.tool_executor.list_tools();

        for turn in 0..max_turns {
            debug!("Tool loop turn {}", turn + 1);

            match provider {
                ModelProvider::Anthropic => {
                    let response = self.send_anthropic_with_tools(&config, &messages, &tools).await?;
                    
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
                        .and_then(|arr| {
                            Some(arr.iter().filter_map(|c| c.get("tool_use").cloned()).collect::<Vec<_>>())
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
                    let response = self.send_openai_with_tools(&config, &messages, &tools).await?;
                    
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
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

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
            "system": "You are TinyClaw, a helpful AI assistant. You have access to tools to help you answer questions.",
            "tools": anthropic_tools
        });

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

        // Parse response as JSON value
        let response: serde_json::Value = response.json().await?;

        debug!("Anthropic tool response received");

        Ok(response)
    }

    /// Send message to OpenAI API with tools
    async fn send_openai_with_tools(
        &self,
        config: &AgentConfig,
        messages: &[serde_json::Value],
        tools: &[Tool],
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

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
            "messages": messages,
            "max_tokens": 1024,
            "tools": openai_tools
        });

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

        let response: serde_json::Value = response.json().await?;

        debug!("OpenAI tool response received");

        Ok(response)
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
