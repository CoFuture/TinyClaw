//! Task Manager - Background task queue management
//!
//! Manages task lifecycle: creation, execution, tracking, and cleanup.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock as ParkingRwLock;
use tokio::sync::RwLock as TokioRwLock;
use tokio::task::JoinHandle;
use tracing::{info, error};

use super::task::{Task, TaskHandle, TaskState, TaskSummary};
use crate::gateway::events::{Event, EventEmitter};
use crate::agent::Agent;
use crate::common::Result;

/// Task cancellation handle (async-compatible)
pub type TaskCancelHandle = Arc<TokioRwLock<bool>>;

/// Task manager - manages all background tasks
pub struct TaskManager {
    /// All tasks by ID (using tokio RwLock for async access)
    tasks: TokioRwLock<HashMap<String, TaskHandle>>,
    /// Running task handles (for cancellation)
    running_tasks: TokioRwLock<HashMap<String, TaskCancelHandle>>,
    /// Event emitter for task events
    event_emitter: Option<Arc<EventEmitter>>,
    /// Agent for executing tasks
    agent: Option<Arc<Agent>>,
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> Self {
        Self {
            tasks: TokioRwLock::new(HashMap::new()),
            running_tasks: TokioRwLock::new(HashMap::new()),
            event_emitter: None,
            agent: None,
        }
    }

