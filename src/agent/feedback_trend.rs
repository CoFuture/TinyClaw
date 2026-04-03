//! Feedback Trend Analysis Module
//!
//! Analyzes user feedback trends over time to identify patterns and measure improvement.
//! This helps both the Agent understand its performance trajectory and users see feedback impact.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::turn_feedback::{FeedbackRating, TurnFeedback, TurnFeedbackManager};

/// Direction of a trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    /// Trend is improving
    Improving,
    /// Trend is declining
    Declining,
    /// Trend is stable
    Stable,
    /// Not enough data to determine trend
    InsufficientData,
}

impl TrendDirection {
    /// Get emoji representation
    #[allow(dead_code)]
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Improving => "↑",
            Self::Declining => "↓",
            Self::Stable => "→",
            Self::InsufficientData => "?",
        }
    }

    /// Get display text
    #[allow(dead_code)]
    pub fn display_text(&self) -> &'static str {
        match self {
            Self::Improving => "improving",
            Self::Declining => "declining",
            Self::Stable => "stable",
            Self::InsufficientData => "insufficient data",
        }
    }
}

/// Statistics for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackPeriodStats {
    /// Period identifier (e.g., "2026-W14" for week 14, "2026-04-03" for day)
    pub period: String,
    /// Start of the period
    pub start_time: DateTime<Utc>,
    /// End of the period
    pub end_time: DateTime<Utc>,
    /// Total feedback count in this period
    pub total_count: u32,
    /// Thumbs up count
    pub thumbs_up_count: u32,
    /// Thumbs down count
    pub thumbs_down_count: u32,
    /// Neutral count
    pub neutral_count: u32,
    /// Positive rate (0.0 to 1.0)
    pub positive_rate: f32,
    /// Whether this is the most recent period
    pub is_current: bool,
}

impl FeedbackPeriodStats {
    /// Create stats from feedback entries for a period
    pub fn from_feedback(period: &str, start: DateTime<Utc>, end: DateTime<Utc>, feedbacks: &[&TurnFeedback], is_current: bool) -> Self {
        let total_count = feedbacks.len() as u32;
        let thumbs_up_count = feedbacks.iter().filter(|f| f.is_positive()).count() as u32;
        let thumbs_down_count = feedbacks.iter().filter(|f| f.is_negative()).count() as u32;
        let neutral_count = feedbacks.iter().filter(|f| f.rating == FeedbackRating::Neutral).count() as u32;
        let positive_rate = if total_count > 0 {
            thumbs_up_count as f32 / total_count as f32
        } else {
            0.0
        };

        Self {
            period: period.to_string(),
            start_time: start,
            end_time: end,
            total_count,
            thumbs_up_count,
            thumbs_down_count,
            neutral_count,
            positive_rate,
            is_current,
        }
    }
}

/// A detected issue pattern in feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackIssuePattern {
    /// Type of issue detected
    pub issue_type: IssueType,
    /// Number of times this pattern occurred
    pub occurrence_count: u32,
    /// First occurrence time
    pub first_seen: DateTime<Utc>,
    /// Most recent occurrence time
    pub last_seen: DateTime<Utc>,
    /// Example comments (up to 3)
    pub example_comments: Vec<String>,
    /// Suggested agent behavior change
    pub suggestion: String,
}

/// Types of issues that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// Response is too verbose
    Verbose,
    /// Response is too brief
    TooBrief,
    /// Provided incorrect information
    IncorrectInfo,
    /// Didn't understand the request
    Misunderstood,
    /// Tool execution failed
    ToolFailed,
    /// Didn't solve the problem
    NotHelpful,
    /// Response was confusing
    Confusing,
    /// Slow to respond
    Slow,
}

