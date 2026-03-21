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
        let skills = sessions.entry(session_id.to_string()).or_insert_with(HashSet::new);
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

    /// Get active skills as full Skill objects
    pub fn get_active_skill_objects(&self, session_id: &str) -> Vec<crate::agent::skill::Skill> {
        self.get_active_skills(session_id)
            .iter()
            .filter_map(|name| self.skill_registry.get(name))
            .collect()
    }

    /// Check if a skill is active for a session
    pub fn is_skill_active(&self, session_id: &str, skill_name: &str) -> bool {
        let sessions = self.session_skills.read();
        sessions
            .get(session_id)
            .map(|s| s.contains(skill_name))
            .unwrap_or(false)
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

    /// Clear all skills for a session
    pub fn clear_session_skills(&self, session_id: &str) {
        let mut sessions = self.session_skills.write();
        sessions.remove(session_id);
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

    /// Get all sessions with a specific skill active
    pub fn get_sessions_with_skill(&self, skill_name: &str) -> Vec<String> {
        let sessions = self.session_skills.read();
        sessions
            .iter()
            .filter(|(_, skills)| skills.contains(skill_name))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Generate system prompt supplement from active skills for a session
    pub fn generate_session_skill_prompt(&self, session_id: &str) -> Option<String> {
        let active_skills = self.get_active_skills(session_id);
        self.skill_registry.generate_skill_prompt(&active_skills)
    }

    /// Remove a skill from all sessions (when a skill is deleted)
    pub fn remove_skill_from_all(&self, skill_name: &str) {
        let mut sessions = self.session_skills.write();
        for skills in sessions.values_mut() {
            skills.remove(skill_name);
        }
    }

    /// Get count of sessions with skills active
    pub fn active_session_count(&self) -> usize {
        let sessions = self.session_skills.read();
        sessions.len()
    }

    /// Get all session IDs with their active skill counts
    pub fn get_session_skill_counts(&self) -> Vec<(String, usize)> {
        let sessions = self.session_skills.read();
        sessions
            .iter()
            .map(|(id, skills)| (id.clone(), skills.len()))
            .collect()
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

        // Check if active
        assert!(manager.is_skill_active("session1", "file_ops"));
        assert!(!manager.is_skill_active("session1", "nonexistent"));
    }

    #[test]
    fn test_disable_skill() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        let result = manager.disable_skill("session1", "file_ops");
        assert!(result);

        assert!(!manager.is_skill_active("session1", "file_ops"));
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
    fn test_clear_session_skills() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        manager.clear_session_skills("session1");

        let skills = manager.get_active_skills("session1");
        assert!(skills.is_empty());
    }

    #[test]
    fn test_generate_session_skill_prompt() {
        let registry = create_test_registry();
        let manager = SessionSkillManager::new(registry);

        manager.enable_skill("session1", "file_ops");
        let prompt = manager.generate_session_skill_prompt("session1");

        assert!(prompt.is_some());
        let prompt_str = prompt.unwrap();
        assert!(prompt_str.contains("file_ops"));
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
