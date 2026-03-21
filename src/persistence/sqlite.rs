//! SQLite-backed persistent storage for session history
//! Uses a dedicated thread for SQLite operations to avoid Sync issues.

use crate::common::{Error, Result};
use crate::types::{Message, Role, SessionHistory};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::thread;
use tracing::{debug, info};

/// Response channel type for SQLite thread commands
type SqliteResp<T> = std::sync::mpsc::Sender<Result<T>>;

/// Commands sent to the SQLite thread
#[allow(dead_code)]
enum SqliteCommand {
    SaveSession(SessionHistory, SqliteResp<()>),
    LoadSession(String, SqliteResp<Option<SessionHistory>>),
    ListSessions(SqliteResp<Vec<String>>),
    DeleteSession(String, SqliteResp<()>),
    SessionCount(SqliteResp<usize>),
    MessageCount(SqliteResp<usize>),
    Shutdown,
}

/// SQLite store with a dedicated thread for thread-safe access
pub struct SqliteStore {
    tx: std::sync::mpsc::Sender<SqliteCommand>,
}

impl SqliteStore {
    /// Open or create a SQLite database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 session_id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS messages (
                 id TEXT PRIMARY KEY,
                 session_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 timestamp TEXT NOT NULL,
                 tool_call_id TEXT,
                 tool_name TEXT,
                 FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
             CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);",
        )?;

        let (tx, rx) = std::sync::mpsc::channel::<SqliteCommand>();

        thread::Builder::new()
            .name("tinyclaw-sqlite".into())
            .spawn(move || {
                let conn = std::sync::Mutex::new(conn);
                for cmd in rx {
                    match cmd {
                        SqliteCommand::SaveSession(history, resp) => {
                            let result = save_session(&conn, &history);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::LoadSession(id, resp) => {
                            let result = load_session(&conn, &id);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::ListSessions(resp) => {
                            let result = list_sessions(&conn);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::DeleteSession(id, resp) => {
                            let result = delete_session(&conn, &id);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::SessionCount(resp) => {
                            let result = session_count(&conn);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::MessageCount(resp) => {
                            let result = message_count(&conn);
                            let _ = resp.send(result);
                        }
                        SqliteCommand::Shutdown => break,
                    }
                }
            })?;

        info!("SQLite store initialized with dedicated thread");
        Ok(Self { tx })
    }

    /// Save or update a session's history
    pub fn save_session(&self, history: &SessionHistory) -> Result<()> {
        let (resp, rx) = std::sync::mpsc::channel();
        let history = history.clone();
        self.tx
            .send(SqliteCommand::SaveSession(history, resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// Load a session's history by session_id
    #[allow(dead_code)]
    pub fn load_session(&self, session_id: &str) -> Result<Option<SessionHistory>> {
        let (resp, rx) = std::sync::mpsc::channel();
        self.tx
            .send(SqliteCommand::LoadSession(session_id.to_string(), resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// List all session IDs
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let (resp, rx) = std::sync::mpsc::channel();
        self.tx
            .send(SqliteCommand::ListSessions(resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// Delete a session and its messages
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let (resp, rx) = std::sync::mpsc::channel();
        self.tx
            .send(SqliteCommand::DeleteSession(session_id.to_string(), resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// Get total session count
    pub fn session_count(&self) -> Result<usize> {
        let (resp, rx) = std::sync::mpsc::channel();
        self.tx
            .send(SqliteCommand::SessionCount(resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// Get total message count
    #[allow(dead_code)]
    pub fn message_count(&self) -> Result<usize> {
        let (resp, rx) = std::sync::mpsc::channel();
        self.tx
            .send(SqliteCommand::MessageCount(resp))
            .map_err(|_| Error::Other("SQLite thread closed".into()))?;
        rx.recv()
            .map_err(|_| Error::Other("SQLite response channel closed".into()))?
    }

    /// Shutdown the SQLite thread
    #[allow(dead_code)]
    pub fn shutdown(&self) {
        let _ = self.tx.send(SqliteCommand::Shutdown);
    }
}

// Internal: save session (called on SQLite thread)
fn save_session(conn: &std::sync::Mutex<Connection>, history: &SessionHistory) -> Result<()> {
    let conn = conn.lock().unwrap();
    conn.execute(
        "INSERT INTO sessions (session_id, created_at, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(session_id) DO UPDATE SET updated_at = ?3",
        params![
            history.session_id,
            history.created_at.to_rfc3339(),
            history.updated_at.to_rfc3339(),
        ],
    )?;

    conn.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![history.session_id],
    )?;

    for msg in &history.messages {
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, tool_call_id, tool_name)
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

    debug!(
        "Saved session {} with {} messages",
        history.session_id,
        history.messages.len()
    );
    Ok(())
}

// Internal: load session (called on SQLite thread)
fn load_session(
    conn: &std::sync::Mutex<Connection>,
    session_id: &str,
) -> Result<Option<SessionHistory>> {
    let conn = conn.lock().unwrap();

    // Get session metadata - extract all data before any further borrows
    let meta: Option<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT session_id, created_at, updated_at FROM sessions WHERE session_id = ?1",
        )?;
        let mut rows = stmt.query(params![session_id])?;
        rows.next()?.map(|row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
            )
        })
    };

    if let Some((sid, ref created_at, ref updated_at)) = meta {
        let messages = load_messages_locked(&conn, &sid)?;
        Ok(Some(SessionHistory {
            session_id: sid,
            messages,
            created_at: DateTime::parse_from_rfc3339(created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }))
    } else {
        Ok(None)
    }
}

// Internal: load messages (caller holds the lock)
fn load_messages_locked(conn: &Connection, session_id: &str) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, role, content, timestamp, tool_call_id, tool_name
         FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC",
    )?;

    let messages = stmt
        .query_map(params![session_id], |row| {
            let role_str: String = row.get(1)?;
            let tool_call_id: Option<String> = row.get(4)?;
            let tool_name: Option<String> = row.get(5)?;
            let timestamp_str: String = row.get(3)?;

            Ok(Message {
                id: row.get(0)?,
                role: string_to_role(&role_str),
                content: row.get(2)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                tool_call_id,
                tool_name,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

// Internal: list all sessions
fn list_sessions(conn: &std::sync::Mutex<Connection>) -> Result<Vec<String>> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT session_id FROM sessions ORDER BY updated_at DESC")?;
    let ids = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

// Internal: delete session
fn delete_session(conn: &std::sync::Mutex<Connection>, session_id: &str) -> Result<()> {
    let conn = conn.lock().unwrap();
    conn.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![session_id],
    )?;
    conn.execute(
        "DELETE FROM sessions WHERE session_id = ?1",
        params![session_id],
    )?;
    debug!("Deleted session {}", session_id);
    Ok(())
}

// Internal: session count
fn session_count(conn: &std::sync::Mutex<Connection>) -> Result<usize> {
    let conn = conn.lock().unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
    Ok(count as usize)
}

// Internal: message count
fn message_count(conn: &std::sync::Mutex<Connection>) -> Result<usize> {
    let conn = conn.lock().unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
    Ok(count as usize)
}

// Helper: convert Role enum to string
fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

// Helper: convert string to Role enum
fn string_to_role(s: &str) -> Role {
    match s {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        _ => Role::User,
    }
}

impl Clone for SqliteStore {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

// SAFETY: SqliteStore is thread-safe because all operations are sent to a single thread via channels
unsafe impl Send for SqliteStore {}
unsafe impl Sync for SqliteStore {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sqlite_store_crud() {
        let temp_path = "/tmp/tinyclaw_test_store.db";
        let _ = fs::remove_file(temp_path);

        let store = SqliteStore::open(temp_path).unwrap();

        let mut history = SessionHistory::new("test-session-1");
        history.add_message(Message::user("Hello"));
        history.add_message(Message::assistant("Hi there"));
        history.add_message(Message::tool("result", "call_1", "exec"));

        store.save_session(&history).unwrap();

        let loaded = store.load_session("test-session-1").unwrap().unwrap();
        assert_eq!(loaded.session_id, "test-session-1");
        assert_eq!(loaded.messages.len(), 3);
        assert_eq!(loaded.messages[0].role, Role::User);
        assert_eq!(loaded.messages[1].role, Role::Assistant);
        assert_eq!(loaded.messages[2].role, Role::Tool);

        let sessions = store.list_sessions().unwrap();
        assert!(sessions.contains(&"test-session-1".to_string()));

        store.delete_session("test-session-1").unwrap();
        assert!(store.load_session("test-session-1").unwrap().is_none());

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_sqlite_store_update() {
        let temp_path = "/tmp/tinyclaw_test_update.db";
        let _ = fs::remove_file(temp_path);

        let store = SqliteStore::open(temp_path).unwrap();

        let mut history = SessionHistory::new("session-update");
        history.add_message(Message::user("First"));
        store.save_session(&history).unwrap();

        history.add_message(Message::assistant("Second"));
        history.add_message(Message::user("Third"));
        store.save_session(&history).unwrap();

        let loaded = store.load_session("session-update").unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 3);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_sqlite_store_counts() {
        let temp_path = "/tmp/tinyclaw_test_counts.db";
        let _ = fs::remove_file(temp_path);

        let store = SqliteStore::open(temp_path).unwrap();

        assert_eq!(store.session_count().unwrap(), 0);
        assert_eq!(store.message_count().unwrap(), 0);

        let mut h1 = SessionHistory::new("s1");
        h1.add_message(Message::user("msg1"));
        store.save_session(&h1).unwrap();

        assert_eq!(store.session_count().unwrap(), 1);
        assert_eq!(store.message_count().unwrap(), 1);

        let mut h2 = SessionHistory::new("s2");
        h2.add_message(Message::user("msg2"));
        h2.add_message(Message::assistant("msg3"));
        store.save_session(&h2).unwrap();

        assert_eq!(store.session_count().unwrap(), 2);
        assert_eq!(store.message_count().unwrap(), 3);

        let _ = fs::remove_file(temp_path);
    }
}
