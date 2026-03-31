//! Skill Recommender Module
//!
//! Automatically recommends relevant skills based on conversation context.
//! Analyzes conversation history, detected topics, and tool usage patterns
//! to suggest skills that might be helpful for the current task.

use crate::agent::skill_registry::SkillRegistry;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A skill recommendation with confidence and reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecommendation {
    /// Unique recommendation ID
    pub id: String,
    /// Name of the recommended skill
    pub skill_name: String,
    /// Skill description
    pub description: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Reasons why this skill was recommended
    pub reasons: Vec<String>,
    /// Keywords that triggered this recommendation
    pub triggered_keywords: Vec<String>,
    /// Whether the skill is already enabled for this session
    pub already_enabled: bool,
}

impl SkillRecommendation {
    /// Create a new skill recommendation
    pub fn new(
        skill_name: String,
        description: String,
        confidence: f32,
        reasons: Vec<String>,
        triggered_keywords: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            skill_name,
            description,
            confidence: confidence.clamp(0.0, 1.0),
            reasons,
            triggered_keywords,
            already_enabled: false,
        }
    }
}

/// Skill recommender engine - analyzes context and generates skill recommendations
pub struct SkillRecommender {
    /// Reference to skill registry
    skill_registry: Arc<SkillRegistry>,
    /// Keyword to skill mapping for recommendations
    keyword_skill_map: HashMap<String, (String, f32)>,
    /// Topic patterns to skill mapping
    topic_skill_map: HashMap<String, (String, f32)>,
    /// Statistics (uses RwLock for interior mutability)
    stats: RwLock<HashMap<String, u64>>,
}

impl Clone for SkillRecommender {
    fn clone(&self) -> Self {
        Self {
            skill_registry: Arc::clone(&self.skill_registry),
            keyword_skill_map: self.keyword_skill_map.clone(),
            topic_skill_map: self.topic_skill_map.clone(),
            stats: RwLock::new(self.stats.read().clone()),
        }
    }
}

impl SkillRecommender {
    /// Create a new skill recommender
    pub fn new(skill_registry: Arc<SkillRegistry>) -> Self {
        let mut recommender = Self {
            skill_registry,
            keyword_skill_map: HashMap::new(),
            topic_skill_map: HashMap::new(),
            stats: RwLock::new(HashMap::new()),
        };
        recommender.initialize_keyword_mappings();
        recommender
    }

