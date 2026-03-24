//! Agent Self-Evaluation Module
//!
//! Enables the Agent to evaluate its own performance after each turn.
//! Evaluates multiple dimensions: task success, tool selection, efficiency, and response quality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use uuid::Uuid;

use super::turn_history::TurnRecord;

/// A specific dimension being evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvaluationDimension {
    /// Whether the turn completed successfully
    TaskSuccess,
    /// Whether appropriate tools were selected and used
    ToolSelection,
    /// How efficiently resources were used
    Efficiency,
    /// Quality of the response generated
    ResponseQuality,
}

impl EvaluationDimension {
    /// Get the display name for this dimension
    pub fn display_name(&self) -> &'static str {
        match self {
            EvaluationDimension::TaskSuccess => "Task Success",
            EvaluationDimension::ToolSelection => "Tool Selection",
            EvaluationDimension::Efficiency => "Efficiency",
            EvaluationDimension::ResponseQuality => "Response Quality",
        }
    }

    /// Get a description of this dimension
    pub fn description(&self) -> &'static str {
        match self {
            EvaluationDimension::TaskSuccess => "Did the turn complete the requested task?",
            EvaluationDimension::ToolSelection => "Were the right tools chosen for the job?",
            EvaluationDimension::Efficiency => "How well were time and tokens used?",
            EvaluationDimension::ResponseQuality => "Was the response helpful and accurate?",
        }
    }
}

/// A score for a specific dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    /// The dimension being scored
    pub dimension: EvaluationDimension,
    /// Score from 0.0 to 1.0
    pub score: f64,
    /// Brief reason for this score
    pub reason: String,
}

/// A complete self-evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfEvaluation {
    /// Unique evaluation ID
    pub id: String,
    /// Turn ID this evaluation is for
    pub turn_id: String,
    /// Session ID
    pub session_id: String,
    /// Overall score from 0.0 to 1.0
    pub overall_score: f64,
    /// Individual dimension scores
    pub dimension_scores: Vec<DimensionScore>,
    /// What went well (list of strings)
    pub strengths: Vec<String>,
    /// What could be improved (list of strings)
    pub weaknesses: Vec<String>,
    /// Suggestions for improvement
    pub improvement_suggestions: Vec<String>,
    /// Evaluation timestamp
    pub created_at: DateTime<Utc>,
}

impl SelfEvaluation {
    /// Create a new self-evaluation
    pub fn new(
        turn_id: String,
        session_id: String,
        overall_score: f64,
        dimension_scores: Vec<DimensionScore>,
        strengths: Vec<String>,
        weaknesses: Vec<String>,
        improvement_suggestions: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            turn_id,
            session_id,
            overall_score,
            dimension_scores,
            strengths,
            weaknesses,
            improvement_suggestions,
            created_at: Utc::now(),
        }
    }

    /// Convert to a summary for list display
    pub fn summary(&self) -> SelfEvaluationSummary {
        SelfEvaluationSummary {
            id: self.id.clone(),
            turn_id: self.turn_id.clone(),
            session_id: self.session_id.clone(),
            overall_score: self.overall_score,
            created_at: self.created_at,
            top_strength: self.strengths.first().cloned(),
            top_weakness: self.weaknesses.first().cloned(),
        }
    }
}

/// Summary of an evaluation for list display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfEvaluationSummary {
    pub id: String,
    pub turn_id: String,
    pub session_id: String,
    pub overall_score: f64,
    pub created_at: DateTime<Utc>,
    pub top_strength: Option<String>,
    pub top_weakness: Option<String>,
}

/// Aggregated statistics for self-evaluations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelfEvaluationStats {
    /// Total evaluations recorded
    pub total_evaluations: u64,
    /// Average overall score
    pub avg_overall_score: f64,
    /// Average scores per dimension
    pub avg_dimension_scores: HashMap<String, f64>,
    /// Score distribution (buckets)
    pub score_distribution: HashMap<String, u64>,
    /// Most common strengths
    pub common_strengths: Vec<String>,
    /// Most common weaknesses
    pub common_weaknesses: Vec<String>,
    /// Evaluations by session
    pub evaluations_by_session: HashMap<String, u64>,
}

