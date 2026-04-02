//! Turn Feedback Module
//!
//! Tracks user feedback on agent responses (thumbs up/down + optional comments).
//! This signal helps improve agent behavior over time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use parking_lot::RwLock;

/// Feedback rating enum representing user's assessment of a turn
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRating {
    /// Positive feedback - user found the response helpful
    ThumbsUp,
    /// Negative feedback - user found the response unhelpful
    ThumbsDown,
    /// Neutral feedback - neither positive nor negative
    #[default]
    Neutral,
}

impl FeedbackRating {
    /// Parse from string (for HTTP API)
    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "thumbs_up" | "thumbsup" | "positive" => Some(Self::ThumbsUp),
            "thumbs_down" | "thumbsdown" | "negative" => Some(Self::ThumbsDown),
            "neutral" => Some(Self::Neutral),
            _ => None,
        }
    }

    /// Convert to string for display
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ThumbsUp => "thumbs_up",
            Self::ThumbsDown => "thumbs_down",
            Self::Neutral => "neutral",
        }
    }

    /// Get emoji representation
    #[allow(dead_code)]
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::ThumbsUp => "👍",
            Self::ThumbsDown => "👎",
            Self::Neutral => "😐",
        }
    }
}

/// Feedback for a specific turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFeedback {
    /// The turn ID this feedback is for
    pub turn_id: String,
    /// The session ID this turn belongs to
    pub session_id: String,
    /// The rating given by the user
    pub rating: FeedbackRating,
    /// Optional comment from the user (max 500 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// When this feedback was created
    pub created_at: DateTime<Utc>,
}

impl TurnFeedback {
    /// Create new feedback
    pub fn new(turn_id: impl Into<String>, session_id: impl Into<String>, rating: FeedbackRating) -> Self {
        Self {
            turn_id: turn_id.into(),
            session_id: session_id.into(),
            rating,
            comment: None,
            created_at: Utc::now(),
        }
    }

    /// Add a comment to the feedback
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        let comment_text = comment.into();
        // Truncate to 500 chars if needed
        self.comment = if comment_text.len() > 500 {
            Some(format!("{}...", &comment_text[..497]))
        } else if comment_text.is_empty() {
            None
        } else {
            Some(comment_text)
        };
        self
    }

    /// Check if this is positive feedback
    pub fn is_positive(&self) -> bool {
        self.rating == FeedbackRating::ThumbsUp
    }

    /// Check if this is negative feedback
    pub fn is_negative(&self) -> bool {
        self.rating == FeedbackRating::ThumbsDown
    }
}

/// Summary of feedback for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFeedbackSummary {
    /// The session ID
    pub session_id: String,
    /// Total number of feedback entries
    pub turn_count: u32,
    /// Number of thumbs up ratings
    pub thumbs_up_count: u32,
    /// Number of thumbs down ratings
    pub thumbs_down_count: u32,
    /// Number of neutral ratings
    pub neutral_count: u32,
    /// Positive rate (thumbs_up / total)
    pub positive_rate: f32,
}

impl TurnFeedbackSummary {
    /// Create an empty summary for a session
    #[allow(dead_code)]
    pub fn empty(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            turn_count: 0,
            thumbs_up_count: 0,
            thumbs_down_count: 0,
            neutral_count: 0,
            positive_rate: 0.0,
        }
    }

    /// Create a summary from a list of feedback entries
    pub fn from_feedback(session_id: impl Into<String>, feedbacks: &[TurnFeedback]) -> Self {
        let session_id = session_id.into();
        let turn_count = feedbacks.len() as u32;
        let thumbs_up_count = feedbacks.iter().filter(|f| f.is_positive()).count() as u32;
        let thumbs_down_count = feedbacks.iter().filter(|f| f.is_negative()).count() as u32;
        let neutral_count = feedbacks.iter().filter(|f| f.rating == FeedbackRating::Neutral).count() as u32;
        
        let positive_rate = if turn_count > 0 {
            thumbs_up_count as f32 / turn_count as f32
        } else {
            0.0
        };

        Self {
            session_id,
            turn_count,
            thumbs_up_count,
            thumbs_down_count,
            neutral_count,
            positive_rate,
        }
    }
}