    /// Initialize keyword to skill mappings
    fn initialize_keyword_mappings(&mut self) {
        // File operations keywords -> file_ops
        let file_ops_keywords = vec![
            ("read file", "file_ops", 0.9),
            ("write file", "file_ops", 0.9),
            ("edit file", "file_ops", 0.9),
            ("create file", "file_ops", 0.9),
            ("delete file", "file_ops", 0.85),
            ("copy file", "file_ops", 0.85),
            ("move file", "file_ops", 0.85),
            ("file", "file_ops", 0.6),
            ("directory", "file_ops", 0.6),
            ("folder", "file_ops", 0.6),
            ("path", "file_ops", 0.5),
            ("mkdir", "file_ops", 0.8),
            ("cat", "file_ops", 0.7),
            ("ls", "file_ops", 0.6),
        ];
        for (keyword, skill, weight) in file_ops_keywords {
            self.keyword_skill_map.insert(keyword.to_lowercase(), (skill.to_string(), weight));
        }

        // Code analysis keywords -> code_analysis
        let code_keywords = vec![
            ("search code", "code_analysis", 0.9),
            ("find code", "code_analysis", 0.9),
            ("grep", "code_analysis", 0.85),
            ("find files", "code_analysis", 0.8),
            ("code", "code_analysis", 0.7),
            ("function", "code_analysis", 0.6),
            ("class", "code_analysis", 0.6),
            ("variable", "code_analysis", 0.5),
            ("import", "code_analysis", 0.5),
            ("search pattern", "code_analysis", 0.8),
            ("replace", "code_analysis", 0.6),
            ("regex", "code_analysis", 0.7),
            ("glob", "code_analysis", 0.7),
            ("tree", "code_analysis", 0.7),
            ("wc", "code_analysis", 0.6),
        ];
        for (keyword, skill, weight) in code_keywords {
            self.keyword_skill_map.insert(keyword.to_lowercase(), (skill.to_string(), weight));
        }

        // System operations keywords -> system_ops
        let system_keywords = vec![
            ("execute command", "system_ops", 0.9),
            ("run command", "system_ops", 0.9),
            ("shell", "system_ops", 0.8),
            ("bash", "system_ops", 0.8),
            ("zsh", "system_ops", 0.8),
            ("terminal", "system_ops", 0.7),
            ("process", "system_ops", 0.6),
            ("ps", "system_ops", 0.7),
            ("kill", "system_ops", 0.7),
            ("chmod", "system_ops", 0.8),
            ("environment", "system_ops", 0.6),
            ("env", "system_ops", 0.6),
            ("path", "system_ops", 0.5),
        ];
        for (keyword, skill, weight) in system_keywords {
            self.keyword_skill_map.insert(keyword.to_lowercase(), (skill.to_string(), weight));
        }

        // Web/HTTP keywords -> web_search
        let web_keywords = vec![
            ("http", "web_search", 0.9),
            ("web", "web_search", 0.8),
            ("api", "web_search", 0.7),
            ("fetch", "web_search", 0.7),
            ("request", "web_search", 0.6),
            ("url", "web_search", 0.7),
            ("endpoint", "web_search", 0.7),
            ("rest", "web_search", 0.7),
            ("json", "web_search", 0.6),
            ("http request", "web_search", 0.9),
        ];
        for (keyword, skill, weight) in web_keywords {
            self.keyword_skill_map.insert(keyword.to_lowercase(), (skill.to_string(), weight));
        }

        // Diff/comparison keywords -> diff_compare
        let diff_keywords = vec![
            ("compare", "diff_compare", 0.9),
            ("diff", "diff_compare", 0.9),
            ("difference", "diff_compare", 0.8),
            ("changes", "diff_compare", 0.7),
            ("hash", "diff_compare", 0.7),
            ("checksum", "diff_compare", 0.7),
            ("identical", "diff_compare", 0.6),
            ("same", "diff_compare", 0.5),
        ];
        for (keyword, skill, weight) in diff_keywords {
            self.keyword_skill_map.insert(keyword.to_lowercase(), (skill.to_string(), weight));
        }

        // Initialize topic patterns (multi-word patterns)
        let topic_patterns = vec![
            ("read the file", "file_ops", 0.95),
            ("write to file", "file_ops", 0.95),
            ("create a file", "file_ops", 0.9),
            ("list files", "file_ops", 0.85),
            ("search for", "code_analysis", 0.85),
            ("find files", "code_analysis", 0.85),
            ("search in", "code_analysis", 0.85),
            ("run a command", "system_ops", 0.9),
            ("execute a command", "system_ops", 0.9),
            ("make an http request", "web_search", 0.95),
            ("compare files", "diff_compare", 0.95),
            ("show differences", "diff_compare", 0.9),
        ];
        for (pattern, skill, weight) in topic_patterns {
            self.topic_skill_map.insert(pattern.to_lowercase(), (skill.to_string(), weight));
        }
    }

    /// Analyze conversation and generate skill recommendations
    /// Returns a list of recommended skills sorted by confidence
    pub fn recommend_skills(
        &self,
        history: &[(String, String)],
        enabled_skills: &[String],
    ) -> Vec<SkillRecommendation> {
        let mut skill_scores: HashMap<String, (f32, Vec<String>, Vec<String>)> = HashMap::new();
        let enabled_set: std::collections::HashSet<&str> = enabled_skills.iter().map(|s| s.as_str()).collect();

        // Analyze each message in history
        for (role, content) in history {
            if role == "system" {
                continue; // Skip system messages
            }
            let content_lower = content.to_lowercase();

            // Check topic patterns first (longer matches)
            for (pattern, (skill_name, weight)) in &self.topic_skill_map {
                if content_lower.contains(pattern) {
                    let entry = skill_scores.entry(skill_name.clone()).or_insert((0.0, Vec::new(), Vec::new()));
                    entry.0 += weight;
                    if !entry.1.contains(pattern) {
                        entry.1.push(pattern.to_string());
                    }
                    entry.2.push(pattern.to_string());
                }
            }

            // Check keyword matches
            for (keyword, (skill_name, weight)) in &self.keyword_skill_map {
                if content_lower.contains(keyword) {
                    let entry = skill_scores.entry(skill_name.clone()).or_insert((0.0, Vec::new(), Vec::new()));
                    entry.0 += weight;
                    if !entry.1.contains(keyword) {
                        entry.1.push(keyword.to_string());
                    }
                    entry.2.push(keyword.to_string());
                }
            }
        }

        // Convert scores to recommendations
        let mut recommendations: Vec<SkillRecommendation> = skill_scores
            .into_iter()
            .filter(|(skill_name, (score, _, _))| {
                // Only recommend if not already enabled and score is meaningful
                !enabled_set.contains(skill_name.as_str()) && *score > 0.5
            })
            .map(|(skill_name, (score, reasons, keywords))| {
                let skill_desc = self.skill_registry.get(&skill_name)
                    .map(|s| s.description.clone())
                    .unwrap_or_else(|| skill_name.clone());

                // Generate human-readable reasons
                let readable_reasons = Self::generate_reasons(&skill_name, &reasons, score);

                SkillRecommendation::new(
                    skill_name.clone(),
                    skill_desc,
                    (score / 3.0).min(1.0), // Normalize score
                    readable_reasons,
                    keywords,
                )
            })
            .collect();

        // Sort by confidence (highest first)
        recommendations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to top 3 recommendations
        recommendations.truncate(3);

        // Mark already enabled skills (for UI to show which are already on)
        let recommendations: Vec<_> = recommendations
            .into_iter()
            .map(|mut r| {
                r.already_enabled = enabled_skills.contains(&r.skill_name);
                r
            })
            .collect();

        // Update stats
        {
            let mut stats = self.stats.write();
            for r in &recommendations {
                *stats.entry(r.skill_name.clone()).or_insert(0) += 1;
            }
        }

        recommendations
    }

