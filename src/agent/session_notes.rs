//! Session Notes Module
//!
//! Allows users to attach persistent notes/todos to sessions.
//! Notes are injected into the agent's context as part of the system prompt.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use parking_lot::RwLock;
use uuid::Uuid;

/// A note attached to a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    /// Unique note ID
    pub id: String,
    /// ID of the session this note belongs to
    pub session_id: String,
    /// Note content (supports markdown-like formatting)
    pub content: String,
    /// Whether this note is pinned (pinned notes appear first)
    #[serde(default)]
    pub pinned: bool,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl SessionNote {
    /// Create a new session note
    pub fn new(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            content: content.into(),
            pinned: false,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with pinned status
    #[allow(dead_code)]
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Create with tags
    #[allow(dead_code)]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Update note fields (for testing)
    #[allow(dead_code)]
    pub fn update(&mut self, update: &SessionNoteUpdate) {
        if let Some(content) = &update.content {
            self.content = content.clone();
        }
        if let Some(pinned) = update.pinned {
            self.pinned = pinned;
        }
        if let Some(tags) = &update.tags {
            self.tags = tags.clone();
        }
        self.updated_at = Utc::now();
    }
}

/// A summary of a session note (for list responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNoteSummary {
    pub id: String,
    pub session_id: String,
    pub content_preview: String,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&SessionNote> for SessionNoteSummary {
    fn from(note: &SessionNote) -> Self {
        let preview = if note.content.len() > 100 {
            format!("{}...", &note.content[..100])
        } else {
            note.content.clone()
        };
        Self {
            id: note.id.clone(),
            session_id: note.session_id.clone(),
            content_preview: preview,
            pinned: note.pinned,
            tags: note.tags.clone(),
            created_at: note.created_at,
            updated_at: note.updated_at,
        }
    }
}

/// Partial update for session notes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionNoteUpdate {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub pinned: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Manager for session notes with file persistence
pub struct SessionNotesManager {
    /// Notes organized by session_id
    notes: RwLock<HashMap<String, HashMap<String, SessionNote>>>,
    /// Base directory for persistence
    base_path: PathBuf,
}

impl SessionNotesManager {
    /// Create a new manager with default path
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("session_notes");
        Self::with_path(path)
    }

    /// Create a manager with custom persistence path
    #[allow(dead_code)]
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let manager = Self {
            notes: RwLock::new(HashMap::new()),
            base_path: path.clone(),
        };
        manager.load_all();
        manager
    }

    /// Load all session notes from disk
    fn load_all(&self) {
        if !self.base_path.exists() {
            return;
        }

        let entries = match fs::read_dir(&self.base_path) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read session notes directory: {}", e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session_notes) = serde_json::from_str::<HashMap<String, SessionNote>>(&content) {
                        if !session_notes.is_empty() {
                            let session_id = session_notes.values().next().map(|n| n.session_id.clone());
                            if let Some(sid) = session_id {
                                let mut notes = self.notes.write();
                                notes.insert(sid, session_notes);
                            }
                        }
                    }
                }
            }
        }
        tracing::info!("Loaded session notes from {:?}", self.base_path);
    }

    /// Save notes for a specific session to disk
    fn save_session(&self, session_id: &str) {
        let notes = self.notes.read();
        if let Some(session_notes) = notes.get(session_id) {
            if session_notes.is_empty() {
                // Remove the file if no notes
                let path = self.base_path.join(format!("{}.json", session_id));
                let _ = fs::remove_file(&path);
                return;
            }

            // Ensure parent directory exists
            if let Some(parent) = self.base_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            match serde_json::to_string_pretty(session_notes) {
                Ok(content) => {
                    let path = self.base_path.join(format!("{}.json", session_id));
                    if let Err(e) = fs::write(&path, content) {
                        tracing::warn!("Failed to save session notes: {}", e);
                    } else {
                        tracing::debug!("Saved session notes to {:?}", path);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to serialize session notes: {}", e);
                }
            }
        }
    }

    /// Add a new note to a session
    pub fn add(&self, session_id: &str, content: &str) -> SessionNote {
        let note = SessionNote::new(session_id, content);
        let mut notes = self.notes.write();
        let session_notes = notes.entry(session_id.to_string()).or_default();
        session_notes.insert(note.id.clone(), note.clone());
        drop(notes);
        self.save_session(session_id);
        note
    }

    /// Get a specific note
    #[allow(dead_code)]
    pub fn get(&self, session_id: &str, note_id: &str) -> Option<SessionNote> {
        let notes = self.notes.read();
        notes.get(session_id).and_then(|s| s.get(note_id).cloned())
    }

    /// List all notes for a session
    pub fn list(&self, session_id: &str) -> Vec<SessionNote> {
        let notes = self.notes.read();
        notes.get(session_id)
            .map(|s| {
                let mut all: Vec<_> = s.values().cloned().collect();
                // Sort: pinned first, then by updated_at descending
                all.sort_by(|a, b| {
                    if a.pinned != b.pinned {
                        b.pinned.cmp(&a.pinned)
                    } else {
                        b.updated_at.cmp(&a.updated_at)
                    }
                });
                all
            })
            .unwrap_or_default()
    }

    /// List summaries for a session (lighter weight)
    pub fn list_summaries(&self, session_id: &str) -> Vec<SessionNoteSummary> {
        self.list(session_id).iter().map(SessionNoteSummary::from).collect()
    }

    /// Update a note
    pub fn update(&self, session_id: &str, note_id: &str, update: SessionNoteUpdate) -> Option<SessionNote> {
        let mut notes = self.notes.write();
        if let Some(session_notes) = notes.get_mut(session_id) {
            if let Some(note) = session_notes.get_mut(note_id) {
                if let Some(content) = update.content {
                    note.content = content;
                }
                if let Some(pinned) = update.pinned {
                    note.pinned = pinned;
                }
                if let Some(tags) = update.tags {
                    note.tags = tags;
                }
                note.updated_at = Utc::now();
                let updated = note.clone();
                drop(notes);
                self.save_session(session_id);
                return Some(updated);
            }
        }
        None
    }

    /// Delete a note
    pub fn delete(&self, session_id: &str, note_id: &str) -> bool {
        let mut notes = self.notes.write();
        if let Some(session_notes) = notes.get_mut(session_id) {
            if session_notes.remove(note_id).is_some() {
                drop(notes);
                self.save_session(session_id);
                return true;
            }
        }
        false
    }

    /// Delete all notes for a session
    #[allow(dead_code)]
    pub fn delete_all(&self, session_id: &str) -> bool {
        let mut notes = self.notes.write();
        if notes.remove(session_id).is_some() {
            drop(notes);
            self.save_session(session_id);
            return true;
        }
        false
    }

    /// Check if a session has any notes
    #[allow(dead_code)]
    pub fn has_notes(&self, session_id: &str) -> bool {
        let notes = self.notes.read();
        notes.get(session_id).map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// Count notes for a session
    #[allow(dead_code)]
    pub fn count(&self, session_id: &str) -> usize {
        let notes = self.notes.read();
        notes.get(session_id).map(|s| s.len()).unwrap_or(0)
    }

    /// Generate system prompt content from session notes
    /// Returns None if no notes exist
    pub fn to_system_prompt_addition(&self, session_id: &str) -> Option<String> {
        let notes = self.list(session_id);
        if notes.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        parts.push("## Session Notes\n".to_string());
        parts.push("The following notes are from previous interactions in this session:\n".to_string());

        for note in &notes {
            let pinned_marker = if note.pinned { "📌 " } else { "" };
            let tags_str = if note.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", note.tags.join(", "))
            };
            parts.push(format!(
                "- {}{}*{}*\n  {}{}",
                pinned_marker,
                note.created_at.format("%Y-%m-%d"),
                tags_str,
                note.content,
                if note.content.ends_with('\n') { "" } else { "\n" }
            ));
        }

        Some(parts.join(""))
    }

    /// List notes across all sessions (for admin purposes)
    #[allow(dead_code)]
    pub fn list_all_session_ids(&self) -> Vec<String> {
        let notes = self.notes.read();
        notes.keys().cloned().collect()
    }
}

