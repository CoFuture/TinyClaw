//! Context Health Monitor Module
//!
//! Tracks and reports the health of agent context management.
//! Monitors context utilization, compression events, truncation patterns,
//! and provides actionable recommendations for optimizing context usage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use parking_lot::RwLock;

/// Health status levels for context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextHealthLevel {
    /// Context is healthy with plenty of room
    Healthy,
    /// Context is getting full, consider summarization
    Warning,
    /// Context is near capacity, truncation active
    Critical,
    /// Context is at or over capacity
    Emergency,
}

impl ContextHealthLevel {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ContextHealthLevel::Healthy => "健康",
            ContextHealthLevel::Warning => "预警",
            ContextHealthLevel::Critical => "危险",
            ContextHealthLevel::Emergency => "紧急",
        }
    }

    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            ContextHealthLevel::Healthy => "🟢",
            ContextHealthLevel::Warning => "🟡",
            ContextHealthLevel::Critical => "🟠",
            ContextHealthLevel::Emergency => "🔴",
        }
    }
}

/// Context utilization breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextComposition {
    /// System prompt tokens
    pub system_prompt_tokens: usize,
    /// Skills and instructions tokens
    pub skills_tokens: usize,
    /// Conversation history tokens
    pub history_tokens: usize,
    /// Memory context tokens
    pub memory_tokens: usize,
    /// Session notes tokens
    pub notes_tokens: usize,
    /// Total utilized tokens
    pub total_tokens: usize,
    /// Maximum context budget
    pub max_tokens: usize,
    /// Utilization percentage (0-100)
    pub utilization_pct: f32,
}

/// A truncation or compression event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionEvent {
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Type of compression
    pub event_type: CompressionEventType,
    /// Number of messages affected
    pub messages_affected: usize,
    /// Tokens before compression
    pub tokens_before: usize,
    /// Tokens after compression
    pub tokens_after: usize,
    /// Compression ratio
    pub compression_ratio: f32,
}

/// Types of compression events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionEventType {
    /// Messages were truncated from context
    Truncation,
    /// AI summarization was applied
    Summarization,
    /// Context was refreshed (old messages removed)
    Refresh,
}

/// A health recommendation for improving context usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRecommendation {
    /// Recommendation ID
    pub id: String,
    /// Category of the recommendation
    pub category: String,
    /// Priority level (1-5, 5 being highest)
    pub priority: u8,
    /// Title of the recommendation
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Actionable suggestion
    pub suggestion: String,
    /// Potential token savings
    pub potential_savings: Option<usize>,
}

/// Overall context health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHealthReport {
    /// Current health level
    pub health_level: ContextHealthLevel,
    /// Overall health score (0-100)
    pub health_score: u8,
    /// Context composition breakdown
    pub composition: ContextComposition,
    /// Statistics since tracking started
    pub stats: ContextHealthStats,
    /// Active recommendations
    pub recommendations: Vec<HealthRecommendation>,
    /// Recent compression events
    pub recent_events: Vec<CompressionEvent>,
    /// Timestamp of this report
    pub timestamp: DateTime<Utc>,
}

/// Statistics about context management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHealthStats {
    /// Total turns processed
    pub total_turns: usize,
    /// Number of truncation events
    pub truncation_count: usize,
    /// Number of summarization events
    pub summarization_count: usize,
    /// Number of refresh events
    pub refresh_count: usize,
    /// Average compression ratio
    pub avg_compression_ratio: f32,
    /// Total tokens saved through compression
    pub total_tokens_saved: usize,
    /// Number of times context was near capacity
    pub near_capacity_count: usize,
    /// Peak utilization percentage
    pub peak_utilization_pct: f32,
    /// Current session ID
    pub current_session: Option<String>,
}

impl Default for ContextHealthStats {
    fn default() -> Self {
        Self {
            total_turns: 0,
            truncation_count: 0,
            summarization_count: 0,
            refresh_count: 0,
            avg_compression_ratio: 0.0,
            total_tokens_saved: 0,
            near_capacity_count: 0,
            peak_utilization_pct: 0.0,
            current_session: None,
        }
    }
}

/// Maximum events to keep in history
const MAX_EVENT_HISTORY: usize = 50;