    /// Set the event emitter
    pub fn with_event_emitter(mut self, emitter: Arc<EventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Set the agent for task execution
    pub fn with_agent(mut self, agent: Arc<Agent>) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Create a new task and add it to the manager
    pub async fn create_task(
        &self,
        description: impl Into<String>,
        session_id: impl Into<String>,
    ) -> TaskHandle {
        let task = Task::new(description, session_id);
        let handle: TaskHandle = Arc::new(ParkingRwLock::new(task));
        
        let task_id = {
            let t = handle.read();
            t.id.clone()
        };
        
        self.tasks.write().await.insert(task_id.clone(), handle.clone());
        
        info!(
            task_id = %task_id,
            session_id = %handle.read().session_id,
            "Created new task"
        );
        
        // Emit task.created event
        if let Some(emitter) = &self.event_emitter {
            let summary = TaskSummary::from(&*handle.read());
            emitter.emit(Event::TaskCreated {
                task_id: task_id.clone(),
                summary,
            });
        }
        
        handle
    }

    /// Start executing a task in the background
    pub async fn start_task(&self, task_id: &str) -> Result<()> {
        let task_handle = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
        };
        
        let task_handle = task_handle.ok_or_else(|| {
            crate::common::Error::Protocol(format!("Task not found: {}", task_id))
        })?;
        
        // Check if task is already running
        {
            let task = task_handle.read();
            if task.state != TaskState::Pending {
                return Err(crate::common::Error::Protocol(
                    format!("Task is not in pending state: {:?}", task.state)
                ));
            }
        }
        
        // Get agent
        let agent = self.agent.clone().ok_or_else(|| {
            crate::common::Error::Protocol(
                "TaskManager not configured with agent".to_string()
            )
        })?;
        
        // Create cancellation handle
        let cancel_handle: TaskCancelHandle = Arc::new(TokioRwLock::new(false));
        self.running_tasks.write().await.insert(task_id.to_string(), cancel_handle.clone());
        
        // Mark task as running
        {
            let mut task = task_handle.write();
            task.start();
        }
        
        info!(task_id = %task_id, "Starting task execution");
        
        // Emit task.started event
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::TaskStarted {
                task_id: task_id.to_string(),
            });
        }
        
        // Spawn background task
        let task_id_str = task_id.to_string();
        let task_handle_clone = task_handle.clone();
        let cancel_clone = cancel_handle.clone();
        let emitter_clone = self.event_emitter.clone();
        
        let _join_handle: JoinHandle<()> = tokio::spawn(async move {
            // Execute the task
            let result = Self::execute_task_internal(
                &task_handle_clone,
                &cancel_clone,
                &agent,
                emitter_clone.clone(),
            ).await;
            
            // Update task state based on result
            {
                let mut task = task_handle_clone.write();
                match result {
                    Ok(response) => {
                        task.complete(&response);
                        info!(task_id = %task_id_str, "Task completed successfully");
                        
                        if let Some(emitter) = &emitter_clone {
                            emitter.emit(Event::TaskCompleted {
                                task_id: task_id_str.clone(),
                                result: response,
                            });
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        task.fail(&err_msg);
                        error!(task_id = %task_id_str, error = %err_msg, "Task failed");
                        
                        if let Some(emitter) = &emitter_clone {
                            emitter.emit(Event::TaskFailed {
                                task_id: task_id_str.clone(),
                                error: err_msg,
                            });
                        }
                    }
                }
            }
            
            // Cleanup: clear cancellation flag
            *cancel_clone.write().await = false;
        });
        
        Ok(())
    }

    /// Internal task execution logic
    async fn execute_task_internal(
        task_handle: &TaskHandle,
        cancel_handle: &TaskCancelHandle,
        agent: &Arc<Agent>,
        event_emitter: Option<Arc<EventEmitter>>,
    ) -> Result<String> {
        let (session_id, description) = {
            let task = task_handle.read();
            (task.session_id.clone(), task.description.clone())
        };
        
        // Check cancellation before starting
        if *cancel_handle.read().await {
            return Err(crate::common::Error::Cancelled);
        }
        
        // Emit initial progress
        if let Some(emitter) = &event_emitter {
            emitter.emit(Event::TaskProgress {
                task_id: task_handle.read().id.clone(),
                step: 0,
                total_steps: 1,
                message: "Starting task execution".to_string(),
            });
        }
        
        // Add first step
        {
            let mut task = task_handle.write();
            task.add_step(format!("Executing: {}", description));
        }
        
        // Check cancellation
        if *cancel_handle.read().await {
            return Err(crate::common::Error::Cancelled);
        }
        
        // Execute via agent - send the task description as a message
        // The agent will process this and return a response
        let response = agent.send_message(
            &session_id,
            &description,
            None,
        ).await;
        
        // Check cancellation after agent call
        if *cancel_handle.read().await {
            return Err(crate::common::Error::Cancelled);
        }
        
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                // Complete the step as failed
                {
                    let mut task = task_handle.write();
                    task.complete_step(format!("Error: {}", e), false);
                }
                return Err(e);
            }
        };
        
        // Complete the step successfully
        {
            let mut task = task_handle.write();
            task.complete_step(&response, true);
        }
        
        // Emit progress update
        if let Some(emitter) = &event_emitter {
            emitter.emit(Event::TaskProgress {
                task_id: task_handle.read().id.clone(),
                step: 1,
                total_steps: 1,
                message: "Task step completed".to_string(),
            });
        }
        
        Ok(response)
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<TaskHandle> {
        self.tasks.read().await.get(task_id).cloned()
    }

    /// List all tasks (optionally filtered by state)
    pub async fn list_tasks(&self, state_filter: Option<TaskState>) -> Vec<TaskSummary> {
        let tasks = self.tasks.read().await;
        let mut summaries: Vec<TaskSummary> = tasks
            .values()
            .filter_map(|handle| {
                let task = handle.read();
                if let Some(filter) = state_filter {
                    if task.state != filter {
                        return None;
                    }
                }
                Some(TaskSummary::from(&*task))
            })
            .collect();
        
        // Sort by created_at descending (newest first)
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        summaries
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        // Check if task exists
        let task_handle = self.tasks.read().await
            .get(task_id)
            .cloned()
            .ok_or_else(|| crate::common::Error::Protocol(format!("Task not found: {}", task_id)))?;
        
        // Check if task is running
        let is_running = {
            let task = task_handle.read();
            task.state == TaskState::Running
        };
        
        if !is_running {
            return Ok(false);
        }
        
        // Set cancellation flag
        if let Some(cancel_handle) = self.running_tasks.read().await.get(task_id) {
            *cancel_handle.write().await = true;
        }
        
        // Mark task as cancelled
        {
            let mut task = task_handle.write();
            task.cancel();
        }
        
        info!(task_id = %task_id, "Task cancelled");
        
        // Emit event
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::TaskCancelled {
                task_id: task_id.to_string(),
            });
        }
        
        Ok(true)
    }

    /// Remove a completed task from the manager
    pub async fn remove_task(&self, task_id: &str) -> Option<TaskHandle> {
        // Only allow removing terminal tasks
        if let Some(handle) = self.tasks.read().await.get(task_id) {
            if !handle.read().is_terminal() {
                return None;
            }
        }
        
        let removed = self.tasks.write().await.remove(task_id);
        if removed.is_some() {
            info!(task_id = %task_id, "Removed task from manager");
        }
        removed
    }

    /// Get count of tasks by state
    pub async fn task_counts(&self) -> HashMap<TaskState, usize> {
        let tasks = self.tasks.read().await;
        let mut counts: HashMap<TaskState, usize> = HashMap::new();
        
        for handle in tasks.values() {
            let state = handle.read().state;
            *counts.entry(state).or_insert(0) += 1;
        }
        
        counts
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_task() {
        let manager = TaskManager::new();
        let handle = manager.create_task("Test task", "session1").await;
        
        assert_eq!(handle.read().description, "Test task");
        assert_eq!(handle.read().session_id, "session1");
        assert_eq!(handle.read().state, TaskState::Pending);
    }

    #[tokio::test]
    async fn test_get_task() {
        let manager = TaskManager::new();
        let handle = manager.create_task("Test task", "session1").await;
        let task_id = handle.read().id.clone();
        
        let retrieved = manager.get_task(&task_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().read().id, task_id);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let manager = TaskManager::new();
        manager.create_task("Task 1", "session1").await;
        manager.create_task("Task 2", "session1").await;
        manager.create_task("Task 3", "session1").await;
        
        let all = manager.list_tasks(None).await;
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_tasks_filtered() {
        let manager = TaskManager::new();
        let handle = manager.create_task("Task 1", "session1").await;
        manager.create_task("Task 2", "session1").await;
        
        // Start and complete the first task
        handle.write().start();
        handle.write().complete("Done");
        
        let pending = manager.list_tasks(Some(TaskState::Pending)).await;
        let completed = manager.list_tasks(Some(TaskState::Completed)).await;
        
        assert_eq!(pending.len(), 1);
        assert_eq!(completed.len(), 1);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let manager = TaskManager::new();
        let handle = manager.create_task("Task 1", "session1").await;
        let task_id = handle.read().id.clone();
        
        // Start the task (simulate running)
        handle.write().start();
        
        // Add to running tasks manually for test
        let cancel_handle: TaskCancelHandle = Arc::new(TokioRwLock::new(false));
        manager.running_tasks.write().await.insert(task_id.clone(), cancel_handle.clone());
        
        let cancelled = manager.cancel_task(&task_id).await.unwrap();
        assert!(cancelled);
        assert_eq!(handle.read().state, TaskState::Cancelled);
    }

    #[tokio::test]
    async fn test_remove_task() {
        let manager = TaskManager::new();
        let handle = manager.create_task("Task 1", "session1").await;
        let task_id = handle.read().id.clone();
        
        // Complete the task
        handle.write().complete("Done");
        
        let removed = manager.remove_task(&task_id).await;
        assert!(removed.is_some());
        
        // Verify removed
        let retrieved = manager.get_task(&task_id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_task_counts() {
        let manager = TaskManager::new();
        let h1 = manager.create_task("Task 1", "s1").await;
        let h2 = manager.create_task("Task 2", "s1").await;
        let h3 = manager.create_task("Task 3", "s1").await;
        
        h1.write().complete("Done");
        h2.write().fail("Error");
        
        let counts = manager.task_counts().await;
        assert_eq!(counts[&TaskState::Completed], 1);
        assert_eq!(counts[&TaskState::Failed], 1);
        assert_eq!(counts[&TaskState::Pending], 1);
    }
}
