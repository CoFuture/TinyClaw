//! 插件管理器

use crate::common::error::Error;
use crate::common::error::Result;
use crate::plugins::traits::{Hook, Plugin, PluginMetadata};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// 插件管理器
///
/// 负责插件的加载、卸载和调用
pub struct PluginManager {
    /// 已加载的插件
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// 注册插件
    pub fn register(&self, plugin: Arc<dyn Plugin>) -> Result<()> {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();
        let name = metadata.name.clone();
        let version = metadata.version.clone();

        // 初始化插件
        let plugin_clone = Arc::clone(&plugin);
        tokio::runtime::Handle::current()
            .block_on(async {
                plugin_clone.init().await
            })
            .map_err(|e| Error::Plugin(format!(
                "Failed to initialize plugin {}: {}",
                id, e
            )))?;

        // 注册插件
        self.plugins.write().insert(id, plugin);

        tracing::info!("Plugin registered: {} v{}", name, version);
        Ok(())
    }

    /// 卸载插件
    pub fn unregister(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write();

        if let Some(plugin) = plugins.remove(plugin_id) {
            let plugin_clone = plugin;
            tokio::runtime::Handle::current()
                .block_on(async {
                    plugin_clone.shutdown().await
                })
                .map_err(|e| Error::Plugin(format!(
                    "Failed to shutdown plugin {}: {}",
                    plugin_id, e
                )))?;

            tracing::info!("Plugin unregistered: {}", plugin_id);
        }

        Ok(())
    }

    /// 获取插件
    pub fn get_plugin(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().get(plugin_id).cloned()
    }

    /// 获取所有插件元数据
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.read()
            .values()
            .map(|p| p.metadata())
            .collect()
    }

    /// 触发钩子
    pub async fn trigger_hook(&self, hook: Hook) -> Result<Option<serde_json::Value>> {
        let plugins: Vec<Arc<dyn Plugin>> = self.plugins.read().values().cloned().collect();

        let mut result: Option<serde_json::Value> = None;

        for plugin in plugins {
            match plugin.on_hook(hook.clone()).await {
                Ok(Some(value)) => {
                    result = Some(value);
                }
                Err(e) => {
                    tracing::warn!("Plugin hook error: {}", e);
                }
                _ => {}
            }
        }

        Ok(result)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
