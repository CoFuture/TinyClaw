//! Task module - Background task execution for autonomous agent operations
//!
//! This module provides the ability for the agent to execute multi-step tasks
//! in the background, with state tracking and progress reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task has been created but not yet started
    Pending,
    /// Task is currently being executed
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled by user
    Cancelled,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::Running => "running",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
        }
    }
}

/// A single step in a multi-step task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Step description (what the agent planned to do)
    pub description: String,
    /// Step result (what actually happened)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Whether the step succeeded
    pub success: bool,
    /// When this step started
    pub started_at: Option<DateTime<Utc>>,
    /// When this step completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl TaskStep {
    #[allow(dead_code)]
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            result: None,
            success: false,
            started_at: None,
            completed_at: None,
        }
    }

    /// Mark step as started
    #[allow(dead_code)]
    pub fn start(&mut self) {
        self.started_at = Some(Utc::now());
    }

    /// Mark step as completed with result
    pub fn complete(&mut self, result: impl Into<String>, success: bool) {
        self.result = Some(result.into());
        self.success = success;
        self.completed_at = Some(Utc::now());
    }
}

/// Task definition - represents an autonomous agent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID
    pub id: String,
    /// Human-readable task description
    pub description: String,
    /// Session ID this task is associated with
    pub session_id: String,
    /// Steps in this task
    pub steps: Vec<TaskStep>,
    /// Current state of the task
    pub state: TaskState,
    /// Final result (final agent response)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Error message if task failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task started execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the task completed (success, failure, or cancellation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Task metadata (flexible key-value storage)
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl Task {
    /// Create a new pending task
    pub fn new(description: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            session_id: session_id.into(),
            steps: Vec::new(),
            state: TaskState::Pending,
            result: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Start the task
    pub fn start(&mut self) {
        self.state = TaskState::Running;
        self.started_at = Some(Utc::now());
    }

    /// Add a step to the task
    pub fn add_step(&mut self, description: impl Into<String>) -> usize {
        let idx = self.steps.len();
        self.steps.push(TaskStep::new(description));
        idx
    }

    /// Complete the current step (the first incomplete one)
    pub fn complete_step(&mut self, result: impl Into<String>, success: bool) {
        // Find the first incomplete step
        if let Some(step) = self.steps.iter_mut().find(|s| s.completed_at.is_none()) {
            step.complete(result, success);
        }
    }

    /// Mark the task as completed successfully
    pub fn complete(&mut self, result: impl Into<String>) {
        self.state = TaskState::Completed;
        self.result = Some(result.into());
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.state = TaskState::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as cancelled
    pub fn cancel(&mut self) {
        self.state = TaskState::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Check if task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }

    /// Get the progress percentage (0-100)
    pub fn progress_percent(&self) -> u8 {
        if self.steps.is_empty() {
            match self.state {
                TaskState::Pending => 0,
                TaskState::Running => 50, // Started but no steps recorded yet
                TaskState::Completed => 100,
                TaskState::Failed | TaskState::Cancelled => 0,
            }
        } else {
            let completed = self.steps.iter().filter(|s| s.completed_at.is_some()).count();
            let total = self.steps.len();
            ((completed as f64 / total as f64) * 100.0) as u8
        }
    }
}

/// Thread-safe wrapper for Task
pub type TaskHandle = Arc<RwLock<Task>>;

/// Summary of a task for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub description: String,
    pub session_id: String,
    pub state: TaskState,
    pub progress_percent: u8,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl From<&Task> for TaskSummary {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            description: if task.description.len() > 100 {
                format!("{}...", &task.description[..100])
            } else {
                task.description.clone()
            },
            session_id: task.session_id.clone(),
            state: task.state,
            progress_percent: task.progress_percent(),
            created_at: task.created_at,
            completed_at: task.completed_at,
            error: task.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task", "session1");
        assert_eq!(task.state, TaskState::Pending);
        assert_eq!(task.description, "Test task");
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_task_start() {
        let mut task = Task::new("Test task", "session1");
        task.start();
        assert_eq!(task.state, TaskState::Running);
        assert!(task.started_at.is_some());
    }

    #[test]
    fn test_task_steps() {
        let mut task = Task::new("Test task", "session1");
        task.add_step("Step 1");
        task.add_step("Step 2");
        assert_eq!(task.steps.len(), 2);
        
        task.complete_step("Result 1", true);
        assert!(task.steps[0].success);
        assert!(task.steps[1].result.is_none());
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new("Test task", "session1");
        task.start();
        task.complete("Final result");
        assert_eq!(task.state, TaskState::Completed);
        assert_eq!(task.result.as_deref(), Some("Final result"));
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_fail() {
        let mut task = Task::new("Test task", "session1");
        task.start();
        task.fail("Something went wrong");
        assert_eq!(task.state, TaskState::Failed);
        assert_eq!(task.error.as_deref(), Some("Something went wrong"));
    }

    #[test]
    fn test_task_cancel() {
        let mut task = Task::new("Test task", "session1");
        task.start();
        task.cancel();
        assert_eq!(task.state, TaskState::Cancelled);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_progress() {
        let mut task = Task::new("Test task", "session1");
        assert_eq!(task.progress_percent(), 0);
        
        task.add_step("Step 1");
        task.add_step("Step 2");
        assert_eq!(task.progress_percent(), 0); // No steps completed yet
        
        task.start();
        assert_eq!(task.progress_percent(), 0); // Running but no steps completed
        
        task.complete_step("R1", true);
        assert_eq!(task.progress_percent(), 50); // 1/2 steps = 50%
        
        task.complete_step("R2", true);
        assert_eq!(task.progress_percent(), 100); // 2/2 steps = 100%
    }

    #[test]
    fn test_task_is_terminal() {
        let mut task = Task::new("Test", "s1");
        assert!(!task.is_terminal());
        
        task.start();
        assert!(!task.is_terminal());
        
        task.complete("Done");
        assert!(task.is_terminal());
        
        let mut task2 = Task::new("Test", "s1");
        task2.start();
        task2.fail("Error");
        assert!(task2.is_terminal());
    }

    #[test]
    fn test_task_summary() {
        let task = Task::new("This is a very long description that exceeds one hundred characters and should definitely be truncated when converted to summary", "session1");
        let summary = TaskSummary::from(&task);
        assert!(summary.description.ends_with("..."));
        assert!(summary.description.len() <= 103); // 100 + "..."
    }
}
