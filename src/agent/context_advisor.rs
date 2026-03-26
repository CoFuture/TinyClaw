//! Context Advisor Module
//!
//! Provides proactive, actionable recommendations based on context management patterns.
//! Analyzes context health trends and emits suggestions when intervention would help.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// A context-related recommendation with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAdvice {
    /// Unique ID for this advice
    pub id: String,
    /// Category of the advice
    pub category: String,
    /// Title of the advice
    pub title: String,
    /// Detailed explanation
    pub explanation: String,
    /// Actionable suggestion for the user
    pub suggestion: String,
    /// Severity level (1-3)
    pub severity: u8,
    /// Whether this is currently applicable
    pub is_urgent: bool,
    /// Timestamp when this was generated
    pub timestamp: DateTime<Utc>,
    /// Pattern that triggered this advice
    pub trigger_pattern: String,
}

impl ContextAdvice {
    pub fn new(
        category: &str,
        title: &str,
        explanation: &str,
        suggestion: &str,
        severity: u8,
        is_urgent: bool,
        trigger_pattern: &str,
    ) -> Self {
        Self {
            id: format!("ctx-{}", uuid::Uuid::new_v4()),
            category: category.to_string(),
            title: title.to_string(),
            explanation: explanation.to_string(),
            suggestion: suggestion.to_string(),
            severity: severity.clamp(1, 3),
            is_urgent,
            timestamp: Utc::now(),
            trigger_pattern: trigger_pattern.to_string(),
        }
    }
}

/// Pattern detected from context health data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum PatternType {
    FrequentTruncation,
    HighUtilization,
    LargeSystemPrompt,
    InefficientSummarization,
    ContextBloating,
    SessionTooLong,
}

/// Advisor that monitors context patterns and generates advice
pub struct ContextAdvisor {
    /// Recent patterns detected
    patterns: HashMap<PatternType, DetectedPattern>,
    /// Recent advice given (to avoid repetition)
    recent_advice: VecDeque<ContextAdvice>,
    /// Session ID
    session_id: Option<String>,
    /// Turn counter for session
    turn_count: usize,
    /// Last context utilization
    last_utilization: f32,
    /// Total tokens processed
    total_tokens_processed: usize,
    /// Compression events count
    compression_count: usize,
    /// Last truncation count (for delta detection)
    last_truncation_count: usize,
    /// Last summarization count (for delta detection)
    last_summarization_count: usize,
}

