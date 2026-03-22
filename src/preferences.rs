//! User Preferences Module
//!
//! Manages user preferences that persist across sessions.
//! Preferences are stored in a JSON file and loaded at startup.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use parking_lot::RwLock;

/// User preferences that persist across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// User's display name
    #[serde(default)]
    pub user_name: Option<String>,
    
    /// User's bio or description
    #[serde(default)]
    pub user_bio: Option<String>,
    
    /// Preferred language code (e.g., "en", "zh", "ja")
    #[serde(default = "default_language")]
    pub preferred_language: String,
    
    /// Default skills to auto-enable for new sessions
    #[serde(default)]
    pub default_skills: Vec<String>,
    
    /// Custom agent persona instructions (added to system prompt)
    #[serde(default)]
    pub agent_persona: Option<String>,
    
    /// User's timezone (e.g., "Asia/Shanghai", "America/New_York")
    #[serde(default = "default_timezone")]
    pub timezone: String,
    
    /// Theme preference for WebUI ("light", "dark", "auto")
    #[serde(default = "default_theme")]
    pub theme: String,
    
    /// Whether to enable streaming responses
    #[serde(default = "default_streaming")]
    pub streaming_enabled: bool,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_theme() -> String {
    "auto".to_string()
}

fn default_streaming() -> bool {
    true
}

impl Default for UserPreferences {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            user_name: None,
            user_bio: None,
            preferred_language: default_language(),
            default_skills: Vec::new(),
            agent_persona: None,
            timezone: default_timezone(),
            theme: default_theme(),
            streaming_enabled: default_streaming(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl UserPreferences {
    /// Update with new preferences (partial update)
    pub fn update(&mut self, other: UserPreferencesUpdate) {
        if let Some(v) = other.user_name {
            self.user_name = Some(v);
        }
        if let Some(v) = other.user_bio {
            self.user_bio = Some(v);
        }
        if let Some(v) = other.preferred_language {
            self.preferred_language = v;
        }
        if let Some(v) = other.default_skills {
            self.default_skills = v;
        }
        if let Some(v) = other.agent_persona {
            self.agent_persona = Some(v);
        }
        if let Some(v) = other.timezone {
            self.timezone = v;
        }
        if let Some(v) = other.theme {
            self.theme = v;
        }
        if let Some(v) = other.streaming_enabled {
            self.streaming_enabled = v;
        }
        self.updated_at = Utc::now();
    }

    /// Generate system prompt additions based on preferences
    #[allow(dead_code)]
    pub fn to_system_prompt_addition(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(ref name) = self.user_name {
            parts.push(format!("User's name: {}", name));
        }
        
        if let Some(ref bio) = self.user_bio {
            parts.push(format!("About user: {}", bio));
        }
        
        if self.preferred_language != "en" {
            parts.push(format!("Preferred language: {}", self.preferred_language));
        }
        
        if self.timezone != "UTC" {
            parts.push(format!("User's timezone: {}", self.timezone));
        }
        
        if let Some(ref persona) = self.agent_persona {
            parts.push(format!("Agent persona: {}", persona));
        }
        
        if parts.is_empty() {
            String::new()
        } else {
            format!("[User Context]\n{}\n[/User Context]", parts.join("\n"))
        }
    }
}

/// Partial update for user preferences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferencesUpdate {
    #[serde(default)]
    pub user_name: Option<String>,
    
    #[serde(default)]
    pub user_bio: Option<String>,
    
    #[serde(default)]
    pub preferred_language: Option<String>,
    
    #[serde(default)]
    pub default_skills: Option<Vec<String>>,
    
    #[serde(default)]
    pub agent_persona: Option<String>,
    
    #[serde(default)]
    pub timezone: Option<String>,
    
    #[serde(default)]
    pub theme: Option<String>,
    
    #[serde(default)]
    pub streaming_enabled: Option<bool>,
}

/// Manager for user preferences with file persistence
pub struct PreferencesManager {
    /// Current preferences
    preferences: RwLock<UserPreferences>,
    
    /// File path for persistence
    file_path: PathBuf,
}

impl PreferencesManager {
    /// Create a new preferences manager with default preferences
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a preferences manager with persistence
    #[allow(dead_code)]
    pub fn new_with_persistence<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let preferences = Self::load_from_file(&path).unwrap_or_default();
        
        Self {
            preferences: RwLock::new(preferences),
            file_path: path,
        }
    }
    
    /// Load preferences from file
    fn load_from_file(path: &PathBuf) -> Option<UserPreferences> {
        if !path.exists() {
            return None;
        }
        
        match fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(prefs) => {
                        tracing::info!("Loaded user preferences from {:?}", path);
                        Some(prefs)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse preferences file: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read preferences file: {}", e);
                None
            }
        }
    }
    
    /// Save preferences to file
    #[allow(dead_code)]
    pub fn save(&self) -> std::io::Result<()> {
        let preferences = self.preferences.read().clone();
        
        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(&preferences)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        fs::write(&self.file_path, content)?;
        tracing::debug!("Saved user preferences to {:?}", self.file_path);
        Ok(())
    }
    
    /// Get current preferences
    pub fn get(&self) -> UserPreferences {
        self.preferences.read().clone()
    }
    
    /// Update preferences (partial update)
    pub fn update(&self, update: UserPreferencesUpdate) {
        let mut preferences = self.preferences.write();
        preferences.update(update);
        drop(preferences);
        let _ = self.save();
    }
    
    /// Get system prompt addition based on preferences
    #[allow(dead_code)]
    pub fn get_system_prompt_addition(&self) -> String {
        self.preferences.read().to_system_prompt_addition()
    }
    
    /// Get default skills for new sessions
    #[allow(dead_code)]
    pub fn get_default_skills(&self) -> Vec<String> {
        self.preferences.read().default_skills.clone()
    }
}

impl Default for PreferencesManager {
    fn default() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("preferences.json");
        
        Self::new_with_persistence(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_default() {
        let prefs = UserPreferences::default();
        assert!(prefs.user_name.is_none());
        assert_eq!(prefs.preferred_language, "en");
        assert_eq!(prefs.timezone, "UTC");
        assert!(prefs.default_skills.is_empty());
    }

    #[test]
    fn test_preferences_update() {
        let mut prefs = UserPreferences::default();
        prefs.update(UserPreferencesUpdate {
            user_name: Some("Test User".to_string()),
            preferred_language: Some("zh".to_string()),
            ..Default::default()
        });
        
        assert_eq!(prefs.user_name, Some("Test User".to_string()));
        assert_eq!(prefs.preferred_language, "zh");
    }

    #[test]
    fn test_system_prompt_addition() {
        let mut prefs = UserPreferences::default();
        prefs.user_name = Some("Alice".to_string());
        prefs.preferred_language = "zh".to_string();
        prefs.timezone = "Asia/Shanghai".to_string();
        
        let addition = prefs.to_system_prompt_addition();
        assert!(addition.contains("Alice"));
        assert!(addition.contains("zh"));
        assert!(addition.contains("Asia/Shanghai"));
    }

    #[test]
    fn test_preferences_serialization() {
        let mut prefs = UserPreferences::default();
        prefs.user_name = Some("Bob".to_string());
        prefs.agent_persona = Some("Helpful assistant".to_string());
        
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: UserPreferences = serde_json::from_str(&json).unwrap();
        
        assert_eq!(loaded.user_name, Some("Bob".to_string()));
        assert_eq!(loaded.agent_persona, Some("Helpful assistant".to_string()));
    }
}