/// Manages turn feedback storage in memory with persistence to JSON
pub struct TurnFeedbackManager {
    /// In-memory feedback by turn_id
    feedback_by_turn: RwLock<HashMap<String, TurnFeedback>>,
    /// Feedback index by session_id for quick lookup
    feedback_by_session: RwLock<HashMap<String, Vec<String>>>,
    /// Persistence directory
    persist_dir: Option<PathBuf>,
    /// Maximum feedback entries per session
    max_feedback_per_session: usize,
}

impl TurnFeedbackManager {
    /// Create a new in-memory feedback manager
    pub fn new() -> Self {
        Self {
            feedback_by_turn: RwLock::new(HashMap::new()),
            feedback_by_session: RwLock::new(HashMap::new()),
            persist_dir: None,
            max_feedback_per_session: 100,
        }
    }

    /// Create a new feedback manager with persistence
    pub fn new_with_persistence<P: AsRef<std::path::Path>>(dir: P) -> crate::common::Result<Self> {
        let persist_dir = dir.as_ref().to_path_buf();
        
        // Ensure directory exists
        fs::create_dir_all(&persist_dir)?;
        
        let mut manager = Self {
            feedback_by_turn: RwLock::new(HashMap::new()),
            feedback_by_session: RwLock::new(HashMap::new()),
            persist_dir: Some(persist_dir),
            max_feedback_per_session: 100,
        };
        
        // Load existing feedback from disk
        manager.load_from_disk()?;
        
        Ok(manager)
    }

