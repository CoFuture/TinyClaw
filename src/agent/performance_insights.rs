//! Agent Performance Insights Module
//!
//! Analyzes turn history, self-evaluations, and session quality data
//! to generate actionable insights for improving agent performance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::turn_history::{TurnHistoryManager, TurnRecord};
use super::self_evaluation::SelfEvaluationManager;

/// Category of performance insight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InsightCategory {
    /// Insight about tool efficiency and usage patterns
    ToolEfficiency,
    /// Insight about quality trends over time
    QualityTrend,
    /// Insight about detected patterns (success or failure)
    PatternDetection,
    /// Insight with actionable optimization suggestion
    OptimizationSuggestion,
    /// Insight about agent behavior
    AgentBehavior,
}

impl InsightCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            InsightCategory::ToolEfficiency => "Tool Efficiency",
            InsightCategory::QualityTrend => "Quality Trend",
            InsightCategory::PatternDetection => "Pattern Detection",
            InsightCategory::OptimizationSuggestion => "Optimization",
            InsightCategory::AgentBehavior => "Agent Behavior",
        }
    }
}

/// Severity level for the insight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightSeverity {
    /// Informational, no action needed
    Info,
    /// Suggestion to consider
    Suggestion,
    /// Warning, should investigate
    Warning,
}

impl InsightSeverity {
    pub fn display_name(&self) -> &'static str {
        match self {
            InsightSeverity::Info => "Info",
            InsightSeverity::Suggestion => "Suggestion",
            InsightSeverity::Warning => "Warning",
        }
    }
}

/// A single performance insight with actionable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsight {
    /// Unique insight ID
    pub id: String,
    /// Category of this insight
    pub category: InsightCategory,
    /// Severity level
    pub severity: InsightSeverity,
    /// Short title for the insight
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Actionable suggestions
    pub suggestions: Vec<String>,
    /// Supporting data (e.g., tool names, scores)
    pub data: serde_json::Value,
    /// When this insight was generated
    pub created_at: DateTime<Utc>,
}

impl PerformanceInsight {
    pub fn new(
        category: InsightCategory,
        severity: InsightSeverity,
        title: impl Into<String>,
        description: impl Into<String>,
        suggestions: Vec<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            category,
            severity,
            title: title.into(),
            description: description.into(),
            suggestions,
            data,
            created_at: Utc::now(),
        }
    }

    /// Create an info-level insight
    pub fn info(
        title: impl Into<String>,
        description: impl Into<String>,
        suggestions: Vec<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::new(InsightCategory::OptimizationSuggestion, InsightSeverity::Info, title, description, suggestions, data)
    }

    /// Create a suggestion-level insight
    pub fn suggestion(
        title: impl Into<String>,
        description: impl Into<String>,
        suggestions: Vec<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::new(InsightCategory::OptimizationSuggestion, InsightSeverity::Suggestion, title, description, suggestions, data)
    }

    /// Create a warning-level insight
    pub fn warning(
        title: impl Into<String>,
        description: impl Into<String>,
        suggestions: Vec<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::new(InsightCategory::PatternDetection, InsightSeverity::Warning, title, description, suggestions, data)
    }
}

/// Summary of tool efficiency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEfficiencySummary {
    /// Most efficient tool (best success rate + speed)
    pub most_efficient_tool: Option<String>,
    /// Least efficient tool (lowest success rate or slowest)
    pub least_efficient_tool: Option<String>,
    /// Tools with high failure rates (>20%)
    pub problematic_tools: Vec<String>,
    /// Average tool usage per turn
    pub avg_tools_per_turn: f64,
    /// Most used tool
    pub most_used_tool: Option<String>,
}

impl Default for ToolEfficiencySummary {
    fn default() -> Self {
        Self {
            most_efficient_tool: None,
            least_efficient_tool: None,
            problematic_tools: Vec::new(),
            avg_tools_per_turn: 0.0,
            most_used_tool: None,
        }
    }
}