/// Self-evaluation engine that evaluates turn performance
pub struct SelfEvaluationEngine;

impl SelfEvaluationEngine {
    /// Evaluate a turn and produce a self-evaluation
    pub fn evaluate(turn: &TurnRecord) -> SelfEvaluation {
        let mut dimension_scores = Vec::new();
        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();
        let mut suggestions = Vec::new();

        // 1. Task Success Score
        let task_score = Self::evaluate_task_success(turn, &mut strengths, &mut weaknesses, &mut suggestions);
        dimension_scores.push(DimensionScore {
            dimension: EvaluationDimension::TaskSuccess,
            score: task_score,
            reason: if turn.success {
                "Turn completed successfully".to_string()
            } else {
                "Turn did not complete successfully".to_string()
            },
        });

        // 2. Tool Selection Score
        let tool_score = Self::evaluate_tool_selection(turn, &mut strengths, &mut weaknesses, &mut suggestions);
        dimension_scores.push(DimensionScore {
            dimension: EvaluationDimension::ToolSelection,
            score: tool_score,
            reason: Self::tool_selection_reason(turn, tool_score),
        });

        // 3. Efficiency Score
        let efficiency_score = Self::evaluate_efficiency(turn, &mut strengths, &mut weaknesses, &mut suggestions);
        dimension_scores.push(DimensionScore {
            dimension: EvaluationDimension::Efficiency,
            score: efficiency_score,
            reason: Self::efficiency_reason(turn, efficiency_score),
        });

        // 4. Response Quality Score
        let response_score = Self::evaluate_response_quality(turn, &mut strengths, &mut weaknesses, &mut suggestions);
        dimension_scores.push(DimensionScore {
            dimension: EvaluationDimension::ResponseQuality,
            score: response_score,
            reason: Self::response_quality_reason(turn, response_score),
        });

        // Calculate overall score (weighted average)
        let overall_score = Self::calculate_overall_score(&dimension_scores);

        SelfEvaluation::new(
            turn.id.clone(),
            turn.session_id.clone(),
            overall_score,
            dimension_scores,
            strengths,
            weaknesses,
            suggestions,
        )
    }

    /// Evaluate task success dimension
    fn evaluate_task_success(
        turn: &TurnRecord,
        strengths: &mut Vec<String>,
        weaknesses: &mut Vec<String>,
        _suggestions: &mut Vec<String>,
    ) -> f64 {
        if turn.success {
            if !turn.response_preview.is_empty() {
                strengths.push("Turn completed successfully".to_string());
            }
            1.0
        } else {
            weaknesses.push("Turn did not complete successfully".to_string());
            0.0
        }
    }

    /// Evaluate tool selection dimension
    fn evaluate_tool_selection(
        turn: &TurnRecord,
        strengths: &mut Vec<String>,
        weaknesses: &mut Vec<String>,
        suggestions: &mut Vec<String>,
    ) -> f64 {
        if turn.tools.is_empty() {
            // No tools used - might be good (simple query) or bad (should have used tools)
            return 0.7; // Neutral score for no tools
        }

        let tool_count = turn.tools.len();
        let successful_tools = turn.tools.iter().filter(|t| t.success).count();
        let tool_success_rate = successful_tools as f64 / tool_count as f64;

        // Calculate score based on success rate and variety
        let mut score = tool_success_rate;

        // Bonus for using multiple different tools (shows capability)
        if tool_count >= 3 {
            score = (score + 0.9).min(1.0);
            strengths.push(format!("Used {} different tools effectively", tool_count));
        } else if tool_count == 1 {
            strengths.push("Used appropriate tool for the task".to_string());
        }

        // Penalty for all tools failing
        if successful_tools == 0 && tool_count > 0 {
            score = 0.2;
            weaknesses.push("All tool executions failed".to_string());
            suggestions.push("Review tool usage strategy - consider different tools or approaches".to_string());
        } else if successful_tools < tool_count {
            let failed = tool_count - successful_tools;
            weaknesses.push(format!("{} tool(s) failed", failed));
            suggestions.push("Investigate failed tool executions and improve error handling".to_string());
        }

        score
    }

