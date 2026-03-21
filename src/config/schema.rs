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

    /// Retry configuration for API calls
    #[serde(default)]
    pub retry: RetryConfig,

    /// Hot reload configuration
    #[serde(default)]
    pub hot_reload: HotReloadConfig,

    /// Persistence configuration
    #[serde(default)]
    pub persistence: PersistenceConfig,

    /// Graceful shutdown configuration
    #[serde(default)]
    pub shutdown: ShutdownConfig,
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Enable SQLite persistence for session history
    #[serde(default)]
    pub enabled: bool,

    /// Database file path (relative to data_dir or absolute)
    #[serde(default = "default_persistence_path")]
    pub path: String,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: default_persistence_path(),
        }
    }
}

fn default_persistence_path() -> String {
    "tinyclaw.db".to_string()
}

/// Graceful shutdown configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownConfig {
    /// Timeout for draining active connections (seconds)
    #[serde(default = "default_shutdown_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
        }
    }
}

fn default_shutdown_timeout_secs() -> u64 {
    30
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

/// Retry configuration for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Enable exponential backoff
    #[serde(default = "default_true")]
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_backoff: true,
        }
    }
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    30000
}

/// Hot reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Enable config hot reload on file change
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Config file path to watch (default: ~/.config/tiny_claw/config.json)
    #[serde(default)]
    pub watch_path: Option<String>,

    /// Poll interval in milliseconds (for file system events)
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            watch_path: None,
            poll_interval_ms: 5000,
        }
    }
}

fn default_poll_interval_ms() -> u64 {
    5000
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_provider_display_name() {
        assert_eq!(ModelProvider::Anthropic.display_name(), "Anthropic");
        assert_eq!(ModelProvider::OpenAI.display_name(), "OpenAI");
        assert_eq!(ModelProvider::Ollama.display_name(), "Ollama");
    }

    #[test]
    fn test_model_provider_default() {
        let provider = ModelProvider::default();
        assert_eq!(provider, ModelProvider::Anthropic);
    }

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        assert_eq!(config.provider, ModelProvider::Anthropic);
        assert!(config.api_key.is_none());
        assert_eq!(config.max_tokens, 1024);
    }

    #[test]
    fn test_model_config_serialization() {
        let config = ModelConfig {
            provider: ModelProvider::OpenAI,
            name: "gpt-4".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
            max_tokens: 2048,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("openai"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_model_config_deserialization() {
        let json = r#"{"provider": "openai", "name": "gpt-4", "api_key": "sk-test", "max_tokens": 2048}"#;
        let config: ModelConfig = serde_json::from_str(json).unwrap();
        
        assert_eq!(config.provider, ModelProvider::OpenAI);
        assert_eq!(config.name, "gpt-4");
        assert_eq!(config.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert!(config.api_key.is_none());
        assert_eq!(config.model, "anthropic/claude-sonnet-4-20250514");
    }

    #[test]
    fn test_gateway_config_default() {
        let config = GatewayConfig::default();
        assert!(config.bind.contains("127.0.0.1"));
        assert!(!config.verbose);
    }

    #[test]
    fn test_tools_config_default() {
        let config = ToolsConfig { exec_enabled: true };
        assert!(config.exec_enabled);
    }

    #[test]
    fn test_ratelimit_config_default() {
        use crate::ratelimit::limiter::RateLimitConfig;
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 60);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.agent.api_key.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("gateway"));
        assert!(json.contains("agent"));
    }
}
