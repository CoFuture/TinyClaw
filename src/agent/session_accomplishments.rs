//! Session Accomplishments Module
//!
//! Tracks what was accomplished during a session by aggregating turn summaries,
//! tool executions, and detected outcomes. Provides a concise accomplishment
//! report for session continuity and user understanding.
//!
//! ## Example Output
//!
//! ```text
//! 📋 Session Accomplishments
//! ├─ Files Modified: 3
//! │  ├─ src/main.rs (modified)
//! │  ├─ Cargo.toml (added dependency)
//! │  └─ docs/README.md (updated)
//! ├─ Tasks Completed: 2
//! │  ├─ Fixed authentication bug
//! │  └─ Added unit tests
//! ├─ Decisions: 1
//! │  └─ Use JWT for auth tokens
//! └─ Tools Used: 12 (90% success)
//!    ├─ read_file: 5 times
//!    ├─ exec: 4 times
//!    └─ grep: 3 times
//! ```

use crate::agent::turn_summary::AgentTurnSummary;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

/// Type of accomplishment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccomplishmentType {
    /// A file was created or modified
    FileModified,
    /// A task or goal was completed
    TaskCompleted,
    /// A decision was made during the conversation
    DecisionMade,
    /// Information was learned or researched
    InformationLearned,
    /// A problem was identified
    ProblemIdentified,
    /// A problem was fixed
    ProblemFixed,
    /// Code was analyzed or reviewed
    CodeAnalyzed,
    /// Configuration was changed
    ConfigChanged,
    /// General action performed
    ActionPerformed,
}

impl AccomplishmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccomplishmentType::FileModified => "file_modified",
            AccomplishmentType::TaskCompleted => "task_completed",
            AccomplishmentType::DecisionMade => "decision_made",
            AccomplishmentType::InformationLearned => "information_learned",
            AccomplishmentType::ProblemIdentified => "problem_identified",
            AccomplishmentType::ProblemFixed => "problem_fixed",
            AccomplishmentType::CodeAnalyzed => "code_analyzed",
            AccomplishmentType::ConfigChanged => "config_changed",
            AccomplishmentType::ActionPerformed => "action_performed",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AccomplishmentType::FileModified => "Files Modified",
            AccomplishmentType::TaskCompleted => "Tasks Completed",
            AccomplishmentType::DecisionMade => "Decisions Made",
            AccomplishmentType::InformationLearned => "Info Learned",
            AccomplishmentType::ProblemIdentified => "Problems Found",
            AccomplishmentType::ProblemFixed => "Problems Fixed",
            AccomplishmentType::CodeAnalyzed => "Code Analyzed",
            AccomplishmentType::ConfigChanged => "Config Changed",
            AccomplishmentType::ActionPerformed => "Actions Taken",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            AccomplishmentType::FileModified => "📄",
            AccomplishmentType::TaskCompleted => "✅",
            AccomplishmentType::DecisionMade => "💡",
            AccomplishmentType::InformationLearned => "📚",
            AccomplishmentType::ProblemIdentified => "⚠️",
            AccomplishmentType::ProblemFixed => "🔧",
            AccomplishmentType::CodeAnalyzed => "🔍",
            AccomplishmentType::ConfigChanged => "⚙️",
            AccomplishmentType::ActionPerformed => "🎯",
        }
    }
}

/// A single accomplishment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accomplishment {
    /// Unique ID for this accomplishment
    pub id: String,
    /// Type of accomplishment
    pub accomplishment_type: AccomplishmentType,
    /// Brief description of what was accomplished
    pub description: String,
    /// Evidence or details (e.g., file paths, command outputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<String>>,
    /// Which turn this was recorded in
    pub turn_id: String,
    /// When this was recorded
    pub recorded_at: DateTime<Utc>,
    /// Confidence level (0.0-1.0) - how sure we are this is a real accomplishment
    pub confidence: f32,
}

impl Accomplishment {
    /// Create a new accomplishment
    pub fn new(
        accomplishment_type: AccomplishmentType,
        description: String,
        turn_id: String,
        confidence: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            accomplishment_type,
            description,
            evidence: None,
            turn_id,
            recorded_at: Utc::now(),
            confidence,
        }
    }

    /// Create with evidence
    pub fn with_evidence(mut self, evidence: Vec<String>) -> Self {
        self.evidence = Some(evidence);
        self
    }
}

