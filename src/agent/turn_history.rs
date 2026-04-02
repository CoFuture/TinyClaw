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

/// Token usage from an AI API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input/prompt tokens
    pub input_tokens: u32,
    /// Number of output/completion tokens
    pub output_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

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
    /// Token usage for this turn (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
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
            token_usage: None,
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
            total_tokens: self.token_usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
        }
    }

    /// Set token usage
    #[allow(dead_code)]
    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
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
    /// Total tokens used in this turn (0 if not available)
    pub total_tokens: u32,
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
    /// Total tokens used across all turns
    pub total_tokens: u64,
    /// Average tokens per turn
    pub avg_tokens: f64,
    /// Tools by name (name -> count)
    pub tools_by_name: HashMap<String, u64>,
    /// Turns by session (session_id -> count)
    pub turns_by_session: HashMap<String, u64>,
    /// Period statistics (for time-series charts)
    pub period_stats: Vec<PeriodStat>,
}

/// Per-tool performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    /// Tool name
    pub tool_name: String,
    /// Total number of calls
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Average execution duration in ms
    pub avg_duration_ms: f64,
    /// Minimum execution duration in ms
    pub min_duration_ms: u64,
    /// Maximum execution duration in ms
    pub max_duration_ms: u64,
    /// Total execution duration in ms (for calculating average)
    total_duration_ms: u64,
}

impl ToolStats {
    /// Create a new tool stats tracker for a tool
    pub fn new(tool_name: String) -> Self {
        Self {
            tool_name,
            total_calls: 0,
            successful_calls: 0,
            success_rate: 0.0,
            avg_duration_ms: 0.0,
            min_duration_ms: u64::MAX,
            max_duration_ms: 0,
            total_duration_ms: 0,
        }
    }

    /// Record a tool execution
    pub fn record_execution(&mut self, duration_ms: u64, success: bool) {
        self.total_calls += 1;
        if success {
            self.successful_calls += 1;
        }
        self.total_duration_ms += duration_ms;
        self.min_duration_ms = self.min_duration_ms.min(duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(duration_ms);
        
        // Calculate averages
        if self.total_calls > 0 {
            self.avg_duration_ms = self.total_duration_ms as f64 / self.total_calls as f64;
            self.success_rate = self.successful_calls as f64 / self.total_calls as f64;
        }
        
        // Handle edge case of min
        if self.total_calls == 1 {
            self.min_duration_ms = duration_ms;
        }
    }
}

/// All tool performance statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPerformanceStats {
    /// Per-tool statistics (sorted by total_calls descending)
    pub tools: Vec<ToolStats>,
    /// Total tool executions across all tools
    pub total_executions: u64,
    /// Overall success rate
    pub overall_success_rate: f64,
    /// Average execution time across all tools
    pub avg_execution_ms: f64,
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
    #[allow(dead_code)]
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

    /// Create a new turn record with full details including token usage (for use from gateway)
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub fn record_turn_full(
        _manager: &Arc<TurnHistoryManager>,
        session_id: &str,
        user_message: &str,
        response: &str,
        duration_ms: u64,
        success: bool,
        tools: Vec<ToolExecution>,
        token_usage: Option<TokenUsage>,
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
        turn.token_usage = token_usage;
        turn
    }

