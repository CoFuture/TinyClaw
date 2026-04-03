//! Session Profile Manager - Persistent session metadata management
//!
//! Provides structured access to session metadata including:
//! - Description: A short human-readable description of the session's purpose
//! - Color tag: A visual color identifier for quick session recognition
//! - Tags: Arbitrary labels for categorizing sessions
//! - Created notes: Initial notes when the session was created

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Available color tags for sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionColor {
    #[default]
    Blue,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Magenta,
    Purple,
    Gray,
}

impl std::fmt::Display for SessionColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionColor::Red => write!(f, "red"),
            SessionColor::Orange => write!(f, "orange"),
            SessionColor::Yellow => write!(f, "yellow"),
            SessionColor::Green => write!(f, "green"),
            SessionColor::Cyan => write!(f, "cyan"),
            SessionColor::Blue => write!(f, "blue"),
            SessionColor::Magenta => write!(f, "magenta"),
            SessionColor::Purple => write!(f, "purple"),
            SessionColor::Gray => write!(f, "gray"),
        }
    }
}

/// Session profile containing metadata for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProfile {
    /// Session ID this profile belongs to
    pub session_id: String,
    /// Human-readable description of the session
    pub description: String,
    /// Color tag for visual identification
    #[serde(default)]
    pub color: SessionColor,
    /// Arbitrary tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Initial notes when session was created
    #[serde(default)]
    pub created_notes: String,
    /// When the profile was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the profile was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SessionProfile {
    pub fn new(session_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id,
            description: String::new(),
            color: SessionColor::default(),
            tags: Vec::new(),
            created_notes: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the description
    pub fn set_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = chrono::Utc::now();
    }

    /// Update the color tag
    pub fn set_color(&mut self, color: SessionColor) {
        self.color = color;
        self.updated_at = chrono::Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Remove a tag (utility method for future use)
    #[allow(dead_code)]
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = chrono::Utc::now();
    }

    /// Set the created notes
    pub fn set_created_notes(&mut self, notes: String) {
        self.created_notes = notes;
        self.updated_at = chrono::Utc::now();
    }
}

/// Summary of a session profile for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProfileSummary {
    pub session_id: String,
    pub description: String,
    pub color: SessionColor,
    pub tags: Vec<String>,
    pub has_notes: bool,
}

/// Storage format for all profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProfilesStorage {
    pub profiles: HashMap<String, SessionProfile>,
    pub version: u32,
}

impl Default for SessionProfilesStorage {
    fn default() -> Self {
        Self {
            profiles: HashMap::new(),
            version: 1,
        }
    }
}

/// Manager for session profiles
pub struct SessionProfileManager {
    profiles: RwLock<HashMap<String, SessionProfile>>,
    storage_path: PathBuf,
}

impl SessionProfileManager {
    /// Create a new manager with the given storage path
    pub fn new(storage_path: PathBuf) -> Self {
        let mut manager = Self {
            profiles: RwLock::new(HashMap::new()),
            storage_path,
        };
        manager.load();
        manager
    }

    /// Get or create a profile for a session
    pub fn get_or_create(&self, session_id: &str) -> Arc<RwLock<SessionProfile>> {
        let mut profiles = self.profiles.write();
        if let Some(profile) = profiles.get(session_id) {
            return Arc::new(RwLock::new(profile.clone()));
        }
        let profile = SessionProfile::new(session_id.to_string());
        let profile_id = profile.session_id.clone();
        profiles.insert(profile_id, profile.clone());
        drop(profiles);
        self.save();
        Arc::new(RwLock::new(profile))
    }

    /// Get a profile by session ID
    pub fn get(&self, session_id: &str) -> Option<Arc<RwLock<SessionProfile>>> {
        let profiles = self.profiles.read();
        profiles.get(session_id).map(|p| Arc::new(RwLock::new(p.clone())))
    }

    /// Update a profile
    pub fn update(&self, profile: &SessionProfile) {
        let mut profiles = self.profiles.write();
        profiles.insert(profile.session_id.clone(), profile.clone());
        drop(profiles);
        self.save();
    }

    /// Delete a profile
    pub fn delete(&self, session_id: &str) -> bool {
        let mut profiles = self.profiles.write();
        let removed = profiles.remove(session_id).is_some();
        drop(profiles);
        if removed {
            self.save();
        }
        removed
    }

    /// List all profiles as summaries
    pub fn list_summaries(&self) -> Vec<SessionProfileSummary> {
        let profiles = self.profiles.read();
        profiles
            .values()
            .map(|p| SessionProfileSummary {
                session_id: p.session_id.clone(),
                description: p.description.clone(),
                color: p.color,
                tags: p.tags.clone(),
                has_notes: !p.created_notes.is_empty() || !p.description.is_empty(),
            })
            .collect()
    }