/// Context Health Monitor
pub struct ContextHealthMonitor {
    /// Statistics since tracking started
    stats: RwLock<ContextHealthStats>,
    /// Recent compression events
    recent_events: RwLock<VecDeque<CompressionEvent>>,
    /// Current composition (updated on each turn)
    current_composition: RwLock<ContextComposition>,
    /// Previous composition (for compression detection)
    prev_composition: RwLock<Option<ContextComposition>>,
    /// Maximum context tokens
    max_context_tokens: usize,
    /// Reserved output tokens
    reserved_output_tokens: usize,
}

impl ContextHealthMonitor {
    /// Create a new context health monitor
    pub fn new(max_context_tokens: usize, reserved_output_tokens: usize) -> Self {
        Self {
            stats: RwLock::new(ContextHealthStats::default()),
            recent_events: RwLock::new(VecDeque::with_capacity(MAX_EVENT_HISTORY)),
            current_composition: RwLock::new(ContextComposition {
                system_prompt_tokens: 0,
                skills_tokens: 0,
                history_tokens: 0,
                memory_tokens: 0,
                notes_tokens: 0,
                total_tokens: 0,
                max_tokens: max_context_tokens - reserved_output_tokens,
                utilization_pct: 0.0,
            }),
            prev_composition: RwLock::new(None),
            max_context_tokens,
            reserved_output_tokens,
        }
    }

    /// Get available context tokens (accounting for output reservation)
    pub fn available_tokens(&self) -> usize {
        self.max_context_tokens.saturating_sub(self.reserved_output_tokens)
    }

    /// Update composition after a turn
    pub fn update_composition(&self, composition: ContextComposition) {
        let mut current = self.current_composition.write();
        // Save current composition as previous before updating
        let mut prev = self.prev_composition.write();
        *prev = Some(current.clone());
        *current = composition.clone();

        // Update peak utilization
        let mut stats = self.stats.write();
        if composition.utilization_pct > stats.peak_utilization_pct {
            stats.peak_utilization_pct = composition.utilization_pct;
        }
        if composition.utilization_pct >= 80.0 {
            stats.near_capacity_count += 1;
        }
    }

    /// Record a turn, automatically detecting compression if history was reduced
    pub fn record_turn(&self) {
        let prev = self.prev_composition.read();
        let current = self.current_composition.read();
        
        let mut stats = self.stats.write();
        stats.total_turns += 1;
        
        // Auto-detect compression: if history tokens decreased significantly
        if let Some(ref prev_comp) = *prev {
            let reduction = prev_comp.history_tokens as f32 - current.history_tokens as f32;
            let threshold = prev_comp.history_tokens as f32 * 0.15; // 15% reduction threshold
            
            if reduction > threshold && reduction > 1000.0 {
                // Compression detected - history reduced by more than 20% and 1000 tokens
                stats.truncation_count += 1;
                stats.total_tokens_saved += reduction as usize;
                
                // Add compression event
                let compression_ratio = if prev_comp.history_tokens > 0 {
                    (reduction / prev_comp.history_tokens as f32).min(1.0)
                } else {
                    0.0
                };
                drop(stats); // Release lock before updating events
                
                let mut events = self.recent_events.write();
                if events.len() >= MAX_EVENT_HISTORY {
                    events.pop_front();
                }
                events.push_back(CompressionEvent {
                    event_type: CompressionEventType::Truncation,
                    timestamp: Utc::now(),
                    tokens_before: prev_comp.history_tokens,
                    tokens_after: current.history_tokens,
                    compression_ratio,
                    messages_affected: 0, // Not tracked in auto-detection
                });
            }
        }
    }

    /// Set current session
    pub fn set_session(&self, session_id: Option<String>) {
        let mut stats = self.stats.write();
        stats.current_session = session_id;
    }

    /// Generate health report
    pub fn generate_report(&self) -> ContextHealthReport {
        let composition = self.current_composition.read().clone();
        let stats = self.stats.read().clone();
        let events = self.recent_events.read();

        let health_level = self.calculate_health_level(composition.utilization_pct);
        let health_score = self.calculate_health_score(&health_level, &stats, composition.utilization_pct);
        let recommendations = self.generate_recommendations(&health_level, &stats, &composition);

        ContextHealthReport {
            health_level,
            health_score,
            composition,
            stats,
            recommendations,
            recent_events: events.iter().cloned().collect(),
            timestamp: Utc::now(),
        }
    }

