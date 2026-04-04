//! Proactive Alert System - Agent proactively alerts user about important events
//!
//! This module provides a system for the agent to proactively notify the user
//! when important conditions are met, making the agent more "alive" and useful.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[allow(dead_code)]
pub enum AlertSeverity {
    /// Low severity - informational
    Info,
    /// Medium severity - warning
    Warning,
    /// High severity - critical issue
    Critical,
    /// Emergency - requires immediate action
    Emergency,
}

impl AlertSeverity {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
            AlertSeverity::Emergency => "emergency",
        }
    }

    #[allow(dead_code)]
    pub fn parse_from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => AlertSeverity::Critical,
            "emergency" => AlertSeverity::Emergency,
            "warning" => AlertSeverity::Warning,
            _ => AlertSeverity::Info,
        }
    }
}

/// Alert category - what type of event triggered the alert
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[allow(dead_code)]
pub enum AlertCategory {
    /// Context health related alert
    ContextHealth,
    /// Feedback trend related alert
    FeedbackTrend,
    /// Execution safety related alert
    Safety,
    /// Scheduled task related alert
    ScheduledTask,
    /// Memory related alert
    Memory,
    /// Session quality related alert
    Quality,
    /// Agent self-evaluation related alert
    SelfEvaluation,
    /// General informational alert
    General,
}

impl AlertCategory {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertCategory::ContextHealth => "context_health",
            AlertCategory::FeedbackTrend => "feedback_trend",
            AlertCategory::Safety => "safety",
            AlertCategory::ScheduledTask => "scheduled_task",
            AlertCategory::Memory => "memory",
            AlertCategory::Quality => "quality",
            AlertCategory::SelfEvaluation => "self_evaluation",
            AlertCategory::General => "general",
        }
    }
}

/// A proactive alert item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProactiveAlert {
    /// Unique alert ID
    pub id: String,
    /// Alert category
    pub category: AlertCategory,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert title
    pub title: String,
    /// Alert message/body
    pub message: String,
    /// Session ID this alert is related to (if applicable)
    pub session_id: Option<String>,
    /// Additional data associated with this alert (JSON)
    pub data: serde_json::Value,
    /// When this alert was created (Unix timestamp)
    pub created_at: u64,
    /// Whether this alert has been acknowledged by the user
    pub acknowledged: bool,
    /// Auto-dismiss after being acknowledged (in seconds)
    pub auto_dismiss_secs: Option<u64>,
}

impl ProactiveAlert {
    /// Create a new alert
    #[allow(dead_code)]
    pub fn new(
        category: AlertCategory,
        severity: AlertSeverity,
        title: &str,
        message: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category,
            severity,
            title: title.to_string(),
            message: message.to_string(),
            session_id: None,
            data: serde_json::Value::Null,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            acknowledged: false,
            auto_dismiss_secs: None,
        }
    }

    /// Create with session ID
    #[allow(dead_code)]
    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Create with additional data
    #[allow(dead_code)]
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Create with auto-dismiss
    #[allow(dead_code)]
    pub fn with_auto_dismiss(mut self, secs: u64) -> Self {
        self.auto_dismiss_secs = Some(secs);
        self
    }
}

/// Alert rule - defines conditions for triggering alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AlertRule {
    /// Unique rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Alert category this rule monitors
    pub category: AlertCategory,
    /// Minimum severity to trigger
    pub min_severity: AlertSeverity,
    /// Whether this rule is enabled
    pub enabled: bool,
    /// Cooldown period in seconds (prevent alert flooding)
    pub cooldown_secs: u64,
    /// Last alert time for this rule (Unix timestamp)
    #[serde(default)]
    pub last_alert_at: Option<u64>,
}

impl AlertRule {
    /// Create a new alert rule
    #[allow(dead_code)]
    pub fn new(name: &str, category: AlertCategory, min_severity: AlertSeverity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            category,
            min_severity,
            enabled: true,
            cooldown_secs: 300, // 5 minutes default cooldown
            last_alert_at: None,
        }
    }

    /// Check if this rule should trigger (respecting cooldown)
    #[allow(dead_code)]
    pub fn should_trigger(&self, severity: AlertSeverity, current_time: u64) -> bool {
        if !self.enabled {
            return false;
        }
        if severity < self.min_severity {
            return false;
        }
        if let Some(last) = self.last_alert_at {
            if current_time - last < self.cooldown_secs {
                return false;
            }
        }
        true
    }

    /// Record that this rule triggered an alert
    #[allow(dead_code)]
    pub fn record_alert(&mut self, current_time: u64) {
        self.last_alert_at = Some(current_time);
    }
}