    /// Create a new turn record with a pre-generated turn_id (for feedback tracking)
    #[allow(clippy::too_many_arguments)]
    pub fn record_turn_with_id(
        _manager: &Arc<TurnHistoryManager>,
        turn_id: &str,
        session_id: &str,
        user_message: &str,
        response: &str,
        duration_ms: u64,
        success: bool,
        tools: Vec<ToolExecution>,
        token_usage: Option<TokenUsage>,
    ) -> TurnRecord {
        let mut turn = TurnRecord::new(session_id, user_message);
        turn.id = turn_id.to_string(); // Use pre-generated turn_id
        turn.response_preview = if response.len() > 500 {
            format!("{}...", &response[..500])
        } else {
            response.to_string()
        };
        turn.duration_ms = duration_ms;
        turn.success = success;
        turn.tools = tools;
        turn.token_usage = token_usage;
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

    /// Get turns for a session (summaries)
    pub fn get_turns(&self, session_id: &str) -> Vec<TurnSummary> {
        let records = self.records.read();
        records
            .get(session_id)
            .map(|r| r.iter().map(|t| t.summary()).collect())
            .unwrap_or_default()
    }

    /// Get full turn records for a session (for analysis)
    #[allow(dead_code)]
    pub fn get_turn_records(&self, session_id: &str) -> Vec<TurnRecord> {
        let records = self.records.read();
        records
            .get(session_id)
            .cloned()
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

    /// Get list of session IDs that have turns
    #[allow(dead_code)]
    pub fn get_sessions_with_turns(&self) -> Vec<String> {
        let records = self.records.read();
        records
            .keys()
            .cloned()
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
                
                // Track token usage
                if let Some(ref usage) = turn.token_usage {
                    stats.total_tokens += usage.total_tokens as u64;
                }
                
                for tool in &turn.tools {
                    *stats.tools_by_name.entry(tool.name.clone()).or_insert(0) += 1;
                }
            }
        }
        
        if stats.total_turns > 0 {
            stats.avg_duration_ms /= stats.total_turns as f64;
            stats.avg_tokens = stats.total_tokens as f64 / stats.total_turns as f64;
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
        
        TurnStats {
            period_stats,
            ..Default::default()
        }
    }

    /// Get per-tool performance statistics
    pub fn get_tool_stats(&self) -> ToolPerformanceStats {
        let records = self.records.read();
        
        // Aggregate tool stats by tool name
        let mut tool_stats_map: HashMap<String, ToolStats> = HashMap::new();
        let mut total_executions: u64 = 0;
        let mut total_successful: u64 = 0;
        let mut total_duration_ms: u64 = 0;
        
        for turn in records.values().flatten() {
            for tool in &turn.tools {
                total_executions += 1;
                total_duration_ms += tool.duration_ms;
                if tool.success {
                    total_successful += 1;
                }
                
                tool_stats_map
                    .entry(tool.name.clone())
                    .or_insert_with(|| ToolStats::new(tool.name.clone()))
                    .record_execution(tool.duration_ms, tool.success);
            }
        }
        
        // Sort by total_calls descending
        let mut tools: Vec<ToolStats> = tool_stats_map.into_values().collect();
        tools.sort_by(|a, b| b.total_calls.cmp(&a.total_calls));
        
        // Calculate overall stats
        let overall_success_rate = if total_executions > 0 {
            total_successful as f64 / total_executions as f64
        } else {
            0.0
        };
        
        let avg_execution_ms = if total_executions > 0 {
            total_duration_ms as f64 / total_executions as f64
        } else {
            0.0
        };
        
        ToolPerformanceStats {
            tools,
            total_executions,
            overall_success_rate,
            avg_execution_ms,
        }
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

    #[test]
    fn test_tool_stats_recording() {
        let mut stats = ToolStats::new("read_file".to_string());
        
        // Record successful execution
        stats.record_execution(50, true);
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 1);
        assert!((stats.success_rate - 1.0).abs() < 0.001);
        assert!((stats.avg_duration_ms - 50.0).abs() < 0.001);
        assert_eq!(stats.min_duration_ms, 50);
        assert_eq!(stats.max_duration_ms, 50);
        
        // Record failed execution
        stats.record_execution(100, false);
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.successful_calls, 1);
        assert!((stats.success_rate - 0.5).abs() < 0.001);
        assert!((stats.avg_duration_ms - 75.0).abs() < 0.001);
        assert_eq!(stats.min_duration_ms, 50);
        assert_eq!(stats.max_duration_ms, 100);
    }

