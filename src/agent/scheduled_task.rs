//! Scheduled Task module - Cron-style scheduled task execution
//!
//! This module provides the ability to schedule tasks for automatic execution
//! at specific times or intervals, enabling 24/7 autonomous agent operation.

use chrono::{DateTime, Utc};
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Schedule type for a scheduled task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    /// Cron expression (e.g., "0 * * * *" for hourly)
    Cron,
    /// Fixed interval in seconds
    Interval,
}

impl ScheduleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleType::Cron => "cron",
            ScheduleType::Interval => "interval",
        }
    }
}

/// A scheduled task that triggers background tasks automatically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique schedule ID
    pub id: String,
    /// Human-readable schedule name/description
    pub name: String,
    /// Schedule type
    pub schedule_type: ScheduleType,
    /// Cron expression (when schedule_type = Cron)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_expression: Option<String>,
    /// Interval in seconds (when schedule_type = Interval)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_seconds: Option<u64>,
    /// The task/command to execute
    pub task_description: String,
    /// Session ID to execute in
    pub session_id: String,
    /// Whether the schedule is enabled
    pub enabled: bool,
    /// Next scheduled run time (UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at: Option<DateTime<Utc>>,
    /// Last run time (UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<DateTime<Utc>>,
    /// How many times this schedule has fired
    pub run_count: u64,
    /// Whether this schedule is paused
    pub paused: bool,
    /// When the schedule was created
    pub created_at: DateTime<Utc>,
    /// When the schedule was last updated
    pub updated_at: DateTime<Utc>,
    /// Last triggered task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_task_id: Option<String>,
}

impl ScheduledTask {
    /// Create a new cron-based scheduled task
    pub fn new_cron(
        name: impl Into<String>,
        cron_expression: impl Into<String>,
        task_description: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Result<Self, String> {
        let cron_expr = cron_expression.into();
        // Validate cron expression by parsing it
        let schedule = CronSchedule::from_str(&cron_expr)
            .map_err(|e| format!("Invalid cron expression: {}", e))?;
        
        let next = schedule.upcoming(Utc).next();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            schedule_type: ScheduleType::Cron,
            cron_expression: Some(cron_expr),
            interval_seconds: None,
            task_description: task_description.into(),
            session_id: session_id.into(),
            enabled: true,
            next_run_at: next,
            last_run_at: None,
            run_count: 0,
            paused: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_task_id: None,
        })
    }

    /// Create a new interval-based scheduled task
    pub fn new_interval(
        name: impl Into<String>,
        interval_seconds: u64,
        task_description: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        let next = Utc::now() + chrono::Duration::seconds(interval_seconds as i64);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            schedule_type: ScheduleType::Interval,
            cron_expression: None,
            interval_seconds: Some(interval_seconds),
            task_description: task_description.into(),
            session_id: session_id.into(),
            enabled: true,
            next_run_at: Some(next),
            last_run_at: None,
            run_count: 0,
            paused: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_task_id: None,
        }
    }

    /// Check if the schedule is due (next_run_at <= now)
    pub fn is_due(&self) -> bool {
        if !self.enabled || self.paused {
            return false;
        }
        if let Some(next) = self.next_run_at {
            return next <= Utc::now();
        }
        false
    }

    /// Calculate and set the next run time
    pub fn advance(&mut self) {
        self.last_run_at = Some(Utc::now());
        self.run_count += 1;
        self.updated_at = Utc::now();

        match self.schedule_type {
            ScheduleType::Cron => {
                if let Some(ref expr) = self.cron_expression {
                    if let Ok(schedule) = CronSchedule::from_str(expr) {
                        self.next_run_at = schedule.upcoming(Utc).next();
                    }
                }
            }
            ScheduleType::Interval => {
                if let Some(interval) = self.interval_seconds {
                    self.next_run_at = Some(
                        Utc::now() + chrono::Duration::seconds(interval as i64)
                    );
                }
            }
        }
    }

    /// Pause the schedule
    pub fn pause(&mut self) {
        self.paused = true;
        self.updated_at = Utc::now();
    }

    /// Resume the schedule
    pub fn resume(&mut self) {
        self.paused = false;
        self.updated_at = Utc::now();
        // Recalculate next run
        if self.paused {
            return; // Actually still paused
        }
        self.advance();
        // Undo the advance - we just want to set next_run_at correctly
        self.run_count -= 1;
        self.last_run_at = None;
        // For cron, recalculate next from now
        if let ScheduleType::Cron = self.schedule_type {
            if let Some(ref expr) = self.cron_expression {
                if let Ok(schedule) = CronSchedule::from_str(expr) {
                    self.next_run_at = schedule.upcoming(Utc).next();
                }
            }
        } else if let Some(interval) = self.interval_seconds {
            self.next_run_at = Some(Utc::now() + chrono::Duration::seconds(interval as i64));
        }
    }

    /// Disable the schedule
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    /// Enable the schedule
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }
}

/// Summary of a scheduled task for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskSummary {
    pub id: String,
    pub name: String,
    pub schedule_type: ScheduleType,
    pub schedule_display: String,
    pub task_description: String,
    pub session_id: String,
    pub enabled: bool,
    pub paused: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub run_count: u64,
}