    /// Generate human-readable reasons for a recommendation
    fn generate_reasons(skill_name: &str, matched: &[String], score: f32) -> Vec<String> {
        let mut reasons = Vec::new();

        let skill_display = match skill_name {
            "file_ops" => "File Operations",
            "code_analysis" => "Code Analysis",
            "system_ops" => "System Operations",
            "web_search" => "Web Search",
            "diff_compare" => "Diff & Compare",
            _ => skill_name,
        };

        if matched.is_empty() {
            reasons.push(format!("{} might be helpful for this task", skill_display));
        } else if matched.len() == 1 {
            reasons.push(format!("Detected focus on '{}'", matched[0]));
        } else {
            reasons.push(format!("Detected {} related topics", matched.len()));
        }

        if score > 2.0 {
            reasons.push(format!("Strong indication of {} needs", skill_display.to_lowercase()));
        }

        reasons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_recommender() -> SkillRecommender {
        let registry = crate::agent::SkillRegistry::new();
        SkillRecommender::new(registry)
    }

    #[test]
    fn test_recommend_file_ops() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Read the file at /tmp/test.txt".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        assert!(!recommendations.is_empty());
        let rec = &recommendations[0];
        assert_eq!(rec.skill_name, "file_ops");
        assert!(rec.confidence > 0.0);
    }

    #[test]
    fn test_recommend_code_analysis() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Search for all functions named 'test' in the codebase".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        assert!(!recommendations.is_empty());
        let rec = &recommendations[0];
        assert_eq!(rec.skill_name, "code_analysis");
    }

    #[test]
    fn test_recommend_web_search() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Make an HTTP request to the API endpoint".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        assert!(!recommendations.is_empty());
        let rec = &recommendations[0];
        assert_eq!(rec.skill_name, "web_search");
    }

    #[test]
    fn test_already_enabled_not_recommended() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Read the file".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &["file_ops".to_string()]);
        
        // Should not recommend file_ops since it's already enabled
        assert!(recommendations.is_empty() || recommendations[0].skill_name != "file_ops");
    }

    #[test]
    fn test_multiple_recommendations() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Read the file and search for patterns in the code".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        // Should recommend at least file_ops or code_analysis
        assert!(recommendations.len() <= 3);
        for rec in &recommendations {
            assert!(["file_ops", "code_analysis"].contains(&rec.skill_name.as_str()));
        }
    }

    #[test]
    fn test_no_false_positives() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "Hello, how are you?".to_string()),
            ("assistant".to_string(), "I'm doing well, thank you!".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        // Generic conversation shouldn't trigger many recommendations
        assert!(recommendations.is_empty() || recommendations.iter().all(|r| r.confidence < 0.5));
    }

    #[test]
    fn test_system_message_ignored() {
        let recommender = create_test_recommender();
        let history = vec![
            ("system".to_string(), "You are a helpful assistant with file_ops enabled".to_string()),
            ("user".to_string(), "Hello".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        // System message should be ignored
        assert!(recommendations.is_empty());
    }

    #[test]
    fn test_keyword_matching_case_insensitive() {
        let recommender = create_test_recommender();
        let history = vec![
            ("user".to_string(), "READ THE FILE".to_string()),
        ];
        let recommendations = recommender.recommend_skills(&history, &[]);
        
        assert!(!recommendations.is_empty());
        assert_eq!(recommendations[0].skill_name, "file_ops");
    }
}