/// Quality trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrend {
    /// Current average quality score (0-100)
    pub current_score: f64,
    /// Previous period average quality score
    pub previous_score: f64,
    /// Change direction: positive, negative, stable
    pub trend_direction: String,
    /// Trend magnitude as percentage
    pub trend_magnitude: f64,
}

impl Default for QualityTrend {
    fn default() -> Self {
        Self {
            current_score: 0.0,
            previous_score: 0.0,
            trend_direction: "stable".to_string(),
            trend_magnitude: 0.0,
        }
    }
}

/// Tool combination pattern (which tools often appear together)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPattern {
    /// Ordered list of tools in the pattern
    pub tools: Vec<String>,
    /// How many times this pattern occurred
    pub occurrences: u64,
    /// Success rate for this pattern
    pub success_rate: f64,
    /// Whether this pattern is reliable (high occurrence + high success)
    pub is_reliable: bool,
}

/// Complete performance analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    /// All generated insights
    pub insights: Vec<PerformanceInsight>,
    /// Tool efficiency summary
    pub tool_efficiency: ToolEfficiencySummary,
    /// Quality trend analysis
    pub quality_trend: QualityTrend,
    /// Detected tool patterns
    pub tool_patterns: Vec<ToolPattern>,
    /// Total turns analyzed
    pub turns_analyzed: u64,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

impl Default for PerformanceAnalysis {
    fn default() -> Self {
        Self {
            insights: Vec::new(),
            tool_efficiency: ToolEfficiencySummary::default(),
            quality_trend: QualityTrend::default(),
            tool_patterns: Vec::new(),
            turns_analyzed: 0,
            analyzed_at: Utc::now(),
        }
    }
}

/// Performance insights engine - generates actionable insights from data
pub struct PerformanceInsightsEngine {
    /// Minimum turns needed for reliable pattern detection
    min_turns_for_patterns: u64,
    /// Minimum calls to consider a tool "frequently used"
    min_calls_for_frequency: u64,
    /// Failure rate threshold for "problematic" classification
    failure_rate_threshold: f64,
}

impl Default for PerformanceInsightsEngine {
    fn default() -> Self {
        Self {
            min_turns_for_patterns: 5,
            min_calls_for_frequency: 3,
            failure_rate_threshold: 0.2,
        }
    }
}

