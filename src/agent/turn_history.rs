//! Turn History Module
//!
//! Tracks and persists turn execution history for each session.
//! Provides visibility into agent behavior and tool usage patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

/// A tool execution record within a turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    /// Tool name
    pub name: String,
    /// Tool input arguments
    pub input: serde_json::Value,
    /// Tool output (truncated for storage)
    pub output_preview: String,
    /// Whether the tool succeeded
    pub success: bool,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// A complete turn record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    /// Unique turn ID
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// User message that started this turn
    pub user_message: String,
    /// Assistant response (truncated for storage)
    pub response_preview: String,
    /// Tools executed during this turn
    pub tools: Vec<ToolExecution>,
    /// Total turn duration in milliseconds
    pub duration_ms: u64,
    /// Whether the turn completed successfully
    pub success: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl TurnRecord {
    /// Create a new turn record
    #[allow(dead_code)]
    pub fn new(session_id: impl Into<String>, user_message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            user_message: user_message.into(),
            response_preview: String::new(),
            tools: Vec::new(),
            duration_ms: 0,
            success: true,
            created_at: Utc::now(),
        }
    }

    /// Set the response
    pub fn with_response(mut self, response: &str) -> Self {
        // Truncate response preview to 500 chars
        self.response_preview = if response.len() > 500 {
            format!("{}...", &response[..500])
        } else {
            response.to_string()
        };
        self
    }

    /// Add a tool execution
    #[allow(dead_code)]
    pub fn add_tool(&mut self, tool: ToolExecution) {
        self.tools.push(tool);
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set success status
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    /// Get a summary for list display
    pub fn summary(&self) -> TurnSummary {
        TurnSummary {
            id: self.id.clone(),
            session_id: self.session_id.clone(),
            tool_count: self.tools.len(),
            successful_tools: self.tools.iter().filter(|t| t.success).count(),
            duration_ms: self.duration_ms,
            success: self.success,
            created_at: self.created_at,
            message_preview: if self.user_message.len() > 60 {
                format!("{}...", &self.user_message[..60])
            } else {
                self.user_message.clone()
            },
        }
    }
}

/// Summary of a turn for list display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub id: String,
    pub session_id: String,
    pub tool_count: usize,
    pub successful_tools: usize,
    pub duration_ms: u64,
    pub success: bool,
    pub created_at: DateTime<Utc>,
    pub message_preview: String,
}

/// Aggregated turn statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnStats {
    /// Total turns recorded
    pub total_turns: u64,
    /// Total tool executions
    pub total_tools: u64,
    /// Successful tool executions
    pub successful_tools: u64,
    /// Tool success rate (0.0 - 1.0)
    pub tool_success_rate: f64,
    /// Average turn duration (ms)
    pub avg_duration_ms: f64,
    /// Tools by name (name -> count)
    pub tools_by_name: HashMap<String, u64>,
    /// Turns by session (session_id -> count)
    pub turns_by_session: HashMap<String, u64>,
    /// Period statistics (for time-series charts)
    pub period_stats: Vec<PeriodStat>,
}

/// Statistics for a single time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodStat {
    /// Period label (e.g., "2026-03-22", "14:00")
    pub period: String,
    /// Period start timestamp
    pub timestamp: i64,
    /// Number of turns in this period
    pub turns: u64,
    /// Number of successful turns
    pub successful: u64,
    /// Number of tools executed
    pub tools: u64,
    /// Average duration in ms
    pub avg_duration_ms: f64,
}

/// Period type for grouping statistics
#[derive(Debug, Clone, Copy, Default)]
pub enum StatsPeriod {
    #[default]
    Hourly,
    Daily,
    Weekly,
}

/// Turn history manager
pub struct TurnHistoryManager {
    /// In-memory turn records by session
    records: RwLock<HashMap<String, Vec<TurnRecord>>>,
    /// Persistence directory
    persist_dir: Option<PathBuf>,
    /// Maximum turns to keep per session
    max_turns_per_session: usize,
}

