//! Event system module

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::agent::scheduled_task::ScheduledTaskSummary;
use crate::agent::turn_log::{TurnLogEntry, TurnLogSummary};
use crate::agent::task::TaskSummary;
use crate::agent::skill_recommender::SkillRecommendation;
use crate::agent::suggestion::Suggestion;

/// Tool call info for action plan preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPreview {
    /// Unique ID for the tool call
    pub id: String,
    /// Name of the tool
    pub name: String,
    /// Arguments to the tool (JSON)
    pub input: serde_json::Value,
}

/// Event types for real-time streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    /// Turn started (agent beginning to process a message)
    #[serde(rename = "turn.started")]
    TurnStarted {
        session_id: String,
        message: String,
    },
    
    /// Agent is thinking
    #[serde(rename = "turn.thinking")]
    TurnThinking {
        session_id: String,
    },
    
    /// Turn completed
    #[serde(rename = "turn.ended")]
    TurnEnded {
        session_id: String,
        response: String,
    },
    
    /// Turn token usage - emitted after a turn completes with token usage info
    #[serde(rename = "turn.usage")]
    TurnUsage {
        session_id: String,
        input_tokens: u32,
        output_tokens: u32,
        total_tokens: u32,
    },
    
    /// Context was summarized - emitted when AI summarizes conversation history
    #[serde(rename = "context.summarized")]
    ContextSummarized {
        session_id: String,
        /// Number of messages that were summarized
        messages_summarized: usize,
        /// Original token count before summarization
        original_tokens: usize,
        /// Token count of the summary
        summary_tokens: usize,
        /// Compression ratio (summary_tokens / original_tokens)
        compression_ratio: f32,
    },
    
    /// Turn was cancelled
    #[serde(rename = "turn.cancelled")]
    TurnCancelled {
        session_id: String,
    },
    
    /// Assistant sent text
    #[serde(rename = "assistant.text")]
    AssistantText {
        session_id: String,
        text: String,
    },
    
    /// Assistant is sending partial/streaming text (incremental update)
    #[serde(rename = "assistant.partial")]
    AssistantPartial {
        session_id: String,
        text: String,
    },
    
    /// Assistant used a tool
    #[serde(rename = "assistant.tool_use")]
    AssistantToolUse {
        session_id: String,
        tool: String,
        input: serde_json::Value,
    },
    
    /// Action plan preview - shows all planned tool calls before execution
    #[serde(rename = "action.plan_preview")]
    ActionPlanPreview {
        session_id: String,
        tools: Vec<ToolCallPreview>,
    },
    
    /// Action plan confirmation request - agent waiting for user to confirm tool execution
    /// This indicates the agent has planned tools and is waiting for confirmation.
    /// The client should call session.confirm_action to confirm or cancel.
    #[serde(rename = "action.plan_confirm")]
    ActionPlanConfirm {
        session_id: String,
        plan_id: String,
        tools: Vec<ToolCallPreview>,
    },
    
    /// Tool result
    #[serde(rename = "tool_result")]
    ToolResult {
        session_id: String,
        tool_call_id: String,
        output: String,
    },

    /// Turn execution log was updated (new action recorded)
    #[serde(rename = "turn.log_updated")]
    TurnLogUpdated {
        session_id: String,
        entry: TurnLogEntry,
    },

    /// Turn execution log was completed
    #[serde(rename = "turn.log_completed")]
    TurnLogCompleted {
        session_id: String,
        summary: TurnLogSummary,
    },

    /// Session created
    #[serde(rename = "session.created")]
    SessionCreated {
        session_id: String,
        kind: String,
    },

    /// Session ended
    #[serde(rename = "session.ended")]
    SessionEnded {
        session_id: String,
    },

    /// Task created
    #[serde(rename = "task.created")]
    TaskCreated {
        task_id: String,
        summary: TaskSummary,
    },

    /// Task started
    #[serde(rename = "task.started")]
    TaskStarted {
        task_id: String,
    },

    /// Task progress update
    #[serde(rename = "task.progress")]
    TaskProgress {
        task_id: String,
        step: usize,
        total_steps: usize,
        message: String,
    },

    /// Task completed successfully
    #[serde(rename = "task.completed")]
    TaskCompleted {
        task_id: String,
        result: String,
    },

    /// Task failed
    #[serde(rename = "task.failed")]
    TaskFailed {
        task_id: String,
        error: String,
    },

    /// Task cancelled
    #[serde(rename = "task.cancelled")]
    TaskCancelled {
        task_id: String,
    },

    /// Scheduled task created
    #[serde(rename = "scheduled.created")]
    ScheduledTaskCreated {
        schedule_id: String,
        summary: ScheduledTaskSummary,
    },

    /// Scheduled task fired (triggered)
    #[serde(rename = "scheduled.fired")]
    ScheduledTaskFired {
        schedule_id: String,
        schedule_name: String,
        task_description: String,
        session_id: String,
        run_count: u64,
    },

    /// Scheduled task failed to start
    #[serde(rename = "scheduled.failed")]
    ScheduledTaskFailed {
        schedule_id: String,
        error: String,
    },

    /// Scheduled task updated (pause/resume/enable/disable)
    #[serde(rename = "scheduled.updated")]
    ScheduledTaskUpdated {
        schedule_id: String,
    },

    /// Scheduled task deleted
    #[serde(rename = "scheduled.deleted")]
    ScheduledTaskDeleted {
        schedule_id: String,
    },

    /// Suggestions generated for a session
    #[serde(rename = "suggestion.generated")]
    SuggestionGenerated {
        session_id: String,
        suggestions: Vec<Suggestion>,
    },
    
    /// Suggestion was accepted by user
    #[serde(rename = "suggestion.accepted")]
    SuggestionAccepted {
        session_id: String,
        suggestion_id: String,
        suggestion_type: String,
    },
    
    /// Suggestion was dismissed by user
    #[serde(rename = "suggestion.dismissed")]
    SuggestionDismissed {
        session_id: String,
        suggestion_id: String,
    },
    
    /// Skill was recommended based on conversation context
    #[serde(rename = "skill.recommended")]
    SkillRecommended {
        session_id: String,
        recommendations: Vec<SkillRecommendation>,
    },
    
    /// Error occurred
    #[serde(rename = "error")]
    Error {
        session_id: String,
        message: String,
    },
    
    /// Status update
    #[serde(rename = "status")]
    Status {
        message: String,
    },
    
    /// Heartbeat to keep connections alive
    #[serde(rename = "heartbeat")]
    Heartbeat {
        timestamp: i64,
    },
    
    /// Action plan was denied by user (confirmation timeout or explicit denial)
    #[serde(rename = "action.denied")]
    ActionDenied {
        session_id: String,
    },
    
    /// Agent self-evaluation completed after a turn
    #[serde(rename = "agent.self_evaluation")]
    SelfEvaluation {
        session_id: String,
        turn_id: String,
        overall_score: f64,
        dimension_scores: Vec<DimensionScoreEvent>,
        strengths: Vec<String>,
        weaknesses: Vec<String>,
    },
    
    /// Session quality analysis updated (periodic or after turn)
    #[serde(rename = "session.quality")]
    SessionQuality {
        session_id: String,
        quality_score: f64,
        turn_count: u32,
        task_completion_rate: f64,
        tool_success_rate: f64,
        rating: u8,
        issue_count: usize,
        suggestions: Vec<String>,
    },
    
    /// Execution safety warning - approaching limit
    #[serde(rename = "execution.warning")]
    ExecutionSafetyWarning {
        session_id: String,
        consecutive_turns: usize,
        max_turns: usize,
        warning_threshold: usize,
    },
    
    /// Execution safety halted - limit reached
    #[serde(rename = "execution.halted")]
    ExecutionSafetyHalted {
        session_id: String,
        consecutive_turns: usize,
        action_taken: String,
    },
    
    /// Performance insights generated - actionable recommendations for agent improvement
    #[serde(rename = "agent.performance_insights")]
    PerformanceInsights {
        session_id: String,
        /// Generated insights
        insights: Vec<PerformanceInsightEvent>,
        /// Tool efficiency summary
        tool_efficiency: ToolEfficiencyEvent,
        /// Quality trend
        quality_trend: QualityTrendEvent,
        /// Turns analyzed
        turns_analyzed: u64,
    },
    
    /// Context health updated - periodic or after compression events
    #[serde(rename = "context.health")]
    ContextHealth {
        session_id: String,
        /// Health level (healthy/warning/critical/emergency)
        health_level: String,
        /// Overall health score (0-100)
        health_score: u8,
        /// Context utilization percentage
        utilization_pct: f32,
        /// Total tokens in context
        total_tokens: usize,
        /// Max tokens available
        max_tokens: usize,
        /// Truncation count
        truncation_count: usize,
        /// Summarization count
        summarization_count: usize,
        /// Recommendations count
        recommendations_count: usize,
    },
}