impl Default for SessionNotesManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_note_new() {
        let note = SessionNote::new("session1", "Test note content");
        assert_eq!(note.session_id, "session1");
        assert_eq!(note.content, "Test note content");
        assert!(!note.pinned);
        assert!(note.tags.is_empty());
        assert!(!note.id.is_empty());
    }

    #[test]
    fn test_session_note_pinned() {
        let note = SessionNote::new("session1", "Pinned note").pinned(true);
        assert!(note.pinned);
    }

    #[test]
    fn test_session_note_with_tags() {
        let note = SessionNote::new("session1", "Tagged note").with_tags(vec!["important".to_string(), "review".to_string()]);
        assert_eq!(note.tags, vec!["important", "review"]);
    }

    #[test]
    fn test_session_note_summary_preview() {
        let long_content = "A".repeat(150);
        let note = SessionNote::new("session1", &long_content);
        let summary = SessionNoteSummary::from(&note);
        assert_eq!(summary.content_preview.len(), 103); // 100 + "..."
        assert!(summary.content_preview.ends_with("..."));
    }

    #[test]
    fn test_note_update() {
        let mut note = SessionNote::new("session1", "Original");
        note.update(&SessionNoteUpdate {
            content: Some("Updated content".to_string()),
            pinned: Some(true),
            tags: Some(vec!["new".to_string()]),
        });
        assert_eq!(note.content, "Updated content");
        assert!(note.pinned);
        assert_eq!(note.tags, vec!["new"]);
    }

    #[test]
    fn test_manager_add_and_get() {
        let manager = SessionNotesManager::new();
        let note = manager.add("session1", "Test note");
        assert_eq!(note.content, "Test note");
        assert_eq!(manager.count("session1"), 1);
        assert_eq!(manager.get("session1", &note.id).unwrap().content, "Test note");
    }

    #[test]
    fn test_manager_list_sorted() {
        let manager = SessionNotesManager::new();
        manager.add("session1", "Note 1");
        let pinned = manager.add("session1", "Pinned note");
        // Manually pin it
        manager.update("session1", &pinned.id, SessionNoteUpdate { pinned: Some(true), ..Default::default() });
        manager.add("session1", "Note 3");

        let notes = manager.list("session1");
        assert_eq!(notes.len(), 3);
        assert!(notes[0].pinned); // Pinned should be first
    }

    #[test]
    fn test_manager_delete() {
        let manager = SessionNotesManager::new();
        let note = manager.add("session1", "To be deleted");
        assert_eq!(manager.count("session1"), 1);
        assert!(manager.delete("session1", &note.id));
        assert_eq!(manager.count("session1"), 0);
    }

    #[test]
    fn test_manager_system_prompt() {
        let manager = SessionNotesManager::new();
        assert!(manager.to_system_prompt_addition("nonexistent").is_none());

        manager.add("session1", "First note");
        let pinned = manager.add("session1", "Important note");
        manager.update("session1", &pinned.id, SessionNoteUpdate { pinned: Some(true), ..Default::default() });

        let prompt = manager.to_system_prompt_addition("session1").unwrap();
        assert!(prompt.contains("Session Notes"));
        assert!(prompt.contains("Important note"));
        assert!(prompt.contains("📌")); // Pinned marker
    }

    #[test]
    fn test_note_update_struct() {
        let update = SessionNoteUpdate {
            content: Some("new".to_string()),
            pinned: None,
            tags: None,
        };
        assert_eq!(update.content, Some("new".to_string()));
        assert!(update.pinned.is_none());
    }
}
