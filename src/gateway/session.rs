//! Session management

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Session ID
pub type SessionId = String;

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: SessionId,
    /// Session label (e.g., "main", "discord:123")
    pub label: Option<String>,
    /// Session type
    pub kind: SessionKind,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_active: chrono::DateTime<chrono::Utc>,
    /// Session metadata
    #[allow(dead_code)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Per-session agent instructions (injected into system prompt)
    /// Example: "You are a code reviewer. Focus on performance and security."
    #[serde(default)]
    pub instructions: Option<String>,
}

/// Session kind
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SessionKind {
    /// Main session (direct chat)
    Main,
    /// Isolated session (sub-agent, etc.)
    Isolated,
    /// Channel session
    Channel { channel: String },
}

impl Session {
    pub fn new(kind: SessionKind) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            label: None,
            kind,
            created_at: now,
            last_active: now,
            metadata: HashMap::new(),
            instructions: None,
        }
    }

    pub fn main() -> Self {
        Self::new(SessionKind::Main)
    }

    #[allow(dead_code)]
    pub fn isolated() -> Self {
        Self::new(SessionKind::Isolated)
    }

    #[allow(dead_code)]
    pub fn channel(channel: impl Into<String>) -> Self {
        Self::new(SessionKind::Channel {
            channel: channel.into(),
        })
    }

    #[allow(dead_code)]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[allow(dead_code)]
    pub fn update_activity(&mut self) {
        self.last_active = chrono::Utc::now();
    }
}

/// Session manager
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<RwLock<Session>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session
    pub fn create(&self, session: Session) -> Arc<RwLock<Session>> {
        let session = Arc::new(RwLock::new(session));
        let id = session.read().id.clone();
        self.sessions.write().insert(id, session.clone());
        session
    }

    /// Get a session by ID
    #[allow(dead_code)]
    pub fn get(&self, id: &SessionId) -> Option<Arc<RwLock<Session>>> {
        self.sessions.read().get(id).cloned()
    }

    /// List all sessions
    pub fn list(&self) -> Vec<Arc<RwLock<Session>>> {
        self.sessions.read().values().cloned().collect()
    }

    /// Remove a session
    #[allow(dead_code)]
    pub fn remove(&self, id: &SessionId) -> Option<Arc<RwLock<Session>>> {
        self.sessions.write().remove(id)
    }

    /// Update a session's label (rename)
    /// Returns true if successful, false if session not found
    pub fn rename(&self, id: &SessionId, new_label: Option<String>) -> bool {
        let sessions = self.sessions.write();
        if let Some(session) = sessions.get(id) {
            session.write().label = new_label;
            session.write().last_active = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Get session instructions
    /// Returns the instructions if set, None otherwise
    pub fn get_instructions(&self, id: &str) -> Option<String> {
        self.sessions
            .read()
            .get(id)
            .and_then(|s| s.read().instructions.clone())
    }

    /// Set session instructions
    /// Returns true if successful, false if session not found
    pub fn set_instructions(&self, id: &str, instructions: Option<String>) -> bool {
        let sessions = self.sessions.write();
        if let Some(session) = sessions.get(id) {
            session.write().instructions = instructions;
            session.write().last_active = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Get or create main session
    pub fn get_or_create_main(&self) -> Arc<RwLock<Session>> {
        let sessions = self.sessions.read();
        
        // Find main session
        for session in sessions.values() {
            if session.read().kind == SessionKind::Main {
                return session.clone();
            }
        }
        drop(sessions);

        // Create main session if not found
        self.create(Session::main())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new(SessionKind::Main);
        assert_eq!(session.kind, SessionKind::Main);
        assert!(!session.id.is_empty());
    }

    #[test]
    fn test_session_main() {
        let session = Session::main();
        assert_eq!(session.kind, SessionKind::Main);
        assert!(session.label.is_none());
    }

    #[test]
    fn test_session_with_label() {
        let session = Session::new(SessionKind::Isolated).with_label("test");
        assert_eq!(session.label, Some("test".to_string()));
        assert_eq!(session.kind, SessionKind::Isolated);
    }

    #[test]
    fn test_session_manager_new() {
        let manager = SessionManager::new();
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_session_manager_create() {
        let manager = SessionManager::new();
        let _session = manager.create(Session::main());
        assert!(!manager.list().is_empty());
    }

    #[test]
    fn test_session_manager_get() {
        let manager = SessionManager::new();
        let created = manager.create(Session::main());
        let id = created.read().id.clone();
        
        let retrieved = manager.get(&id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_session_manager_get_or_create_main() {
        let manager = SessionManager::new();
        let main1 = manager.get_or_create_main();
        let main2 = manager.get_or_create_main();
        
        // Should be the same id
        let id1 = main1.read().id.clone();
        let id2 = main2.read().id.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_session_manager_list() {
        let manager = SessionManager::new();
        manager.create(Session::main());
        manager.create(Session::new(SessionKind::Isolated).with_label("test"));
        
        let sessions = manager.list();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_kind_serialization() {
        let kind = SessionKind::Main;
        let json = serde_json::to_string(&kind).unwrap();
        // SessionKind uses internally tagged serialization
        assert!(json.contains("Main"));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new(SessionKind::Isolated);
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("Isolated"));
        assert!(json.contains("id"));
    }
}
