//! Plugin HTTP API

use crate::common::error::Result;
use crate::plugins::traits::PluginMetadata;
use serde::{Deserialize, Serialize};
use parking_lot::RwLock;

/// Plugin status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatus {
    pub metadata: PluginMetadata,
    pub enabled: bool,
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Plugin API state
pub struct PluginApi {
    plugins: RwLock<std::collections::HashMap<String, PluginStatus>>,
}

impl PluginApi {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a plugin
    pub fn register(&self, id: String, metadata: PluginMetadata, enabled: bool) {
        let status = PluginStatus {
            metadata,
            enabled,
            loaded_at: Some(chrono::Utc::now()),
        };
        self.plugins.write().insert(id, status);
    }

    /// Unregister a plugin
    pub fn unregister(&self, id: &str) -> Option<PluginStatus> {
        self.plugins.write().remove(id)
    }

    /// Enable a plugin
    pub fn enable(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(id) {
            plugin.enabled = true;
            Ok(())
        } else {
            Err(crate::common::error::Error::Plugin(format!("Plugin {} not found", id)))
        }
    }

    /// Disable a plugin
    pub fn disable(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(id) {
            plugin.enabled = false;
            Ok(())
        } else {
            Err(crate::common::error::Error::Plugin(format!("Plugin {} not found", id)))
        }
    }

    /// Get plugin status
    pub fn get(&self, id: &str) -> Option<PluginStatus> {
        self.plugins.read().get(id).cloned()
    }

    /// List all plugins
    pub fn list(&self) -> Vec<PluginStatus> {
        self.plugins.read().values().cloned().collect()
    }

    /// List enabled plugins
    pub fn list_enabled(&self) -> Vec<PluginStatus> {
        self.plugins.read()
            .values()
            .filter(|p| p.enabled)
            .cloned()
            .collect()
    }
}

impl Default for PluginApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin API request/response types
#[derive(Debug, Deserialize)]
pub struct EnablePluginRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginStatus>,
}

#[derive(Debug, Serialize)]
pub struct PluginResponse {
    pub plugin: PluginStatus,
}
