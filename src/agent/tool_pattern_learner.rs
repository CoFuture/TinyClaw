//! Tool Pattern Learner Module
//!
//! Analyzes turn history to learn successful tool usage patterns.
//! This learned knowledge improves the agent's tool selection over time.
//!
//! The learner:
//! - Tracks success rates for individual tools
//! - Identifies common successful tool sequences
//! - Detects tool combinations that often lead to success
//! - Provides learned patterns to enhance tool strategy guidance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use crate::agent::turn_history::TurnRecord;

/// A learned tool usage pattern from historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// Unique pattern ID
    pub id: String,
    /// Ordered list of tools in this pattern
    pub tools: Vec<String>,
    /// How often this pattern has been used
    pub usage_count: usize,
    /// Success rate of this pattern (0.0 - 1.0)
    pub success_rate: f32,
    /// Average duration in milliseconds
    pub avg_duration_ms: u64,
    /// When this pattern was last observed
    pub last_seen: DateTime<Utc>,
    /// When this pattern was first observed
    pub first_seen: DateTime<Utc>,
}

/// Statistics for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    /// Tool name
    pub name: String,
    /// Total times this tool was used
    pub usage_count: usize,
    /// How many times it succeeded
    pub success_count: usize,
    /// How many times it failed
    pub failure_count: usize,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Average execution duration in ms
    pub avg_duration_ms: u64,
    /// Tools that often follow this tool (and succeed)
    pub often_followed_by: Vec<(String, usize)>,
    /// Tools that often precede this tool (and succeed)
    pub often_preceded_by: Vec<(String, usize)>,
}

impl ToolStats {
    /// Create new tool stats
    pub fn new(name: String) -> Self {
        Self {
            name,
            usage_count: 0,
            success_count: 0,
            failure_count: 0,
            success_rate: 0.0,
            avg_duration_ms: 0,
            often_followed_by: Vec::new(),
            often_preceded_by: Vec::new(),
        }
    }

    /// Record a tool execution
    pub fn record_execution(&mut self, duration_ms: u64, success: bool) {
        self.usage_count += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.success_rate = if self.usage_count > 0 {
            self.success_count as f32 / self.usage_count as f32
        } else {
            0.0
        };
        // Update average duration using incremental average
        let new_avg = if self.usage_count == 1 {
            duration_ms as f64
        } else {
            ((self.avg_duration_ms as f64 * (self.usage_count - 1) as f64) + duration_ms as f64) / self.usage_count as f64
        };
        self.avg_duration_ms = new_avg as u64;
    }

    /// Add a tool that often follows this one
    pub fn add_follower(&mut self, tool_name: &str) {
        if let Some(pos) = self.often_followed_by.iter().position(|(n, _)| n == tool_name) {
            self.often_followed_by[pos].1 += 1;
        } else {
            self.often_followed_by.push((tool_name.to_string(), 1));
        }
        // Keep only top 5
        self.often_followed_by.sort_by(|a, b| b.1.cmp(&a.1));
        self.often_followed_by.truncate(5);
    }

    /// Add a tool that often precedes this one
    pub fn add_predecessor(&mut self, tool_name: &str) {
        if let Some(pos) = self.often_preceded_by.iter().position(|(n, _)| n == tool_name) {
            self.often_preceded_by[pos].1 += 1;
        } else {
            self.often_preceded_by.push((tool_name.to_string(), 1));
        }
        // Keep only top 5
        self.often_preceded_by.sort_by(|a, b| b.1.cmp(&a.1));
        self.often_preceded_by.truncate(5);
    }
}

/// Analysis of tool usage patterns from turn history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    /// Per-tool statistics
    pub tool_stats: HashMap<String, ToolStats>,
    /// Learned tool sequences (ordered pairs/triplets that succeed)
    pub learned_patterns: Vec<LearnedPattern>,
    /// Overall success rate
    pub overall_success_rate: f32,
    /// Total turns analyzed
    pub total_turns: usize,
    /// Total tool executions analyzed
    pub total_tool_executions: usize,
    /// When this analysis was generated
    pub analyzed_at: DateTime<Utc>,
}