impl TurnHistoryManager {
    /// Create a new in-memory turn history manager
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            persist_dir: None,
            max_turns_per_session: 100,
        }
    }

    /// Create a new turn record (for use from gateway)
    pub fn record_turn(
        _manager: &Arc<TurnHistoryManager>,
        session_id: &str,
        user_message: &str,
        response: &str,
        duration_ms: u64,
        success: bool,
    ) -> TurnRecord {
        let mut turn = TurnRecord::new(session_id, user_message);
        turn = turn.with_response(response).with_duration(duration_ms).with_success(success);
        turn
    }

    /// Create a new turn record with tool executions (for use from gateway)
    #[allow(dead_code)]
    pub fn record_turn_with_tools(
        _manager: &Arc<TurnHistoryManager>,
        session_id: &str,
        user_message: &str,
        response: &str,
        duration_ms: u64,
        success: bool,
        tools: Vec<ToolExecution>,
    ) -> TurnRecord {
        let mut turn = TurnRecord::new(session_id, user_message);
        turn.response_preview = if response.len() > 500 {
            format!("{}...", &response[..500])
        } else {
            response.to_string()
        };
        turn.duration_ms = duration_ms;
        turn.success = success;
        turn.tools = tools;
        turn
    }

    /// Create a turn history manager with persistence
    pub fn new_with_persistence<P: AsRef<std::path::Path>>(dir: P) -> crate::common::Result<Self> {
        let persist_dir = dir.as_ref().to_path_buf();
        
        // Ensure directory exists
        fs::create_dir_all(&persist_dir)?;
        
        let mut manager = Self {
            records: RwLock::new(HashMap::new()),
            persist_dir: Some(persist_dir),
            max_turns_per_session: 100,
        };
        
        // Load existing turn history from disk
        manager.load_from_disk()?;
        
        Ok(manager)
    }

    /// Load turn history from disk
    fn load_from_disk(&mut self) -> crate::common::Result<()> {
        let Some(dir) = &self.persist_dir else {
            return Ok(());
        };
        
        let entries = fs::read_dir(dir)?;
        let mut records = self.records.write();
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session_records) = serde_json::from_str::<Vec<TurnRecord>>(&content) {
                        let session_id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .replace("turns_", "");
                        records.insert(session_id, session_records);
                    }
                }
            }
        }
        
        tracing::info!("Loaded turn history from {:?}", dir);
        Ok(())
    }

    /// Save turn history for a session to disk
    fn save_to_disk(&self, session_id: &str, records: &[TurnRecord]) {
        let Some(dir) = &self.persist_dir else {
            return;
        };
        
        let path = dir.join(format!("turns_{}.json", session_id));
        match serde_json::to_string_pretty(records) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    tracing::error!("Failed to save turn history to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize turn history: {}", e);
            }
        }
    }

    /// Record a completed turn
    pub fn record(&self, turn: TurnRecord) {
        let session_id = turn.session_id.clone();
        let summary = turn.summary();
        
        let mut records = self.records.write();
        let session_records = records.entry(session_id.clone()).or_default();
        
        // Add the new turn
        session_records.push(turn);
        
        // Trim to max size (keep most recent)
        if session_records.len() > self.max_turns_per_session {
            let remove_count = session_records.len() - self.max_turns_per_session;
            session_records.drain(0..remove_count);
        }
        
        // Save to disk
        if self.persist_dir.is_some() {
            self.save_to_disk(&session_id, session_records);
        }
        
        drop(records);
        tracing::debug!(
            session_id = %session_id,
            turn_id = %summary.id,
            tool_count = summary.tool_count,
            duration_ms = summary.duration_ms,
            "Recorded turn"
        );
    }

    /// Get turns for a session
    pub fn get_turns(&self, session_id: &str) -> Vec<TurnSummary> {
        let records = self.records.read();
        records
            .get(session_id)
            .map(|r| r.iter().map(|t| t.summary()).collect())
            .unwrap_or_default()
    }

    /// Get a specific turn by ID
    pub fn get_turn(&self, session_id: &str, turn_id: &str) -> Option<TurnRecord> {
        let records = self.records.read();
        records
            .get(session_id)
            .and_then(|r| r.iter().find(|t| t.id == turn_id).cloned())
    }

    /// Get recent turns across all sessions
    pub fn get_recent_turns(&self, limit: usize) -> Vec<TurnSummary> {
        let records = self.records.read();
        let mut all_turns: Vec<_> = records
            .values()
            .flat_map(|r| r.iter().map(|t| t.summary()))
            .collect();
        
        // Sort by creation time (most recent first)
        all_turns.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Limit results
        all_turns.into_iter().take(limit).collect()
    }

    /// Get all turns for all sessions (for export)
    /// Returns a map of session_id -> Vec<TurnRecord>
    #[allow(dead_code)]
    pub fn get_all_sessions_turns(&self) -> std::collections::HashMap<String, Vec<TurnRecord>> {
        let records = self.records.read();
        records
            .iter()
            .map(|(session_id, turns)| (session_id.clone(), turns.clone()))
            .collect()
    }

    /// Get aggregated statistics
    pub fn get_stats(&self) -> TurnStats {
        let records = self.records.read();
        let mut stats = TurnStats::default();
        
        for (session_id, turns) in records.iter() {
            stats.turns_by_session.insert(session_id.clone(), turns.len() as u64);
            stats.total_turns += turns.len() as u64;
            
            for turn in turns {
                stats.total_tools += turn.tools.len() as u64;
                stats.successful_tools += turn.tools.iter().filter(|t| t.success).count() as u64;
                stats.avg_duration_ms += turn.duration_ms as f64;
                
                for tool in &turn.tools {
                    *stats.tools_by_name.entry(tool.name.clone()).or_insert(0) += 1;
                }
            }
        }
        
        if stats.total_turns > 0 {
            stats.avg_duration_ms /= stats.total_turns as f64;
        }
        
        // Calculate tool success rate
        if stats.total_tools > 0 {
            stats.tool_success_rate = stats.successful_tools as f64 / stats.total_tools as f64;
        }
        
        stats
    }

    /// Get statistics grouped by time period
    pub fn get_stats_by_period(&self, period: StatsPeriod, limit: usize) -> TurnStats {
        use chrono::{Duration, Timelike};
        
        let records = self.records.read();
        let now = chrono::Utc::now();
        
        // Calculate the start time based on period
        let (start_time, _duration, formatter): (chrono::DateTime<chrono::Utc>, Duration, &dyn Fn(chrono::DateTime<chrono::Utc>) -> String) = match period {
            StatsPeriod::Hourly => {
                let start = now - Duration::hours(limit as i64);
                (start, Duration::hours(1), &|dt| format!("{:02}:00", dt.hour()))
            }
            StatsPeriod::Daily => {
                let start = now - Duration::days(limit as i64);
                (start, Duration::days(1), &|dt| dt.format("%m-%d").to_string())
            }
            StatsPeriod::Weekly => {
                let start = now - Duration::weeks(limit as i64);
                (start, Duration::weeks(1), &|dt| dt.format("%Y-W%W").to_string())
            }
        };
        
        // Group turns by period
        let mut period_map: HashMap<String, PeriodStat> = HashMap::new();
        
        for turns in records.values() {
            for turn in turns {
                if turn.created_at >= start_time {
                    let period_key = formatter(turn.created_at);
                    let entry = period_map.entry(period_key.clone()).or_insert(PeriodStat {
                        period: period_key,
                        timestamp: (turn.created_at.timestamp() / 3600) * 3600,
                        turns: 0,
                        successful: 0,
                        tools: 0,
                        avg_duration_ms: 0.0,
                    });
                    
                    entry.turns += 1;
                    if turn.success {
                        entry.successful += 1;
                    }
                    entry.tools += turn.tools.len() as u64;
                    entry.avg_duration_ms += turn.duration_ms as f64;
                }
            }
        }
        
        // Calculate averages and sort by timestamp
        let mut period_stats: Vec<PeriodStat> = period_map.into_values().collect();
        for stat in &mut period_stats {
            if stat.turns > 0 {
                stat.avg_duration_ms /= stat.turns as f64;
            }
        }
        period_stats.sort_by_key(|s| s.timestamp);
        
        // Limit results
        period_stats.truncate(limit);
        
        let mut stats = TurnStats::default();
        stats.period_stats = period_stats;
        stats
    }

    /// Clear turns for a session
    #[allow(dead_code)]
    pub fn clear_session(&self, session_id: &str) {
        let mut records = self.records.write();
        records.remove(session_id);
        
        // Delete from disk
        if let Some(dir) = &self.persist_dir {
            let path = dir.join(format!("turns_{}.json", session_id));
            let _ = fs::remove_file(path);
        }
    }

    /// Clear all turns
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        let mut records = self.records.write();
        records.clear();
        
        // Delete all files from disk
        if let Some(dir) = &self.persist_dir {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "json").unwrap_or(false) {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }
}