/// Alert statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProactiveAlertStats {
    /// Total alerts generated
    pub total_alerts: u64,
    /// Alerts by severity
    pub by_severity: HashMap<String, u64>,
    /// Alerts by category
    pub by_category: HashMap<String, u64>,
    /// Last alert time
    pub last_alert_at: Option<u64>,
    /// Currently active (unacknowledged) alerts
    pub active_count: u64,
}

impl ProactiveAlertStats {
    /// Create new stats
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an alert
    #[allow(dead_code)]
    pub fn record_alert(&mut self, alert: &ProactiveAlert) {
        self.total_alerts += 1;
        *self.by_severity.entry(alert.severity.as_str().to_string()).or_insert(0) += 1;
        *self.by_category.entry(alert.category.as_str().to_string()).or_insert(0) += 1;
        self.last_alert_at = Some(alert.created_at);
    }

    /// Update active count
    #[allow(dead_code)]
    pub fn set_active_count(&mut self, count: u64) {
        self.active_count = count;
    }
}

/// Proactive Alert Manager - manages alert rules and alert history
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProactiveAlertManager {
    /// Alert rules by category
    rules: HashMap<AlertCategory, Vec<AlertRule>>,
    /// Recent alerts (kept in memory, max size)
    recent_alerts: Vec<ProactiveAlert>,
    /// Maximum recent alerts to keep
    max_recent_alerts: usize,
    /// Statistics
    stats: ProactiveAlertStats,
    /// Channel to send alerts to event system
    alert_sender: Option<tokio::sync::broadcast::Sender<ProactiveAlert>>,
}