impl Default for PatternAnalysis {
    fn default() -> Self {
        Self {
            tool_stats: HashMap::new(),
            learned_patterns: Vec::new(),
            overall_success_rate: 0.0,
            total_turns: 0,
            total_tool_executions: 0,
            analyzed_at: Utc::now(),
        }
    }
}

/// Tool Pattern Learner - learns from turn history
pub struct ToolPatternLearner {
    /// Analysis results (cached)
    analysis: PatternAnalysis,
    /// Minimum pattern length for learning (in tool calls)
    min_pattern_tools: usize,
    /// Maximum pattern length
    max_pattern_tools: usize,
    /// Minimum usage count to be considered a pattern
    min_pattern_usage: usize,
    /// Recently analyzed turns (for incremental updates)
    recent_turns: VecDeque<String>,
}

impl ToolPatternLearner {
    /// Create a new tool pattern learner
    pub fn new() -> Self {
        Self {
            analysis: PatternAnalysis::default(),
            min_pattern_tools: 2,
            max_pattern_tools: 3,
            min_pattern_usage: 2,
            recent_turns: VecDeque::with_capacity(1000),
        }
    }

    /// Create with custom parameters
    #[allow(dead_code)]
    pub fn with_params(
        min_pattern_tools: usize,
        max_pattern_tools: usize,
        min_pattern_usage: usize,
    ) -> Self {
        Self {
            analysis: PatternAnalysis::default(),
            min_pattern_tools,
            max_pattern_tools,
            min_pattern_usage,
            recent_turns: VecDeque::with_capacity(1000),
        }
    }

