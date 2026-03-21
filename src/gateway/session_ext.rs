//! Advanced session management

use crate::common::error::{Error, Result};
use crate::gateway::session::{Session, SessionId, SessionManager};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Session tag
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SessionTag {
    pub name: String,
    pub color: Option<String>,
}

/// Extended session info with tags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ExtendedSession {
    #[serde(flatten)]
    pub session: Session,
    pub tags: Vec<SessionTag>,
    pub priority: i32,
    pub pinned: bool,
}

#[allow(dead_code)]
impl ExtendedSession {
    pub fn from_session(session: Session) -> Self {
        Self {
            session,
            tags: Vec::new(),
            priority: 0,
            pinned: false,
        }
    }

    pub fn with_tags(mut self, tags: Vec<SessionTag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

/// Advanced session manager
#[allow(dead_code)]
pub struct AdvancedSessionManager {
    inner: Arc<SessionManager>,
    extended: RwLock<HashMap<SessionId, ExtendedSession>>,
    tags: RwLock<HashMap<String, SessionTag>>,
}

#[allow(dead_code)]
impl AdvancedSessionManager {
    pub fn new(inner: Arc<SessionManager>) -> Self {
        Self {
            inner,
            extended: RwLock::new(HashMap::new()),
            tags: RwLock::new(HashMap::new()),
        }
    }

    /// Create a session with extended info
    pub fn create_extended(&self, session: Session, extended: ExtendedSession) -> Arc<RwLock<Session>> {
        let arc = self.inner.create(session);
        let id = arc.read().id.clone();
        self.extended.write().insert(id, extended);
        arc
    }

    /// Get extended session info
    pub fn get_extended(&self, id: &SessionId) -> Option<ExtendedSession> {
        self.extended.read().get(id).cloned()
    }

    /// Update session tags
    pub fn set_tags(&self, id: &SessionId, tags: Vec<SessionTag>) -> Result<()> {
        let mut extended = self.extended.write();
        if let Some(session) = extended.get_mut(id) {
            session.tags = tags;
            Ok(())
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// Add tag to session
    pub fn add_tag(&self, id: &SessionId, tag: SessionTag) -> Result<()> {
        let mut extended = self.extended.write();
        if let Some(session) = extended.get_mut(id) {
            if !session.tags.iter().any(|t| t.name == tag.name) {
                session.tags.push(tag);
            }
            Ok(())
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// Remove tag from session
    pub fn remove_tag(&self, id: &SessionId, tag_name: &str) -> Result<()> {
        let mut extended = self.extended.write();
        if let Some(session) = extended.get_mut(id) {
            session.tags.retain(|t| t.name != tag_name);
            Ok(())
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// Set session priority
    pub fn set_priority(&self, id: &SessionId, priority: i32) -> Result<()> {
        let mut extended = self.extended.write();
        if let Some(session) = extended.get_mut(id) {
            session.priority = priority;
            Ok(())
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// Pin/unpin session
    pub fn set_pinned(&self, id: &SessionId, pinned: bool) -> Result<()> {
        let mut extended = self.extended.write();
        if let Some(session) = extended.get_mut(id) {
            session.pinned = pinned;
            Ok(())
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// List all tags
    pub fn list_tags(&self) -> Vec<SessionTag> {
        self.tags.read().values().cloned().collect()
    }

    /// Register a tag
    pub fn register_tag(&self, tag: SessionTag) {
        self.tags.write().insert(tag.name.clone(), tag);
    }

    /// Find sessions by tag
    pub fn find_by_tag(&self, tag_name: &str) -> Vec<SessionId> {
        self.extended.read()
            .iter()
            .filter(|(_, ext)| ext.tags.iter().any(|t| t.name == tag_name))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all sessions sorted by priority (pinned first, then by priority)
    pub fn list_sorted(&self) -> Vec<ExtendedSession> {
        let mut sessions: Vec<ExtendedSession> = self.extended.read().values().cloned().collect();
        sessions.sort_by(|a, b| {
            match (a.pinned, b.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.priority.cmp(&a.priority),
            }
        });
        sessions
    }

    /// Export session to JSON
    pub fn export_session(&self, id: &SessionId) -> Result<serde_json::Value> {
        let extended = self.extended.read();
        if let Some(session) = extended.get(id) {
            Ok(serde_json::to_value(session).map_err(|e| Error::Other(e.to_string()))?)
        } else {
            Err(Error::SessionNotFound(id.clone()))
        }
    }

    /// Import session from JSON
    pub fn import_session(&self, data: serde_json::Value) -> Result<SessionId> {
        let extended: ExtendedSession = serde_json::from_value(data)
            .map_err(|e| Error::Other(e.to_string()))?;
        
        let id = extended.session.id.clone();
        self.extended.write().insert(id.clone(), extended);
        Ok(id)
    }

    /// Forward to inner session manager
    pub fn inner(&self) -> &Arc<SessionManager> {
        &self.inner
    }
}