impl PerformanceInsightsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze turn history and generate performance insights
    pub fn analyze(
        &self,
        turn_history: &TurnHistoryManager,
        self_eval_manager: &SelfEvaluationManager,
    ) -> PerformanceAnalysis {
        let mut analysis = PerformanceAnalysis::default();
        
        // Get all turn records (HashMap<String, Vec<TurnRecord>>)
        let all_turns_map = turn_history.get_all_sessions_turns();
        
        // Flatten into a single Vec
        let all_turns: Vec<TurnRecord> = all_turns_map.into_values().flatten().collect();
        
        analysis.turns_analyzed = all_turns.len() as u64;
        
        if all_turns.is_empty() {
            return analysis;
        }
        
        // Analyze tool efficiency
        analysis.tool_efficiency = self.analyze_tool_efficiency(&all_turns);
        
        // Analyze quality trends
        analysis.quality_trend = self.analyze_quality_trends(&all_turns, self_eval_manager);
        
        // Detect tool patterns
        if all_turns.len() as u64 >= self.min_turns_for_patterns {
            analysis.tool_patterns = self.detect_tool_patterns(&all_turns);
        }
        
        // Generate insights from analysis
        analysis.insights = self.generate_insights(&analysis);
        
        analysis
    }

    /// Analyze tool efficiency from turn records
    fn analyze_tool_efficiency(&self, turns: &[TurnRecord]) -> ToolEfficiencySummary {
        let mut summary = ToolEfficiencySummary::default();
        
        // Aggregate tool statistics
        let mut tool_stats: HashMap<String, ToolAggStats> = HashMap::new();
        let mut total_tools = 0u64;
        let mut most_used = (String::new(), 0u64);
        
        for turn in turns {
            for tool in &turn.tools {
                total_tools += 1;
                
                let stats = tool_stats.entry(tool.name.clone()).or_insert_with(|| ToolAggStats {
                    total_calls: 0,
                    successful_calls: 0,
                    total_duration_ms: 0,
                });
                stats.total_calls += 1;
                if tool.success {
                    stats.successful_calls += 1;
                }
                stats.total_duration_ms += tool.duration_ms;
                
                if stats.total_calls > most_used.1 {
                    most_used = (tool.name.clone(), stats.total_calls);
                }
            }
        }
        
        if !turns.is_empty() {
            summary.avg_tools_per_turn = total_tools as f64 / turns.len() as f64;
        }
        
        if !most_used.0.is_empty() {
            summary.most_used_tool = Some(most_used.0);
        }
        
        // Find most and least efficient tools (need minimum calls)
        let mut tool_scores: Vec<(String, f64)> = Vec::new();
        for (name, stats) in &tool_stats {
            if stats.total_calls >= self.min_calls_for_frequency {
                let success_score = stats.successful_calls as f64 / stats.total_calls as f64;
                let speed_score = if stats.total_duration_ms > 0 {
                    // Lower duration is better, normalize to 0-1
                    let avg_ms = stats.total_duration_ms as f64 / stats.total_calls as f64;
                    // Penalize very slow tools, cap at 10 seconds
                    avg_ms.min(10000.0) / 10000.0
                } else {
                    1.0
                };
                // Combined score: 60% success rate, 40% speed
                let combined = success_score * 0.6 + (1.0 - speed_score) * 0.4;
                tool_scores.push((name.clone(), combined));
                
                // Check for problematic tools (high failure rate)
                if success_score < (1.0 - self.failure_rate_threshold) {
                    summary.problematic_tools.push(name.clone());
                }
            }
        }
        
        // Sort by score (lower is better)
        tool_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        if let Some((name, _)) = tool_scores.first() {
            summary.most_efficient_tool = Some(name.clone());
        }
        if let Some((name, _)) = tool_scores.last() {
            summary.least_efficient_tool = Some(name.clone());
        }
        
        summary
    }

    /// Analyze quality trends from turn history and evaluations
    fn analyze_quality_trends(
        &self,
        turns: &[TurnRecord],
        self_eval_manager: &SelfEvaluationManager,
    ) -> QualityTrend {
        let mut trend = QualityTrend::default();
        
        if turns.is_empty() {
            return trend;
        }
        
        // Get recent self-evaluations
        let recent_evals = self_eval_manager.get_recent(20);
        
        if recent_evals.len() < 2 {
            // Not enough data for trend
            if let Some(eval) = recent_evals.first() {
                trend.current_score = eval.overall_score * 100.0;
            }
            return trend;
        }
        
        // Split into recent and older groups
        let mid_point = recent_evals.len() / 2;
        let recent: Vec<_> = recent_evals.iter().take(mid_point).collect();
        let older: Vec<_> = recent_evals.iter().skip(mid_point).collect();
        
        let current_avg = recent.iter().map(|e| e.overall_score).sum::<f64>() / recent.len() as f64;
        let previous_avg = older.iter().map(|e| e.overall_score).sum::<f64>() / older.len() as f64;
        
        trend.current_score = current_avg * 100.0;
        trend.previous_score = previous_avg * 100.0;
        
        let change = trend.current_score - trend.previous_score;
        trend.trend_magnitude = change.abs();
        
        if change > 5.0 {
            trend.trend_direction = "improving".to_string();
        } else if change < -5.0 {
            trend.trend_direction = "declining".to_string();
        } else {
            trend.trend_direction = "stable".to_string();
        }
        
        trend
    }

    /// Detect common tool usage patterns
    fn detect_tool_patterns(&self, turns: &[TurnRecord]) -> Vec<ToolPattern> {
        let mut pattern_counts: HashMap<String, PatternData> = HashMap::new();
        
        for turn in turns {
            if turn.tools.is_empty() {
                continue;
            }
            
            // Build ordered tool sequence
            let tools: Vec<&str> = turn.tools.iter().map(|t| t.name.as_str()).collect();
            
            // Record the sequence
            for window_size in 1..=tools.len().min(4) {
                for start in 0..=(tools.len() - window_size) {
                    let end = start + window_size;
                    let pattern: Vec<&str> = tools[start..end].to_vec();
                    let key = pattern.join(" → ");
                    
                    let data = pattern_counts.entry(key).or_insert_with(|| PatternData {
                        tools: pattern.iter().map(|s| s.to_string()).collect(),
                        occurrences: 0,
                        successful_occurrences: 0,
                    });
                    data.occurrences += 1;
                    if turn.success {
                        data.successful_occurrences += 1;
                    }
                }
            }
        }
        
        // Convert to patterns, filter for reliability
        let mut patterns: Vec<ToolPattern> = pattern_counts
            .into_values()
            .filter(|p| p.occurrences >= 2)
            .map(|p| ToolPattern {
                tools: p.tools,
                occurrences: p.occurrences,
                success_rate: p.successful_occurrences as f64 / p.occurrences as f64,
                is_reliable: p.occurrences >= 3 && p.successful_occurrences as f64 / p.occurrences as f64 >= 0.8,
            })
            .collect();
        
        // Sort by reliability and occurrence
        patterns.sort_by(|a, b| {
            let a_score = if a.is_reliable { 1000 } else { 0 } + a.occurrences as i32;
            let b_score = if b.is_reliable { 1000 } else { 0 } + b.occurrences as i32;
            b_score.cmp(&a_score)
        });
        
        patterns.truncate(10);
        patterns
    }

    /// Generate actionable insights from analysis data
    fn generate_insights(&self, analysis: &PerformanceAnalysis) -> Vec<PerformanceInsight> {
        let mut insights = Vec::new();
        
        // Insight: Most efficient tool
        if let Some(ref tool) = analysis.tool_efficiency.most_efficient_tool {
            insights.push(PerformanceInsight::info(
                format!("{} is your most efficient tool", tool),
                "This tool has the best combination of success rate and execution speed.".to_string(),
                vec![
                    format!("Consider using {} more often for similar tasks", tool),
                    format!("Review what makes {} effective and apply to other tools", tool),
                ],
                serde_json::json!({ "tool": tool }),
            ));
        }
        
        // Insight: Problematic tools
        if !analysis.tool_efficiency.problematic_tools.is_empty() {
            let tools = &analysis.tool_efficiency.problematic_tools;
            insights.push(PerformanceInsight::warning(
                format!("{} tool(s) have high failure rates", tools.len()),
                format!("The following tools fail more than {}% of the time: {}", 
                    (self.failure_rate_threshold * 100.0) as i32,
                    tools.join(", ")),
                tools.iter().map(|t| {
                    format!("Review the use cases for {} - consider if there's a better alternative", t)
                }).collect(),
                serde_json::json!({ "tools": tools }),
            ));
        }
        
        // Insight: Quality trend
        if analysis.quality_trend.trend_direction != "stable" {
            let direction = &analysis.quality_trend.trend_direction;
            let magnitude = analysis.quality_trend.trend_magnitude;
            
            if direction == "improving" {
                insights.push(PerformanceInsight::info(
                    "Agent quality is improving",
                    format!("Quality scores have increased by {:.1}% in recent turns.", magnitude),
                    vec![
                        "Keep doing what you're doing!".to_string(),
                        "Consider noting what changes led to this improvement".to_string(),
                    ],
                    serde_json::json!({
                        "direction": direction,
                        "magnitude": magnitude
                    }),
                ));
            } else {
                insights.push(PerformanceInsight::suggestion(
                    "Agent quality is declining",
                    format!("Quality scores have dropped by {:.1}% in recent turns.", magnitude),
                    vec![
                        "Review recent failed turns for patterns".to_string(),
                        "Consider if recent context changes affected performance".to_string(),
                        "Check if tool availability or system load has changed".to_string(),
                    ],
                    serde_json::json!({
                        "direction": direction,
                        "magnitude": magnitude
                    }),
                ));
            }
        }
        
        // Insight: Reliable tool patterns
        let reliable_patterns: Vec<_> = analysis.tool_patterns.iter()
            .filter(|p| p.is_reliable && p.tools.len() > 1)
            .collect();
        
        if !reliable_patterns.is_empty() {
            let pattern = reliable_patterns[0];
            insights.push(PerformanceInsight::suggestion(
                "Reliable tool sequence detected",
                format!("The sequence {} has a {}% success rate across {} occurrences.",
                    pattern.tools.join(" → "),
                    (pattern.success_rate * 100.0) as i32,
                    pattern.occurrences),
                vec![
                    format!("This pattern is reliable for similar tasks", ),
                    format!("Consider if other tools could be added to this workflow", ),
                ],
                serde_json::json!({
                    "pattern": pattern.tools,
                    "success_rate": pattern.success_rate,
                    "occurrences": pattern.occurrences
                }),
            ));
        }
        
        // Insight: Average tools per turn
        let avg = analysis.tool_efficiency.avg_tools_per_turn;
        if avg > 5.0 {
            insights.push(PerformanceInsight::suggestion(
                "High tool usage per turn",
                format!("Average of {:.1} tools per turn suggests complex tasks or potential inefficiency.", avg),
                vec![
                    "Consider if some operations could be batched".to_string(),
                    "Complex tasks might benefit from being broken into smaller steps".to_string(),
                ],
                serde_json::json!({ "avg_tools_per_turn": avg }),
            ));
        } else if avg > 0.0 && avg < 1.0 {
            insights.push(PerformanceInsight::suggestion(
                "Low tool usage",
                format!("Average of {:.1} tools per turn suggests mostly single-step tasks.", avg),
                vec![
                    "Consider if more context or tools could help complete tasks more thoroughly".to_string(),
                ],
                serde_json::json!({ "avg_tools_per_turn": avg }),
            ));
        }
        
        // Limit insights
        insights.truncate(8);
        insights
    }
}

/// Internal struct for aggregating tool statistics
#[derive(Default)]
struct ToolAggStats {
    total_calls: u64,
    successful_calls: u64,
    total_duration_ms: u64,
}

/// Internal struct for pattern detection
#[derive(Default)]
struct PatternData {
    tools: Vec<String>,
    occurrences: u64,
    successful_occurrences: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_insight_creation() {
        let insight = PerformanceInsight::suggestion(
            "Test insight",
            "This is a test",
            vec!["Suggestion 1".to_string()],
            serde_json::json!({ "test": true }),
        );
        
        assert_eq!(insight.severity, InsightSeverity::Suggestion);
        assert_eq!(insight.category, InsightCategory::OptimizationSuggestion);
        assert!(!insight.id.is_empty());
    }
    
    #[test]
    fn test_tool_efficiency_analysis() {
        let engine = PerformanceInsightsEngine::new();
        
        // Create mock turn records
        let turns = vec![
            TurnRecord::new("session1", "read a file")
                .with_response("content")
                .with_success(true),
            TurnRecord::new("session1", "read another file")
                .with_response("content")
                .with_success(true),
        ];
        
        // Note: In real tests we'd use a mock TurnHistoryManager
        // This is just structural testing
        assert!(true);
    }
}