/// Aggregated statistics for a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionAccomplishmentStats {
    /// Total accomplishments recorded
    pub total_count: usize,
    /// Count by type
    pub by_type: HashMap<String, usize>,
    /// Total turns with accomplishments
    pub turns_with_accomplishments: usize,
    /// Tool usage summary (tool_name -> count)
    pub tool_usage: HashMap<String, usize>,
    /// Tool success summary (tool_name -> success_count)
    pub tool_success: HashMap<String, usize>,
    /// Files modified (deduplicated set)
    pub files_modified: Vec<String>,
    /// Success rate (0.0-1.0)
    pub success_rate: f32,
}

impl SessionAccomplishmentStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate success rate from tool success hashmap
    pub fn calculate_success_rate(&mut self) {
        let total: usize = self.tool_success.values().sum();
        let successes: usize = self.tool_success.values().filter(|&&v| v > 0).sum();
        self.success_rate = if total > 0 {
            successes as f32 / total as f32
        } else {
            0.0
        };
    }
}

/// Session accomplishment summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAccomplishmentSummary {
    /// Session ID
    pub session_id: String,
    /// All recorded accomplishments
    pub accomplishments: Vec<Accomplishment>,
    /// Aggregated statistics
    pub stats: SessionAccomplishmentStats,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When this summary was generated
    pub generated_at: DateTime<Utc>,
}

impl SessionAccomplishmentSummary {
    /// Generate a text summary for display
    pub fn to_text_summary(&self) -> String {
        let mut lines = vec![format!("📋 Session Accomplishments ({} turns)", self.stats.turns_with_accomplishments)];

        // Files modified
        if !self.stats.files_modified.is_empty() {
            lines.push(format!("├─ 📄 Files Modified: {}", self.stats.files_modified.len()));
            for (i, file) in self.stats.files_modified.iter().take(5).enumerate() {
                let prefix = if i == self.stats.files_modified.iter().take(5).count().min(self.stats.files_modified.len()) - 1 {
                    "└─"
                } else {
                    "├─"
                };
                lines.push(format!("│  {} {}", prefix, file));
            }
            if self.stats.files_modified.len() > 5 {
                lines.push(format!("│  └─ ... and {} more", self.stats.files_modified.len() - 5));
            }
        }

        // Tasks completed
        let tasks: Vec<_> = self.accomplishments.iter()
            .filter(|a| a.accomplishment_type == AccomplishmentType::TaskCompleted)
            .collect();
        if !tasks.is_empty() {
            lines.push(format!("├─ ✅ Tasks Completed: {}", tasks.len()));
            for (i, task) in tasks.iter().take(5).enumerate() {
                let prefix = if i == tasks.len().min(5) - 1 { "└─" } else { "├─" };
                lines.push(format!("│  {} {}", prefix, task.description));
            }
        }

        // Problems fixed
        let problems: Vec<_> = self.accomplishments.iter()
            .filter(|a| a.accomplishment_type == AccomplishmentType::ProblemFixed)
            .collect();
        if !problems.is_empty() {
            lines.push(format!("├─ 🔧 Problems Fixed: {}", problems.len()));
            for (i, prob) in problems.iter().take(5).enumerate() {
                let prefix = if i == problems.len().min(5) - 1 { "└─" } else { "├─" };
                lines.push(format!("│  {} {}", prefix, prob.description));
            }
        }

        // Tool usage
        if !self.stats.tool_usage.is_empty() {
            let total_tools: usize = self.stats.tool_usage.values().sum();
            let success_pct = (self.stats.success_rate * 100.0) as usize;
            lines.push(format!("├─ 🔧 Tools Used: {} total ({}% success)", total_tools, success_pct));
            let mut sorted_tools: Vec<_> = self.stats.tool_usage.iter().collect();
            sorted_tools.sort_by(|a, b| b.1.cmp(a.1));
            for (i, (tool, count)) in sorted_tools.iter().take(5).enumerate() {
                let prefix = if i == sorted_tools.iter().take(5).count().min(sorted_tools.len()) - 1 {
                    "└─"
                } else {
                    "├─"
                };
                lines.push(format!("│  {} {}: {} times", prefix, tool, count));
            }
        }

        lines.join("\n")
    }

