//! Scheduler module - Background scheduler for automatic task triggering
//!
//! Manages scheduled tasks, runs a background polling loop, and triggers
//! tasks when their schedules become due.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock as ParkingRwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{error, info};

use super::scheduled_task::{ScheduledTask, ScheduledTaskSummary};
use super::task_manager::TaskManager;
use crate::gateway::events::{Event, EventEmitter};

/// Thread-safe wrapper for ScheduledTask
pub type ScheduledTaskHandle = Arc<ParkingRwLock<ScheduledTask>>;

/// Scheduler - manages scheduled tasks and triggers them when due
pub struct Scheduler {
    /// All scheduled tasks by ID
    schedules: ParkingRwLock<HashMap<String, ScheduledTaskHandle>>,
    /// Background task handle (for shutdown)
    background_handle: ParkingRwLock<Option<JoinHandle<()>>>,
    /// Event emitter
    event_emitter: Option<Arc<EventEmitter>>,
    /// Task manager (to create tasks when schedules fire)
    task_manager: Option<Arc<TaskManager>>,
    /// Shutdown signal receiver
    shutdown_rx: ParkingRwLock<Option<broadcast::Receiver<()>>>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            schedules: ParkingRwLock::new(HashMap::new()),
            background_handle: ParkingRwLock::new(None),
            event_emitter: None,
            task_manager: None,
            shutdown_rx: ParkingRwLock::new(None),
        }
    }

    /// Set the event emitter
    pub fn with_event_emitter(mut self, emitter: Arc<EventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Set the task manager
    pub fn with_task_manager(mut self, task_manager: Arc<TaskManager>) -> Self {
        self.task_manager = Some(task_manager);
        self
    }

    /// Set the shutdown receiver
    pub fn with_shutdown_rx(self, rx: broadcast::Receiver<()>) -> Self {
        *self.shutdown_rx.write() = Some(rx);
        self
    }

    /// Start the background scheduler loop
    pub fn start(&self) {
        let schedules = Arc::new(ParkingRwLock::new(HashMap::<String, ScheduledTaskHandle>::new()));
        // Copy handles to the background task
        {
            let mut bg_schedules = schedules.write();
            let my_schedules = self.schedules.read();
            for (id, handle) in my_schedules.iter() {
                bg_schedules.insert(id.clone(), handle.clone());
            }
        }
        
        let event_emitter = self.event_emitter.clone();
        let task_manager = self.task_manager.clone();
        let shutdown_rx_opt = self.shutdown_rx.write().take();
        
        let handle = tokio::spawn(async move {
            Self::run_loop(schedules, event_emitter, task_manager, shutdown_rx_opt).await;
        });
        
        *self.background_handle.write() = Some(handle);
        info!("Scheduler background loop started");
    }

    /// Main scheduler loop
    async fn run_loop(
        schedules: Arc<ParkingRwLock<HashMap<String, ScheduledTaskHandle>>>,
        event_emitter: Option<Arc<EventEmitter>>,
        task_manager: Option<Arc<TaskManager>>,
        mut shutdown_rx: Option<broadcast::Receiver<()>>,
    ) {
        let mut tick_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        
        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    Self::check_and_fire_schedules(&schedules, &event_emitter, &task_manager).await;
                }
                _ = async {
                    if let Some(ref mut rx) = shutdown_rx {
                        rx.recv().await
                    } else {
                        tokio::time::sleep(tokio::time::Duration::MAX).await;
                        Ok(())
                    }
                } => {
                    info!("Scheduler received shutdown signal");
                    break;
                }
            }
        }
    }

    /// Check all schedules and fire any that are due
    async fn check_and_fire_schedules(
        schedules: &Arc<ParkingRwLock<HashMap<String, ScheduledTaskHandle>>>,
        event_emitter: &Option<Arc<EventEmitter>>,
        task_manager: &Option<Arc<TaskManager>>,
    ) {
        let due_ids: Vec<String> = {
            let scheds = schedules.read();
            scheds.values()
                .filter(|h| h.read().is_due())
                .map(|h| h.read().id.clone())
                .collect()
        };

        for schedule_id in due_ids {
            let task_manager = task_manager.clone();
            let event_emitter = event_emitter.clone();
            
            let (task_desc, session_id, schedule_handle) = {
                let scheds = schedules.read();
                let handle = scheds.get(&schedule_id);
                if let Some(h) = handle {
                    let st = h.read();
                    (st.task_description.clone(), st.session_id.clone(), h.clone())
                } else {
                    continue;
                }
            };

            // Mark as fired (advance the schedule)
            {
                let mut st = schedule_handle.write();
                st.advance();
            }

            // Emit scheduled.fired event
            if let Some(emitter) = &event_emitter {
                let st = schedule_handle.read();
                emitter.emit(Event::ScheduledTaskFired {
                    schedule_id: schedule_id.clone(),
                    schedule_name: st.name.clone(),
                    task_description: task_desc.clone(),
                    session_id: session_id.clone(),
                    run_count: st.run_count,
                });
            }

            // Create and start a background task
            if let Some(tm) = &task_manager {
                let schedule_id_clone = schedule_id.clone();
                let schedule_handle_clone = schedule_handle.clone();
                
                // Create the task in the task manager
                let task_handle = tm.create_task(
                    task_desc.clone(),
                    session_id.clone(),
                ).await;
                
                // Extract the task ID before the borrow ends
                let task_id = task_handle.read().id.clone();
                
                // Store the task ID in the schedule
                {
                    let mut st = schedule_handle_clone.write();
                    st.last_task_id = Some(task_id.clone());
                }
                
                // Start the task
                if let Err(e) = tm.start_task(&task_id).await {
                    error!(
                        schedule_id = %schedule_id_clone,
                        task_id = %task_id,
                        error = %e,
                        "Failed to start scheduled task"
                    );
                    
                    if let Some(emitter) = &event_emitter {
                        emitter.emit(Event::ScheduledTaskFailed {
                            schedule_id: schedule_id_clone.clone(),
                            error: format!("Failed to start task: {}", e),
                        });
                    }
                } else {
                    info!(
                        schedule_id = %schedule_id_clone,
                        task_id = %task_id,
                        "Scheduled task triggered"
                    );
                }
            }
        }
    }

    /// Add a cron-based scheduled task
    pub fn add_cron(
        &self,
        name: impl Into<String>,
        cron_expression: impl Into<String>,
        task_description: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Result<ScheduledTaskHandle, String> {
        let task = ScheduledTask::new_cron(
            name,
            cron_expression,
            task_description,
            session_id,
        )?;
        let handle: ScheduledTaskHandle = Arc::new(ParkingRwLock::new(task));
        
        let id = {
            let t = handle.read();
            t.id.clone()
        };
        
        self.schedules.write().insert(id.clone(), handle.clone());
        
        info!(
            schedule_id = %id,
            schedule_name = %handle.read().name,
            "Added cron scheduled task"
        );
        
        // Emit event
        if let Some(emitter) = &self.event_emitter {
            let summary = ScheduledTaskSummary::from(&*handle.read());
            emitter.emit(Event::ScheduledTaskCreated {
                schedule_id: id,
                summary,
            });
        }
        
        Ok(handle)
    }

    /// Add an interval-based scheduled task
    pub fn add_interval(
        &self,
        name: impl Into<String>,
        interval_seconds: u64,
        task_description: impl Into<String>,
        session_id: impl Into<String>,
    ) -> ScheduledTaskHandle {
        let task = ScheduledTask::new_interval(
            name,
            interval_seconds,
            task_description,
            session_id,
        );
        let handle: ScheduledTaskHandle = Arc::new(ParkingRwLock::new(task));
        
        let id = {
            let t = handle.read();
            t.id.clone()
        };
        
        self.schedules.write().insert(id.clone(), handle.clone());
        
        info!(
            schedule_id = %id,
            schedule_name = %handle.read().name,
            interval_seconds = %interval_seconds,
            "Added interval scheduled task"
        );
        
        // Emit event
        if let Some(emitter) = &self.event_emitter {
            let summary = ScheduledTaskSummary::from(&*handle.read());
            emitter.emit(Event::ScheduledTaskCreated {
                schedule_id: id,
                summary,
            });
        }
        
        handle
    }

    /// Get a scheduled task by ID
    pub fn get(&self, schedule_id: &str) -> Option<ScheduledTaskHandle> {
        self.schedules.read().get(schedule_id).cloned()
    }

    /// List all scheduled tasks
    pub fn list(&self) -> Vec<ScheduledTaskSummary> {
        let schedules = self.schedules.read();
        schedules
            .values()
            .map(|h| ScheduledTaskSummary::from(&*h.read()))
            .collect()
    }

    /// List only enabled/active scheduled tasks
    pub fn list_enabled(&self) -> Vec<ScheduledTaskSummary> {
        let schedules = self.schedules.read();
        schedules
            .values()
            .filter(|h| h.read().enabled && !h.read().paused)
            .map(|h| ScheduledTaskSummary::from(&*h.read()))
            .collect()
    }

    /// Pause a scheduled task
    pub fn pause(&self, schedule_id: &str) -> Result<(), String> {
        let handle = self.schedules.read()
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", schedule_id))?;
        
        handle.write().pause();
        
        info!(schedule_id = %schedule_id, "Paused scheduled task");
        
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::ScheduledTaskUpdated {
                schedule_id: schedule_id.to_string(),
            });
        }
        
        Ok(())
    }

    /// Resume a scheduled task
    pub fn resume(&self, schedule_id: &str) -> Result<(), String> {
        let handle = self.schedules.read()
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", schedule_id))?;
        
        handle.write().resume();
        
        info!(schedule_id = %schedule_id, "Resumed scheduled task");
        
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::ScheduledTaskUpdated {
                schedule_id: schedule_id.to_string(),
            });
        }
        
        Ok(())
    }

    /// Delete a scheduled task
    pub fn delete(&self, schedule_id: &str) -> Option<ScheduledTaskHandle> {
        let removed = self.schedules.write().remove(schedule_id);
        
        if removed.is_some() {
            info!(schedule_id = %schedule_id, "Deleted scheduled task");
            
            if let Some(emitter) = &self.event_emitter {
                emitter.emit(Event::ScheduledTaskDeleted {
                    schedule_id: schedule_id.to_string(),
                });
            }
        }
        
        removed
    }

    /// Enable a scheduled task
    pub fn enable(&self, schedule_id: &str) -> Result<(), String> {
        let handle = self.schedules.read()
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", schedule_id))?;
        
        handle.write().enable();
        
        info!(schedule_id = %schedule_id, "Enabled scheduled task");
        
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::ScheduledTaskUpdated {
                schedule_id: schedule_id.to_string(),
            });
        }
        
        Ok(())
    }

    /// Disable a scheduled task
    pub fn disable(&self, schedule_id: &str) -> Result<(), String> {
        let handle = self.schedules.read()
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", schedule_id))?;
        
        handle.write().disable();
        
        info!(schedule_id = %schedule_id, "Disabled scheduled task");
        
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::ScheduledTaskUpdated {
                schedule_id: schedule_id.to_string(),
            });
        }
        
        Ok(())
    }

    /// Manually fire a scheduled task now (bypass schedule)
    #[allow(dead_code)]
    pub async fn fire_now(&self, schedule_id: &str) -> Result<(), String> {
        let handle = self.schedules.read()
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", schedule_id))?;
        
        let (task_desc, session_id) = {
            let st = handle.read();
            (st.task_description.clone(), st.session_id.clone())
        };
        
        // Emit fired event
        if let Some(emitter) = &self.event_emitter {
            emitter.emit(Event::ScheduledTaskFired {
                schedule_id: schedule_id.to_string(),
                schedule_name: handle.read().name.clone(),
                task_description: task_desc.clone(),
                session_id: session_id.clone(),
                run_count: handle.read().run_count + 1,
            });
        }
        
        // Advance the schedule
        handle.write().advance();
        
        // Create and start a task
        let task_manager = self.task_manager.clone()
            .ok_or_else(|| "Task manager not configured".to_string())?;
        
        let task_handle = task_manager.create_task(task_desc, session_id).await;
        
        // Extract IDs before await points - parking_lot guards are not Send
        let task_id = task_handle.read().id.clone();
        
        {
            let mut st = handle.write();
            st.last_task_id = Some(task_id.clone());
        }
        
        task_manager.start_task(&task_id).await
            .map_err(|e| format!("{}", e))?;
        
        info!(schedule_id = %schedule_id, task_id = %task_id, "Manually fired scheduled task");
        
        Ok(())
    }

    /// Get count of schedules
    pub fn count(&self) -> usize {
        self.schedules.read().len()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_cron_schedule() {
        let scheduler = Scheduler::new();
        // Cron crate uses 6-field format
        let handle = scheduler.add_cron(
            "Hourly backup",
            "0 0 * * * *",
            "Run backup script",
            "main",
        ).unwrap();
        
        assert_eq!(handle.read().name, "Hourly backup");
        assert_eq!(scheduler.count(), 1);
    }

    #[test]
    fn test_add_interval_schedule() {
        let scheduler = Scheduler::new();
        let handle = scheduler.add_interval(
            "Every 5 minutes",
            300,
            "Health check",
            "main",
        );
        
        assert_eq!(handle.read().name, "Every 5 minutes");
        assert_eq!(scheduler.count(), 1);
    }

    #[test]
    fn test_get_schedule() {
        let scheduler = Scheduler::new();
        let handle = scheduler.add_interval("Test", 60, "Test task", "main");
        let id = handle.read().id.clone();
        
        let retrieved = scheduler.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().read().id, id);
    }

    #[test]
    fn test_list_schedules() {
        let scheduler = Scheduler::new();
        scheduler.add_interval("Task 1", 60, "Task 1", "main");
        scheduler.add_interval("Task 2", 120, "Task 2", "main");
        
        let list = scheduler.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_pause_resume() {
        let scheduler = Scheduler::new();
        let handle = scheduler.add_interval("Test", 60, "Test task", "main");
        let id = handle.read().id.clone();
        
        scheduler.pause(&id).unwrap();
        assert!(scheduler.get(&id).unwrap().read().paused);
        
        scheduler.resume(&id).unwrap();
        assert!(!scheduler.get(&id).unwrap().read().paused);
    }

    #[test]
    fn test_enable_disable() {
        let scheduler = Scheduler::new();
        let handle = scheduler.add_interval("Test", 60, "Test task", "main");
        let id = handle.read().id.clone();
        
        scheduler.disable(&id).unwrap();
        assert!(!scheduler.get(&id).unwrap().read().enabled);
        
        scheduler.enable(&id).unwrap();
        assert!(scheduler.get(&id).unwrap().read().enabled);
    }

    #[test]
    fn test_delete_schedule() {
        let scheduler = Scheduler::new();
        let handle = scheduler.add_interval("Test", 60, "Test task", "main");
        let id = handle.read().id.clone();
        
        let removed = scheduler.delete(&id);
        assert!(removed.is_some());
        assert_eq!(scheduler.count(), 0);
        
        // Should be gone
        assert!(scheduler.get(&id).is_none());
    }

    #[test]
    fn test_invalid_cron() {
        let scheduler = Scheduler::new();
        let result = scheduler.add_cron(
            "Bad",
            "invalid cron",
            "Task",
            "main",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_nonexistent() {
        let scheduler = Scheduler::new();
        let result = scheduler.pause("nonexistent");
        assert!(result.is_err());
    }
}
