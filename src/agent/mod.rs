//! Agent module - AI model client and tool execution

pub mod client;
pub mod context;
pub mod context_advisor;
pub mod context_health;
pub mod context_manager;
pub mod context_summarizer;
pub mod conversation_summary;
pub mod error_recovery;
pub mod execution_safety;
pub mod memory;
pub mod memory_extractor;
pub mod performance_insights;
pub mod retry;
pub mod runtime;
pub mod scheduled_task;
pub mod scheduler;
pub mod self_evaluation;
pub mod session_notes;
pub mod session_quality;
pub mod skill;
pub mod skill_manager;
pub mod skill_recommender;
pub mod skill_registry;
pub mod suggestion;
pub mod suggestion_manager;
pub mod task;
pub mod task_manager;
pub mod tools;
pub mod tool_result_formatter;
pub mod tool_strategy;
pub mod tool_pattern_learner;
pub mod session_accomplishments;
pub mod feedback_trend;
pub mod turn_feedback;
pub mod turn_history;
pub mod turn_log;
pub mod turn_summary;

pub use client::Agent;
#[allow(unused_imports)]
pub use context_summarizer::{ContextSummarizer, ContextSummary, SummarizerConfig, SummaryHistory, SummaryHistoryEntry, SummaryHistoryManager, SummaryHistoryStats};
#[allow(unused_imports)]
pub use memory::{FactCategory, MemoryFact, MemoryFactSummary, MemoryManager};
pub use scheduled_task::ScheduledTaskSummary;
#[allow(unused_imports)]
pub use scheduled_task::ScheduleType;
pub use scheduler::Scheduler;
#[allow(unused_imports)]
pub use self_evaluation::{SelfEvaluation, SelfEvaluationEngine, SelfEvaluationManager, SelfEvaluationStats, SelfEvaluationSummary, DimensionScore, EvaluationDimension};
#[allow(unused_imports)]
pub use session_quality::{DetectedIssue, QualityIssue, SessionQuality, SessionQualityAnalyzer, SessionQualityManager, SessionQualitySummary};
pub use session_notes::{SessionNoteUpdate, SessionNotesManager};
#[allow(unused_imports)]
pub use session_notes::{SessionNote, SessionNoteSummary};
pub use skill::Skill;
#[allow(unused_imports)]
pub use skill::SkillTemplate;
pub use skill_registry::SkillRegistry;
pub use skill_manager::SessionSkillManager;
#[allow(unused_imports)]
pub use skill_recommender::{SkillRecommendation, SkillRecommender};
#[allow(unused_imports)]
pub use suggestion::{Suggestion, SuggestionEngine, SuggestionSummary, SuggestionType};
#[allow(unused_imports)]
pub use suggestion_manager::{SuggestionManager, SuggestionFeedback, TrackedSuggestion, TrackedSuggestionSummary};
pub use task::{TaskState, TaskSummary};
pub use task_manager::TaskManager;
#[allow(unused_imports)]
pub use session_accomplishments::{SessionAccomplishments, SessionAccomplishmentsManager, SessionAccomplishmentSummary, SessionAccomplishmentStats, Accomplishment, AccomplishmentType};
#[allow(unused_imports)]
pub use turn_history::{TurnHistoryManager, TurnRecord, TurnSummary, TurnStats, ToolExecution};
#[allow(unused_imports)]
pub use feedback_trend::{FeedbackTrendAnalysis, FeedbackPeriodStats, FeedbackIssuePattern, FeedbackTrendAnalyzer, IssueType, TrendDirection};
#[allow(unused_imports)]
pub use turn_feedback::{FeedbackRating, TurnFeedback, TurnFeedbackSummary, TurnFeedbackManager, GlobalFeedbackStats};
#[allow(unused_imports)]
pub use turn_log::{TurnLog, TurnLogEntry, TurnLogSummary};
#[allow(unused_imports)]
pub use turn_summary::{AgentTurnSummary, ToolExecutionSummary};
#[allow(unused_imports)]
pub use conversation_summary::{ConversationSummary, ConversationSummaryManager};
#[allow(unused_imports)]
pub use execution_safety::{SafetyAction, ExecutionSafetyConfig, ExecutionSafetyManager, ExecutionSafetyState, ExecutionSafetyStats, SafetyCheckResult};
#[allow(unused_imports)]
pub use performance_insights::{InsightCategory, InsightSeverity, PerformanceInsight, PerformanceAnalysis, PerformanceInsightsEngine, ToolEfficiencySummary, QualityTrend, ToolPattern};
#[allow(unused_imports)]
pub use context_health::{ContextHealthLevel, ContextHealthMonitor, ContextHealthReport, ContextHealthStats, ContextComposition, CompressionEvent, CompressionEventType, HealthRecommendation};
#[allow(unused_imports)]
pub use context_advisor::{ContextAdvisor, ContextAdvice, ContextAdvisorStats, PatternType};
#[allow(unused_imports)]
pub use tool_strategy::{ToolStrategy, UserIntent, ToolGuidance, WorkflowPattern};
#[allow(unused_imports)]
pub use tool_pattern_learner::{ToolPatternLearner, LearnedPattern, ToolStats, PatternAnalysis};
