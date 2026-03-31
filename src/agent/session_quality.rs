//! Session Quality Analysis Module
//!
//! Analyzes the overall quality of agent sessions and provides improvement suggestions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

use super::turn_history::TurnRecord;
use super::self_evaluation::SelfEvaluation;

/// Quality issue types that can occur in a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QualityIssue {
    /// Repeated similar questions from user
    RepeatedQuestions,
    /// Too many tool errors
    ToolErrors,
    /// High token usage
    HighTokenUsage,
    /// Slow responses
    SlowResponses,
    /// Low success rate
    LowSuccessRate,
    /// Inefficient tool usage
    InefficientTools,
    /// User dissatisfaction signals
    Dissatisfaction,
}

impl QualityIssue {
    /// Get severity level (1-5, 5 being most severe)
    pub fn severity(&self) -> u8 {
        match self {
            QualityIssue::RepeatedQuestions => 2,
            QualityIssue::ToolErrors => 4,
            QualityIssue::HighTokenUsage => 2,
            QualityIssue::SlowResponses => 3,
            QualityIssue::LowSuccessRate => 4,
            QualityIssue::InefficientTools => 3,
            QualityIssue::Dissatisfaction => 5,
        }
    }
}

/// A quality issue detected in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedIssue {
    /// The type of issue
    pub issue_type: QualityIssue,
    /// Severity level (1-5)
    pub severity: u8,
    /// Description of the issue
    pub description: String,
    /// Number of occurrences
    pub count: u32,
    /// Suggested fix
    pub suggestion: String,
}

/// Overall session quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionQuality {
    /// Session ID
    pub session_id: String,
    /// Overall quality score (0.0 - 1.0)
    pub quality_score: f64,
    /// Number of turns in session
    pub turn_count: u32,
    /// Task completion rate (0.0 - 1.0)
    pub task_completion_rate: f64,
    /// Average response quality
    pub avg_response_quality: f64,
    /// Average efficiency score
    pub avg_efficiency: f64,
    /// Tool success rate
    pub tool_success_rate: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Average response time (ms)
    pub avg_response_time_ms: u64,
    /// Detected issues
    pub issues: Vec<DetectedIssue>,
    /// Improvement suggestions
    pub suggestions: Vec<String>,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Quality rating (1-5 stars)
    pub rating: u8,
}

/// Summary of session quality for list display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionQualitySummary {
    pub session_id: String,
    pub quality_score: f64,
    pub turn_count: u32,
    pub issue_count: u32,
    pub rating: u8,
    pub last_activity: DateTime<Utc>,
}

/// Session quality analyzer
pub struct SessionQualityAnalyzer;

impl SessionQualityAnalyzer {
    /// Analyze a session's quality based on turn history and evaluations
    pub fn analyze_session(
        session_id: &str,
        turns: &[TurnRecord],
        evaluations: &[SelfEvaluation],
    ) -> SessionQuality {
        if turns.is_empty() {
            return SessionQuality {
                session_id: session_id.to_string(),
                quality_score: 0.5,
                turn_count: 0,
                task_completion_rate: 0.5,
                avg_response_quality: 0.5,
                avg_efficiency: 0.5,
                tool_success_rate: 0.5,
                total_tokens: 0,
                avg_response_time_ms: 0,
                issues: Vec::new(),
                suggestions: Vec::new(),
                started_at: Utc::now(),
                last_activity: Utc::now(),
                rating: 3,
            };
        }
        
        let turn_count = turns.len() as u32;
        
        // Calculate metrics
        let successful_turns = turns.iter().filter(|t| t.success).count() as f64;
        let task_completion_rate = successful_turns / turn_count as f64;
        
        // Calculate average from self-evaluations
        let avg_response_quality = if !evaluations.is_empty() {
            let sum: f64 = evaluations.iter()
                .filter_map(|e| e.dimension_scores.iter()
                    .find(|ds| ds.dimension == super::self_evaluation::EvaluationDimension::ResponseQuality))
                .map(|ds| ds.score)
                .sum();
            sum / evaluations.len() as f64
        } else {
            0.5
        };
        
        let avg_efficiency = if !evaluations.is_empty() {
            let sum: f64 = evaluations.iter()
                .filter_map(|e| e.dimension_scores.iter()
                    .find(|ds| ds.dimension == super::self_evaluation::EvaluationDimension::Efficiency))
                .map(|ds| ds.score)
                .sum();
            sum / evaluations.len() as f64
        } else {
            0.5
        };
        
        // Tool success rate
        let all_tools: Vec<_> = turns.iter().flat_map(|t| t.tools.iter()).collect();
        let tool_success_rate = if !all_tools.is_empty() {
            all_tools.iter().filter(|t| t.success).count() as f64 / all_tools.len() as f64
        } else {
            0.5
        };
        
        // Total tokens
        let total_tokens = turns.iter()
            .filter_map(|t| t.token_usage.as_ref())
            .map(|u| u.total_tokens as u64)
            .sum();
        
        // Average response time
        let total_duration: u64 = turns.iter().map(|t| t.duration_ms).sum();
        let avg_response_time_ms = if turn_count > 0 {
            total_duration / turn_count as u64
        } else {
            0
        };
        
        // Detect issues
        let issues = Self::detect_issues(turns, evaluations, task_completion_rate, tool_success_rate, total_tokens, avg_response_time_ms);
        
        // Generate suggestions
        let suggestions = Self::generate_suggestions(&issues, avg_response_quality, avg_efficiency, tool_success_rate);
        
        // Calculate overall quality score
        let quality_score = Self::calculate_quality_score(
            task_completion_rate,
            avg_response_quality,
            avg_efficiency,
            tool_success_rate,
            &issues,
        );
        
        // Calculate rating (1-5 stars)
        let rating = Self::calculate_rating(quality_score);
        
        // Get timestamps
        let started_at = turns.iter()
            .map(|t| t.created_at)
            .min()
            .unwrap_or_else(Utc::now);
        
        let last_activity = turns.iter()
            .map(|t| t.created_at)
            .max()
            .unwrap_or_else(Utc::now);
        
        SessionQuality {
            session_id: session_id.to_string(),
            quality_score,
            turn_count,
            task_completion_rate,
            avg_response_quality,
            avg_efficiency,
            tool_success_rate,
            total_tokens,
            avg_response_time_ms,
            issues,
            suggestions,
            started_at,
            last_activity,
            rating,
        }
    }
    