impl IssueType {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Verbose => "Response too verbose",
            Self::TooBrief => "Response too brief",
            Self::IncorrectInfo => "Incorrect information",
            Self::Misunderstood => "Misunderstood request",
            Self::ToolFailed => "Tool execution failed",
            Self::NotHelpful => "Didn't solve problem",
            Self::Confusing => "Confusing response",
            Self::Slow => "Slow to respond",
        }
    }

    /// Get emoji representation
    #[allow(dead_code)]
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Verbose => "📝",
            Self::TooBrief => "💬",
            Self::IncorrectInfo => "❌",
            Self::Misunderstood => "❓",
            Self::ToolFailed => "🔧",
            Self::NotHelpful => "🤔",
            Self::Confusing => "😕",
            Self::Slow => "⏳",
        }
    }

    /// Detect issue type from comment text
    pub fn detect_from_comment(comment: &str) -> Option<Self> {
        let lower = comment.to_lowercase();

        // Check for specific patterns
        if lower.contains("too long") || lower.contains("too verbose") || lower.contains("too detailed") || lower.contains("wordy") {
            Some(Self::Verbose)
        } else if lower.contains("too short") || lower.contains("too brief") || lower.contains("not enough") || lower.contains("lacking detail") {
            Some(Self::TooBrief)
        } else if lower.contains("wrong") || lower.contains("incorrect") || lower.contains("inaccurate") || lower.contains("bad information") {
            Some(Self::IncorrectInfo)
        } else if lower.contains("misunderstand") || lower.contains("didn't understand") || lower.contains("don't understand") || lower.contains("wrong topic") {
            Some(Self::Misunderstood)
        } else if lower.contains("didn't work") || lower.contains("doesn't work") || lower.contains("failed") || lower.contains("error") {
            Some(Self::ToolFailed)
        } else if lower.contains("not helpful") || lower.contains("didn't help") || lower.contains("doesn't help") || lower.contains("couldn't solve") {
            Some(Self::NotHelpful)
        } else if lower.contains("confusing") || lower.contains("unclear") || lower.contains("hard to understand") || lower.contains("didn't make sense") {
            Some(Self::Confusing)
        } else if lower.contains("slow") || lower.contains("taking too long") || lower.contains("takes forever") || lower.contains("too slow") {
            Some(Self::Slow)
        } else {
            None
        }
    }
}

/// Complete trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTrendAnalysis {
    /// Overall trend direction
    pub overall_trend: TrendDirection,
    /// Trend magnitude (0.0 to 1.0, where 1.0 is very strong)
    pub trend_strength: f32,
    /// Statistics for recent periods
    pub period_stats: Vec<FeedbackPeriodStats>,
    /// Detected issue patterns
    pub issue_patterns: Vec<FeedbackIssuePattern>,
    /// Summary text
    pub summary: String,
    /// Number of periods analyzed
    pub periods_analyzed: u32,
    /// Total feedback entries analyzed
    pub total_feedback_analyzed: u32,
}

/// Feedback trend analyzer that operates on TurnFeedbackManager data
#[allow(dead_code)]
pub struct FeedbackTrendAnalyzer;

impl FeedbackTrendAnalyzer {
    /// Analyze feedback trends across all sessions or a specific session
    pub fn analyze_trends(manager: &TurnFeedbackManager, session_id: Option<&str>) -> FeedbackTrendAnalysis {
        // Get relevant feedback
        let all_feedback: Vec<TurnFeedback> = if let Some(sid) = session_id {
            manager.get_session_feedback(sid)
        } else {
            // Get feedback from all sessions
            let sessions = manager.get_sessions_with_feedback();
            sessions.iter()
                .flat_map(|s| manager.get_session_feedback(s))
                .collect()
        };

        // Sort by creation time
        let mut sorted_feedback = all_feedback.clone();
        sorted_feedback.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Calculate period stats (last 7 days)
        let period_stats = Self::calculate_period_stats(&sorted_feedback);

        // Detect issue patterns
        let issue_patterns = Self::detect_issue_patterns(&sorted_feedback);

        // Calculate trend
        let (overall_trend, trend_strength) = Self::calculate_trend(&period_stats);

        // Generate summary
        let summary = Self::generate_summary(&overall_trend, trend_strength, &period_stats, &issue_patterns);

        let periods_analyzed = period_stats.len() as u32;
        let total_feedback_analyzed = sorted_feedback.len() as u32;

        FeedbackTrendAnalysis {
            overall_trend,
            trend_strength,
            period_stats,
            issue_patterns,
            summary,
            periods_analyzed,
            total_feedback_analyzed,
        }
    }

    /// Calculate statistics for each period (daily for last 7 days)
    fn calculate_period_stats(feedback: &[TurnFeedback]) -> Vec<FeedbackPeriodStats> {
        let now = Utc::now();
        let mut stats = Vec::new();

        for days_ago in 0..7 {
            let period_end = now - Duration::days(days_ago);
            let period_start = period_end - Duration::days(1);

            // Format period as "Mon", "Tue", etc. for display
            let period_label = if days_ago == 0 {
                "Today".to_string()
            } else if days_ago == 1 {
                "Yesterday".to_string()
            } else {
                period_end.format("%a").to_string() // Day name
            };

            // Filter feedback in this period
            let period_feedback: Vec<_> = feedback.iter()
                .filter(|f| f.created_at >= period_start && f.created_at < period_end)
                .collect();

            let is_current = days_ago == 0;
            stats.push(FeedbackPeriodStats::from_feedback(
                &period_label,
                period_start,
                period_end,
                &period_feedback,
                is_current,
            ));
        }

        // Reverse to get chronological order
        stats.reverse();
        stats
    }