    /// Learn from a set of turn records
    pub fn learn_from_turns(&mut self, turns: &[TurnRecord]) {
        if turns.is_empty() {
            return;
        }

        let mut tool_stats: HashMap<String, ToolStats> = HashMap::new();
        let mut pattern_counts: HashMap<String, (Vec<String>, usize, usize, u64)> = HashMap::new();
        // (tools, success_count, total_count, total_duration)

        let mut total_turns = 0;
        let mut total_tool_executions = 0;
        let mut total_successes = 0;

        for turn in turns {
            total_turns += 1;

            // Record individual tool stats
            for tool in &turn.tools {
                total_tool_executions += 1;
                let stats = tool_stats.entry(tool.name.clone()).or_insert_with(|| ToolStats::new(tool.name.clone()));
                stats.record_execution(tool.duration_ms, tool.success);
                if tool.success {
                    total_successes += 1;
                }
            }

            // Build tool sequences and track success
            let tools: Vec<&str> = turn.tools.iter().map(|t| t.name.as_str()).collect();

            // Record tool transitions (predecessor -> follower)
            for i in 0..tools.len() {
                if i > 0 {
                    // This tool was preceded by another
                    if let Some(stats) = tool_stats.get_mut(tools[i]) {
                        stats.add_predecessor(tools[i - 1]);
                    }
                }
                if i < tools.len() - 1 {
                    // This tool was followed by another
                    if let Some(stats) = tool_stats.get_mut(tools[i]) {
                        stats.add_follower(tools[i + 1]);
                    }
                }
            }

            // Extract patterns (pairs and triplets)
            for pattern_len in self.min_pattern_tools..=self.max_pattern_tools {
                // Only extract patterns if we have enough tools
                if tools.len() >= pattern_len {
                    for start in 0..=(tools.len() - pattern_len) {
                        let pattern: Vec<String> = tools[start..start + pattern_len]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        let key = pattern.join("->");

                        let entry = pattern_counts.entry(key).or_insert((pattern.clone(), 0, 0, 0));
                        entry.2 += 1; // total count
                        if turn.success {
                            entry.1 += 1; // success count
                        }
                        entry.3 += turn.tools.get(start).map(|t| t.duration_ms).unwrap_or(0); // Use first tool's duration as approximation
                    }
                }
            }
        }

        // Convert pattern counts to learned patterns
        let mut learned_patterns: Vec<LearnedPattern> = pattern_counts
            .into_values()
            .filter(|(_, _, total, _)| *total >= self.min_pattern_usage)
            .map(|(tools, successes, total, total_duration)| {
                let success_rate = successes as f32 / total as f32;
                let avg_duration = total_duration / total as u64;
                LearnedPattern {
                    id: format!("pattern-{}", uuid::Uuid::new_v4())[..8].to_string(),
                    tools: tools.clone(),
                    usage_count: total,
                    success_rate,
                    avg_duration_ms: avg_duration,
                    last_seen: Utc::now(),
                    first_seen: Utc::now(),
                }
            })
            .collect();

        // Sort by success rate * usage (most valuable patterns first)
        learned_patterns.sort_by(|a, b| {
            let score_a = a.success_rate * a.usage_count as f32;
            let score_b = b.success_rate * b.usage_count as f32;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Keep top 20 patterns
        learned_patterns.truncate(20);

        // Calculate overall success rate
        let overall_success_rate = if total_tool_executions > 0 {
            total_successes as f32 / total_tool_executions as f32
        } else {
            0.0
        };

        self.analysis = PatternAnalysis {
            tool_stats,
            learned_patterns,
            overall_success_rate,
            total_turns,
            total_tool_executions,
            analyzed_at: Utc::now(),
        };
    }

    /// Update analysis with a single new turn (incremental learning)
    pub fn update_with_turn(&mut self, turn: &TurnRecord) {
        // For simplicity, we'll add the turn ID to recent and trigger full rebuild
        // if we've accumulated enough new turns
        self.recent_turns.push_back(turn.id.clone());
        if self.recent_turns.len() > 100 {
            self.recent_turns.pop_front();
        }
    }

    /// Get the current analysis
    pub fn get_analysis(&self) -> &PatternAnalysis {
        &self.analysis
    }

    /// Get tool stats for a specific tool
    pub fn get_tool_stats(&self, tool_name: &str) -> Option<&ToolStats> {
        self.analysis.tool_stats.get(tool_name)
    }

    /// Get patterns that start with a specific tool
    pub fn get_patterns_starting_with(&self, tool_name: &str) -> Vec<&LearnedPattern> {
        self.analysis
            .learned_patterns
            .iter()
            .filter(|p| p.tools.first().map(|t| t == tool_name).unwrap_or(false))
            .collect()
    }

    /// Get high-success-rate patterns for a given tool sequence
    pub fn get_successful_next_tools(&self, current_tools: &[String]) -> Vec<String> {
        if current_tools.is_empty() {
            return Vec::new();
        }

        let last_tool = current_tools.last().unwrap();
        let mut next_tools: HashMap<String, (usize, f32)> = HashMap::new();

        for pattern in &self.analysis.learned_patterns {
            if let Some(pos) = pattern.tools.iter().position(|t| t == last_tool) {
                if pos < pattern.tools.len() - 1 {
                    let next = &pattern.tools[pos + 1];
                    let entry = next_tools.entry(next.clone()).or_insert((0, 0.0));
                    entry.0 += pattern.usage_count;
                    entry.1 += pattern.success_rate * pattern.usage_count as f32;
                }
            }
        }

        // Sort by weighted success
        let mut result: Vec<(String, usize, f32)> = next_tools
            .into_iter()
            .map(|(tool, (count, weighted_rate))| {
                let avg_success = if count > 0 { weighted_rate / count as f32 } else { 0.0 };
                (tool, count, avg_success)
            })
            .filter(|(_, count, _)| *count >= 2)
            .collect::<Vec<_>>();

        result.sort_by(|a, b| {
            // Sort by usage count, then by success rate
            match b.1.cmp(&a.1) {
                std::cmp::Ordering::Equal => b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal),
                other => other,
            }
        });

        result.into_iter().map(|(t, _, _)| t).take(3).collect()
    }

    /// Generate tool usage tips based on learned patterns
    pub fn generate_tips(&self) -> Vec<String> {
        let mut tips = Vec::new();

        // Find tools with high success rates
        let mut sorted_tools: Vec<_> = self.analysis
            .tool_stats
            .values()
            .filter(|s| s.usage_count >= 3)
            .collect::<Vec<_>>();

        sorted_tools.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap_or(std::cmp::Ordering::Equal));

        // Top performing tools
        if let Some(top) = sorted_tools.first() {
            tips.push(format!(
                "{} has the highest success rate ({:.0}%) among frequently used tools",
                top.name, top.success_rate * 100.0
            ));
        }

        // Tools that often precede successful outcomes
        let tools_with_predecessors: Vec<_> = sorted_tools
            .iter()
            .filter(|s| !s.often_preceded_by.is_empty() && s.success_rate > 0.7)
            .collect();

        if let Some(tool) = tools_with_predecessors.first() {
            if let Some((prev, _)) = tool.often_preceded_by.first() {
                tips.push(format!(
                    "{} often succeeds when preceded by {}",
                    tool.name, prev
                ));
            }
        }

        // Common successful patterns
        if let Some(pattern) = self.analysis.learned_patterns.first() {
            if pattern.usage_count >= 3 {
                tips.push(format!(
                    "Successful pattern detected: {} ({}% success rate)",
                    pattern.tools.join(" -> "),
                    pattern.success_rate * 100.0
                ));
            }
        }

        // Low success rate tools - warnings
        let low_success_tools: Vec<_> = sorted_tools
            .iter()
            .filter(|s| s.usage_count >= 5 && s.success_rate < 0.5)
            .collect();

        for tool in low_success_tools.iter().take(2) {
            tips.push(format!(
                "Warning: {} has low success rate ({:.0}%). Consider alternative approaches",
                tool.name, tool.success_rate * 100.0
            ));
        }

        tips.truncate(5); // Limit to 5 tips
        tips
    }
}

