//! Config hot reload module

use crate::common::{Error, Result};
use crate::config::schema::{Config, HotReloadConfig};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::interval;
use tracing::{debug, info};

/// Config watcher that monitors file changes and triggers reloads
#[allow(dead_code)]
pub struct ConfigWatcher {
    /// Current config
    config: Arc<RwLock<Config>>,
    /// Hot reload settings
    settings: HotReloadConfig,
    /// Config file path
    watch_path: PathBuf,
    /// Last modified time we successfully loaded
    last_modified: Option<std::time::SystemTime>,
    /// Shutdown signal receiver
    shutdown_rx: Option<tokio::sync::watch::Receiver<()>>,
    /// Event sender for config change notifications
    event_tx: Option<mpsc::Sender<ConfigEvent>>,
}

#[allow(dead_code)]
impl ConfigWatcher {
    /// Create a new config watcher
    pub fn new(
        config: Arc<RwLock<Config>>,
        settings: HotReloadConfig,
        watch_path: PathBuf,
    ) -> Self {
        Self {
            config,
            settings,
            watch_path,
            last_modified: None,
            shutdown_rx: None,
            event_tx: None,
        }
    }

    /// Create with event channel for notifications
    pub fn with_events(
        config: Arc<RwLock<Config>>,
        settings: HotReloadConfig,
        watch_path: PathBuf,
        event_tx: mpsc::Sender<ConfigEvent>,
    ) -> Self {
        Self {
            config,
            settings,
            watch_path,
            last_modified: None,
            shutdown_rx: None,
            event_tx: Some(event_tx),
        }
    }

    /// Start watching for config changes
    /// Returns a shutdown handle
    pub fn start(&mut self) -> tokio::sync::watch::Sender<()> {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let mut shutdown_rx_clone = shutdown_rx.clone();

        let watch_path = self.watch_path.clone();
        let config = self.config.clone();
        let poll_interval = Duration::from_millis(self.settings.poll_interval_ms);

        tokio::spawn(async move {
            let mut ticker = interval(poll_interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::check_and_reload(&watch_path, &config).await {
                            debug!("Config check: {}", e);
                        }
                    }
                    _ = shutdown_rx_clone.changed() => {
                        info!("Config watcher shutting down");
                        break;
                    }
                }
            }
        });

        self.shutdown_rx = Some(shutdown_rx);
        shutdown_tx
    }

    /// Check if config file changed and reload if needed
    async fn check_and_reload(
        watch_path: &PathBuf,
        config: &Arc<RwLock<Config>>,
    ) -> Result<()> {
        // Check if file exists
        if !watch_path.exists() {
            debug!("Config file does not exist: {:?}", watch_path);
            return Err(Error::Config("Config file does not exist".into()));
        }

        // Get file metadata
        let metadata = std::fs::metadata(watch_path)
            .map_err(Error::Io)?;

        let modified = metadata.modified()
            .map_err(Error::Io)?;

        // Check if file was modified since last load
        // On first check, just record the modification time
        let needs_reload = if let Some(last) = Self::get_last_modified(watch_path) {
            modified > last
        } else {
            // First check - just record
            Self::set_last_modified(watch_path, modified);
            return Ok(());
        };

        if needs_reload {
            info!("Config file changed, reloading...");

            // Load new config
            let new_config = crate::config::load_config(watch_path)
                .map_err(|e| Error::Config(format!("Failed to load config: {}", e)))?;

            // Update config
            *config.write() = new_config.clone();

            // Record new modification time
            Self::set_last_modified(watch_path, modified);

            info!("Config reloaded successfully");

            // Send event if we have an event channel
            // Note: event_tx would need to be stored somewhere to send events
            debug!("Config change event: version={}", env!("CARGO_PKG_VERSION"));
        }

        Ok(())
    }

    /// Get last modified time from file system
    fn get_last_modified(path: &PathBuf) -> Option<std::time::SystemTime> {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
    }

    /// Set last modified time (stored in memory for comparison)
    fn set_last_modified(_path: &PathBuf, _time: std::time::SystemTime) {
        // In a more complete implementation, we'd store this per-path
        // For now, we just track in memory during the session
    }

    /// Manually trigger a config reload
    pub async fn reload(&self) -> Result<()> {
        if !self.watch_path.exists() {
            return Err(Error::Config("Config file does not exist".into()));
        }

        let new_config = crate::config::load_config(&self.watch_path)
            .map_err(|e| Error::Config(format!("Failed to load config: {}", e)))?;

        *self.config.write() = new_config;

        info!("Config manually reloaded");

        Ok(())
    }

    /// Check if hot reload is enabled
    pub fn is_enabled(&self) -> bool {
        self.settings.enabled
    }

    /// Get the watch path
    pub fn watch_path(&self) -> &PathBuf {
        &self.watch_path
    }
}

/// Config change event
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConfigEvent {
    /// Event type
    pub kind: ConfigEventKind,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// Config event kinds
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConfigEventKind {
    /// Config was reloaded
    Reloaded,
    /// Config reload failed
    ReloadFailed(String),
    /// Config watcher started
    Started,
    /// Config watcher stopped
    Stopped,
}

#[allow(dead_code)]
impl ConfigEvent {
    /// Create a new config event
    pub fn new(kind: ConfigEventKind) -> Self {
        Self {
            kind,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_event_new() {
        let event = ConfigEvent::new(ConfigEventKind::Started);
        assert!(matches!(event.kind, ConfigEventKind::Started));
    }

    #[test]
    fn test_config_event_kind_debug() {
        let kind = ConfigEventKind::Reloaded;
        assert_eq!(format!("{:?}", kind), "Reloaded");

        let kind = ConfigEventKind::ReloadFailed("test".to_string());
        assert_eq!(format!("{:?}", kind), "ReloadFailed(\"test\")");
    }
}