    /// Calculate health level based on utilization
    fn calculate_health_level(&self, utilization_pct: f32) -> ContextHealthLevel {
        if utilization_pct >= 95.0 {
            ContextHealthLevel::Emergency
        } else if utilization_pct >= 80.0 {
            ContextHealthLevel::Critical
        } else if utilization_pct >= 60.0 {
            ContextHealthLevel::Warning
        } else {
            ContextHealthLevel::Healthy
        }
    }

    /// Calculate overall health score (0-100)
    fn calculate_health_score(&self, level: &ContextHealthLevel, stats: &ContextHealthStats, utilization_pct: f32) -> u8 {
        // Base score from level
        let base_score: u8 = match level {
            ContextHealthLevel::Healthy => 100,
            ContextHealthLevel::Warning => 75,
            ContextHealthLevel::Critical => 50,
            ContextHealthLevel::Emergency => 25,
        };

        // Deduct for frequent compressions
        let compression_penalty: u8 = if stats.total_turns > 0 {
            let compression_rate = (stats.truncation_count + stats.summarization_count) as f32 / stats.total_turns as f32;
            (compression_rate * 30.0) as u8
        } else {
            0
        };

        // Deduct for high utilization
        let utilization_penalty: u8 = if utilization_pct > 70.0 {
            ((utilization_pct - 70.0) / 30.0 * 20.0) as u8
        } else {
            0
        };

        base_score.saturating_sub(compression_penalty).saturating_sub(utilization_penalty)
    }

    /// Generate actionable recommendations
    fn generate_recommendations(&self, _level: &ContextHealthLevel, stats: &ContextHealthStats, composition: &ContextComposition) -> Vec<HealthRecommendation> {
        let mut recommendations = Vec::new();
        let mut next_id = 1;

        // High utilization recommendation
        if composition.utilization_pct >= 80.0 {
            let potential = (composition.history_tokens as f32 * 0.3) as usize;
            recommendations.push(HealthRecommendation {
                id: format!("rec-{}", next_id),
                category: "Context Usage".to_string(),
                priority: 5,
                title: "上下文使用率过高".to_string(),
                description: format!("当前上下文使用率达到 {:.1}%，接近容量限制", composition.utilization_pct),
                suggestion: "考虑启用上下文摘要功能或开启AI摘要来压缩历史对话".to_string(),
                potential_savings: Some(potential),
            });
            next_id += 1;
        }

        // Frequent truncation recommendation
        if stats.total_turns > 5 {
            let compression_rate = (stats.truncation_count + stats.summarization_count) as f32 / stats.total_turns as f32;
            if compression_rate > 0.3 {
                recommendations.push(HealthRecommendation {
                    id: format!("rec-{}", next_id),
                    category: "Compression".to_string(),
                    priority: 4,
                    title: "压缩事件频繁".to_string(),
                    description: format!(" {:.0}% 的对话轮次触发了上下文压缩", compression_rate * 100.0),
                    suggestion: "增加 token 阈值以减少压缩频率，或使用 AI 摘要替代简单截断".to_string(),
                    potential_savings: None,
                });
                next_id += 1;
            }
        }

        // Large history recommendation
        if composition.history_tokens > 50000 {
            let potential = (composition.history_tokens as f32 * 0.2) as usize;
            recommendations.push(HealthRecommendation {
                id: format!("rec-{}", next_id),
                category: "History".to_string(),
                priority: 3,
                title: "对话历史较大".to_string(),
                description: format!("对话历史占用 {} tokens", composition.history_tokens),
                suggestion: "定期总结对话要点，减少冗余信息".to_string(),
                potential_savings: Some(potential),
            });
            next_id += 1;
        }

        // Skills overhead recommendation
        if composition.skills_tokens > 10000 {
            recommendations.push(HealthRecommendation {
                id: format!("rec-{}", next_id),
                category: "Skills".to_string(),
                priority: 2,
                title: "技能指令开销较大".to_string(),
                description: format!("技能和指令占用 {} tokens", composition.skills_tokens),
                suggestion: "精简技能指令，只保留当前任务相关的技能".to_string(),
                potential_savings: Some(composition.skills_tokens / 2),
            });
            next_id += 1;
        }

        // System prompt check
        if composition.system_prompt_tokens > 8000 {
            recommendations.push(HealthRecommendation {
                id: format!("rec-{}", next_id),
                category: "System".to_string(),
                priority: 2,
                title: "系统提示词较大".to_string(),
                description: format!("系统提示词占用 {} tokens", composition.system_prompt_tokens),
                suggestion: "简化系统提示词，移除冗余的格式说明".to_string(),
                potential_savings: Some(composition.system_prompt_tokens / 3),
            });
        }

        // Sort by priority
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Limit to top 5
        recommendations.truncate(5);

        recommendations
    }