impl ProactiveAlertManager {
    /// Create a new alert manager
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut manager = Self {
            rules: HashMap::new(),
            recent_alerts: Vec::new(),
            max_recent_alerts: 100,
            stats: ProactiveAlertStats::new(),
            alert_sender: None,
        };
        manager.init_default_rules();
        manager
    }

    /// Initialize with default alert rules
    fn init_default_rules(&mut self) {
        // Context health rules
        self.add_rule(AlertRule::new("Context Emergency", AlertCategory::ContextHealth, AlertSeverity::Emergency));
        self.add_rule(AlertRule::new("Context Critical", AlertCategory::ContextHealth, AlertSeverity::Critical));
        self.add_rule(AlertRule::new("Context Warning", AlertCategory::ContextHealth, AlertSeverity::Warning));

        // Feedback trend rules
        self.add_rule(AlertRule::new("Feedback Declining", AlertCategory::FeedbackTrend, AlertSeverity::Warning));
        self.add_rule(AlertRule::new("Feedback Critical", AlertCategory::FeedbackTrend, AlertSeverity::Critical));

        // Safety rules
        self.add_rule(AlertRule::new("Safety Warning", AlertCategory::Safety, AlertSeverity::Warning));
        self.add_rule(AlertRule::new("Safety Halted", AlertCategory::Safety, AlertSeverity::Critical));

        // Quality rules
        self.add_rule(AlertRule::new("Quality Warning", AlertCategory::Quality, AlertSeverity::Warning));
        self.add_rule(AlertRule::new("Quality Critical", AlertCategory::Quality, AlertSeverity::Critical));

        // General rules
        self.add_rule(AlertRule::new("General Info", AlertCategory::General, AlertSeverity::Info));
    }

    /// Add an alert rule
    #[allow(dead_code)]
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules
            .entry(rule.category)
            .or_default()
            .push(rule);
    }

    /// Set the alert sender (for SSE integration)
    #[allow(dead_code)]
    pub fn set_alert_sender(&mut self, sender: tokio::sync::broadcast::Sender<ProactiveAlert>) {
        self.alert_sender = Some(sender);
    }

    /// Generate an alert based on conditions
    #[allow(dead_code)]
    pub fn generate_alert(
        &mut self,
        category: AlertCategory,
        severity: AlertSeverity,
        title: &str,
        message: &str,
    ) -> Option<ProactiveAlert> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if any rule should trigger
        let should_trigger = self
            .rules
            .get(&category)
            .map(|rules| rules.iter().any(|r| r.should_trigger(severity, current_time)))
            .unwrap_or(false);

        if !should_trigger {
            return None;
        }

        // Create the alert
        let alert = ProactiveAlert::new(category, severity, title, message);

        // Record that a rule triggered
        if let Some(rules) = self.rules.get_mut(&category) {
            for rule in rules.iter_mut() {
                if rule.should_trigger(severity, current_time) {
                    rule.record_alert(current_time);
                    break; // Only record for first matching rule
                }
            }
        }

        // Add to recent alerts
        self.recent_alerts.insert(0, alert.clone());
        if self.recent_alerts.len() > self.max_recent_alerts {
            self.recent_alerts.pop();
        }

        // Update stats
        self.stats.record_alert(&alert);

        // Send to event system if sender is set
        if let Some(ref sender) = self.alert_sender {
            let _ = sender.send(alert.clone());
        }

        Some(alert)
    }

    /// Generate a context health alert
    #[allow(dead_code)]
    pub fn alert_context_health(
        &mut self,
        session_id: &str,
        health_level: &str,
        health_score: u8,
        utilization_pct: f32,
    ) -> Option<ProactiveAlert> {
        let severity = match health_level {
            "emergency" => AlertSeverity::Emergency,
            "critical" => AlertSeverity::Critical,
            "warning" => AlertSeverity::Warning,
            _ => return None,
        };

        let title = format!("Context Health: {}", health_level.to_uppercase());
        let message = format!(
            "Session {} context health is {} (score: {}/100, utilization: {:.1}%)",
            &session_id[..8.min(session_id.len())],
            health_level,
            health_score,
            utilization_pct
        );

        let alert = self.generate_alert(AlertCategory::ContextHealth, severity, &title, &message);
        alert.map(|a| a.with_session(session_id))
    }

    /// Generate a feedback trend alert
    #[allow(dead_code)]
    pub fn alert_feedback_trend(
        &mut self,
        session_id: &str,
        trend_direction: &str,
        trend_strength: f32,
    ) -> Option<ProactiveAlert> {
        if trend_direction != "declining" {
            return None;
        }

        let severity = if trend_strength > 0.7 {
            AlertSeverity::Critical
        } else {
            AlertSeverity::Warning
        };

        let title = "Feedback Trend Declining".to_string();
        let message = format!(
            "User feedback quality is declining (strength: {:.0}%)",
            trend_strength * 100.0
        );

        let alert = self.generate_alert(AlertCategory::FeedbackTrend, severity, &title, &message);
        alert.map(|a| a.with_session(session_id))
    }

    /// Generate a safety alert
    #[allow(dead_code)]
    pub fn alert_safety_event(
        &mut self,
        session_id: &str,
        event_type: &str,
        message: &str,
    ) -> Option<ProactiveAlert> {
        let severity = match event_type {
            "halted" => AlertSeverity::Critical,
            "warning" => AlertSeverity::Warning,
            _ => AlertSeverity::Info,
        };

        let title = format!("Safety Event: {}", event_type.to_uppercase());

        let alert = self.generate_alert(AlertCategory::Safety, severity, &title, message);
        alert.map(|a| a.with_session(session_id))
    }

    /// Generate a quality alert
    #[allow(dead_code)]
    pub fn alert_quality(
        &mut self,
        session_id: &str,
        quality_score: f32,
        issues: usize,
    ) -> Option<ProactiveAlert> {
        if issues == 0 {
            return None;
        }

        let severity = if quality_score < 0.4 {
            AlertSeverity::Critical
        } else if quality_score < 0.6 {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        };

        let title = "Session Quality Issues Detected".to_string();
        let message = format!(
            "Session {} has {} quality issue(s) (score: {:.0}/100)",
            &session_id[..8.min(session_id.len())],
            issues,
            quality_score * 100.0
        );

        let alert = self.generate_alert(AlertCategory::Quality, severity, &title, &message);
        alert.map(|a| a.with_session(session_id))
    }

    /// Acknowledge an alert
    #[allow(dead_code)]
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        if let Some(alert) = self.recent_alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            self.stats.set_active_count(self.recent_alerts.iter().filter(|a| !a.acknowledged).count() as u64);
            true
        } else {
            false
        }
    }

    /// Get unacknowledged alerts
    #[allow(dead_code)]
    pub fn get_active_alerts(&self) -> Vec<ProactiveAlert> {
        self.recent_alerts
            .iter()
            .filter(|a| !a.acknowledged)
            .cloned()
            .collect()
    }

    /// Get recent alerts (last N)
    #[allow(dead_code)]
    pub fn get_recent_alerts(&self, limit: usize) -> Vec<ProactiveAlert> {
        self.recent_alerts
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get statistics
    #[allow(dead_code)]
    pub fn get_stats(&self) -> ProactiveAlertStats {
        let mut stats = self.stats.clone();
        stats.set_active_count(self.recent_alerts.iter().filter(|a| !a.acknowledged).count() as u64);
        stats
    }

    /// Update rule configuration
    #[allow(dead_code)]
    pub fn update_rule(&mut self, category: AlertCategory, rule_name: &str, enabled: Option<bool>, cooldown_secs: Option<u64>) -> bool {
        if let Some(rules) = self.rules.get_mut(&category) {
            for rule in rules.iter_mut() {
                if rule.name.to_lowercase() == rule_name.to_lowercase() {
                    if let Some(e) = enabled {
                        rule.enabled = e;
                    }
                    if let Some(c) = cooldown_secs {
                        rule.cooldown_secs = c;
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Get rules for a category
    #[allow(dead_code)]
    pub fn get_rules(&self, category: Option<AlertCategory>) -> Vec<AlertRule> {
        match category {
            Some(cat) => self.rules.get(&cat).cloned().unwrap_or_default(),
            None => self.rules.values().flatten().cloned().collect(),
        }
    }

    /// Clear all alerts
    #[allow(dead_code)]
    pub fn clear_alerts(&mut self) {
        self.recent_alerts.clear();
        self.stats.set_active_count(0);
    }
}

impl Default for ProactiveAlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = ProactiveAlert::new(
            AlertCategory::ContextHealth,
            AlertSeverity::Warning,
            "Test Alert",
            "This is a test",
        );
        assert_eq!(alert.category, AlertCategory::ContextHealth);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn test_alert_with_session() {
        let alert = ProactiveAlert::new(
            AlertCategory::General,
            AlertSeverity::Info,
            "Test",
            "Test",
        )
        .with_session("test-session-123");

        assert_eq!(alert.session_id, Some("test-session-123".to_string()));
    }

    #[test]
    fn test_alert_rule_cooldown() {
        let rule = AlertRule::new("Test", AlertCategory::General, AlertSeverity::Warning);
        let current_time = 1000;

        // First trigger should work
        assert!(rule.should_trigger(AlertSeverity::Warning, current_time));

        // Create a mutable copy to test cooldown
        let mut rule2 = AlertRule::new("Test2", AlertCategory::General, AlertSeverity::Warning);
        rule2.record_alert(current_time);

        // Within cooldown should not trigger
        assert!(!rule2.should_trigger(AlertSeverity::Warning, current_time + 100));

        // After cooldown should trigger again
        assert!(rule2.should_trigger(AlertSeverity::Warning, current_time + 400));
    }

    #[test]
    fn test_alert_manager_default_rules() {
        let manager = ProactiveAlertManager::new();
        let rules = manager.get_rules(Some(AlertCategory::ContextHealth));
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_alert_context_health() {
        let mut manager = ProactiveAlertManager::new();
        manager.clear_alerts();

        // Should generate alert for critical
        let alert = manager.alert_context_health("session-123", "critical", 30, 85.0);
        assert!(alert.is_some());

        // Should not generate alert for healthy
        let alert2 = manager.alert_context_health("session-123", "healthy", 90, 50.0);
        assert!(alert2.is_none());
    }

    #[test]
    fn test_acknowledge_alert() {
        let mut manager = ProactiveAlertManager::new();
        manager.clear_alerts();

        // Generate an alert
        manager.alert_context_health("session-123", "warning", 50, 75.0);

        let active = manager.get_active_alerts();
        assert_eq!(active.len(), 1);

        // Acknowledge it
        let alert_id = &active[0].id;
        assert!(manager.acknowledge_alert(alert_id));

        // Should be no more active alerts
        let active2 = manager.get_active_alerts();
        assert!(active2.is_empty());
    }

    #[test]
    fn test_feedback_trend_alert() {
        let mut manager = ProactiveAlertManager::new();
        manager.clear_alerts();

        // Should generate alert for declining
        let alert = manager.alert_feedback_trend("session-123", "declining", 0.8);
        assert!(alert.is_some());

        // Should not generate alert for improving
        let alert2 = manager.alert_feedback_trend("session-123", "improving", 0.5);
        assert!(alert2.is_none());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AlertSeverity::Emergency > AlertSeverity::Critical);
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }
}