    /// Evaluate efficiency dimension
    fn evaluate_efficiency(
        turn: &TurnRecord,
        strengths: &mut Vec<String>,
        weaknesses: &mut Vec<String>,
        suggestions: &mut Vec<String>,
    ) -> f64 {
        let _score = 0.5; // Base score

        // Factor 1: Duration efficiency
        let duration = turn.duration_ms;
        if duration == 0 {
            return 0.5; // Unknown duration, neutral
        }

        // Score duration (理想: < 5s 为优秀, 5-15s 为良好, 15-30s 为一般, > 30s 为差)
        let duration_score = if duration < 5000 {
            strengths.push(format!("Quick response in {}ms", duration));
            1.0
        } else if duration < 15000 {
            0.8
        } else if duration < 30000 {
            weaknesses.push(format!("Response took {}ms", duration));
            0.5
        } else {
            weaknesses.push(format!("Slow response: {}ms", duration));
            suggestions.push("Consider optimizing tool usage or breaking down complex tasks".to_string());
            0.2
        };

        // Factor 2: Token efficiency (if available)
        let token_score = if let Some(ref usage) = turn.token_usage {
            let total = usage.total_tokens;
            if total == 0 {
                0.5 // Unknown
            } else if total < 1000 {
                strengths.push(format!("Efficient token usage: {} tokens", total));
                1.0
            } else if total < 3000 {
                0.8
            } else if total < 5000 {
                0.6
            } else {
                weaknesses.push(format!("High token usage: {} tokens", total));
                suggestions.push("Consider being more concise in responses".to_string());
                0.4
            }
        } else {
            0.5 // No token data, neutral
        };

        // Combine scores
        (duration_score + token_score) / 2.0
    }

    /// Evaluate response quality dimension
    fn evaluate_response_quality(
        turn: &TurnRecord,
        strengths: &mut Vec<String>,
        weaknesses: &mut Vec<String>,
        suggestions: &mut Vec<String>,
    ) -> f64 {
        let _score = 0.5; // Base score

        // Factor 1: Response length appropriateness
        let response_len = turn.response_preview.len();
        if response_len == 0 {
            weaknesses.push("Empty response".to_string());
            return 0.0;
        }

        // Score based on response length (too short or too long is bad)
        let length_score = if response_len < 50 {
            weaknesses.push("Response is very short".to_string());
            0.4
        } else if response_len < 200 {
            // Short but potentially complete
            0.7
        } else if response_len < 1000 {
            // Good length
            strengths.push("Response has appropriate detail".to_string());
            0.9
        } else if response_len < 3000 {
            // Detailed but maybe too long
            0.7
        } else {
            weaknesses.push("Response is very long".to_string());
            suggestions.push("Consider being more concise".to_string());
            0.5
        };

        // Factor 2: Tool usage in response
        let tool_score = if turn.tools.is_empty() {
            // No tools - response quality depends on content
            if response_len > 100 {
                0.8 // Good text response
            } else {
                0.5
            }
        } else {
            // Had tools - score based on success
            let success_rate = turn.tools.iter().filter(|t| t.success).count() as f64 / turn.tools.len() as f64;
            success_rate
        };

        // Factor 3: Efficiency combined with quality
        let combined = (length_score + tool_score) / 2.0;

        // Adjust for token usage
        if let Some(ref usage) = turn.token_usage {
            if usage.total_tokens > 0 {
                let tokens_per_char = usage.total_tokens as f64 / response_len.max(1) as f64;
                // Normal is ~2-4 tokens per character for typical text
                if tokens_per_char < 1.0 {
                    suggestions.push("Response may be inefficient with token usage".to_string());
                }
            }
        }

        combined
    }

