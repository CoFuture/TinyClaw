//! Session management

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Session ID
pub type SessionId = String;

/// Session state
#[derive(Debug, Clone)]
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
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session kind
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }

    pub fn main() -> Self {
        Self::new(SessionKind::Main)
    }

    pub fn isolated() -> Self {
        Self::new(SessionKind::Isolated)
    }

    pub fn channel(channel: impl Into<String>) -> Self {
        Self::new(SessionKind::Channel {
            channel: channel.into(),
        })
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

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
    pub fn get(&self, id: &SessionId) -> Option<Arc<RwLock<Session>>> {
        self.sessions.read().get(id).cloned()
    }

    /// List all sessions
    pub fn list(&self) -> Vec<Arc<RwLock<Session>>> {
        self.sessions.read().values().cloned().collect()
    }

    /// Remove a session
    pub fn remove(&self, id: &SessionId) -> Option<Arc<RwLock<Session>>> {
        self.sessions.write().remove(id)
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