    /// Reset stats (for new session)
    pub fn reset(&self) {
        let mut stats = self.stats.write();
        *stats = ContextHealthStats::default();
        self.recent_events.write().clear();
    }
}

impl Default for ContextHealthMonitor {
    fn default() -> Self {
        // Default: Claude 200k context with 4k output reservation
        Self::new(180_000, 4000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_level_calculation() {
        let monitor = ContextHealthMonitor::new(180_000, 4000);

        assert_eq!(monitor.calculate_health_level(30.0), ContextHealthLevel::Healthy);
        assert_eq!(monitor.calculate_health_level(60.0), ContextHealthLevel::Warning);
        assert_eq!(monitor.calculate_health_level(85.0), ContextHealthLevel::Critical);
        assert_eq!(monitor.calculate_health_level(97.0), ContextHealthLevel::Emergency);
    }

    #[test]
    fn test_auto_truncation_detection() {
        // Test that record_turn auto-detects compression from composition changes
        let monitor = ContextHealthMonitor::default();
        
        // First composition (before compression)
        let composition_before = ContextComposition {
            system_prompt_tokens: 1000,
            skills_tokens: 500,
            history_tokens: 10000,  // High history tokens
            memory_tokens: 300,
            notes_tokens: 100,
            total_tokens: 11900,
            max_tokens: 176000,
            utilization_pct: 6.8,
        };
        monitor.update_composition(composition_before);
        
        // Second composition (after truncation - significantly reduced)
        let composition_after = ContextComposition {
            system_prompt_tokens: 1000,
            skills_tokens: 500,
            history_tokens: 6000,  // Significantly reduced (>20% reduction)
            memory_tokens: 300,
            notes_tokens: 100,
            total_tokens: 7900,
            max_tokens: 176000,
            utilization_pct: 4.5,
        };
        monitor.update_composition(composition_after);
        
        // record_turn should auto-detect the truncation
        monitor.record_turn();

        let report = monitor.generate_report();
        assert_eq!(report.stats.truncation_count, 1);
        assert_eq!(report.stats.total_tokens_saved, 4000);
        assert_eq!(report.stats.total_turns, 1);
    }

    #[test]
    fn test_auto_summarization_detection() {
        // Test auto-detection of aggressive compression (like summarization)
        let monitor = ContextHealthMonitor::default();
        
        // Large history before summarization
        let composition_before = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 50000,  // Very large
            memory_tokens: 1000,
            notes_tokens: 500,
            total_tokens: 59500,
            max_tokens: 176000,
            utilization_pct: 33.8,
        };
        monitor.update_composition(composition_before);
        
        // Much smaller after summarization
        let composition_after = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 3000,  // Massive reduction (>20%)
            memory_tokens: 1000,
            notes_tokens: 500,
            total_tokens: 9500,
            max_tokens: 176000,
            utilization_pct: 5.4,
        };
        monitor.update_composition(composition_after);
        
        monitor.record_turn();

        let report = monitor.generate_report();
        assert_eq!(report.stats.truncation_count, 1);
        assert_eq!(report.stats.total_tokens_saved, 47000);
    }

    #[test]
    fn test_composition_update() {
        let monitor = ContextHealthMonitor::default();
        let composition = ContextComposition {
            system_prompt_tokens: 1000,
            skills_tokens: 500,
            history_tokens: 2000,
            memory_tokens: 300,
            notes_tokens: 100,
            total_tokens: 3900,
            max_tokens: 176000,
            utilization_pct: 2.2,
        };
        monitor.update_composition(composition);

        // Read back via generate_report().composition
        let report = monitor.generate_report();
        assert_eq!(report.composition.total_tokens, 3900);
        assert_eq!(report.composition.utilization_pct, 2.2);
    }