    /// Find profiles matching a tag (utility method for future use)
    #[allow(dead_code)]
    pub fn find_by_tag(&self, tag: &str) -> Vec<Arc<RwLock<SessionProfile>>> {
        let profiles = self.profiles.read();
        profiles
            .values()
            .filter(|p| p.tags.iter().any(|t| t == tag))
            .map(|p| Arc::new(RwLock::new(p.clone())))
            .collect()
    }

    /// Find profiles matching a color (utility method for future use)
    #[allow(dead_code)]
    pub fn find_by_color(&self, color: SessionColor) -> Vec<Arc<RwLock<SessionProfile>>> {
        let profiles = self.profiles.read();
        profiles
            .values()
            .filter(|p| p.color == color)
            .map(|p| Arc::new(RwLock::new(p.clone())))
            .collect()
    }

    /// Get all unique tags across all profiles
    pub fn all_tags(&self) -> Vec<String> {
        let profiles = self.profiles.read();
        let mut tags: Vec<String> = profiles
            .values()
            .flat_map(|p| p.tags.clone())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Clean up profiles for sessions that no longer exist (utility method for future use)
    #[allow(dead_code)]
    pub fn cleanup(&self, active_session_ids: &[String]) {
        let mut profiles = self.profiles.write();
        profiles.retain(|session_id, _| active_session_ids.contains(session_id));
        drop(profiles);
        self.save();
    }

    /// Load profiles from disk
    fn load(&mut self) {
        if self.storage_path.exists() {
            if let Ok(contents) = fs::read_to_string(&self.storage_path) {
                if let Ok(storage) = serde_json::from_str::<SessionProfilesStorage>(&contents) {
                    let mut profiles = self.profiles.write();
                    *profiles = storage.profiles;
                }
            }
        }
    }

    /// Save profiles to disk
    fn save(&self) {
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let storage = SessionProfilesStorage {
            profiles: self.profiles.read().clone(),
            version: 1,
        };
        if let Ok(json) = serde_json::to_string_pretty(&storage) {
            let _ = fs::write(&self.storage_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_profile_new() {
        let profile = SessionProfile::new("test-session".to_string());
        assert_eq!(profile.session_id, "test-session");
        assert!(profile.description.is_empty());
        assert_eq!(profile.color, SessionColor::Blue);
        assert!(profile.tags.is_empty());
    }

    #[test]
    fn test_session_profile_set_description() {
        let mut profile = SessionProfile::new("test".to_string());
        profile.set_description("My coding session".to_string());
        assert_eq!(profile.description, "My coding session");
    }

    #[test]
    fn test_session_profile_tags() {
        let mut profile = SessionProfile::new("test".to_string());
        profile.add_tag("rust".to_string());
        profile.add_tag("coding".to_string());
        profile.add_tag("rust".to_string()); // duplicate, should not add
        assert_eq!(profile.tags.len(), 2);
        profile.remove_tag("rust");
        assert_eq!(profile.tags.len(), 1);
        assert!(profile.tags.contains(&"coding".to_string()));
    }

    #[test]
    fn test_session_color_display() {
        assert_eq!(SessionColor::Red.to_string(), "red");
        assert_eq!(SessionColor::Blue.to_string(), "blue");
        assert_eq!(SessionColor::Green.to_string(), "green");
    }

    #[test]
    fn test_manager_crud() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("profiles.json");
        let manager = SessionProfileManager::new(storage_path);

        // Create profile
        let profile = manager.get_or_create("session-1");
        {
            let mut p = profile.write();
            p.set_description("Test session".to_string());
            p.add_tag("test".to_string());
        }
        manager.update(&profile.read());

        // Get profile
        let retrieved = manager.get("session-1").unwrap();
        assert_eq!(retrieved.read().description, "Test session");

        // List summaries
        let summaries = manager.list_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].description, "Test session");

        // Delete profile
        assert!(manager.delete("session-1"));
        assert!(manager.get("session-1").is_none());
    }

    #[test]
    fn test_find_by_tag() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionProfileManager::new(temp_dir.path().join("profiles.json"));

        let p1 = manager.get_or_create("s1");
        p1.write().add_tag("coding".to_string());
        manager.update(&p1.read());

        let p2 = manager.get_or_create("s2");
        p2.write().add_tag("coding".to_string());
        p2.write().add_tag("rust".to_string());
        manager.update(&p2.read());

        let results = manager.find_by_tag("coding");
        assert_eq!(results.len(), 2);

        let rust_only = manager.find_by_tag("rust");
        assert_eq!(rust_only.len(), 1);
    }

    #[test]
    fn test_all_tags() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionProfileManager::new(temp_dir.path().join("profiles.json"));

        let p1 = manager.get_or_create("s1");
        p1.write().add_tag("coding".to_string());
        p1.write().add_tag("rust".to_string());
        manager.update(&p1.read());

        let p2 = manager.get_or_create("s2");
        p2.write().add_tag("coding".to_string());
        p2.write().add_tag("python".to_string());
        manager.update(&p2.read());

        let tags = manager.all_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"coding".to_string()));
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"python".to_string()));
    }
}