    /// Detect quality issues in the session
    fn detect_issues(
        turns: &[TurnRecord],
        evaluations: &[SelfEvaluation],
        task_completion_rate: f64,
        tool_success_rate: f64,
        total_tokens: u64,
        avg_response_time_ms: u64,
    ) -> Vec<DetectedIssue> {
        let mut issues = Vec::new();
        
        // Check for repeated questions
        let user_messages: Vec<_> = turns.iter()
            .map(|t| t.user_message.to_lowercase())
            .collect();
        
        let mut message_counts: HashMap<String, u32> = HashMap::new();
        for msg in &user_messages {
            // Group similar messages (first 30 chars)
            let key = msg.chars().take(30).collect::<String>();
            *message_counts.entry(key).or_insert(0) += 1;
        }
        
        let repeated: Vec<_> = message_counts.iter().filter(|(_, &c)| c >= 3).collect();
        if !repeated.is_empty() {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::RepeatedQuestions,
                severity: QualityIssue::RepeatedQuestions.severity(),
                description: format!("发现 {} 种重复问题模式", repeated.len()),
                count: repeated.iter().map(|(_, &c)| c).sum(),
                suggestion: "考虑创建 FAQ 或使用记忆功能记住用户偏好".to_string(),
            });
        }
        
        // Check for tool errors
        let tool_errors = turns.iter()
            .flat_map(|t| t.tools.iter())
            .filter(|t| !t.success)
            .count() as u32;
        
        if tool_errors >= 3 {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::ToolErrors,
                severity: QualityIssue::ToolErrors.severity(),
                description: format!("{} 次工具执行失败", tool_errors),
                count: tool_errors,
                suggestion: "检查工具参数和权限设置".to_string(),
            });
        }
        
        // Check for high token usage
        let avg_tokens_per_turn = if !turns.is_empty() {
            total_tokens as f64 / turns.len() as f64
        } else {
            0.0
        };
        
        if avg_tokens_per_turn > 3000.0 {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::HighTokenUsage,
                severity: QualityIssue::HighTokenUsage.severity(),
                description: format!("平均每轮 {} tokens", avg_tokens_per_turn as u32),
                count: 1,
                suggestion: "考虑使用上下文摘要减少 token 使用".to_string(),
            });
        }
        
        // Check for slow responses
        if avg_response_time_ms > 30000 {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::SlowResponses,
                severity: QualityIssue::SlowResponses.severity(),
                description: format!("平均响应时间 {}ms", avg_response_time_ms),
                count: 1,
                suggestion: "考虑优化工具调用或增加并行处理".to_string(),
            });
        }
        
        // Check for low success rate
        if task_completion_rate < 0.5 {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::LowSuccessRate,
                severity: QualityIssue::LowSuccessRate.severity(),
                description: format!("任务完成率仅 {:.0}%", task_completion_rate * 100.0),
                count: 1,
                suggestion: "分析失败原因，改进 Agent 指令或工具选择".to_string(),
            });
        }
        
        // Check for low tool success rate
        if tool_success_rate < 0.6 && turns.iter().any(|t| !t.tools.is_empty()) {
            issues.push(DetectedIssue {
                issue_type: QualityIssue::InefficientTools,
                severity: QualityIssue::InefficientTools.severity(),
                description: format!("工具成功率仅 {:.0}%", tool_success_rate * 100.0),
                count: 1,
                suggestion: "检查工具使用策略和错误处理".to_string(),
            });
        }
        
        // Check evaluations for dissatisfaction signals
        if !evaluations.is_empty() {
            let low_score_count = evaluations.iter()
                .filter(|e| e.overall_score < 0.4)
                .count() as u32;
            
            if low_score_count >= 2 {
                issues.push(DetectedIssue {
                    issue_type: QualityIssue::Dissatisfaction,
                    severity: QualityIssue::Dissatisfaction.severity(),
                    description: format!("{} 次低质量评估", low_score_count),
                    count: low_score_count,
                    suggestion: "分析评估结果，识别具体问题".to_string(),
                });
            }
        }
        
        // Sort by severity
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));
        
        issues
    }
    
    /// Generate improvement suggestions based on detected issues
    fn generate_suggestions(
        issues: &[DetectedIssue],
        avg_response_quality: f64,
        avg_efficiency: f64,
        tool_success_rate: f64,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Generate suggestions from issues
        for issue in issues.iter().take(3) {
            suggestions.push(issue.suggestion.clone());
        }
        
        // Generate suggestions from metrics
        if avg_response_quality < 0.5 && suggestions.len() < 3 {
            suggestions.push("响应质量有提升空间，考虑提供更详细的答案".to_string());
        }
        
        if avg_efficiency < 0.5 && suggestions.len() < 3 {
            suggestions.push("执行效率可以优化，考虑减少不必要的工具调用".to_string());
        }
        
        if tool_success_rate < 0.7 && suggestions.len() < 3 {
            suggestions.push("工具使用策略需要改进".to_string());
        }
        
        // Default suggestion if no issues
        if suggestions.is_empty() {
            suggestions.push("会话质量良好继续保持".to_string());
        }
        
        suggestions
    }
    
    /// Calculate overall quality score
    fn calculate_quality_score(
        task_completion_rate: f64,
        avg_response_quality: f64,
        avg_efficiency: f64,
        tool_success_rate: f64,
        issues: &[DetectedIssue],
    ) -> f64 {
        // Base score from metrics
        let metrics_score = task_completion_rate * 0.3 +
            avg_response_quality * 0.25 +
            avg_efficiency * 0.2 +
            tool_success_rate * 0.25;
        
        // Deduct for issues (severity * 0.05)
        let issue_penalty: f64 = issues.iter()
            .map(|i| i.severity as f64 * 0.03)
            .sum();
        
        // Final score
        let score = metrics_score - issue_penalty;
        
        // Clamp to 0.0 - 1.0
        score.clamp(0.0, 1.0)
    }
    
    /// Calculate rating (1-5 stars)
    fn calculate_rating(quality_score: f64) -> u8 {
        if quality_score >= 0.9 { 5 }
        else if quality_score >= 0.7 { 4 }
        else if quality_score >= 0.5 { 3 }
        else if quality_score >= 0.3 { 2 }
        else { 1 }
    }
}

