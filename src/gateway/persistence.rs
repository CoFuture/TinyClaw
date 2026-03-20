//! Persistence module for session data
//!
//! Provides simple JSON-based persistence for sessions and messages.

use crate::gateway::history::{Message, SessionHistory};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Persistent storage for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SessionStore {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&SessionHistory> for SessionStore {
    fn from(history: &SessionHistory) -> Self {
        Self {
            session_id: history.session_id.clone(),
            messages: history.messages.clone(),
            created_at: history.created_at,
            updated_at: history.updated_at,
        }
    }
}

/// Persistence manager
#[allow(dead_code)]
pub struct PersistenceManager {
    data_dir: PathBuf,
}

#[allow(dead_code)]
impl PersistenceManager {
    /// Create a new persistence manager
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Ensure data directory exists
    pub async fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir).await?;
        fs::create_dir_all(self.data_dir.join("sessions")).await?;
        Ok(())
    }

    /// Save a session to disk
    pub async fn save_session(&self, history: &SessionHistory) -> Result<()> {
        self.ensure_dir().await?;
        
        let store = SessionStore::from(history);
        let path = self.data_dir.join("sessions").join(format!("{}.json", history.session_id));
        let json = serde_json::to_string_pretty(&store)?;
        fs::write(path, json).await?;
        
        Ok(())
    }

    /// Load a session from disk
    pub async fn load_session(&self, session_id: &str) -> Result<Option<SessionHistory>> {
        let path = self.data_dir.join("sessions").join(format!("{}.json", session_id));
        
        if !path.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(path).await?;
        let store: SessionStore = serde_json::from_str(&json)?;
        
        let mut history = SessionHistory::new(store.session_id);
        history.messages = store.messages;
        // Note: We don't restore created_at and updated_at exactly
        // since SessionHistory doesn't expose setters
        
        Ok(Some(history))
    }

    /// List all persisted session IDs
    pub async fn list_sessions(&self) -> Result<Vec<String>> {
        let sessions_dir = self.data_dir.join("sessions");
        
        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = fs::read_dir(sessions_dir).await?;
        let mut session_ids = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    session_ids.push(stem.to_string());
                }
            }
        }
        
        Ok(session_ids)
    }

    /// Delete a session from disk
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.data_dir.join("sessions").join(format!("{}.json", session_id));
        
        if path.exists() {
            fs::remove_file(path).await?;
        }
        
        Ok(())
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw");
        
        Self::new(data_dir)
    }
}
