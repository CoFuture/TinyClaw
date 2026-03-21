//! Config hot reload module with proper file tracking and event emission

use crate::common::{Error, Result};
use crate::config::schema::{Config, HotReloadConfig};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::interval;
use tracing::{debug, error, info};

/// Config watcher that monitors file changes and triggers reloads
#[allow(dead_code)]
pub struct ConfigWatcher {
    /// Current config
    config: Arc<RwLock<Config>>,
    /// Hot reload settings
    settings: HotReloadConfig,
    /// Config file path
    watch_path: PathBuf,
    /// Last modified times per path (proper tracking)
    last_modified_map: RwLock<HashMap<PathBuf, SystemTime>>,
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
            last_modified_map: RwLock::new(HashMap::new()),
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
            last_modified_map: RwLock::new(HashMap::new()),
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
        let config = Arc::clone(&self.config);
        let last_modified_map = Arc::new(RwLock::new(HashMap::new()));
        let event_tx = self.event_tx.clone();
        let poll_interval = Duration::from_millis(self.settings.poll_interval_ms);

        // Initialize last modified time
        if watch_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&watch_path) {
                if let Ok(modified) = metadata.modified() {
                    last_modified_map.write().insert(watch_path.clone(), modified);
                }
            }
        }

        let last_modified_map_clone = Arc::clone(&last_modified_map);

        tokio::spawn(async move {
            let mut ticker = interval(poll_interval);

            // Send started event
            if let Some(ref tx) = event_tx {
                let event = ConfigEvent::new(ConfigEventKind::Started);
                let _ = tx.send(event).await;
            }

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::check_and_reload(
                            &watch_path,
                            &config,
                            &last_modified_map_clone,
                            &event_tx,
                        ).await {
                            debug!("Config check: {}", e);
                        }
                    }
                    _ = shutdown_rx_clone.changed() => {
                        info!("Config watcher shutting down");

                        // Send stopped event
                        if let Some(ref tx) = event_tx {
                            let event = ConfigEvent::new(ConfigEventKind::Stopped);
                            let _ = tx.send(event).await;
                        }
                        break;
                    }
                }
            }
        });

        self.shutdown_rx = Some(shutdown_rx);
        shutdown_tx
    }

    /// Validate config on reload
    fn validate_config(config: &Config) -> Result<()> {
        // Validate bind address format
        if config.gateway.bind.is_empty() {
            return Err(Error::Config("Gateway bind address cannot be empty".into()));
        }

        // Validate model config
        if config.agent.model.is_empty() {
            return Err(Error::Config("Agent model cannot be empty".into()));
        }

        // Validate retry settings
        if config.retry.max_retries > 10 {
            return Err(Error::Config("Max retries cannot exceed 10".into()));
        }

        if config.retry.initial_delay_ms > config.retry.max_delay_ms {
            return Err(Error::Config("Initial delay cannot exceed max delay".into()));
        }

        // Validate hot reload settings
        if config.hot_reload.poll_interval_ms < 1000 {
            return Err(Error::Config("Poll interval must be at least 1000ms".into()));
        }

        Ok(())
    }

    /// Check if config file changed and reload if needed
    async fn check_and_reload(
        watch_path: &PathBuf,
        config: &Arc<RwLock<Config>>,
        last_modified_map: &Arc<RwLock<HashMap<PathBuf, SystemTime>>>,
        event_tx: &Option<mpsc::Sender<ConfigEvent>>,
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
        let needs_reload = {
            let map = last_modified_map.read();
            match map.get(watch_path) {
                Some(last) => modified > *last,
                None => {
                    // First check - just record and return
                    drop(map);
                    last_modified_map.write().insert(watch_path.clone(), modified);
                    return Ok(());
                }
            }
        };

        if needs_reload {
            info!("Config file changed, reloading...");

            // Load new config
            let new_config = match crate::config::load_config(watch_path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!("Failed to load config: {}", e);

                    // Send reload failed event
                    if let Some(ref tx) = event_tx {
                        let event = ConfigEvent::new(ConfigEventKind::ReloadFailed(e.to_string()));
                        let _ = tx.send(event).await;
                    }

                    return Err(Error::Config(format!("Failed to load config: {}", e)));
                }
            };

            // Validate new config
            if let Err(e) = Self::validate_config(&new_config) {
                error!("Config validation failed: {}", e);

                if let Some(ref tx) = event_tx {
                    let event = ConfigEvent::new(ConfigEventKind::ReloadFailed(e.to_string()));
                    let _ = tx.send(event).await;
                }

                return Err(e);
            }

            // Update config
            *config.write() = new_config.clone();

            // Record new modification time
            last_modified_map.write().insert(watch_path.clone(), modified);

            info!("Config reloaded successfully");

            // Send reloaded event
            if let Some(ref tx) = event_tx {
                let event = ConfigEvent::new(ConfigEventKind::Reloaded);
                let _ = tx.send(event).await;
            }
        }

        Ok(())
    }

    /// Manually trigger a config reload
    pub async fn reload(&self) -> Result<()> {
        if !self.watch_path.exists() {
            return Err(Error::Config("Config file does not exist".into()));
        }

        let new_config = crate::config::load_config(&self.watch_path)
            .map_err(|e| Error::Config(format!("Failed to load config: {}", e)))?;

        // Validate
        Self::validate_config(&new_config)?;

        *self.config.write() = new_config;

        // Update last modified
        if let Ok(metadata) = std::fs::metadata(&self.watch_path) {
            if let Ok(modified) = metadata.modified() {
                self.last_modified_map.write().insert(self.watch_path.clone(), modified);
            }
        }

        info!("Config manually reloaded");

        // Send event
        if let Some(ref tx) = self.event_tx {
            let event = ConfigEvent::new(ConfigEventKind::Reloaded);
            let _ = tx.send(event).await;
        }

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

    /// Get event sender for subscription
    #[allow(dead_code)]
    pub fn event_sender(&self) -> &Option<mpsc::Sender<ConfigEvent>> {
        &self.event_tx
    }
}

/// Config change event
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConfigEvent {
    /// Event type
    pub kind: ConfigEventKind,
    /// Timestamp
    pub timestamp: SystemTime,
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

impl ConfigEvent {
    /// Create a new config event
    pub fn new(kind: ConfigEventKind) -> Self {
        Self {
            kind,
            timestamp: SystemTime::now(),
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

    #[test]
    fn test_validate_config_empty_bind() {
        let mut config = Config::default();
        config.gateway.bind = "".to_string();
        let result = ConfigWatcher::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_empty_model() {
        let mut config = Config::default();
        config.agent.model = "".to_string();
        let result = ConfigWatcher::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_bad_retry() {
        let mut config = Config::default();
        config.retry.max_retries = 100; // Too high
        let result = ConfigWatcher::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_good() {
        let config = Config::default();
        let result = ConfigWatcher::validate_config(&config);
        assert!(result.is_ok());
    }
}