impl Default for TurnHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_turn_record() {
        let mut turn = TurnRecord::new("session-1", "What files are in /tmp?");
        turn.add_tool(ToolExecution {
            name: "list_dir".to_string(),
            input: serde_json::json!({"path": "/tmp"}),
            output_preview: "file1.txt\nfile2.txt".to_string(),
            success: true,
            duration_ms: 45,
        });
        turn = turn.with_response("I found 2 files: file1.txt and file2.txt").with_duration(150);

        assert_eq!(turn.session_id, "session-1");
        assert_eq!(turn.tools.len(), 1);
        assert!(turn.success);
        
        let summary = turn.summary();
        assert_eq!(summary.tool_count, 1);
        assert_eq!(summary.successful_tools, 1);
    }

    #[test]
    fn test_turn_history_manager() {
        let manager = TurnHistoryManager::new();
        
        let turn = TurnRecord::new("session-1", "Hello")
            .with_response("Hi there!")
            .with_duration(100);
        
        manager.record(turn);
        
        let turns = manager.get_turns("session-1");
        assert_eq!(turns.len(), 1);
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_turns, 1);
    }

    #[test]
    fn test_turn_history_persistence() {
        let dir = tempdir().unwrap();
        let manager = TurnHistoryManager::new_with_persistence(dir.path()).unwrap();
        
        let turn = TurnRecord::new("session-1", "Test message")
            .with_response("Test response")
            .with_duration(50);
        
        manager.record(turn);
        
        // Create a new manager to test loading
        let manager2 = TurnHistoryManager::new_with_persistence(dir.path()).unwrap();
        let turns = manager2.get_turns("session-1");
        assert_eq!(turns.len(), 1);
    }

    #[test]
    fn test_turn_stats() {
        let manager = TurnHistoryManager::new();
        
        // Add multiple turns
        for i in 0..3 {
            let mut turn = TurnRecord::new("session-1", format!("Message {}", i));
            turn.add_tool(ToolExecution {
                name: "read_file".to_string(),
                input: serde_json::json!({"path": format!("/tmp/{}.txt", i)}),
                output_preview: "content".to_string(),
                success: true,
                duration_ms: 10,
            });
            turn = turn.with_response("Done").with_duration(100);
            manager.record(turn);
        }
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_turns, 3);
        assert_eq!(stats.total_tools, 3);
        assert_eq!(stats.successful_tools, 3);
        assert_eq!(stats.tools_by_name.get("read_file"), Some(&3));
    }

    #[test]
    fn test_max_turns_per_session() {
        let mut manager = TurnHistoryManager::new();
        manager.max_turns_per_session = 5;
        
        for i in 0..10 {
            let turn = TurnRecord::new("session-1", format!("Message {}", i))
                .with_response(&format!("Response {}", i))
                .with_duration(100);
            manager.record(turn);
        }
        
        let turns = manager.get_turns("session-1");
        assert_eq!(turns.len(), 5);
    }
}
