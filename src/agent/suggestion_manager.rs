//! Suggestion Manager Module
//!
//! Manages active suggestions for sessions with feedback tracking.
//! Suggestions can be accepted or dismissed, and this feedback is used
//! to improve future suggestions.

use crate::agent::suggestion::{Suggestion, SuggestionType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;

/// Maximum suggestions to keep per session
const MAX_SUGGESTIONS_PER_SESSION: usize = 20;
/// How long suggestions stay active (in seconds)
const SUGGESTION_TTL_SECS: i64 = 3600; // 1 hour

/// A suggestion with feedback state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedSuggestion {
    /// The suggestion itself
    pub suggestion: Suggestion,
    /// Feedback state
    #[serde(default)]
    pub feedback: SuggestionFeedback,
    /// When this suggestion expires
    pub expires_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl TrackedSuggestion {
    /// Create a new tracked suggestion
    pub fn new(suggestion: Suggestion) -> Self {
        Self {
            suggestion,
            feedback: SuggestionFeedback::Pending,
            expires_at: Utc::now() + chrono::Duration::seconds(SUGGESTION_TTL_SECS),
        }
    }

    /// Check if this suggestion has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if this suggestion has received feedback
    pub fn has_feedback(&self) -> bool {
        self.feedback != SuggestionFeedback::Pending
    }
}

/// Feedback state for a suggestion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionFeedback {
    /// No feedback yet
    #[default]
    Pending,
    /// User accepted this suggestion
    Accepted,
    /// User dismissed this suggestion
    Dismissed,
}

/// Summary of a tracked suggestion for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedSuggestionSummary {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub action_label: String,
    pub feedback: SuggestionFeedback,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl From<&TrackedSuggestion> for TrackedSuggestionSummary {
    fn from(ts: &TrackedSuggestion) -> Self {
        Self {
            id: ts.suggestion.id.clone(),
            suggestion_type: ts.suggestion.suggestion_type.clone(),
            title: ts.suggestion.title.clone(),
            description: ts.suggestion.description.clone(),
            action_label: ts.suggestion.action_label.clone(),
            feedback: ts.feedback.clone(),
            confidence: ts.suggestion.confidence,
            created_at: ts.suggestion.created_at,
            expires_at: ts.expires_at,
        }
    }
}

/// Suggestion feedback summary for learning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackStats {
    /// Count of accepted suggestions by type
    pub accepted_by_type: HashMap<String, u32>,
    /// Count of dismissed suggestions by type
    pub dismissed_by_type: HashMap<String, u32>,
    /// Dismissed suggestion signatures (to avoid similar suggestions)
    #[serde(default)]
    pub dismissed_signatures: Vec<String>,
}

impl FeedbackStats {
    /// Record that a suggestion was accepted
    pub fn record_accept(&mut self, suggestion: &Suggestion) {
        let type_key = format!("{:?}", suggestion.suggestion_type);
        *self.accepted_by_type.entry(type_key).or_insert(0) += 1;
    }

    /// Record that a suggestion was dismissed
    pub fn record_dismiss(&mut self, suggestion: &Suggestion) {
        let type_key = format!("{:?}", suggestion.suggestion_type);
        *self.dismissed_by_type.entry(type_key).or_insert(0) += 1;
        // Add signature to avoid similar suggestions
        let sig = self.suggestion_signature(suggestion);
        if !self.dismissed_signatures.contains(&sig) {
            self.dismissed_signatures.push(sig);
        }
    }

    /// Check if a suggestion matches a dismissed pattern
    pub fn is_dismissed(&self, suggestion: &Suggestion) -> bool {
        let sig = self.suggestion_signature(suggestion);
        self.dismissed_signatures.contains(&sig)
    }

    /// Get a signature for a suggestion (used to detect dismissed similar suggestions)
    fn suggestion_signature(&self, suggestion: &Suggestion) -> String {
        let type_key = format!("{:?}", suggestion.suggestion_type);
        let title_lower = suggestion.title.to_lowercase();
        format!("{}:{}", type_key, title_lower)
    }

    /// Calculate preference score for a suggestion type (0.0 - 1.0)
    pub fn type_preference(&self, suggestion_type: &SuggestionType) -> f32 {
        let type_key = format!("{:?}", suggestion_type);
        let accepted = self.accepted_by_type.get(&type_key).copied().unwrap_or(0) as f32;
        let dismissed = self.dismissed_by_type.get(&type_key).copied().unwrap_or(0) as f32;
        let total = accepted + dismissed;
        if total == 0.0 {
            0.5 // Neutral
        } else {
            accepted / total
        }
    }
}