    /// Get reason string for tool selection score
    fn tool_selection_reason(turn: &TurnRecord, _score: f64) -> String {
        if turn.tools.is_empty() {
            "No tools used (simple query)".to_string()
        } else {
            let tool_names: Vec<_> = turn.tools.iter().map(|t| t.name.as_str()).collect();
            let unique_tools = tool_names.iter().collect::<std::collections::HashSet<_>>().len();
            let successful = turn.tools.iter().filter(|t| t.success).count();
            format!(
                "{} tool(s), {} unique, {} successful",
                turn.tools.len(),
                unique_tools,
                successful
            )
        }
    }

    /// Get reason string for efficiency score
    fn efficiency_reason(turn: &TurnRecord, _score: f64) -> String {
        let mut parts = Vec::new();
        
        if turn.duration_ms > 0 {
            parts.push(format!("{}ms", turn.duration_ms));
        }
        
        if let Some(ref usage) = turn.token_usage {
            parts.push(format!("{} tokens", usage.total_tokens));
        }
        
        if parts.is_empty() {
            "No metrics available".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Get reason string for response quality score
    fn response_quality_reason(turn: &TurnRecord, _score: f64) -> String {
        let len = turn.response_preview.len();
        if len == 0 {
            "No response content".to_string()
        } else {
            format!("{} characters", len)
        }
    }

    /// Calculate overall score from dimension scores
    fn calculate_overall_score(dimension_scores: &[DimensionScore]) -> f64 {
        // Weights for each dimension
        let weights: HashMap<EvaluationDimension, f64> = [
            (EvaluationDimension::TaskSuccess, 0.35),
            (EvaluationDimension::ToolSelection, 0.25),
            (EvaluationDimension::Efficiency, 0.20),
            (EvaluationDimension::ResponseQuality, 0.20),
        ].iter().cloned().collect();

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for ds in dimension_scores {
            if let Some(&weight) = weights.get(&ds.dimension) {
                weighted_sum += ds.score * weight;
                total_weight += weight;
            }
        }

        if total_weight > 0.0 {
            (weighted_sum / total_weight * 100.0).round() / 100.0
        } else {
            0.5
        }
    }
}

/// Manager for storing and retrieving self-evaluations
pub struct SelfEvaluationManager {
    /// In-memory storage for evaluations
    evaluations: RwLock<Vec<SelfEvaluation>>,
    /// Persistence directory
    persist_dir: Option<PathBuf>,
}

impl SelfEvaluationManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            evaluations: RwLock::new(Vec::new()),
            persist_dir: None,
        }
    }

    /// Create a new manager with persistence
    pub fn with_persistence(persist_dir: PathBuf) -> Self {
        let mut manager = Self::new();
        manager.persist_dir = Some(persist_dir.clone());
        
        // Ensure directory exists
        if let Err(e) = std::fs::create_dir_all(&persist_dir) {
            tracing::warn!("Failed to create self-evaluation persistence dir: {}", e);
        }
        
        // Load existing evaluations
        manager.load();
        manager
    }

    /// Record a new evaluation
    pub fn record(&self, evaluation: SelfEvaluation) {
        let mut evaluations = self.evaluations.write();
        evaluations.push(evaluation);
        
        // Keep only last 1000 evaluations to prevent memory bloat
        if evaluations.len() > 1000 {
            evaluations.drain(0..100);
        }
        
        drop(evaluations);
        self.persist();
    }

    /// Record an evaluation for a turn
    pub fn evaluate_turn(&self, turn: &TurnRecord) -> SelfEvaluation {
        let evaluation = SelfEvaluationEngine::evaluate(turn);
        self.record(evaluation.clone());
        evaluation
    }

    /// Get all evaluations
    pub fn get_all(&self) -> Vec<SelfEvaluation> {
        self.evaluations.read().clone()
    }

    /// Get evaluations for a session
    pub fn get_by_session(&self, session_id: &str) -> Vec<SelfEvaluation> {
        self.evaluations
            .read()
            .iter()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Get evaluation by turn ID
    pub fn get_by_turn(&self, turn_id: &str) -> Option<SelfEvaluation> {
        self.evaluations
            .read()
            .iter()
            .find(|e| e.turn_id == turn_id)
            .cloned()
    }

    /// Get recent evaluations
    pub fn get_recent(&self, limit: usize) -> Vec<SelfEvaluation> {
        let evaluations = self.evaluations.read();
        let start = evaluations.len().saturating_sub(limit);
        evaluations[start..].to_vec()
    }

    /// Get evaluation statistics
    pub fn get_stats(&self) -> SelfEvaluationStats {
        let evaluations = self.evaluations.read();
        
        if evaluations.is_empty() {
            return SelfEvaluationStats::default();
        }
        
        let total = evaluations.len() as f64;
        
        // Calculate average overall score
        let avg_overall: f64 = evaluations.iter().map(|e| e.overall_score).sum::<f64>() / total;
        
        // Calculate average per dimension
        let mut dim_sums: HashMap<String, f64> = HashMap::new();
        let mut dim_counts: HashMap<String, u64> = HashMap::new();
        
        for eval in evaluations.iter() {
            for ds in &eval.dimension_scores {
                let name = ds.dimension.display_name().to_string();
                *dim_sums.entry(name.clone()).or_insert(0.0) += ds.score;
                *dim_counts.entry(name).or_insert(0) += 1;
            }
        }
        
        let avg_dimension_scores: HashMap<String, f64> = dim_sums
            .into_iter()
            .map(|(k, v)| {
                let count = dim_counts.get(&k).copied().unwrap_or(1) as f64;
                (k, (v / count * 100.0).round() / 100.0)
            })
            .collect();
        
        // Score distribution
        let mut distribution: HashMap<String, u64> = HashMap::new();
        for eval in evaluations.iter() {
            let bucket = if eval.overall_score >= 0.9 {
                "excellent (90-100%)"
            } else if eval.overall_score >= 0.7 {
                "good (70-89%)"
            } else if eval.overall_score >= 0.5 {
                "fair (50-69%)"
            } else if eval.overall_score >= 0.3 {
                "poor (30-49%)"
            } else {
                "very poor (0-29%)"
            };
            *distribution.entry(bucket.to_string()).or_insert(0) += 1;
        }
        
        // Common strengths/weaknesses (top 5)
        let mut strength_counts: HashMap<String, u64> = HashMap::new();
        let mut weakness_counts: HashMap<String, u64> = HashMap::new();
        
        for eval in evaluations.iter() {
            for s in &eval.strengths {
                *strength_counts.entry(s.clone()).or_insert(0) += 1;
            }
            for w in &eval.weaknesses {
                *weakness_counts.entry(w.clone()).or_insert(0) += 1;
            }
        }
        
        let mut common_strengths: Vec<_> = strength_counts.into_iter().collect();
        common_strengths.sort_by(|a, b| b.1.cmp(&a.1));
        let common_strengths: Vec<String> = common_strengths.into_iter().take(5).map(|(s, _)| s).collect();
        
        let mut common_weaknesses: Vec<_> = weakness_counts.into_iter().collect();
        common_weaknesses.sort_by(|a, b| b.1.cmp(&a.1));
        let common_weaknesses: Vec<String> = common_weaknesses.into_iter().take(5).map(|(w, _)| w).collect();
        
        // Evaluations by session
        let mut by_session: HashMap<String, u64> = HashMap::new();
        for eval in evaluations.iter() {
            *by_session.entry(eval.session_id.clone()).or_insert(0) += 1;
        }
        
        SelfEvaluationStats {
            total_evaluations: evaluations.len() as u64,
            avg_overall_score: (avg_overall * 100.0).round() / 100.0,
            avg_dimension_scores,
            score_distribution: distribution,
            common_strengths,
            common_weaknesses,
            evaluations_by_session: by_session,
        }
    }

    /// Persist evaluations to disk
    fn persist(&self) {
        if let Some(ref dir) = self.persist_dir {
            let path = dir.join("evaluations.json");
            let evaluations = self.evaluations.read();
            if let Err(e) = serde_json::to_string_pretty(&*evaluations)
                .map(|json| std::fs::write(&path, json))
            {
                tracing::warn!("Failed to persist self-evaluations: {}", e);
            }
        }
    }

    /// Load evaluations from disk
    fn load(&mut self) {
        if let Some(ref dir) = self.persist_dir {
            let path = dir.join("evaluations.json");
            if path.exists() {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(evaluations) = serde_json::from_str::<Vec<SelfEvaluation>>(&json) {
                        *self.evaluations.write() = evaluations;
                        tracing::info!("Loaded {} self-evaluations from disk", self.evaluations.read().len());
                    }
                }
            }
        }
    }
}

