//! Configuration schema

use serde::{Deserialize, Serialize};

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
    "127.0.0.1:18789".to_string()
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,

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