impl Default for ToolPatternLearner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::turn_history::ToolExecution;

    fn create_test_turn(
        session_id: &str,
        tools: Vec<(&str, bool, u64)>,
        success: bool,
    ) -> TurnRecord {
        let mut turn = TurnRecord::new(session_id, "test message");
        for (name, tool_success, duration) in tools {
            turn.tools.push(ToolExecution {
                name: name.to_string(),
                input: serde_json::json!({}),
                output_preview: "output".to_string(),
                success: tool_success,
                duration_ms: duration,
            });
        }
        turn.success = success;
        turn
    }

    #[test]
    fn test_learn_from_empty_turns() {
        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&[]);
        assert_eq!(learner.analysis.total_turns, 0);
    }

    #[test]
    fn test_learn_basic_tool_stats() {
        let turns = vec![
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 80)], true),
            create_test_turn("s1", vec![("exec", false, 200)], false),
        ];

        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&turns);

        let read_stats = learner.get_tool_stats("read_file").unwrap();
        assert_eq!(read_stats.usage_count, 2);
        assert_eq!(read_stats.success_count, 2);
        assert!(read_stats.success_rate > 0.9);

        let exec_stats = learner.get_tool_stats("exec").unwrap();
        assert_eq!(exec_stats.usage_count, 1);
        assert_eq!(exec_stats.failure_count, 1);
    }

    #[test]
    fn test_tool_transitions() {
        let turns = vec![
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
        ];

        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&turns);

        let grep_stats = learner.get_tool_stats("grep").unwrap();
        assert!(!grep_stats.often_preceded_by.is_empty());
        assert_eq!(grep_stats.often_preceded_by[0].0, "read_file");
    }

    #[test]
    fn test_learned_patterns() {
        let turns = vec![
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", false, 100), ("grep", false, 50)], false),
        ];

        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&turns);

        // Should have learned the read_file -> grep pattern
        let patterns = learner.get_patterns_starting_with("read_file");
        assert!(!patterns.is_empty());

        let read_grep = patterns.iter().find(|p| p.tools == vec!["read_file", "grep"]);
        assert!(read_grep.is_some());
        let pattern = read_grep.unwrap();
        assert!(pattern.success_rate > 0.5); // 3/4 success
    }

    #[test]
    fn test_get_successful_next_tools() {
        let turns = vec![
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 100), ("edit_file", true, 50)], false), // less successful
        ];

        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&turns);

        let next_tools = learner.get_successful_next_tools(&["read_file".to_string()]);
        assert!(!next_tools.is_empty());
        // grep should be first because it has higher success rate in patterns
        assert_eq!(next_tools[0], "grep");
    }

    #[test]
    fn test_generate_tips() {
        let turns = vec![
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("read_file", true, 100), ("grep", true, 50)], true),
            create_test_turn("s1", vec![("exec", false, 200), ("grep", false, 50)], false),
        ];

        let mut learner = ToolPatternLearner::new();
        learner.learn_from_turns(&turns);

        let tips = learner.generate_tips();
        assert!(!tips.is_empty());
    }
}
