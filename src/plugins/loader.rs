//! 插件加载器

use crate::common::error::Result;
use crate::plugins::traits::{Hook, Plugin, PluginMetadata};
use async_trait::async_trait;
use std::sync::Arc;

/// 插件加载器
///
/// 负责从文件系统加载插件
pub struct PluginLoader {
    /// 插件目录
    plugin_dir: std::path::PathBuf,
}

impl PluginLoader {
    /// 创建新的插件加载器
    pub fn new(plugin_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            plugin_dir: plugin_dir.into(),
        }
    }

    /// 加载指定目录下的所有插件
    ///
    /// 当前支持加载内置插件
    pub fn load_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let mut plugins = Vec::new();

        // 创建插件目录（如果不存在）
        if !self.plugin_dir.exists() {
            std::fs::create_dir_all(&self.plugin_dir)
                .map_err(|e| crate::common::error::Error::Plugin(format!(
                    "Failed to create plugin directory: {}",
                    e
                )))?;
        }

        // 加载内置插件
        plugins.push(self.load_builtin_echo_plugin()?);
        plugins.push(self.load_builtin_logger_plugin()?);
        plugins.push(self.load_builtin_validator_plugin()?);

        tracing::info!("Loaded {} plugins from {:?}", plugins.len(), self.plugin_dir);
        Ok(plugins)
    }

    /// 加载内置 Echo 插件
    fn load_builtin_echo_plugin(&self) -> Result<Arc<dyn Plugin>> {
        #[derive(Debug)]
        struct EchoPlugin;

        impl Plugin for EchoPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata {
                    id: "builtin.echo".to_string(),
                    name: "Echo Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "A simple plugin that echoes back messages".to_string(),
                    author: "TinyClaw".to_string(),
                }
            }
        }

        Ok(Arc::new(EchoPlugin))
    }

    /// 加载内置 Logger 插件
    fn load_builtin_logger_plugin(&self) -> Result<Arc<dyn Plugin>> {
        #[derive(Debug)]
        struct LoggerPlugin;

        #[async_trait]
        impl Plugin for LoggerPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata {
                    id: "builtin.logger".to_string(),
                    name: "Logger Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Logs all messages and events".to_string(),
                    author: "TinyClaw".to_string(),
                }
            }

            async fn on_hook(&self, hook: Hook) -> Result<Option<serde_json::Value>> {
                match hook {
                    Hook::PreMessage { session_id, message } => {
                        tracing::debug!("[Logger] PreMessage - session: {}, message: {}", session_id, message);
                    }
                    Hook::PostMessage { session_id, message } => {
                        let short_msg = message.chars().take(50).collect::<String>();
                        tracing::debug!("[Logger] PostMessage - session: {}, message: {}", session_id, short_msg);
                    }
                    Hook::SessionCreated { session_id } => {
                        tracing::info!("[Logger] Session created: {}", session_id);
                    }
                    Hook::SessionClosed { session_id } => {
                        tracing::info!("[Logger] Session closed: {}", session_id);
                    }
                    Hook::PreToolExecute { tool_name } => {
                        tracing::debug!("[Logger] PreToolExecute - tool: {}", tool_name);
                    }
                    Hook::PostToolExecute { tool_name, success } => {
                        tracing::debug!("[Logger] PostToolExecute - tool: {}, success: {}", tool_name, success);
                    }
                }
                Ok(None)
            }
        }

        Ok(Arc::new(LoggerPlugin))
    }

    /// 加载内置 Validator 插件
    fn load_builtin_validator_plugin(&self) -> Result<Arc<dyn Plugin>> {
        #[derive(Debug)]
        struct ValidatorPlugin;

        impl Plugin for ValidatorPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata {
                    id: "builtin.validator".to_string(),
                    name: "Validator Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Validates messages and parameters".to_string(),
                    author: "TinyClaw".to_string(),
                }
            }
        }

        Ok(Arc::new(ValidatorPlugin))
    }
}
