//! TUI Persistence - Local SQLite storage for TUI message histories
//!
//! This module provides lightweight persistence for TUI session histories,
//! separate from the gateway's persistence. This ensures TUI messages
//! are saved locally even if the gateway is not running.

use crate::types::{Message, Role, SessionHistory};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{debug, info};

/// TUI-specific persistence manager using SQLite
/// Uses synchronous SQLite operations for simplicity and reliability
pub struct TuiPersistence {
    conn: Mutex<Connection>,
}

impl TuiPersistence {
    /// Create a new TUI persistence manager
    /// Opens (or creates) a SQLite database at the default location
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_path(Self::default_path()?)
    }

    /// Create a new TUI persistence manager with a custom path
    pub fn new_with_path<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS tui_sessions (
                 session_id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tui_messages (
                 id TEXT PRIMARY KEY,
                 session_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 timestamp TEXT NOT NULL,
                 tool_call_id TEXT,
                 tool_name TEXT,
                 FOREIGN KEY (session_id) REFERENCES tui_sessions(session_id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_tui_messages_session ON tui_messages(session_id);",
        )?;

        info!("TUI persistence initialized at {:?}", path);

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Get the default path for TUI persistence
    fn default_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let base = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("tiny_claw");
        Ok(base.join("tui_history.db"))
    }

    /// Save a session's history to disk (synchronous)
    pub fn save_history(&self, history: &SessionHistory) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = save_history_impl(&conn, history) {
            tracing::warn!("Failed to save history: {}", e);
        }
    }

    /// Load all session histories from disk (synchronous)
    pub fn load_all(&self) -> Vec<SessionHistory> {
        let conn = self.conn.lock().unwrap();
        load_all_histories_impl(&conn)
    }

    /// Delete a session from disk (synchronous)
    pub fn delete_session(&self, session_id: &str) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = delete_session_impl(&conn, session_id) {
            tracing::warn!("Failed to delete session: {}", e);
        }
    }
}

impl Default for TuiPersistence {
    fn default() -> Self {
        Self::new().expect("Failed to create TUI persistence")
    }
}

/// Save a session history to SQLite (internal implementation)
fn save_history_impl(conn: &Connection, history: &SessionHistory) -> Result<(), rusqlite::Error> {
    // Upsert session
    conn.execute(
        "INSERT INTO tui_sessions (session_id, created_at, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(session_id) DO UPDATE SET updated_at = ?3",
        params![
            history.session_id,
            history.created_at.to_rfc3339(),
            history.updated_at.to_rfc3339(),
        ],
    )?;

    // Delete existing messages for this session (simpler than incremental update)
    conn.execute(
        "DELETE FROM tui_messages WHERE session_id = ?1",
        params![history.session_id],
    )?;

    // Insert all messages
    for msg in &history.messages {
        conn.execute(
            "INSERT INTO tui_messages (id, session_id, role, content, timestamp, tool_call_id, tool_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                msg.id,
                history.session_id,
                role_to_string(&msg.role),
                msg.content,
                msg.timestamp.to_rfc3339(),
                msg.tool_call_id,
                msg.tool_name,
            ],
        )?;
    }

    debug!("Saved {} messages for session {}", history.messages.len(), history.session_id);
    Ok(())
}

/// Load all session histories from SQLite (internal implementation)
fn load_all_histories_impl(conn: &Connection) -> Vec<SessionHistory> {
    let mut histories = Vec::new();

    let mut stmt = match conn.prepare("SELECT session_id, created_at, updated_at FROM tui_sessions") {
        Ok(stmt) => stmt,
        Err(e) => {
            tracing::warn!("Failed to prepare statement: {}", e);
            return histories;
        }
    };

    let session_rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect::<Vec<_>>(),
        Err(e) => {
            tracing::warn!("Failed to query sessions: {}", e);
            return histories;
        }
    };

    for (session_id, created_at, updated_at) in session_rows {
        let mut msg_stmt = match conn.prepare(
            "SELECT id, role, content, timestamp, tool_call_id, tool_name
             FROM tui_messages WHERE session_id = ?1 ORDER BY timestamp ASC",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                tracing::warn!("Failed to prepare messages statement: {}", e);
                continue;
            }
        };

        let messages: Vec<Message> = match msg_stmt.query_map(params![session_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                role: string_to_role(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                tool_call_id: row.get(4)?,
                tool_name: row.get(5)?,
            })
        }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                tracing::warn!("Failed to query messages: {}", e);
                Vec::new()
            }
        };

        histories.push(SessionHistory {
            session_id,
            messages,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        });
    }

    info!("Loaded {} session histories from TUI persistence", histories.len());
    histories
}

/// Delete a session and its messages (internal implementation)
fn delete_session_impl(conn: &Connection, session_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM tui_messages WHERE session_id = ?1",
        params![session_id],
    )?;
    conn.execute(
        "DELETE FROM tui_sessions WHERE session_id = ?1",
        params![session_id],
    )?;
    debug!("Deleted session {} from TUI persistence", session_id);
    Ok(())
}

/// Convert Role to string for storage
fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

/// Convert string back to Role
fn string_to_role(s: &str) -> Role {
    match s {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        _ => Role::Assistant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_persistence_save_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_tui.db");

        let persist = TuiPersistence::new_with_path(&db_path).unwrap();

        // Create a session with messages
        let mut history = SessionHistory::new("test-session");
        history.add_message(Message::user("Hello"));
        history.add_message(Message::assistant("Hi there!"));
        history.add_message(Message::user("How are you?"));

        // Save it
        persist.save_history(&history);

        // Create new persistence instance and load
        let persist2 = TuiPersistence::new_with_path(&db_path).unwrap();
        let histories = persist2.load_all();

        assert_eq!(histories.len(), 1);
        assert_eq!(histories[0].session_id, "test-session");
        assert_eq!(histories[0].messages.len(), 3);
        assert_eq!(histories[0].messages[0].content, "Hello");
        assert_eq!(histories[0].messages[1].content, "Hi there!");
    }

    #[test]
    fn test_persistence_delete() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_tui.db");

        let persist = TuiPersistence::new_with_path(&db_path).unwrap();

        // Create and save a session
        let mut history = SessionHistory::new("delete-me");
        history.add_message(Message::user("Test"));
        persist.save_history(&history);

        // Verify it exists
        let histories = persist.load_all();
        assert_eq!(histories.len(), 1);

        // Delete it
        persist.delete_session("delete-me");

        // Verify it's gone
        let histories = persist.load_all();
        assert_eq!(histories.len(), 0);
    }
}