impl ContextAdvisor {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            recent_advice: VecDeque::with_capacity(50),
            session_id: None,
            turn_count: 0,
            last_utilization: 0.0,
            total_tokens_processed: 0,
            compression_count: 0,
            last_truncation_count: 0,
            last_summarization_count: 0,
        }
    }

    /// Set the current session
    pub fn set_session(&mut self, session_id: String) {
        if self.session_id.as_ref() != Some(&session_id) {
            self.session_id = Some(session_id);
            self.reset_session_data();
        }
    }

    /// Reset data for a new session
    pub fn reset_session_data(&mut self) {
        self.patterns.clear();
        self.recent_advice.clear();
        self.turn_count = 0;
        self.last_utilization = 0.0;
        self.total_tokens_processed = 0;
        self.compression_count = 0;
        self.last_truncation_count = 0;
        self.last_summarization_count = 0;
    }

    /// Record a turn
    pub fn record_turn(&mut self, utilization_pct: f32, total_tokens: usize) {
        self.turn_count += 1;
        self.last_utilization = utilization_pct;
        self.total_tokens_processed += total_tokens;
        
        // Check for high utilization pattern
        if utilization_pct >= 80.0 {
            self.record_pattern(PatternType::HighUtilization);
        }
        
        // Check for context bloating (rapid increase in tokens)
        if self.turn_count > 1 && utilization_pct > self.last_utilization + 20.0 {
            self.record_pattern(PatternType::ContextBloating);
        }
    }

    /// Update advisor with health report data - comprehensive integration
    /// This method hooks up all the pattern detection methods that were previously unused
    pub fn update_with_health_data(&mut self, report: &crate::agent::context_health::ContextHealthReport, message_count: usize) {
        // Basic turn tracking
        self.record_turn(report.composition.utilization_pct, report.composition.total_tokens);
        
        // Track compression events (truncation/summarization) from health stats
        let truncation_delta = report.stats.truncation_count.saturating_sub(self.last_truncation_count);
        let summarization_delta = report.stats.summarization_count.saturating_sub(self.last_summarization_count);
        
        // Record each new truncation event
        for _ in 0..truncation_delta {
            self.record_compression();
        }
        
        // Record each new summarization event (inefficient if happening frequently)
        for _ in 0..summarization_delta {
            self.record_inefficient_summarization();
        }
        
        // Update tracking state
        self.last_truncation_count = report.stats.truncation_count;
        self.last_summarization_count = report.stats.summarization_count;
        
        // Track large system prompt
        self.record_large_system_prompt(report.composition.system_prompt_tokens);
        
        // Track session length
        self.check_session_length(message_count);
    }

    /// Record a compression event (truncation or summarization)
    pub fn record_compression(&mut self) {
        self.compression_count += 1;
        
        // Frequent truncation pattern
        if self.turn_count > 0 {
            let compression_rate = self.compression_count as f32 / self.turn_count as f32;
            if compression_rate >= 0.3 {
                self.record_pattern(PatternType::FrequentTruncation);
            }
        }
    }

    /// Record that summarization was attempted but may be inefficient
    pub fn record_inefficient_summarization(&mut self) {
        self.record_pattern(PatternType::InefficientSummarization);
    }

    /// Record a detected pattern
    fn record_pattern(&mut self, pattern_type: PatternType) {
        let now = Utc::now();
        if let Some(existing) = self.patterns.get_mut(&pattern_type) {
            existing.count += 1;
            existing.last_seen = now;
        } else {
            self.patterns.insert(pattern_type, DetectedPattern {
                pattern_type,
                count: 1,
                first_seen: now,
                last_seen: now,
            });
        }
    }

    /// Record a large system prompt
    pub fn record_large_system_prompt(&mut self, tokens: usize) {
        if tokens > 8000 {
            self.record_pattern(PatternType::LargeSystemPrompt);
        }
    }

    /// Check if session is getting too long
    pub fn check_session_length(&mut self, message_count: usize) {
        if message_count > 50 {
            self.record_pattern(PatternType::SessionTooLong);
        }
    }

    /// Check if we should recommend starting a new session
    pub fn should_suggest_new_session(&self) -> bool {
        if let Some(pattern) = self.patterns.get(&PatternType::SessionTooLong) {
            return pattern.count >= 1;
        }
        if let Some(pattern) = self.patterns.get(&PatternType::FrequentTruncation) {
            return pattern.count >= 2;
        }
        false
    }

    /// Generate all current advice based on detected patterns
    pub fn generate_advice(&self) -> Vec<ContextAdvice> {
        let mut advice_list = Vec::new();

        for (pattern_type, pattern) in &self.patterns {
            match pattern_type {
                PatternType::FrequentTruncation => {
                    if pattern.count >= 2 {
                        advice_list.push(ContextAdvice::new(
                            "Context",
                            "上下文压缩频繁",
                            &format!(
                                "在当前会话中已发生 {} 次上下文压缩。这表明对话历史正在快速消耗可用上下文空间。",
                                pattern.count
                            ),
                            "考虑开启 AI 摘要功能来压缩历史对话，或主动总结当前讨论的要点。",
                            if pattern.count >= 3 { 3 } else { 2 },
                            pattern.count >= 3,
                            "frequent_compression",
                        ));
                    }
                }
                PatternType::HighUtilization => {
                    if self.last_utilization >= 90.0 {
                        advice_list.push(ContextAdvice::new(
                            "Context",
                            "上下文使用率极高",
                            &format!(
                                "当前上下文使用率达到 {:.1}%，已接近容量限制。",
                                self.last_utilization
                            ),
                            "立即开启摘要或总结当前对话要点，以避免重要信息被截断。",
                            3,
                            true,
                            "high_utilization",
                        ));
                    } else if self.last_utilization >= 80.0 {
                        advice_list.push(ContextAdvice::new(
                            "Context",
                            "上下文使用率较高",
                            &format!(
                                "当前上下文使用率为 {:.1}%。建议准备进行摘要以避免后续截断。",
                                self.last_utilization
                            ),
                            "可以输入 ':summarize' 来触发 AI 摘要，或主动总结当前进展。",
                            2,
                            false,
                            "elevated_utilization",
                        ));
                    }
                }
                PatternType::ContextBloating => {
                    advice_list.push(ContextAdvice::new(
                        "Context",
                        "上下文快速增长",
                        "上下文使用率在短时间内大幅增长，可能有大型文件或输出被加入。",
                        "检查最近的 Agent 输出是否包含大段内容，考虑清理不必要的对话历史。",
                        2,
                        false,
                        "context_bloat",
                    ));
                }
                PatternType::LargeSystemPrompt => {
                    advice_list.push(ContextAdvice::new(
                        "System",
                        "系统提示词较大",
                        &format!(
                            "系统提示词占用较大空间 ({}+ tokens)，可能挤压可用对话空间。",
                            pattern.count * 2000
                        ),
                        "精简系统提示词或减少启用的技能数量，以释放对话空间。",
                        1,
                        false,
                        "large_system_prompt",
                    ));
                }
                PatternType::SessionTooLong => {
                    advice_list.push(ContextAdvice::new(
                        "Session",
                        "会话过长",
                        &format!(
                            "当前会话包含 {} 条消息，上下文管理可能变得低效。",
                            self.turn_count * 2
                        ),
                        "考虑开启新会话，将当前讨论的结论和决定带入新会话继续。",
                        if self.turn_count > 10 { 3 } else { 2 },
                        self.turn_count > 15,
                        "long_session",
                    ));
                }
                PatternType::InefficientSummarization => {
                    advice_list.push(ContextAdvice::new(
                        "Context",
                        "摘要效果不理想",
                        "最近的 AI 摘要可能未能有效压缩上下文，或摘要质量不高。",
                        "尝试手动总结关键信息，使用更精确的摘要提示词。",
                        2,
                        false,
                        "inefficient_summarization",
                    ));
                }
            }
        }

        // Sort by severity (highest first) then by timestamp (newest first)
        advice_list.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });

        advice_list
    }

    /// Get urgent advice only (severity 3 or is_urgent)
    pub fn get_urgent_advice(&self) -> Vec<ContextAdvice> {
        self.generate_advice()
            .into_iter()
            .filter(|a| a.severity >= 3 || a.is_urgent)
            .collect()
    }

    /// Get advice count
    pub fn advice_count(&self) -> usize {
        self.recent_advice.len()
    }

    /// Get session stats summary
    pub fn get_stats(&self) -> ContextAdvisorStats {
        ContextAdvisorStats {
            session_id: self.session_id.clone(),
            turn_count: self.turn_count,
            total_tokens_processed: self.total_tokens_processed,
            compression_count: self.compression_count,
            current_utilization: self.last_utilization,
            active_patterns: self.patterns.len(),
            advice_count: self.generate_advice().len(),
            patterns: self.patterns.iter().map(|(k, v)| (format!("{:?}", k), v.count)).collect(),
        }
    }

    /// Check if a similar advice was given recently
    pub fn is_advice_redundant(&self, title: &str) -> bool {
        // Check recent advice history (simulate with current advice)
        let current = self.generate_advice();
        current.iter().any(|a| a.title == title)
    }
}