/// Manager for session quality analysis
pub struct SessionQualityManager {
    /// Cache of session quality results
    cache: RwLock<HashMap<String, SessionQuality>>,
}

impl SessionQualityManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }
    
    /// Analyze a session's quality
    pub fn analyze_session(
        &self,
        session_id: &str,
        turns: &[TurnRecord],
        evaluations: &[SelfEvaluation],
    ) -> SessionQuality {
        let quality = SessionQualityAnalyzer::analyze_session(session_id, turns, evaluations);
        
        // Cache the result
        self.cache.write().insert(session_id.to_string(), quality.clone());
        
        quality
    }
    
    /// Get cached quality for a session
    pub fn get_cached(&self, session_id: &str) -> Option<SessionQuality> {
        self.cache.read().get(session_id).cloned()
    }
    
    /// Clear cache for a session
    pub fn invalidate(&self, session_id: &str) {
        self.cache.write().remove(session_id);
    }
    
    /// Get summary list for all cached sessions
    pub fn get_summaries(&self) -> Vec<SessionQualitySummary> {
        self.cache.read()
            .values()
            .map(|q| SessionQualitySummary {
                session_id: q.session_id.clone(),
                quality_score: q.quality_score,
                turn_count: q.turn_count,
                issue_count: q.issues.len() as u32,
                rating: q.rating,
                last_activity: q.last_activity,
            })
            .collect()
    }
}

