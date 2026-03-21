//! Configuration schema

use serde::{Deserialize, Serialize};

/// Supported model providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelProvider {
    #[default]
    Anthropic,
    OpenAI,
    Ollama,
}

impl ModelProvider {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelProvider::Anthropic => "Anthropic",
            ModelProvider::OpenAI => "OpenAI",
            ModelProvider::Ollama => "Ollama",
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model provider
    #[serde(default)]
    pub provider: ModelProvider,

    /// Model name
    #[serde(default)]
    pub name: String,

    /// API key (for cloud providers)
    #[serde(default)]
    pub api_key: Option<String>,

    /// API base URL
    #[serde(default)]
    pub base_url: Option<String>,

    /// Max tokens for response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: ModelProvider::Anthropic,
            name: default_model(),
            api_key: None,
            base_url: Some(default_api_base()),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_max_tokens() -> u32 {
    1024
}

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Gateway configuration
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Agent configuration
    #[serde(default)]
    pub agent: AgentConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Models configuration (for multi-model support)
    #[serde(default)]
    pub models: Vec<ModelConfig>,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Bind address (e.g., "127.0.0.1:18789")
    #[serde(default = "default_bind_address")]
    pub bind: String,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose: bool,

    /// Data directory
    #[serde(default)]
    pub data_dir: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind: default_bind_address(),
            verbose: false,
            data_dir: None,
        }
    }
}

fn default_bind_address() -> String {
    "127.0.0.1:18790".to_string()
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,

    /// Model provider
    #[serde(default)]
    pub provider: Option<ModelProvider>,

    /// Model API key
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model API base URL
    #[serde(default = "default_api_base")]
    pub api_base: String,

    /// Workspace directory
    #[serde(default)]
    pub workspace: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: None,
            api_key: None,
            api_base: default_api_base(),
            workspace: None,
        }
    }
}

fn default_model() -> String {
    "anthropic/claude-sonnet-4-20250514".to_string()
}

fn default_api_base() -> String {
    "https://api.anthropic.com".to_string()
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Enable exec tool
    #[serde(default = "default_true")]
    pub exec_enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Load configuration from file
pub fn load_config(path: &std::path::Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

/// Get default config path
pub fn default_config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("tiny_claw").join("config.json"))
}