/// Manager for session suggestions with feedback tracking
#[allow(dead_code)]
pub struct SuggestionManager {
    /// Active suggestions per session
    suggestions: RwLock<HashMap<String, Vec<TrackedSuggestion>>>,
    /// Feedback stats per session
    feedback_stats: RwLock<HashMap<String, FeedbackStats>>,
    /// Base path for persistence
    base_path: PathBuf,
}

#[allow(dead_code)]
impl SuggestionManager {
    /// Create a new manager with default path
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("suggestions");
        Self::with_path(path)
    }

    /// Create a manager with custom path
    #[allow(dead_code)]
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let manager = Self {
            suggestions: RwLock::new(HashMap::new()),
            feedback_stats: RwLock::new(HashMap::new()),
            base_path: path.clone(),
        };
        manager.load();
        manager
    }

    /// Load suggestions and feedback stats from disk
    fn load(&self) {
        if !self.base_path.exists() {
            return;
        }

        // Load suggestions
        let suggestions_path = self.base_path.join("suggestions.json");
        if suggestions_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&suggestions_path) {
                if let Ok(data) = serde_json::from_str::<HashMap<String, Vec<TrackedSuggestion>>>(&content) {
                    let mut suggestions = self.suggestions.write();
                    *suggestions = data;
                    // Filter out expired suggestions
                    for (_session_id, session_sugs) in suggestions.iter_mut() {
                        session_sugs.retain(|s| !s.is_expired());
                        if session_sugs.is_empty() {
                            // Mark for removal
                        }
                    }
                    suggestions.retain(|_, v| !v.is_empty());
                }
            }
        }

        // Load feedback stats
        let stats_path = self.base_path.join("feedback_stats.json");
        if stats_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&stats_path) {
                if let Ok(data) = serde_json::from_str::<HashMap<String, FeedbackStats>>(&content) {
                    let mut stats = self.feedback_stats.write();
                    *stats = data;
                }
            }
        }

        tracing::info!("Loaded suggestion manager from {:?}", self.base_path);
    }

    /// Save suggestions and stats to disk
    fn save(&self) {
        // Ensure directory exists
        if let Some(parent) = self.base_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Save suggestions
        let suggestions = self.suggestions.read();
        if let Ok(content) = serde_json::to_string_pretty(&*suggestions) {
            let path = self.base_path.join("suggestions.json");
            let _ = std::fs::write(&path, content);
        }

        // Save feedback stats
        let stats = self.feedback_stats.read();
        if let Ok(content) = serde_json::to_string_pretty(&*stats) {
            let path = self.base_path.join("feedback_stats.json");
            let _ = std::fs::write(&path, content);
        }
    }

    /// Add new suggestions for a session (replacing any expired/old ones)
    pub fn set_suggestions(&self, session_id: &str, new_suggestions: Vec<Suggestion>) {
        let mut suggestions = self.suggestions.write();
        
        // Get existing pending suggestions
        let existing: Vec<_> = suggestions
            .get(session_id)
            .map(|sugs| {
                sugs.iter()
                    .filter(|s| !s.is_expired() && s.feedback == SuggestionFeedback::Pending)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Also keep recently acted-on suggestions briefly for UX
        let recent_acted: Vec<_> = suggestions
            .get(session_id)
            .map(|sugs| {
                sugs.iter()
                    .filter(|s| {
                        s.feedback != SuggestionFeedback::Pending 
                        && Utc::now() - s.suggestion.created_at < chrono::Duration::minutes(5)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Convert new suggestions to tracked
        let tracked: Vec<TrackedSuggestion> = new_suggestions
            .into_iter()
            .map(TrackedSuggestion::new)
            .collect();

        // Merge: keep existing pending + recent acted + new
        let mut all: Vec<TrackedSuggestion> = Vec::new();
        all.extend(existing);
        all.extend(recent_acted);
        all.extend(tracked);

        // Limit total per session
        all.sort_by(|a, b| b.suggestion.confidence.partial_cmp(&a.suggestion.confidence).unwrap());
        all.truncate(MAX_SUGGESTIONS_PER_SESSION);

        suggestions.insert(session_id.to_string(), all);
        drop(suggestions);
        self.save();
    }

    /// List active suggestions for a session (excludes expired)
    pub fn list(&self, session_id: &str) -> Vec<TrackedSuggestion> {
        let suggestions = self.suggestions.read();
        suggestions
            .get(session_id)
            .map(|sugs| {
                sugs.iter()
                    .filter(|s| !s.is_expired() && s.feedback == SuggestionFeedback::Pending)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all tracked suggestions (including acted upon, for UI)
    pub fn list_all(&self, session_id: &str) -> Vec<TrackedSuggestion> {
        let suggestions = self.suggestions.read();
        suggestions
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// List summaries for API responses
    pub fn list_summaries(&self, session_id: &str) -> Vec<TrackedSuggestionSummary> {
        self.list_all(session_id)
            .iter()
            .map(TrackedSuggestionSummary::from)
            .collect()
    }

    /// Accept a suggestion
    pub fn accept(&self, session_id: &str, suggestion_id: &str) -> Option<Suggestion> {
        let mut suggestions = self.suggestions.write();
        if let Some(session_sugs) = suggestions.get_mut(session_id) {
            if let Some(ts) = session_sugs.iter_mut().find(|s| s.suggestion.id == suggestion_id) {
                ts.feedback = SuggestionFeedback::Accepted;
                let suggestion = ts.suggestion.clone();
                
                // Update feedback stats
                drop(suggestions);
                let mut stats = self.feedback_stats.write();
                stats.entry(session_id.to_string()).or_default().record_accept(&suggestion);
                drop(stats);
                self.save();
                
                return Some(suggestion);
            }
        }
        None
    }

    /// Dismiss a suggestion
    pub fn dismiss(&self, session_id: &str, suggestion_id: &str) -> bool {
        let mut suggestions = self.suggestions.write();
        if let Some(session_sugs) = suggestions.get_mut(session_id) {
            if let Some(ts) = session_sugs.iter_mut().find(|s| s.suggestion.id == suggestion_id) {
                ts.feedback = SuggestionFeedback::Dismissed;
                let suggestion = ts.suggestion.clone();
                
                // Update feedback stats
                drop(suggestions);
                let mut stats = self.feedback_stats.write();
                stats.entry(session_id.to_string()).or_default().record_dismiss(&suggestion);
                drop(stats);
                self.save();
                
                return true;
            }
        }
        false
    }

    /// Get feedback stats for a session (to help filter suggestions)
    pub fn get_feedback_stats(&self, session_id: &str) -> FeedbackStats {
        let stats = self.feedback_stats.read();
        stats.get(session_id).cloned().unwrap_or_default()
    }

    /// Check if a suggestion matches a dismissed pattern
    pub fn should_filter(&self, session_id: &str, suggestion: &Suggestion) -> bool {
        let stats = self.feedback_stats.read();
        stats
            .get(session_id)
            .map(|s| s.is_dismissed(suggestion))
            .unwrap_or(false)
    }

    /// Get preferred suggestion types for a session
    pub fn get_preferred_types(&self, session_id: &str) -> HashMap<SuggestionType, f32> {
        let stats = self.feedback_stats.read();
        let default_stats = FeedbackStats::default();
        let session_stats = stats.get(session_id).unwrap_or(&default_stats);
        let mut result = HashMap::new();
        for st in [
            SuggestionType::FollowUp,
            SuggestionType::Action,
            SuggestionType::Information,
            SuggestionType::Reminder,
            SuggestionType::Task,
        ] {
            result.insert(st.clone(), session_stats.type_preference(&st));
        }
        result
    }

    /// Clear all suggestions for a session
    #[allow(dead_code)]
    pub fn clear(&self, session_id: &str) -> bool {
        let mut suggestions = self.suggestions.write();
        if suggestions.remove(session_id).is_some() {
            drop(suggestions);
            self.save();
            return true;
        }
        false
    }

    /// Count active suggestions for a session
    pub fn count(&self, session_id: &str) -> usize {
        self.list(session_id).len()
    }

    /// Clean up expired suggestions (can be called periodically)
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) {
        let mut suggestions = self.suggestions.write();
        for session_sugs in suggestions.values_mut() {
            session_sugs.retain(|s| !s.is_expired());
        }
        suggestions.retain(|_, v| !v.is_empty());
        drop(suggestions);
        self.save();
    }
}

impl Default for SuggestionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_suggestion() -> Suggestion {
        Suggestion::new(
            SuggestionType::Action,
            "Read file",
            "Would you like me to read this file?",
            "Do it",
            0.8,
        )
    }

    #[test]
    fn test_tracked_suggestion_new() {
        let suggestion = create_test_suggestion();
        let ts = TrackedSuggestion::new(suggestion.clone());
        assert_eq!(ts.suggestion.title, "Read file");
        assert_eq!(ts.feedback, SuggestionFeedback::Pending);
        assert!(!ts.is_expired());
    }

    #[test]
    fn test_feedback_stats_record_accept() {
        let mut stats = FeedbackStats::default();
        let suggestion = create_test_suggestion();
        stats.record_accept(&suggestion);
        assert_eq!(stats.accepted_by_type.get("Action"), Some(&1));
    }

    #[test]
    fn test_feedback_stats_record_dismiss() {
        let mut stats = FeedbackStats::default();
        let suggestion = create_test_suggestion();
        stats.record_dismiss(&suggestion);
        assert_eq!(stats.dismissed_by_type.get("Action"), Some(&1));
        assert!(stats.is_dismissed(&suggestion));
    }

    #[test]
    fn test_feedback_stats_type_preference() {
        let mut stats = FeedbackStats::default();
        let suggestion = create_test_suggestion();
        stats.record_accept(&suggestion);
        stats.record_dismiss(&suggestion);
        assert_eq!(stats.type_preference(&SuggestionType::Action), 0.5);
    }

    #[test]
    fn test_suggestion_manager_add_and_list() {
        let manager = SuggestionManager::new();
        let suggestion = create_test_suggestion();
        manager.set_suggestions("session1", vec![suggestion.clone()]);
        
        let list = manager.list("session1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].suggestion.title, "Read file");
    }

    #[test]
    fn test_suggestion_manager_accept() {
        let manager = SuggestionManager::new();
        let suggestion = create_test_suggestion();
        let id = suggestion.id.clone();
        manager.set_suggestions("session1", vec![suggestion]);
        
        let accepted = manager.accept("session1", &id);
        assert!(accepted.is_some());
        assert_eq!(manager.count("session1"), 0); // Accepted removed from active list
    }

    #[test]
    fn test_suggestion_manager_dismiss() {
        let manager = SuggestionManager::new();
        let suggestion = create_test_suggestion();
        let id = suggestion.id.clone();
        manager.set_suggestions("session1", vec![suggestion]);
        
        assert!(manager.dismiss("session1", &id));
        assert_eq!(manager.count("session1"), 0);
        
        // Check feedback stats
        let stats = manager.get_feedback_stats("session1");
        assert!(stats.is_dismissed(&create_test_suggestion())); // Similar suggestion should be filtered
    }

    #[test]
    fn test_suggestion_manager_filter() {
        let manager = SuggestionManager::new();
        let suggestion = create_test_suggestion();
        manager.set_suggestions("session1", vec![suggestion.clone()]);
        manager.dismiss("session1", &suggestion.id);
        
        // Similar suggestion should be filtered
        let similar = Suggestion::new(
            SuggestionType::Action,
            "Read file", // Same title
            "Another description",
            "Do it",
            0.8,
        );
        assert!(manager.should_filter("session1", &similar));
        
        // Different suggestion should not be filtered
        let different = Suggestion::new(
            SuggestionType::FollowUp,
            "Explain more", // Different title
            "Would you like an explanation?",
            "Explain",
            0.7,
        );
        assert!(!manager.should_filter("session1", &different));
    }

    #[test]
    fn test_suggestion_manager_clear() {
        let manager = SuggestionManager::new();
        manager.set_suggestions("session1", vec![create_test_suggestion()]);
        assert_eq!(manager.count("session1"), 1);
        manager.clear("session1");
        assert_eq!(manager.count("session1"), 0);
    }

    #[test]
    fn test_list_summaries() {
        let manager = SuggestionManager::new();
        manager.set_suggestions("session1", vec![create_test_suggestion()]);
        
        let summaries = manager.list_summaries("session1");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Read file");
        assert_eq!(summaries[0].feedback, SuggestionFeedback::Pending);
    }

    #[test]
    fn test_preferred_types() {
        let manager = SuggestionManager::new();
        
        let mut suggestion = create_test_suggestion();
        suggestion.suggestion_type = SuggestionType::Reminder;
        let suggestion_id = suggestion.id.clone();
        manager.set_suggestions("session1", vec![suggestion]);
        manager.accept("session1", &suggestion_id);
        
        let prefs = manager.get_preferred_types("session1");
        assert_eq!(prefs[&SuggestionType::Reminder], 1.0);
    }
}
