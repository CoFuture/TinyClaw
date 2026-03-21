//! History manager with optional SQLite persistence

use crate::persistence::sqlite::SqliteStore;
use crate::types::{Message, SessionHistory};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// History manager with optional SQLite persistence
pub struct HistoryManager {
    histories: RwLock<HashMap<String, Arc<RwLock<SessionHistory>>>>,
    sqlite_store: Option<SqliteStore>,
}

impl HistoryManager {
    /// Create a new in-memory history manager
    pub fn new() -> Self {
        Self {
            histories: RwLock::new(HashMap::new()),
            sqlite_store: None,
        }
    }

    /// Create a history manager with SQLite persistence
    pub fn new_with_persistence<P: AsRef<std::path::Path>>(path: P) -> crate::common::Result<Self> {
        let store = SqliteStore::open(path)?;
        let histories = RwLock::new(HashMap::new());

        // Recover sessions from SQLite on startup
        if let Ok(session_ids) = store.list_sessions() {
            tracing::info!("Recovering {} sessions from SQLite...", session_ids.len());
            let mut guard = histories.write();
            for session_id in session_ids {
                if let Ok(Some(history)) = store.load_session(&session_id) {
                    let arc = Arc::new(RwLock::new(history));
                    guard.insert(session_id.clone(), arc);
                    tracing::debug!("Recovered session: {}", session_id);
                }
            }
            drop(guard);
            tracing::info!("Session recovery complete");
        }

        Ok(Self {
            histories,
            sqlite_store: Some(store),
        })
    }

    /// Sync a session to SQLite if persistence is enabled
    fn sync_to_sqlite(&self, session_id: &str) {
        if let Some(ref store) = self.sqlite_store {
            if let Some(history) = self.histories.read().get(session_id) {
                let history = history.read().clone();
                if let Err(e) = store.save_session(&history) {
                    tracing::warn!("Failed to persist session {}: {}", session_id, e);
                }
            }
        }
    }

    /// Get or create history for a session
    pub fn get_or_create(&self, session_id: &str) -> Arc<RwLock<SessionHistory>> {
        let mut histories = self.histories.write();
        if let Some(history) = histories.get(session_id) {
            return history.clone();
        }

        let history = Arc::new(RwLock::new(SessionHistory::new(session_id)));
        histories.insert(session_id.to_string(), history.clone());
        history
    }

    /// Add a message to a session's history
    pub fn add_message(&self, session_id: &str, message: Message) {
        let history = self.get_or_create(session_id);
        history.write().add_message(message);
        self.sync_to_sqlite(session_id);
    }

    /// Get history for a session
    pub fn get(&self, session_id: &str) -> Option<Arc<RwLock<SessionHistory>>> {
        self.histories.read().get(session_id).cloned()
    }

    /// List all histories
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<Arc<RwLock<SessionHistory>>> {
        self.histories.read().values().cloned().collect()
    }

    /// Clear history for a session
    #[allow(dead_code)]
    pub fn clear(&self, session_id: &str) {
        if let Some(history) = self.histories.read().get(session_id) {
            history.write().clear();
        }
        self.sync_to_sqlite(session_id);
    }

    /// Remove history for a session
    #[allow(dead_code)]
    pub fn remove(&self, session_id: &str) {
        self.histories.write().remove(session_id);
        if let Some(ref store) = self.sqlite_store {
            if let Err(e) = store.delete_session(session_id) {
                tracing::warn!("Failed to delete session {} from SQLite: {}", session_id, e);
            }
        }
    }

    /// List all session IDs
    #[allow(dead_code)]
    pub fn list_session_ids(&self) -> Vec<String> {
        if let Some(ref store) = self.sqlite_store {
            store.list_sessions().unwrap_or_default()
        } else {
            self.histories.read().keys().cloned().collect()
        }
    }

    /// Get session count
    #[allow(dead_code)]
    pub fn session_count(&self) -> usize {
        if let Some(ref store) = self.sqlite_store {
            store.session_count().unwrap_or(0)
        } else {
            self.histories.read().len()
        }
    }

    /// Shutdown the persistence layer
    #[allow(dead_code)]
    pub fn shutdown_persistence(&self) {
        if let Some(ref store) = self.sqlite_store {
            store.shutdown();
        }
    }

    /// Import a session directly (for session restore/import)
    pub fn import_session(&self, session_id: &str, history: SessionHistory) {
        let history_arc = Arc::new(RwLock::new(history));
        let mut histories = self.histories.write();
        histories.insert(session_id.to_string(), history_arc);
        drop(histories);
        self.sync_to_sqlite(session_id);
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