    #[test]
    fn test_generate_report() {
        let monitor = ContextHealthMonitor::default();
        
        // First composition (before compression)
        let composition_before = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 80000,
            memory_tokens: 2000,
            notes_tokens: 500,
            total_tokens: 90500,
            max_tokens: 176000,
            utilization_pct: 51.4,
        };
        monitor.update_composition(composition_before);
        
        // Second composition (after truncation)
        let composition_after = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 65000,  // Reduced by 15000 (>1000 threshold)
            memory_tokens: 2000,
            notes_tokens: 500,
            total_tokens: 75500,
            max_tokens: 176000,
            utilization_pct: 42.9,
        };
        monitor.update_composition(composition_after);
        monitor.record_turn();

        let report = monitor.generate_report();
        assert_eq!(report.health_level, ContextHealthLevel::Healthy);
        assert!(report.health_score >= 70);
        assert_eq!(report.stats.truncation_count, 1);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_recommendations_sorting() {
        let monitor = ContextHealthMonitor::default();
        
        let composition_before = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 80000,
            memory_tokens: 2000,
            notes_tokens: 500,
            total_tokens: 90500,
            max_tokens: 176000,
            utilization_pct: 51.4,
        };
        monitor.update_composition(composition_before);
        
        let composition_after = ContextComposition {
            system_prompt_tokens: 5000,
            skills_tokens: 3000,
            history_tokens: 65000,
            memory_tokens: 2000,
            notes_tokens: 500,
            total_tokens: 75500,
            max_tokens: 176000,
            utilization_pct: 42.9,
        };
        monitor.update_composition(composition_after);
        monitor.record_turn();

        let report = monitor.generate_report();
        // First recommendation should be highest priority
        if !report.recommendations.is_empty() {
            assert!(report.recommendations[0].priority >= report.recommendations.last().map(|r| r.priority).unwrap_or(0));
        }
    }

    #[test]
    fn test_event_history_limit() {
        let monitor = ContextHealthMonitor::default();
        for i in 0..60 {
            // Each pair of update_composition calls simulates compression
            // Ensure reduction > 1000 to meet minimum threshold
            let composition_before = ContextComposition {
                system_prompt_tokens: 1000,
                skills_tokens: 500,
                history_tokens: 2500 + i * 100,  // Start higher to ensure reduction > 1000
                memory_tokens: 300,
                notes_tokens: 100,
                total_tokens: 4400 + i * 100,
                max_tokens: 176000,
                utilization_pct: 2.5 + (i as f32) * 0.05,
            };
            monitor.update_composition(composition_before);
            
            let composition_after = ContextComposition {
                system_prompt_tokens: 1000,
                skills_tokens: 500,
                history_tokens: 1000 + i * 50,  // At least 1500 reduction (>1000 threshold)
                memory_tokens: 300,
                notes_tokens: 100,
                total_tokens: 2900 + i * 50,
                max_tokens: 176000,
                utilization_pct: 1.6 + (i as f32) * 0.03,
            };
            monitor.update_composition(composition_after);
            monitor.record_turn();
        }

        let report = monitor.generate_report();
        assert_eq!(report.stats.truncation_count, 60);
        // Should only keep MAX_EVENT_HISTORY recent events
        assert!(report.recent_events.len() <= MAX_EVENT_HISTORY);
    }

    #[test]
    fn test_health_score_deduction() {
        let monitor = ContextHealthMonitor::default();
        
        // Good case: low utilization, no compressions
        let composition = ContextComposition {
            system_prompt_tokens: 1000,
            skills_tokens: 500,
            history_tokens: 2000,
            memory_tokens: 300,
            notes_tokens: 100,
            total_tokens: 3900,
            max_tokens: 176000,
            utilization_pct: 2.2,
        };
        monitor.update_composition(composition);
        monitor.record_turn();  // No compression detected (not a significant reduction)
        
        let report = monitor.generate_report();
        assert!(report.health_score >= 90);
    }
}