impl From<&ScheduledTask> for ScheduledTaskSummary {
    fn from(st: &ScheduledTask) -> Self {
        let schedule_display = match st.schedule_type {
            ScheduleType::Cron => st.cron_expression.clone().unwrap_or_default(),
            ScheduleType::Interval => {
                if let Some(secs) = st.interval_seconds {
                    if secs < 60 {
                        format!("every {}s", secs)
                    } else if secs < 3600 {
                        format!("every {}m", secs / 60)
                    } else {
                        format!("every {}h", secs / 3600)
                    }
                } else {
                    "unknown".to_string()
                }
            }
        };

        Self {
            id: st.id.clone(),
            name: st.name.clone(),
            schedule_type: st.schedule_type,
            schedule_display,
            task_description: if st.task_description.len() > 60 {
                format!("{}...", &st.task_description[..60])
            } else {
                st.task_description.clone()
            },
            session_id: st.session_id.clone(),
            enabled: st.enabled,
            paused: st.paused,
            next_run_at: st.next_run_at,
            last_run_at: st.last_run_at,
            run_count: st.run_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduled_task_cron_creation() {
        // Cron crate uses 6-field format (second minute hour day month dow)
        let task = ScheduledTask::new_cron(
            "Hourly check",
            "0 0 * * * *",
            "Check system status",
            "main",
        ).unwrap();
        
        assert_eq!(task.name, "Hourly check");
        assert_eq!(task.schedule_type, ScheduleType::Cron);
        assert!(task.cron_expression.is_some());
        assert!(task.next_run_at.is_some());
        assert!(task.enabled);
        assert!(!task.paused);
    }

    #[test]
    fn test_scheduled_task_interval_creation() {
        let task = ScheduledTask::new_interval(
            "Every 5 minutes",
            300,
            "Run health check",
            "main",
        );
        
        assert_eq!(task.name, "Every 5 minutes");
        assert_eq!(task.schedule_type, ScheduleType::Interval);
        assert_eq!(task.interval_seconds, Some(300));
        assert!(task.next_run_at.is_some());
    }

    #[test]
    fn test_invalid_cron_expression() {
        let result = ScheduledTask::new_cron(
            "Bad cron",
            "not a valid cron",
            "Do something",
            "main",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_cron() {
        // Cron crate uses 6-field format
        let mut task = ScheduledTask::new_cron(
            "Test",
            "0 */5 * * * *",
            "Test task",
            "main",
        ).unwrap();
        
        let original_next = task.next_run_at;
        task.advance();
        
        assert!(task.next_run_at.is_some());
        assert!(task.next_run_at > original_next || task.next_run_at == original_next);
        assert_eq!(task.run_count, 1);
    }

    #[test]
    fn test_advance_interval() {
        let mut task = ScheduledTask::new_interval(
            "Test",
            60,
            "Test task",
            "main",
        );
        
        let original_next = task.next_run_at;
        task.advance();
        
        assert!(task.next_run_at.is_some());
        // After advance, next_run_at should be in the future
        // and last_run_at should be set
        assert!(task.last_run_at.is_some());
        if let Some(last) = task.last_run_at {
            if let Some(next) = task.next_run_at {
                let diff = (next - last).num_seconds();
                assert_eq!(diff, 60);
            }
        }
        assert_eq!(task.run_count, 1);
    }

    #[test]
    fn test_pause_resume() {
        let mut task = ScheduledTask::new_interval("Test", 60, "Test", "main");
        
        task.pause();
        assert!(task.paused);
        assert!(!task.is_due());
        
        task.resume();
        assert!(!task.paused);
    }

    #[test]
    fn test_disable_enable() {
        let mut task = ScheduledTask::new_interval("Test", 60, "Test", "main");
        
        task.disable();
        assert!(!task.enabled);
        assert!(!task.is_due());
        
        task.enable();
        assert!(task.enabled);
    }

    #[test]
    fn test_is_due() {
        let mut task = ScheduledTask::new_interval("Test", 60, "Test", "main");
        
        // Should be due immediately since next_run_at was set to now+60s
        // But after creation, it shouldn't be due yet
        assert!(!task.is_due());
        
        // Force next_run_at to the past
        task.next_run_at = Some(Utc::now() - chrono::Duration::seconds(10));
        assert!(task.is_due());
        
        // But if paused
        task.pause();
        assert!(!task.is_due());
    }

    #[test]
    fn test_summary() {
        let task = ScheduledTask::new_interval("My Schedule", 3600, "A very long task description that exceeds sixty characters and should be truncated in the summary view", "main");
        let summary = ScheduledTaskSummary::from(&task);
        
        assert_eq!(summary.name, "My Schedule");
        assert!(summary.task_description.ends_with("..."));
        assert_eq!(summary.schedule_display, "every 1h");
    }

    #[test]
    fn test_various_intervals() {
        // Test seconds
        let t1 = ScheduledTask::new_interval("s", 30, "t", "s");
        let s1 = ScheduledTaskSummary::from(&t1);
        assert_eq!(s1.schedule_display, "every 30s");
        
        // Test minutes
        let t2 = ScheduledTask::new_interval("m", 300, "t", "s");
        let s2 = ScheduledTaskSummary::from(&t2);
        assert_eq!(s2.schedule_display, "every 5m");
        
        // Test hours
        let t3 = ScheduledTask::new_interval("h", 7200, "t", "s");
        let s3 = ScheduledTaskSummary::from(&t3);
        assert_eq!(s3.schedule_display, "every 2h");
    }
}