    /// Generate a brief context snippet for injection into system prompt
    pub fn to_context_snippet(&self) -> String {
        let mut parts = vec![];

        if !self.stats.files_modified.is_empty() {
            parts.push(format!("Files modified: {}", self.stats.files_modified.join(", ")));
        }

        let tasks: Vec<_> = self.accomplishments.iter()
            .filter(|a| a.accomplishment_type == AccomplishmentType::TaskCompleted)
            .collect();
        if !tasks.is_empty() {
            let task_descriptions: Vec<_> = tasks.iter().map(|t| t.description.clone()).collect();
            parts.push(format!("Tasks completed: {}", task_descriptions.join("; ")));
        }

        let problems: Vec<_> = self.accomplishments.iter()
            .filter(|a| a.accomplishment_type == AccomplishmentType::ProblemFixed)
            .collect();
        if !problems.is_empty() {
            let prob_descriptions: Vec<_> = problems.iter().map(|p| p.description.clone()).collect();
            parts.push(format!("Problems fixed: {}", prob_descriptions.join("; ")));
        }

        if parts.is_empty() {
            "No major accomplishments recorded yet.".to_string()
        } else {
            format!("Session accomplishments so far: {}", parts.join(". "))
        }
    }
}

/// Manages accomplishments for a session
#[derive(Debug, Clone)]
pub struct SessionAccomplishments {
    /// Session ID
    session_id: String,
    /// Accomplishments recorded in this session
    accomplishments: Vec<Accomplishment>,
    /// When the session started
    started_at: DateTime<Utc>,
}

impl SessionAccomplishments {
    /// Create new accomplishments tracker for a session
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            accomplishments: Vec::new(),
            started_at: Utc::now(),
        }
    }

    /// Record an accomplishment
    pub fn record(&mut self, accomplishment: Accomplishment) {
        self.accomplishments.push(accomplishment);
    }

    /// Record an accomplishment from a turn summary
    pub fn record_from_turn_summary(&mut self, turn_summary: &AgentTurnSummary) {
        let turn_id = turn_summary.turn_id.clone();

        // Record tool executions as accomplishments
        for tool in &turn_summary.tool_summaries {
            if let Some(ac_type) = Self::tool_to_accomplishment_type(&tool.tool_name, &tool.summary, tool.success) {
                let ac = Accomplishment::new(
                    ac_type,
                    tool.summary.clone(),
                    turn_id.clone(),
                    if tool.success { 0.9 } else { 0.5 },
                );
                self.record(ac);
            }
        }

        // If turn was successful and has accomplishment text, record it
        if turn_summary.success && !turn_summary.accomplishment.is_empty() {
            let ac = Accomplishment::new(
                AccomplishmentType::TaskCompleted,
                turn_summary.accomplishment.clone(),
                turn_id,
                0.8,
            );
            self.record(ac);
        }
    }

    /// Determine accomplishment type from tool name and summary
    fn tool_to_accomplishment_type(tool_name: &str, summary: &str, success: bool) -> Option<AccomplishmentType> {
        if !success {
            return None;
        }

        let tool_name = tool_name.to_lowercase();
        let summary_lower = summary.to_lowercase();

        if tool_name.contains("write_file") || tool_name.contains("edit") || tool_name.contains("create") {
            // Check if it's a file modification
            if summary_lower.contains("created") || summary_lower.contains("written") {
                return Some(AccomplishmentType::FileModified);
            }
        }

        if tool_name.contains("exec") {
            if summary_lower.contains("installed") || summary_lower.contains("added") {
                return Some(AccomplishmentType::ConfigChanged);
            }
            if summary_lower.contains("completed") || summary_lower.contains("success") {
                return Some(AccomplishmentType::TaskCompleted);
            }
        }

        if (tool_name.contains("grep") || tool_name.contains("find") || tool_name.contains("read"))
            && (summary_lower.contains("found") || summary_lower.contains("analyzed")) {
            return Some(AccomplishmentType::CodeAnalyzed);
        }

        Some(AccomplishmentType::ActionPerformed)
    }

    /// Extract file paths from accomplishments
    pub fn extract_file_paths(&self) -> Vec<String> {
        let mut files = Vec::new();
        for ac in &self.accomplishments {
            if let Some(ref evidence) = ac.evidence {
                for e in evidence {
                    if e.contains('/') || e.contains('\\') || e.ends_with(".rs") || e.ends_with(".toml") || e.ends_with(".json") {
                        files.push(e.clone());
                    }
                }
            }
            // Also check description for file paths
            let desc = &ac.description;
            let parts: Vec<_> = desc.split_whitespace().collect();
            for part in parts {
                if (part.contains('/') || part.contains('\\')) && (part.ends_with(".rs") || part.ends_with(".toml") || part.ends_with(".json") || part.ends_with(".md") || part.ends_with(".txt") || part.ends_with(".yaml") || part.ends_with(".yml")) {
                    files.push(part.to_string());
                }
            }
        }
        // Deduplicate
        files.sort();
        files.dedup();
        files
    }

    /// Generate statistics
    pub fn generate_stats(&self) -> SessionAccomplishmentStats {
        let mut stats = SessionAccomplishmentStats::new();
        stats.total_count = self.accomplishments.len();
        stats.turns_with_accomplishments = self.accomplishments
            .iter()
            .map(|a| a.turn_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        // Count by type
        for ac in &self.accomplishments {
            let key = ac.accomplishment_type.as_str().to_string();
            *stats.by_type.entry(key).or_insert(0) += 1;
        }

        // Extract file paths
        stats.files_modified = self.extract_file_paths();

        stats
    }

    /// Generate a full summary
    pub fn generate_summary(&self) -> SessionAccomplishmentSummary {
        let mut stats = self.generate_stats();

        // Aggregate tool usage from turn summaries
        // (tool_usage and tool_success would need to be populated during recording)

        stats.calculate_success_rate();

        SessionAccomplishmentSummary {
            session_id: self.session_id.clone(),
            accomplishments: self.accomplishments.clone(),
            stats,
            started_at: self.started_at,
            generated_at: Utc::now(),
        }
    }

    /// Get all accomplishments
    pub fn get_accomplishments(&self) -> &[Accomplishment] {
        &self.accomplishments
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.accomplishments.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.accomplishments.is_empty()
    }
}

