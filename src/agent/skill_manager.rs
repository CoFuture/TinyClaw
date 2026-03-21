//! Session Skill Manager Module
//!
//! Manages which skills are active for each session.
//! Skills can be enabled/disabled per-session.

use crate::agent::skill_registry::SkillRegistry;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

/// Manages active skills per session
pub struct SessionSkillManager {
    /// Map of session_id -> set of active skill names
    session_skills: RwLock<std::collections::HashMap<String, HashSet<String>>>,
    /// Reference to skill registry
    skill_registry: Arc<SkillRegistry>,
}

impl SessionSkillManager {
    /// Create a new session skill manager
    pub fn new(skill_registry: Arc<SkillRegistry>) -> Self {
        Self {
            session_skills: RwLock::new(std::collections::HashMap::new()),
            skill_registry,
        }
    }

    /// Enable a skill for a session
    /// Returns true if the skill was newly enabled, false if already enabled
    pub fn enable_skill(&self, session_id: &str, skill_name: &str) -> bool {
        // Verify skill exists
        if !self.skill_registry.exists(skill_name) {
            return false;
        }

        let mut sessions = self.session_skills.write();
        let skills = sessions.entry(session_id.to_string()).or_default();
        skills.insert(skill_name.to_string())
    }

    /// Disable a skill for a session
    /// Returns true if the skill was removed, false if not active
    pub fn disable_skill(&self, session_id: &str, skill_name: &str) -> bool {
        let mut sessions = self.session_skills.write();
        if let Some(skills) = sessions.get_mut(session_id) {
            skills.remove(skill_name)
        } else {
            false
        }
    }

    /// Get all active skills for a session
    pub fn get_active_skills(&self, session_id: &str) -> Vec<String> {
        let sessions = self.session_skills.read();
        sessions
            .get(session_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get a skill from the registry by name
    pub fn get_skill(&self, name: &str) -> Option<crate::agent::Skill> {
        self.skill_registry.get(name)
    }

    /// Set all active skills for a session (replaces existing)
    pub fn set_active_skills(&self, session_id: &str, skill_names: Vec<String>) {
        // Validate all skill names exist
        let valid_skills: Vec<String> = skill_names
            .into_iter()
            .filter(|name| self.skill_registry.exists(name))
            .collect();

        let mut sessions = self.session_skills.write();
        sessions.insert(session_id.to_string(), valid_skills.into_iter().collect());
    }

    /// Enable default skills for a new session
    /// Uses skills that have enabled_by_default = true
    pub fn enable_defaults_for_session(&self, session_id: &str) {
        let default_skills: Vec<String> = self.skill_registry.list()
            .into_iter()
            .filter(|s| s.enabled_by_default)
            .map(|s| s.name)
            .collect();

        if !default_skills.is_empty() {
            let mut sessions = self.session_skills.write();
            sessions.insert(session_id.to_string(), default_skills.into_iter().collect());
        }
    }

    /// Remove a skill from all sessions (when a skill is deleted)
    pub fn remove_skill_from_all(&self, skill_name: &str) {
        let mut sessions = self.session_skills.write();
        for skills in sessions.values_mut() {
            skills.remove(skill_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> Arc<SkillRegistry> {
        SkillRegistry::new()
    }

    #[test]
    fn test_enable_disable_skill() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        // Enable a skill
        let result = manager.enable_skill("session1", "file_ops");
        assert!(result);

        // Enable again (should return false - already enabled)
        let result = manager.enable_skill("session1", "file_ops");
        assert!(!result);

        // Check if active via get_active_skills
        let skills = manager.get_active_skills("session1");
        assert!(skills.contains(&"file_ops".to_string()));
        
        let skills_nonexistent = manager.get_active_skills("session1");
        assert!(!skills_nonexistent.contains(&"nonexistent".to_string()));
    }

    #[test]
    fn test_disable_skill() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        let result = manager.disable_skill("session1", "file_ops");
        assert!(result);

        let skills = manager.get_active_skills("session1");
        assert!(!skills.contains(&"file_ops".to_string()));
    }

    #[test]
    fn test_get_active_skills() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        manager.enable_skill("session1", "code_analysis");

        let skills = manager.get_active_skills("session1");
        assert_eq!(skills.len(), 2);
        assert!(skills.contains(&"file_ops".to_string()));
        assert!(skills.contains(&"code_analysis".to_string()));
    }

    #[test]
    fn test_set_active_skills() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        manager.set_active_skills("session1", vec!["code_analysis".to_string()]);

        let skills = manager.get_active_skills("session1");
        assert_eq!(skills.len(), 1);
        assert!(skills.contains(&"code_analysis".to_string()));
        assert!(!skills.contains(&"file_ops".to_string()));
    }

    #[test]
    fn test_enable_defaults() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        // file_ops and code_analysis have enabled_by_default = true
        manager.enable_defaults_for_session("session1");

        let skills = manager.get_active_skills("session1");
        assert!(skills.len() >= 2); // At least file_ops and code_analysis
        assert!(skills.contains(&"file_ops".to_string()));
        assert!(skills.contains(&"code_analysis".to_string()));
    }

    #[test]
    fn test_nonexistent_skill_enable() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        let result = manager.enable_skill("session1", "nonexistent_skill");
        assert!(!result);

        let skills = manager.get_active_skills("session1");
        assert!(skills.is_empty());
    }
}