impl Default for SessionQualityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::turn_history::{TokenUsage, ToolExecution};
    use crate::agent::self_evaluation::{DimensionScore, EvaluationDimension};
    use uuid::Uuid;

    fn create_test_turn(message: &str, success: bool, tools: Vec<ToolExecution>, duration_ms: u64, tokens: u32) -> TurnRecord {
        TurnRecord {
            id: Uuid::new_v4().to_string(),
            session_id: "test".to_string(),
            user_message: message.to_string(),
            response_preview: "Test response".to_string(),
            tools,
            duration_ms,
            success,
            created_at: Utc::now(),
            token_usage: Some(TokenUsage {
                input_tokens: tokens / 3,
                output_tokens: tokens * 2 / 3,
                total_tokens: tokens,
            }),
        }
    }

    fn create_test_evaluation(overall_score: f64) -> SelfEvaluation {
        SelfEvaluation {
            id: Uuid::new_v4().to_string(),
            turn_id: Uuid::new_v4().to_string(),
            session_id: "test".to_string(),
            overall_score,
            dimension_scores: vec![
                DimensionScore {
                    dimension: EvaluationDimension::ResponseQuality,
                    score: overall_score,
                    reason: "Test".to_string(),
                },
                DimensionScore {
                    dimension: EvaluationDimension::Efficiency,
                    score: overall_score,
                    reason: "Test".to_string(),
                },
            ],
            strengths: vec!["Good".to_string()],
            weaknesses: vec!["Could be better".to_string()],
            improvement_suggestions: vec!["Improve".to_string()],
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_empty_session() {
        let quality = SessionQualityAnalyzer::analyze_session("test", &[], &[]);
        
        assert_eq!(quality.session_id, "test");
        assert_eq!(quality.turn_count, 0);
        assert_eq!(quality.rating, 3); // Default rating
    }

    #[test]
    fn test_successful_session() {
        let tools = vec![ToolExecution {
            name: "test".to_string(),
            input: serde_json::json!({}),
            output_preview: "ok".to_string(),
            success: true,
            duration_ms: 100,
        }];
        
        let turns = vec![create_test_turn("Hello", true, tools.clone(), 1000, 200)];
        let evaluations = vec![create_test_evaluation(0.8)];
        
        let quality = SessionQualityAnalyzer::analyze_session("test", &turns, &evaluations);
        
        assert!(quality.quality_score > 0.5);
        assert!(quality.rating >= 3);
        assert_eq!(quality.turn_count, 1);
    }

    #[test]
    fn test_failed_session() {
        let tools = vec![ToolExecution {
            name: "test".to_string(),
            input: serde_json::json!({}),
            output_preview: "error".to_string(),
            success: false,
            duration_ms: 100,
        }];
        
        let turns = vec![create_test_turn("Hello", false, tools.clone(), 1000, 200)];
        
        let quality = SessionQualityAnalyzer::analyze_session("test", &turns, &[]);
        
        assert!(quality.quality_score < 0.7);
        assert!(!quality.issues.is_empty() || quality.task_completion_rate < 1.0);
    }

    #[test]
    fn test_repeated_questions_detection() {
        let turns = vec![
            create_test_turn("How are you?", true, vec![], 100, 100),
            create_test_turn("How are you?", true, vec![], 100, 100),
            create_test_turn("How are you?", true, vec![], 100, 100),
        ];
        
        let quality = SessionQualityAnalyzer::analyze_session("test", &turns, &[]);
        
        let has_repeated = quality.issues.iter()
            .any(|i| i.issue_type == QualityIssue::RepeatedQuestions);
        
        assert!(has_repeated, "Should detect repeated questions");
    }

    #[test]
    fn test_high_token_usage() {
        let turns = vec![
            create_test_turn("Test", true, vec![], 100, 5000),
            create_test_turn("Test", true, vec![], 100, 5000),
        ];
        
        let quality = SessionQualityAnalyzer::analyze_session("test", &turns, &[]);
        
        let has_high_tokens = quality.issues.iter()
            .any(|i| i.issue_type == QualityIssue::HighTokenUsage);
        
        assert!(has_high_tokens, "Should detect high token usage");
    }

    #[test]
    fn test_rating_calculation() {
        assert_eq!(SessionQualityAnalyzer::calculate_rating(0.95), 5);
        assert_eq!(SessionQualityAnalyzer::calculate_rating(0.8), 4);
        assert_eq!(SessionQualityAnalyzer::calculate_rating(0.6), 3);
        assert_eq!(SessionQualityAnalyzer::calculate_rating(0.4), 2);
        assert_eq!(SessionQualityAnalyzer::calculate_rating(0.2), 1);
    }

    #[test]
    fn test_manager_cache() {
        let manager = SessionQualityManager::new();
        
        let turns = vec![create_test_turn("Test", true, vec![], 100, 100)];
        
        let quality1 = manager.analyze_session("test", &turns, &[]);
        let quality2 = manager.get_cached("test").unwrap();
        
        assert_eq!(quality1.session_id, quality2.session_id);
    }
}