/// Manager for session accomplishments across all sessions
#[derive(Clone)]
pub struct SessionAccomplishmentsManager {
    /// Per-session accomplishments (Arc<RwLock> so clones share state)
    sessions: Arc<parking_lot::RwLock<HashMap<String, Arc<RwLock<SessionAccomplishments>>>>>,
    /// Data directory for persistence
    #[allow(dead_code)]
    data_dir: PathBuf,
}

impl SessionAccomplishmentsManager {
    /// Create a new manager
    pub fn new(data_dir: PathBuf) -> Self {
        // Ensure directory exists
        std::fs::create_dir_all(&data_dir).ok();
        Self {
            sessions: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Get or create accomplishments for a session
    pub fn get_or_create(&self, session_id: &str) -> Arc<RwLock<SessionAccomplishments>> {
        let mut sessions = self.sessions.write();
        if let Some(existing) = sessions.get(session_id) {
            return existing.clone();
        }
        let new_ac = Arc::new(RwLock::new(SessionAccomplishments::new(session_id.to_string())));
        sessions.insert(session_id.to_string(), new_ac.clone());
        new_ac
    }

    /// Record an accomplishment for a session
    pub fn record_accomplishment(&self, session_id: &str, accomplishment: Accomplishment) {
        let ac = self.get_or_create(session_id);
        ac.write().record(accomplishment);
    }

    /// Record from a turn summary
    pub fn record_from_turn_summary(&self, session_id: &str, turn_summary: &AgentTurnSummary) {
        let ac = self.get_or_create(session_id);
        ac.write().record_from_turn_summary(turn_summary);
    }

    /// Get summary for a session
    pub fn get_summary(&self, session_id: &str) -> Option<SessionAccomplishmentSummary> {
        let sessions = self.sessions.read();
        sessions.get(session_id).map(|ac| ac.read().generate_summary())
    }

    /// Get all session IDs with accomplishments
    pub fn get_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read();
        sessions.keys().cloned().collect()
    }

    /// Clear accomplishments for a session
    pub fn clear(&self, session_id: &str) {
        let mut sessions = self.sessions.write();
        sessions.remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accomplishment_creation() {
        let ac = Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Fixed authentication bug".to_string(),
            "turn_1".to_string(),
            0.9,
        );
        assert_eq!(ac.accomplishment_type, AccomplishmentType::TaskCompleted);
        assert_eq!(ac.description, "Fixed authentication bug");
        assert_eq!(ac.confidence, 0.9);
    }

    #[test]
    fn test_accomplishment_with_evidence() {
        let ac = Accomplishment::new(
            AccomplishmentType::FileModified,
            "Updated config".to_string(),
            "turn_1".to_string(),
            0.95,
        ).with_evidence(vec!["config.toml".to_string(), "settings.json".to_string()]);

        assert!(ac.evidence.is_some());
        assert_eq!(ac.evidence.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_session_accomplishments_record() {
        let mut sac = SessionAccomplishments::new("session_1".to_string());

        sac.record(Accomplishment::new(
            AccomplishmentType::FileModified,
            "Created src/main.rs".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        sac.record(Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Set up project structure".to_string(),
            "turn_1".to_string(),
            0.85,
        ));

        assert_eq!(sac.len(), 2);
    }

    #[test]
    fn test_session_accomplishments_stats() {
        let mut sac = SessionAccomplishments::new("session_1".to_string());

        sac.record(Accomplishment::new(
            AccomplishmentType::FileModified,
            "Created main.rs".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        sac.record(Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Set up project".to_string(),
            "turn_1".to_string(),
            0.85,
        ));

        sac.record(Accomplishment::new(
            AccomplishmentType::ProblemFixed,
            "Fixed bug".to_string(),
            "turn_2".to_string(),
            0.9,
        ));

        let stats = sac.generate_stats();
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.by_type.get("file_modified").copied(), Some(1));
        assert_eq!(stats.by_type.get("task_completed").copied(), Some(1));
        assert_eq!(stats.by_type.get("problem_fixed").copied(), Some(1));
        assert_eq!(stats.turns_with_accomplishments, 2);
    }

    #[test]
    fn test_tool_to_accomplishment_type() {
        // Write file that creates
        let result = SessionAccomplishments::tool_to_accomplishment_type(
            "write_file",
            "Created src/main.rs with 150 lines",
            true,
        );
        assert_eq!(result, Some(AccomplishmentType::FileModified));

        // Exec that installs
        let result = SessionAccomplishments::tool_to_accomplishment_type(
            "exec",
            "Installed 3 new packages",
            true,
        );
        assert_eq!(result, Some(AccomplishmentType::ConfigChanged));

        // Failed exec should not produce accomplishment
        let result = SessionAccomplishments::tool_to_accomplishment_type(
            "exec",
            "Command failed",
            false,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_file_paths() {
        let mut sac = SessionAccomplishments::new("session_1".to_string());

        sac.record(Accomplishment::new(
            AccomplishmentType::FileModified,
            "Modified src/main.rs and src/lib.rs".to_string(),
            "turn_1".to_string(),
            0.9,
        ).with_evidence(vec!["Cargo.toml".to_string(), "target/debug".to_string()]));

        let files = sac.extract_file_paths();
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn test_session_summary_text() {
        let mut sac = SessionAccomplishments::new("session_1".to_string());

        sac.record(Accomplishment::new(
            AccomplishmentType::FileModified,
            "Created src/main.rs".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        sac.record(Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Set up project".to_string(),
            "turn_1".to_string(),
            0.85,
        ));

        let summary = sac.generate_summary();
        let text = summary.to_text_summary();

        assert!(text.contains("Session Accomplishments"));
        assert!(text.contains("Files Modified"));
        assert!(text.contains("Tasks Completed"));
    }

    #[test]
    fn test_context_snippet() {
        let mut sac = SessionAccomplishments::new("session_1".to_string());

        sac.record(Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Set up project structure".to_string(),
            "turn_1".to_string(),
            0.85,
        ));

        sac.record(Accomplishment::new(
            AccomplishmentType::ProblemFixed,
            "Fixed authentication bug".to_string(),
            "turn_2".to_string(),
            0.9,
        ));

        let summary = sac.generate_summary();
        let snippet = summary.to_context_snippet();

        assert!(snippet.contains("Tasks completed"));
        assert!(snippet.contains("Problems fixed"));
        assert!(snippet.contains("Set up project structure"));
    }

    #[test]
    fn test_manager_get_or_create() {
        let manager = SessionAccomplishmentsManager::new(PathBuf::from("/tmp/test_accomplishments"));

        let ac1 = manager.get_or_create("session_1");
        assert_eq!(ac1.read().len(), 0);

        manager.record_accomplishment("session_1", Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Test task".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        let ac2 = manager.get_or_create("session_1");
        assert_eq!(ac2.read().len(), 1);

        // Creating new session
        let ac3 = manager.get_or_create("session_2");
        assert_eq!(ac3.read().len(), 0);
    }

    #[test]
    fn test_manager_get_summary() {
        let manager = SessionAccomplishmentsManager::new(PathBuf::from("/tmp/test_accomplishments"));

        manager.record_accomplishment("session_1", Accomplishment::new(
            AccomplishmentType::FileModified,
            "Created file".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        let summary = manager.get_summary("session_1");
        assert!(summary.is_some());
        let summary = summary.unwrap();
        assert_eq!(summary.session_id, "session_1");
        assert_eq!(summary.accomplishments.len(), 1);
    }

    #[test]
    fn test_manager_clear() {
        let manager = SessionAccomplishmentsManager::new(PathBuf::from("/tmp/test_accomplishments"));

        manager.record_accomplishment("session_1", Accomplishment::new(
            AccomplishmentType::TaskCompleted,
            "Test".to_string(),
            "turn_1".to_string(),
            0.9,
        ));

        assert!(manager.get_summary("session_1").is_some());

        manager.clear("session_1");
        assert!(manager.get_summary("session_1").is_none());
    }
}