/// Performance insight for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsightEvent {
    /// Insight ID
    pub id: String,
    /// Category
    pub category: String,
    /// Severity
    pub severity: String,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Suggestions
    pub suggestions: Vec<String>,
}

/// Tool efficiency summary for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEfficiencyEvent {
    /// Most efficient tool
    pub most_efficient_tool: Option<String>,
    /// Least efficient tool
    pub least_efficient_tool: Option<String>,
    /// Problematic tools (high failure rate)
    pub problematic_tools: Vec<String>,
    /// Average tools per turn
    pub avg_tools_per_turn: f64,
    /// Most used tool
    pub most_used_tool: Option<String>,
}

/// Quality trend for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrendEvent {
    /// Current quality score (0-100)
    pub current_score: f64,
    /// Previous period score
    pub previous_score: f64,
    /// Trend direction (improving/declining/stable)
    pub trend_direction: String,
    /// Trend magnitude (percentage change)
    pub trend_magnitude: f64,
}

/// Dimension score for self-evaluation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScoreEvent {
    /// Dimension name
    pub dimension: String,
    /// Score from 0.0 to 1.0
    pub score: f64,
    /// Reason for this score
    pub reason: String,
}

/// Event emitter for broadcasting events
pub struct EventEmitter {
    sender: broadcast::Sender<Event>,
}

impl EventEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    /// Emit an event
    pub fn emit(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get subscriber count
    #[allow(dead_code)]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