    /// Detect issue patterns from negative feedback comments
    fn detect_issue_patterns(feedback: &[TurnFeedback]) -> Vec<FeedbackIssuePattern> {
        /// Internal type for tracking pattern statistics
        type PatternStats = (u32, DateTime<Utc>, DateTime<Utc>, Vec<String>);

        let mut pattern_map: HashMap<IssueType, PatternStats> = HashMap::new();

        for f in feedback.iter().filter(|f| f.is_negative()) {
            if let Some(ref comment) = f.comment {
                if let Some(issue_type) = IssueType::detect_from_comment(comment) {
                    let entry = pattern_map.entry(issue_type).or_insert((
                        0,
                        f.created_at,
                        f.created_at,
                        Vec::new(),
                    ));
                    entry.0 += 1;
                    if f.created_at < entry.1 {
                        entry.1 = f.created_at;
                    }
                    if f.created_at > entry.2 {
                        entry.2 = f.created_at;
                    }
                    if entry.3.len() < 3 && !entry.3.contains(comment) {
                        entry.3.push(comment.clone());
                    }
                }
            }
        }

        // Convert to sorted vec (most frequent first)
        let mut patterns: Vec<_> = pattern_map.into_iter()
            .map(|(issue_type, (count, first, last, examples))| {
                let suggestion = Self::get_suggestion_for_issue(&issue_type, count);
                FeedbackIssuePattern {
                    issue_type,
                    occurrence_count: count,
                    first_seen: first,
                    last_seen: last,
                    example_comments: examples,
                    suggestion,
                }
            })
            .collect();

        patterns.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));
        patterns
    }

    /// Get a suggestion for an issue type
    fn get_suggestion_for_issue(issue: &IssueType, count: u32) -> String {
        let count_text = if count > 3 { "frequently" } else { "occasionally" };
        match issue {
            IssueType::Verbose => format!("Be more concise — {} reported as too verbose", count_text),
            IssueType::TooBrief => format!("Provide more detail — {} reported as too brief", count_text),
            IssueType::IncorrectInfo => format!("Verify information carefully — {} reported incorrect info", count_text),
            IssueType::Misunderstood => format!("Ask clarifying questions — {} misunderstood the request", count_text),
            IssueType::ToolFailed => format!("Check tool errors more carefully — {} had tool failures", count_text),
            IssueType::NotHelpful => format!("Try different approaches — {} still unresolved", count_text),
            IssueType::Confusing => format!("Structure responses better — {} found confusing", count_text),
            IssueType::Slow => format!("Optimize response time — {} reported slow", count_text),
        }
    }

    /// Calculate trend direction and strength from period stats
    fn calculate_trend(periods: &[FeedbackPeriodStats]) -> (TrendDirection, f32) {
        // Filter periods with data
        let periods_with_data: Vec<_> = periods.iter()
            .filter(|p| p.total_count > 0)
            .collect();

        if periods_with_data.len() < 2 {
            return (TrendDirection::InsufficientData, 0.0);
        }

        // Calculate average positive rate for first half vs second half
        let mid = periods_with_data.len() / 2;
        let first_half: f32 = periods_with_data[..mid].iter()
            .map(|p| p.positive_rate)
            .sum::<f32>() / mid as f32;

        let second_half: f32 = periods_with_data[mid..].iter()
            .map(|p| p.positive_rate)
            .sum::<f32>() / (periods_with_data.len() - mid) as f32;

        let change = second_half - first_half;

        // Determine direction and strength
        let direction = if change > 0.1 {
            TrendDirection::Improving
        } else if change < -0.1 {
            TrendDirection::Declining
        } else {
            TrendDirection::Stable
        };

        // Trend strength is the absolute magnitude, capped at 1.0
        let strength = change.abs().min(1.0);

        (direction, strength)
    }

    /// Generate a human-readable summary
    fn generate_summary(
        trend: &TrendDirection,
        strength: f32,
        periods: &[FeedbackPeriodStats],
        patterns: &[FeedbackIssuePattern],
    ) -> String {
        let mut parts = Vec::new();

        // Trend summary
        match trend {
            TrendDirection::Improving => {
                if strength > 0.5 {
                    parts.push("Strong improvement in feedback quality over the analyzed period.".to_string());
                } else {
                    parts.push("Slight improvement in feedback quality over the analyzed period.".to_string());
                }
            }
            TrendDirection::Declining => {
                if strength > 0.5 {
                    parts.push("Significant decline in feedback quality — review recent changes.".to_string());
                } else {
                    parts.push("Slight decline in feedback quality.".to_string());
                }
            }
            TrendDirection::Stable => {
                parts.push("Feedback quality has remained stable.".to_string());
            }
            TrendDirection::InsufficientData => {
                parts.push("Not enough data to determine a trend.".to_string());
            }
        }

        // Period stats summary
        let periods_with_data = periods.iter().filter(|p| p.total_count > 0).count();
        if periods_with_data > 0 {
            let total = periods.iter().map(|p| p.total_count).sum::<u32>();
            let thumbs_up = periods.iter().map(|p| p.thumbs_up_count).sum::<u32>();
            let overall_rate = if total > 0 { thumbs_up as f32 / total as f32 } else { 0.0 };
            parts.push(format!(
                "Analyzed {} feedback entries across {} periods with {:.0}% positive rate.",
                total, periods_with_data, overall_rate * 100.0
            ));
        } else {
            parts.push("No feedback data available for analysis.".to_string());
        }

        // Top issue pattern
        if let Some(top_issue) = patterns.first() {
            parts.push(format!(
                "Most common issue: {} ({} occurrences).",
                top_issue.issue_type.display_name(),
                top_issue.occurrence_count
            ));
        }

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_issue_type_detection() {
        assert_eq!(IssueType::detect_from_comment("Response is too long"), Some(IssueType::Verbose));
        assert_eq!(IssueType::detect_from_comment("TOO VERBOSE"), Some(IssueType::Verbose));
        assert_eq!(IssueType::detect_from_comment("too short and lacking detail"), Some(IssueType::TooBrief));
        assert_eq!(IssueType::detect_from_comment("wrong information provided"), Some(IssueType::IncorrectInfo));
        assert_eq!(IssueType::detect_from_comment("didn't understand what I wanted"), Some(IssueType::Misunderstood));
        assert_eq!(IssueType::detect_from_comment("didn't work at all"), Some(IssueType::ToolFailed));
        assert_eq!(IssueType::detect_from_comment("not helpful at all"), Some(IssueType::NotHelpful));
        assert_eq!(IssueType::detect_from_comment("very confusing response"), Some(IssueType::Confusing));
        assert_eq!(IssueType::detect_from_comment("too slow to respond"), Some(IssueType::Slow));
        assert_eq!(IssueType::detect_from_comment("great job!"), None);
    }

    #[test]
    fn test_trend_direction_display() {
        assert_eq!(TrendDirection::Improving.emoji(), "↑");
        assert_eq!(TrendDirection::Declining.emoji(), "↓");
        assert_eq!(TrendDirection::Stable.emoji(), "→");
        assert_eq!(TrendDirection::InsufficientData.emoji(), "?");
        assert_eq!(TrendDirection::Improving.display_text(), "improving");
    }

    #[test]
    fn test_feedback_period_stats() {
        let now = Utc::now();
        let feedback = vec![
            TurnFeedback::new("t1", "s1", FeedbackRating::ThumbsUp),
            TurnFeedback::new("t2", "s1", FeedbackRating::ThumbsDown),
        ];

        let stats = FeedbackPeriodStats::from_feedback(
            "Test",
            now - Duration::hours(1),
            now,
            &feedback.iter().collect::<Vec<_>>(),
            false,
        );

        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.thumbs_up_count, 1);
        assert_eq!(stats.thumbs_down_count, 1);
        assert!((stats.positive_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_issue_type_display() {
        assert_eq!(IssueType::Verbose.display_name(), "Response too verbose");
        assert_eq!(IssueType::IncorrectInfo.display_name(), "Incorrect information");
        assert_eq!(IssueType::ToolFailed.emoji(), "🔧");
    }

    #[test]
    fn test_get_suggestion_for_issue() {
        let suggestion = FeedbackTrendAnalyzer::get_suggestion_for_issue(&IssueType::Verbose, 5);
        assert!(suggestion.contains("frequently"));
        assert!(suggestion.contains("verbose"));

        let suggestion = FeedbackTrendAnalyzer::get_suggestion_for_issue(&IssueType::Verbose, 2);
        assert!(suggestion.contains("occasionally"));
    }
}