/// Stats about the context advisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAdvisorStats {
    pub session_id: Option<String>,
    pub turn_count: usize,
    pub total_tokens_processed: usize,
    pub compression_count: usize,
    pub current_utilization: f32,
    pub active_patterns: usize,
    pub advice_count: usize,
    pub patterns: HashMap<String, usize>,
}

impl Default for ContextAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequent_truncation_advice() {
        let mut advisor = ContextAdvisor::new();
        advisor.set_session("test".to_string());
        
        // Simulate 5 turns with 3 truncations
        for _ in 0..5 {
            advisor.record_turn(30.0, 5000);
        }
        advisor.record_compression();
        advisor.record_compression();
        advisor.record_compression();
        
        let advice = advisor.generate_advice();
        assert!(!advice.is_empty());
        assert!(advice.iter().any(|a| a.title.contains("频繁")));
    }

    #[test]
    fn test_high_utilization_advice() {
        let mut advisor = ContextAdvisor::new();
        advisor.set_session("test".to_string());
        
        advisor.record_turn(92.0, 170000);
        
        let advice = advisor.generate_advice();
        assert!(!advice.is_empty());
        assert!(advice.iter().any(|a| a.severity == 3));
    }

    #[test]
    fn test_session_length_advice() {
        let mut advisor = ContextAdvisor::new();
        advisor.set_session("test".to_string());
        
        advisor.check_session_length(60);
        
        let advice = advisor.generate_advice();
        assert!(!advice.is_empty());
        assert!(advice.iter().any(|a| a.title.contains("过长")));
    }

    #[test]
    fn test_advice_sorted_by_severity() {
        let mut advisor = ContextAdvisor::new();
        advisor.set_session("test".to_string());
        
        advisor.record_turn(92.0, 170000);
        advisor.record_compression();
        
        let advice = advisor.generate_advice();
        if advice.len() >= 2 {
            assert!(advice[0].severity >= advice[1].severity);
        }
    }

    #[test]
    fn test_reset_session() {
        let mut advisor = ContextAdvisor::new();
        advisor.set_session("session1".to_string());
        advisor.record_turn(50.0, 10000);
        
        advisor.set_session("session2".to_string());
        
        let stats = advisor.get_stats();
        assert_eq!(stats.turn_count, 0);
        assert_eq!(stats.compression_count, 0);
    }
}