impl Default for SelfEvaluationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::turn_history::{TokenUsage, ToolExecution};

    fn create_test_turn(success: bool, tools: Vec<ToolExecution>, duration_ms: u64) -> TurnRecord {
        TurnRecord {
            id: Uuid::new_v4().to_string(),
            session_id: "test-session".to_string(),
            user_message: "Test message".to_string(),
            response_preview: "This is a test response with some content.".to_string(),
            tools,
            duration_ms,
            success,
            created_at: Utc::now(),
            token_usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 200,
                total_tokens: 300,
            }),
        }
    }

    fn create_successful_tool(name: &str) -> ToolExecution {
        ToolExecution {
            name: name.to_string(),
            input: serde_json::json!({}),
            output_preview: "Success".to_string(),
            success: true,
            duration_ms: 100,
        }
    }

    fn create_failed_tool(name: &str) -> ToolExecution {
        ToolExecution {
            name: name.to_string(),
            input: serde_json::json!({}),
            output_preview: "Failed".to_string(),
            success: false,
            duration_ms: 50,
        }
    }

    #[test]
    fn test_successful_turn_evaluation() {
        let turn = create_test_turn(true, vec![create_successful_tool("read_file")], 1000);
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        assert_eq!(eval.session_id, "test-session");
        assert_eq!(eval.dimension_scores.len(), 4);
        assert!(eval.overall_score >= 0.7); // Should be at least 70%
    }

    #[test]
    fn test_failed_turn_evaluation() {
        let turn = create_test_turn(false, vec![], 5000);
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        // Task success should be 0
        let task_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::TaskSuccess)
            .unwrap();
        assert_eq!(task_score.score, 0.0);
    }

    #[test]
    fn test_tool_selection_no_tools() {
        let turn = create_test_turn(true, vec![], 500);
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let tool_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::ToolSelection)
            .unwrap();
        assert_eq!(tool_score.score, 0.7); // Neutral for no tools
    }

    #[test]
    fn test_tool_selection_all_successful() {
        let tools = vec![
            create_successful_tool("read_file"),
            create_successful_tool("grep"),
        ];
        let turn = create_test_turn(true, tools, 2000);
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let tool_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::ToolSelection)
            .unwrap();
        assert!(tool_score.score >= 0.9); // Should be high
    }

    #[test]
    fn test_tool_selection_some_failed() {
        let tools = vec![
            create_successful_tool("read_file"),
            create_failed_tool("write_file"),
        ];
        let turn = create_test_turn(true, tools, 2000);
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let tool_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::ToolSelection)
            .unwrap();
        assert!(tool_score.score < 1.0); // Should be penalized
        assert!(!eval.weaknesses.is_empty());
    }

    #[test]
    fn test_efficiency_fast_response() {
        let tools = vec![create_successful_tool("read_file")];
        let turn = create_test_turn(true, tools, 500); // Very fast
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let efficiency_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::Efficiency)
            .unwrap();
        assert!(efficiency_score.score >= 0.8);
    }

    #[test]
    fn test_efficiency_slow_response() {
        // Create a turn with slow response AND very high token usage
        let tools = vec![create_successful_tool("read_file")];
        let turn = TurnRecord {
            id: Uuid::new_v4().to_string(),
            session_id: "test-session".to_string(),
            user_message: "Test message".to_string(),
            response_preview: "This is a test response with some content.".to_string(),
            tools,
            duration_ms: 60000, // Very slow - 60 seconds
            success: true,
            created_at: Utc::now(),
            token_usage: Some(TokenUsage {
                input_tokens: 2000,
                output_tokens: 4000,
                total_tokens: 6000, // Very high token usage
            }),
        };
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let efficiency_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::Efficiency)
            .unwrap();
        // Slow response + very high token usage should result in low score
        // duration_score = 0.2 (>= 30s), token_score = 0.4 (>= 5000 tokens)
        // Combined = (0.2 + 0.4) / 2 = 0.3
        assert!(efficiency_score.score < 0.4, "Expected low score for slow+very-high-token, got {}", efficiency_score.score);
    }

    #[test]
    fn test_response_quality_empty() {
        let turn = TurnRecord {
            id: Uuid::new_v4().to_string(),
            session_id: "test".to_string(),
            user_message: "Test".to_string(),
            response_preview: String::new(),
            tools: vec![],
            duration_ms: 100,
            success: true,
            created_at: Utc::now(),
            token_usage: None,
        };
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let quality_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::ResponseQuality)
            .unwrap();
        assert_eq!(quality_score.score, 0.0);
    }

    #[test]
    fn test_response_quality_good_length() {
        // Create a turn with a good-length response (200-1000 chars)
        let turn = TurnRecord {
            id: Uuid::new_v4().to_string(),
            session_id: "test-session".to_string(),
            user_message: "Test message".to_string(),
            response_preview: "This is a detailed and helpful response that provides substantial information to the user. It explains the topic thoroughly and addresses the user's question with appropriate depth and clarity.".to_string(), // ~220 chars
            tools: vec![],
            duration_ms: 500,
            success: true,
            created_at: Utc::now(),
            token_usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 200,
                total_tokens: 300,
            }),
        };
        let eval = SelfEvaluationEngine::evaluate(&turn);
        
        let quality_score = eval.dimension_scores.iter()
            .find(|ds| ds.dimension == EvaluationDimension::ResponseQuality)
            .unwrap();
        assert!(quality_score.score >= 0.7, "Expected >= 0.7 for good-length response, got {}", quality_score.score);
    }

    #[test]
    fn test_manager_record_and_retrieve() {
        let manager = SelfEvaluationManager::new();
        let turn = create_test_turn(true, vec![create_successful_tool("test")], 100);
        let eval = manager.evaluate_turn(&turn);
        
        assert_eq!(manager.get_all().len(), 1);
        assert_eq!(manager.get_by_turn(&turn.id).map(|e| e.id), Some(eval.id));
    }

    #[test]
    fn test_manager_stats() {
        let manager = SelfEvaluationManager::new();
        
        // Add several evaluations
        for i in 0..5 {
            let tools = if i % 2 == 0 {
                vec![create_successful_tool("tool1")]
            } else {
                vec![create_failed_tool("tool1")]
            };
            let turn = create_test_turn(i > 0, tools, 1000 + i as u64 * 100);
            manager.evaluate_turn(&turn);
        }
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_evaluations, 5);
        assert!(stats.avg_overall_score > 0.0);
        assert!(!stats.avg_dimension_scores.is_empty());
    }
}