    /// Get the default persistence path
    #[allow(dead_code)]
    pub fn default_persist_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("turn_feedback")
    }

    /// Load feedback from disk
    fn load_from_disk(&mut self) -> crate::common::Result<()> {
        let Some(dir) = &self.persist_dir else {
            return Ok(());
        };
        
        if !dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)?;
        let mut feedback_by_turn = self.feedback_by_turn.write();
        let mut feedback_by_session = self.feedback_by_session.write();
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(feedback) = serde_json::from_str::<TurnFeedback>(&content) {
                        // Index by turn_id
                        feedback_by_turn.insert(feedback.turn_id.clone(), feedback.clone());
                        
                        // Index by session_id
                        feedback_by_session
                            .entry(feedback.session_id.clone())
                            .or_default()
                            .push(feedback.turn_id.clone());
                    }
                }
            }
        }
        
        tracing::info!("Loaded {} feedback entries from {:?}", feedback_by_turn.len(), dir);
        Ok(())
    }

    /// Save a single feedback to disk
    fn save_to_disk(&self, feedback: &TurnFeedback) {
        let Some(dir) = &self.persist_dir else {
            return;
        };
        
        let filename = format!("{}.json", feedback.turn_id);
        let path = dir.join(filename);
        
        match serde_json::to_string_pretty(feedback) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    tracing::error!("Failed to save feedback to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize feedback: {}", e);
            }
        }
    }

    /// Delete feedback from disk
    fn delete_from_disk(&self, turn_id: &str) {
        let Some(dir) = &self.persist_dir else {
            return;
        };
        
        let filename = format!("{}.json", turn_id);
        let path = dir.join(filename);
        let _ = fs::remove_file(path);
    }

    /// Record feedback for a turn
    pub fn record_feedback(
        &self,
        turn_id: impl Into<String>,
        session_id: impl Into<String>,
        rating: FeedbackRating,
        comment: Option<String>,
    ) -> TurnFeedback {
        let turn_id = turn_id.into();
        let session_id = session_id.into();
        
        let mut feedback = TurnFeedback::new(turn_id.clone(), session_id.clone(), rating);
        if let Some(c) = comment {
            feedback = feedback.with_comment(c);
        }
        
        // Store in memory
        {
            let mut by_turn = self.feedback_by_turn.write();
            by_turn.insert(turn_id.clone(), feedback.clone());
        }
        
        {
            let mut by_session = self.feedback_by_session.write();
            let session_feedback = by_session.entry(session_id.clone()).or_default();
            
            // Check if turn_id already exists
            if !session_feedback.contains(&turn_id) {
                session_feedback.push(turn_id.clone());
                
                // Trim to max size (keep most recent)
                if session_feedback.len() > self.max_feedback_per_session {
                    let remove_count = session_feedback.len() - self.max_feedback_per_session;
                    let removed_ids: Vec<_> = session_feedback.drain(0..remove_count).collect();
                    
                    // Remove from by_turn index
                    let mut by_turn = self.feedback_by_turn.write();
                    for id in removed_ids {
                        by_turn.remove(&id);
                        self.delete_from_disk(&id);
                    }
                }
            }
        }
        
        // Save to disk
        self.save_to_disk(&feedback);
        
        tracing::debug!(
            turn_id = %feedback.turn_id,
            session_id = %feedback.session_id,
            rating = ?feedback.rating,
            "Recorded turn feedback"
        );
        
        feedback
    }

    /// Get feedback for a specific turn
    pub fn get_turn_feedback(&self, turn_id: &str) -> Option<TurnFeedback> {
        let by_turn = self.feedback_by_turn.read();
        by_turn.get(turn_id).cloned()
    }

    /// Get feedback summary for a session
    pub fn get_session_feedback_summary(&self, session_id: &str) -> TurnFeedbackSummary {
        let feedbacks = self.get_session_feedback(session_id);
        TurnFeedbackSummary::from_feedback(session_id, &feedbacks)
    }

    /// Get all feedback for a session
    pub fn get_session_feedback(&self, session_id: &str) -> Vec<TurnFeedback> {
        let by_session = self.feedback_by_session.read();
        let by_turn = self.feedback_by_turn.read();
        
        by_session
            .get(session_id)
            .map(|turn_ids| {
                turn_ids
                    .iter()
                    .filter_map(|id| by_turn.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get recent feedback entries for a session
    #[allow(dead_code)]
    pub fn get_recent_feedback(&self, session_id: &str, limit: usize) -> Vec<TurnFeedback> {
        let mut feedbacks = self.get_session_feedback(session_id);
        
        // Sort by creation time (most recent first)
        feedbacks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Limit results
        feedbacks.into_iter().take(limit).collect()
    }

    /// Get IDs of positively-rated turns for a session (for learning)
    #[allow(dead_code)]
    pub fn get_positive_turn_ids(&self, session_id: &str) -> Vec<String> {
        let feedbacks = self.get_session_feedback(session_id);
        feedbacks
            .into_iter()
            .filter(|f| f.is_positive())
            .map(|f| f.turn_id)
            .collect()
    }

    /// Get IDs of negatively-rated turns for a session (for learning)
    #[allow(dead_code)]
    pub fn get_negative_turn_ids(&self, session_id: &str) -> Vec<String> {
        let feedbacks = self.get_session_feedback(session_id);
        feedbacks
            .into_iter()
            .filter(|f| f.is_negative())
            .map(|f| f.turn_id)
            .collect()
    }

    /// Get all sessions that have feedback
    pub fn get_sessions_with_feedback(&self) -> Vec<String> {
        let by_session = self.feedback_by_session.read();
        by_session.keys().cloned().collect()
    }

    /// Get total feedback count
    #[allow(dead_code)]
    pub fn total_feedback_count(&self) -> usize {
        let by_turn = self.feedback_by_turn.read();
        by_turn.len()
    }

    /// Get global feedback statistics
    pub fn get_global_stats(&self) -> GlobalFeedbackStats {
        let by_turn = self.feedback_by_turn.read();
        
        let total = by_turn.len();
        let thumbs_up = by_turn.values().filter(|f| f.is_positive()).count();
        let thumbs_down = by_turn.values().filter(|f| f.is_negative()).count();
        let neutral = by_turn.values().filter(|f| f.rating == FeedbackRating::Neutral).count();
        
        GlobalFeedbackStats {
            total_feedback: total as u32,
            thumbs_up_count: thumbs_up as u32,
            thumbs_down_count: thumbs_down as u32,
            neutral_count: neutral as u32,
            positive_rate: if total > 0 { thumbs_up as f32 / total as f32 } else { 0.0 },
        }
    }

    /// Clear all feedback for a session
    #[allow(dead_code)]
    pub fn clear_session_feedback(&self, session_id: &str) {
        let turn_ids: Vec<String>;
        
        // Get turn IDs to remove
        {
            let mut by_session = self.feedback_by_session.write();
            turn_ids = by_session.remove(session_id).unwrap_or_default();
        }
        
        // Remove from by_turn and disk
        {
            let mut by_turn = self.feedback_by_turn.write();
            for id in &turn_ids {
                by_turn.remove(id);
                self.delete_from_disk(id);
            }
        }
        
        tracing::info!(session_id = %session_id, count = turn_ids.len(), "Cleared session feedback");
    }

    /// Clear all feedback
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        // Get all turn IDs
        let turn_ids: Vec<String>;
        {
            let by_turn = self.feedback_by_turn.read();
            turn_ids = by_turn.keys().cloned().collect();
        }
        
        // Remove from memory
        {
            let mut by_turn = self.feedback_by_turn.write();
            by_turn.clear();
        }
        {
            let mut by_session = self.feedback_by_session.write();
            by_session.clear();
        }
        
        // Remove from disk
        for id in &turn_ids {
            self.delete_from_disk(id);
        }
        
        tracing::info!("Cleared all feedback");
    }
}

impl Default for TurnFeedbackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global feedback statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFeedbackStats {
    /// Total number of feedback entries
    pub total_feedback: u32,
    /// Number of thumbs up ratings
    pub thumbs_up_count: u32,
    /// Number of thumbs down ratings
    pub thumbs_down_count: u32,
    /// Number of neutral ratings
    pub neutral_count: u32,
    /// Positive rate (thumbs_up / total)
    pub positive_rate: f32,
}

// ============================================================================
// HTTP API Types
// ============================================================================

/// Request body for submitting feedback
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitFeedbackRequest {
    /// The turn ID this feedback is for
    pub turn_id: String,
    /// The session ID this turn belongs to
    pub session_id: String,
    /// The rating (as string: "thumbs_up", "thumbs_down", "neutral")
    pub rating: String,
    /// Optional comment (max 500 chars)
    #[serde(default)]
    pub comment: Option<String>,
}

impl SubmitFeedbackRequest {
    /// Parse the rating string into FeedbackRating
    pub fn parse_rating(&self) -> Option<FeedbackRating> {
        FeedbackRating::parse_from_str(&self.rating)
    }
}

/// Response for feedback submission
#[derive(Debug, Clone, Serialize)]
pub struct SubmitFeedbackResponse {
    /// Whether the submission was successful
    pub success: bool,
    /// The turn ID
    pub turn_id: String,
    /// The session ID
    pub session_id: String,
    /// The rating that was recorded
    pub rating: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for getting feedback
#[derive(Debug, Clone, Serialize)]
pub struct GetFeedbackResponse {
    /// The feedback entry, if found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<TurnFeedback>,
    /// Error message if not found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for session feedback list
#[derive(Debug, Clone, Serialize)]
pub struct SessionFeedbackResponse {
    /// The session ID
    pub session_id: String,
    /// List of feedback entries
    pub feedback: Vec<TurnFeedback>,
    /// Total count
    pub count: usize,
}

/// Response for session feedback summary
#[derive(Debug, Clone, Serialize)]
pub struct SessionFeedbackSummaryResponse {
    /// The summary
    pub summary: TurnFeedbackSummary,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_rating_parsing() {
        assert_eq!(FeedbackRating::parse_from_str("thumbs_up"), Some(FeedbackRating::ThumbsUp));
        assert_eq!(FeedbackRating::parse_from_str("thumbs_down"), Some(FeedbackRating::ThumbsDown));
        assert_eq!(FeedbackRating::parse_from_str("neutral"), Some(FeedbackRating::Neutral));
        assert_eq!(FeedbackRating::parse_from_str("THUMBS_UP"), Some(FeedbackRating::ThumbsUp));
        assert_eq!(FeedbackRating::parse_from_str("invalid"), None);
    }

    #[test]
    fn test_feedback_rating_serialization() {
        let rating = FeedbackRating::ThumbsUp;
        let json = serde_json::to_string(&rating).unwrap();
        assert_eq!(json, "\"thumbs_up\"");
        
        let parsed: FeedbackRating = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, FeedbackRating::ThumbsUp);
    }

    #[test]
    fn test_turn_feedback_creation() {
        let feedback = TurnFeedback::new("turn-123", "session-456", FeedbackRating::ThumbsUp)
            .with_comment("Great response!");
        
        assert_eq!(feedback.turn_id, "turn-123");
        assert_eq!(feedback.session_id, "session-456");
        assert_eq!(feedback.rating, FeedbackRating::ThumbsUp);
        assert_eq!(feedback.comment, Some("Great response!".to_string()));
        assert!(feedback.is_positive());
        assert!(!feedback.is_negative());
    }

    #[test]
    fn test_turn_feedback_comment_truncation() {
        let long_comment = "x".repeat(600);
        let feedback = TurnFeedback::new("turn-1", "session-1", FeedbackRating::ThumbsUp)
            .with_comment(long_comment.clone());
        
        assert!(feedback.comment.unwrap().len() <= 500);
    }

    #[test]
    fn test_turn_feedback_summary() {
        let feedbacks = vec![
            TurnFeedback::new("t1", "s1", FeedbackRating::ThumbsUp),
            TurnFeedback::new("t2", "s1", FeedbackRating::ThumbsUp),
            TurnFeedback::new("t3", "s1", FeedbackRating::ThumbsDown),
            TurnFeedback::new("t4", "s1", FeedbackRating::Neutral),
        ];
        
        let summary = TurnFeedbackSummary::from_feedback("s1", &feedbacks);
        
        assert_eq!(summary.session_id, "s1");
        assert_eq!(summary.turn_count, 4);
        assert_eq!(summary.thumbs_up_count, 2);
        assert_eq!(summary.thumbs_down_count, 1);
        assert_eq!(summary.neutral_count, 1);
        assert!((summary.positive_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_turn_feedback_manager() {
        let manager = TurnFeedbackManager::new();
        
        // Record feedback
        let feedback = manager.record_feedback(
            "turn-1",
            "session-1",
            FeedbackRating::ThumbsUp,
            Some("Great!".to_string()),
        );
        
        assert_eq!(feedback.turn_id, "turn-1");
        assert_eq!(feedback.rating, FeedbackRating::ThumbsUp);
        
        // Get feedback
        let retrieved = manager.get_turn_feedback("turn-1").unwrap();
        assert_eq!(retrieved.turn_id, "turn-1");
        
        // Get session feedback
        let session_feedback = manager.get_session_feedback("session-1");
        assert_eq!(session_feedback.len(), 1);
        
        // Get summary
        let summary = manager.get_session_feedback_summary("session-1");
        assert_eq!(summary.turn_count, 1);
        assert_eq!(summary.thumbs_up_count, 1);
        
        // Get positive turn IDs
        let positive_ids = manager.get_positive_turn_ids("session-1");
        assert_eq!(positive_ids, vec!["turn-1"]);
        
        // Get negative turn IDs
        let negative_ids = manager.get_negative_turn_ids("session-1");
        assert!(negative_ids.is_empty());
    }

    #[test]
    fn test_turn_feedback_manager_multiple_sessions() {
        let manager = TurnFeedbackManager::new();
        
        // Record feedback for multiple sessions
        manager.record_feedback("t1", "s1", FeedbackRating::ThumbsUp, None);
        manager.record_feedback("t2", "s1", FeedbackRating::ThumbsDown, None);
        manager.record_feedback("t3", "s2", FeedbackRating::ThumbsUp, None);
        
        // Check each session
        let s1_feedback = manager.get_session_feedback("s1");
        assert_eq!(s1_feedback.len(), 2);
        
        let s2_feedback = manager.get_session_feedback("s2");
        assert_eq!(s2_feedback.len(), 1);
        
        // Check global stats
        let stats = manager.get_global_stats();
        assert_eq!(stats.total_feedback, 3);
        assert_eq!(stats.thumbs_up_count, 2);
        assert_eq!(stats.thumbs_down_count, 1);
    }

    #[test]
    fn test_turn_feedback_manager_max_per_session() {
        let mut manager = TurnFeedbackManager::new();
        manager.max_feedback_per_session = 5;
        
        // Record more than max
        for i in 0..10 {
            manager.record_feedback(format!("t{}", i), "s1", FeedbackRating::ThumbsUp, None);
        }
        
        // Should only have max_feedback_per_session
        let feedback = manager.get_session_feedback("s1");
        assert_eq!(feedback.len(), 5);
        
        // Should have the most recent ones (t5-t9)
        let ids: Vec<_> = feedback.iter().map(|f| f.turn_id.as_str()).collect();
        assert!(ids.contains(&"t5"));
        assert!(ids.contains(&"t9"));
        assert!(!ids.contains(&"t0"));
    }

    #[test]
    fn test_turn_feedback_persistence() {
        use tempfile::tempdir;
        
        let dir = tempdir().unwrap();
        let manager = TurnFeedbackManager::new_with_persistence(dir.path()).unwrap();
        
        // Record feedback
        manager.record_feedback("t1", "s1", FeedbackRating::ThumbsUp, Some("Test".to_string()));
        
        // Create new manager to test loading
        let manager2 = TurnFeedbackManager::new_with_persistence(dir.path()).unwrap();
        
        let feedback = manager2.get_turn_feedback("t1").unwrap();
        assert_eq!(feedback.turn_id, "t1");
        assert_eq!(feedback.rating, FeedbackRating::ThumbsUp);
        assert_eq!(feedback.comment, Some("Test".to_string()));
    }

    #[test]
    fn test_submit_feedback_request_parsing() {
        let request = SubmitFeedbackRequest {
            turn_id: "t1".to_string(),
            session_id: "s1".to_string(),
            rating: "thumbs_up".to_string(),
            comment: Some("Great!".to_string()),
        };
        
        let rating = request.parse_rating().unwrap();
        assert_eq!(rating, FeedbackRating::ThumbsUp);
    }

    #[test]
    fn test_global_stats() {
        let manager = TurnFeedbackManager::new();
        
        manager.record_feedback("t1", "s1", FeedbackRating::ThumbsUp, None);
        manager.record_feedback("t2", "s1", FeedbackRating::ThumbsUp, None);
        manager.record_feedback("t3", "s2", FeedbackRating::ThumbsDown, None);
        
        let stats = manager.get_global_stats();
        assert_eq!(stats.total_feedback, 3);
        assert_eq!(stats.thumbs_up_count, 2);
        assert_eq!(stats.thumbs_down_count, 1);
        assert_eq!(stats.neutral_count, 0);
        assert!((stats.positive_rate - 0.6666667).abs() < 0.001);
    }
}