    #[test]
    fn test_tool_stats_empty() {
        let stats = ToolStats::new("nonexistent".to_string());
        assert_eq!(stats.total_calls, 0);
        assert!((stats.success_rate - 0.0).abs() < 0.001);
        assert!((stats.avg_duration_ms - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_get_tool_stats() {
        let manager = TurnHistoryManager::new();
        
        // Add turns with different tools
        let mut turn1 = TurnRecord::new("session-1", "Read a file");
        turn1.add_tool(ToolExecution {
            name: "read_file".to_string(),
            input: serde_json::json!({"path": "/tmp/test.txt"}),
            output_preview: "file content".to_string(),
            success: true,
            duration_ms: 50,
        });
        turn1 = turn1.with_response("File content").with_duration(100);
        manager.record(turn1);
        
        let mut turn2 = TurnRecord::new("session-1", "Write a file");
        turn2.add_tool(ToolExecution {
            name: "write_file".to_string(),
            input: serde_json::json!({"path": "/tmp/out.txt", "content": "hello"}),
            output_preview: "Written 5 bytes".to_string(),
            success: true,
            duration_ms: 30,
        });
        turn2.add_tool(ToolExecution {
            name: "read_file".to_string(),
            input: serde_json::json!({"path": "/tmp/out.txt"}),
            output_preview: "hello".to_string(),
            success: false,
            duration_ms: 200, // Failed read
        });
        turn2 = turn2.with_response("Done").with_duration(300);
        manager.record(turn2);
        
        let tool_stats = manager.get_tool_stats();
        
        // Should have 2 tools: read_file (2 calls) and write_file (1 call)
        assert_eq!(tool_stats.total_executions, 3);
        assert!((tool_stats.overall_success_rate - 0.666).abs() < 0.01); // 2/3 successful
        
        let read_file_stats = tool_stats.tools.iter().find(|t| t.tool_name == "read_file").unwrap();
        assert_eq!(read_file_stats.total_calls, 2);
        assert_eq!(read_file_stats.successful_calls, 1);
        assert!((read_file_stats.success_rate - 0.5).abs() < 0.001);
        assert!((read_file_stats.avg_duration_ms - 125.0).abs() < 0.001); // (50+200)/2
        assert_eq!(read_file_stats.min_duration_ms, 50);
        assert_eq!(read_file_stats.max_duration_ms, 200);
        
        let write_file_stats = tool_stats.tools.iter().find(|t| t.tool_name == "write_file").unwrap();
        assert_eq!(write_file_stats.total_calls, 1);
        assert_eq!(write_file_stats.successful_calls, 1);
        assert!((write_file_stats.success_rate - 1.0).abs() < 0.001);
        assert!((write_file_stats.avg_duration_ms - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_tool_stats_sorted_by_usage() {
        let manager = TurnHistoryManager::new();
        
        // Add multiple tools with different usage counts
        for i in 0..5 {
            let mut turn = TurnRecord::new("session-1", format!("Turn {}", i));
            turn.add_tool(ToolExecution {
                name: "grep".to_string(),
                input: serde_json::json!({"pattern": "test"}),
                output_preview: "found 3 matches".to_string(),
                success: true,
                duration_ms: 20,
            });
            turn = turn.with_response("Found").with_duration(50);
            manager.record(turn);
        }
        
        for i in 0..3 {
            let mut turn = TurnRecord::new("session-1", format!("Turn2 {}", i));
            turn.add_tool(ToolExecution {
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "/tmp/test.txt"}),
                output_preview: "content".to_string(),
                success: true,
                duration_ms: 30,
            });
            turn = turn.with_response("Read").with_duration(60);
            manager.record(turn);
        }
        
        let tool_stats = manager.get_tool_stats();
        
        // Should be sorted by total_calls descending
        assert_eq!(tool_stats.tools.len(), 2);
        assert_eq!(tool_stats.tools[0].tool_name, "grep"); // 5 calls
        assert_eq!(tool_stats.tools[0].total_calls, 5);
        assert_eq!(tool_stats.tools[1].tool_name, "read_file"); // 3 calls
        assert_eq!(tool_stats.tools[1].total_calls, 3);
    }
}
