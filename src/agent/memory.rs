//! Agent Memory Module
//!
//! Long-term memory system that stores important facts extracted from conversations.
//! Facts are automatically extracted and retrieved to provide context continuity.
//!
//! Unlike Session Notes (user manually created), Memory is automatically extracted
//! by the Agent from conversations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;

/// Maximum facts to keep per category
const MAX_FACTS_PER_CATEGORY: usize = 50;
/// How long facts stay relevant (in seconds) - 30 days
const FACT_TTL_SECS: i64 = 30 * 24 * 3600;

/// Category of memory fact
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactCategory {
    /// User's personal information or preferences
    UserPreference,
    /// Important decisions or conclusions from conversation
    Decision,
    /// Technical facts about projects or code
    Technical,
    /// Project context or current work status
    ProjectContext,
    /// Meeting notes or action items
    ActionItem,
    /// General knowledge extracted from conversation
    General,
}

impl FactCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FactCategory::UserPreference => "user_preference",
            FactCategory::Decision => "decision",
            FactCategory::Technical => "technical",
            FactCategory::ProjectContext => "project_context",
            FactCategory::ActionItem => "action_item",
            FactCategory::General => "general",
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "user_preference" => FactCategory::UserPreference,
            "decision" => FactCategory::Decision,
            "technical" => FactCategory::Technical,
            "project_context" => FactCategory::ProjectContext,
            "action_item" => FactCategory::ActionItem,
            _ => FactCategory::General,
        }
    }
}

/// A single memory fact extracted from conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    /// Unique ID for this fact
    pub id: String,
    /// The fact content
    pub content: String,
    /// Category of the fact
    pub category: FactCategory,
    /// Session ID this fact was extracted from
    pub source_session: String,
    /// When this fact was created
    pub created_at: DateTime<Utc>,
    /// When this fact expires
    pub expires_at: DateTime<Utc>,
    /// Importance score (0.0 - 1.0), higher = more important
    pub importance: f32,
    /// Keywords for retrieval
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl MemoryFact {
    /// Create a new memory fact
    pub fn new(
        content: String,
        category: FactCategory,
        source_session: String,
        importance: f32,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let keywords = Self::extract_keywords(&content);
        
        Self {
            id,
            content,
            category,
            source_session,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(FACT_TTL_SECS),
            importance: importance.clamp(0.0, 1.0),
            keywords,
        }
    }

    /// Extract keywords from content for retrieval
    fn extract_keywords(content: &str) -> Vec<String> {
        // Extract words longer than 3 characters, lowercase
        let words: Vec<String> = content
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase())
            .collect();
        
        // Deduplicate and limit
        let unique: Vec<String> = words.into_iter().collect::<std::collections::HashSet<_>>()
            .into_iter().take(10).collect();
        unique
    }

    /// Check if this fact has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if this fact matches a query (keyword search)
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        // Check against content - exact phrase match
        if self.content.to_lowercase().contains(&query_lower) {
            return true;
        }
        
        // Check against keywords - all query words must match at least one keyword
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return false;
        }
        
        // For each query word, check if it matches any keyword
        for query_word in &query_words {
            let word_matches = self.keywords.iter().any(|keyword| {
                keyword == query_word || keyword.contains(query_word) || query_word.contains(keyword)
            });
            if !word_matches {
                // This query word doesn't match any keyword - fail
                return false;
            }
        }
        
        // All query words matched
        true
    }

    /// Generate a prompt addition for this fact
    pub fn to_prompt(&self) -> String {
        format!(
            "[{}] {}",
            self.category.as_str().replace('_', " "),
            self.content
        )
    }
}

/// Summary of a memory fact for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFactSummary {
    pub id: String,
    pub content: String,
    pub category: String,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
}

impl From<&MemoryFact> for MemoryFactSummary {
    fn from(fact: &MemoryFact) -> Self {
        Self {
            id: fact.id.clone(),
            content: fact.content.clone(),
            category: fact.category.as_str().to_string(),
            importance: fact.importance,
            created_at: fact.created_at,
        }
    }
}

/// Manager for agent long-term memory
pub struct MemoryManager {
    /// All stored facts, organized by category
    facts: RwLock<HashMap<FactCategory, Vec<MemoryFact>>>,
    /// Base path for persistence
    base_path: PathBuf,
}

impl MemoryManager {
    /// Create a new memory manager with default path
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("memory");
        Self::with_path(path)
    }

    /// Create a manager with custom path
    #[allow(dead_code)]
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let manager = Self {
            facts: RwLock::new(HashMap::new()),
            base_path: path,
        };
        manager.load();
        manager
    }

    /// Load facts from disk
    fn load(&self) {
        if !self.base_path.exists() {
            return;
        }

        for category in [
            FactCategory::UserPreference,
            FactCategory::Decision,
            FactCategory::Technical,
            FactCategory::ProjectContext,
            FactCategory::ActionItem,
            FactCategory::General,
        ] {
            let path = self.base_path.join(format!("{}.json", category.as_str()));
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(facts) = serde_json::from_str::<Vec<MemoryFact>>(&content) {
                        // Filter out expired facts
                        let valid: Vec<_> = facts.into_iter().filter(|f| !f.is_expired()).collect();
                        self.facts.write().insert(category, valid);
                    }
                }
            }
        }

        tracing::info!("Loaded memory manager from {:?}", self.base_path);
    }

    /// Save facts for a category to disk
    fn save_category(&self, category: &FactCategory) {
        let facts = self.facts.read();
        if let Some(category_facts) = facts.get(category) {
            if let Ok(content) = serde_json::to_string_pretty(category_facts) {
                let path = self.base_path.join(format!("{}.json", category.as_str()));
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, content);
            }
        }
    }

    /// Add a new fact to memory
    pub fn add_fact(&self, fact: MemoryFact) {
        let mut facts = self.facts.write();
        let category = fact.category.clone();
        
        // Get or create the category vec
        let category_facts = facts.entry(category.clone()).or_default();
        
        // Add the new fact
        category_facts.push(fact);
        
        // Sort by importance (descending) and limit size
        category_facts.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        category_facts.truncate(MAX_FACTS_PER_CATEGORY);
        
        drop(facts);
        
        // Persist
        self.save_category(&category);
    }

    /// Add multiple facts at once
    pub fn add_facts(&self, new_facts: Vec<MemoryFact>) {
        // Group by category
        let mut by_category: HashMap<FactCategory, Vec<MemoryFact>> = HashMap::new();
        for fact in new_facts {
            by_category.entry(fact.category.clone()).or_default().push(fact);
        }
        
        // Add each category
        for (category, mut facts) in by_category {
            let mut all_facts = self.facts.write();
            let category_facts = all_facts.entry(category.clone()).or_default();
            category_facts.append(&mut facts);
            
            // Sort and limit
            category_facts.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
            category_facts.truncate(MAX_FACTS_PER_CATEGORY);
            
            // Persist each category after modifying
            drop(all_facts);
            self.save_category(&category);
        }
        
        // Note: all_facts guard is dropped at end of each iteration
    }

    /// Search for facts matching a query
    pub fn search(&self, query: &str) -> Vec<MemoryFact> {
        let facts = self.facts.read();
        let mut results: Vec<&MemoryFact> = Vec::new();
        
        for category_facts in facts.values() {
            for fact in category_facts {
                if !fact.is_expired() && fact.matches_query(query) {
                    results.push(fact);
                }
            }
        }
        
        // Sort by importance
        results.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        
        // Limit results
        results.truncate(20);
        
        results.into_iter().cloned().collect()
    }

    /// Get all facts for a specific category
    #[allow(dead_code)]
    pub fn get_by_category(&self, category: &FactCategory) -> Vec<MemoryFact> {
        let facts = self.facts.read();
        facts.get(category)
            .map(|v| v.iter().filter(|f| !f.is_expired()).cloned().collect())
            .unwrap_or_default()
    }

    /// Get facts relevant to a session (same session or related topics)
    pub fn get_for_session(&self, session_id: &str, limit: usize) -> Vec<MemoryFact> {
        let facts = self.facts.read();
        let mut results: Vec<&MemoryFact> = Vec::new();
        
        for category_facts in facts.values() {
            for fact in category_facts {
                if !fact.is_expired() {
                    // Include facts from the same session or high importance facts
                    if fact.source_session == session_id || fact.importance >= 0.8 {
                        results.push(fact);
                    }
                }
            }
        }
        
        // Sort by importance and recency
        results.sort_by(|a, b| {
            let a_score = a.importance + (a.created_at.timestamp() as f32 / 1_000_000.0);
            let b_score = b.importance + (b.created_at.timestamp() as f32 / 1_000_000.0);
            b_score.partial_cmp(&a_score).unwrap()
        });
        
        results.truncate(limit);
        results.into_iter().cloned().collect()
    }

    /// Get all facts as summaries for API responses
    pub fn list_all(&self) -> Vec<MemoryFactSummary> {
        let facts = self.facts.read();
        let mut all: Vec<_> = Vec::new();
        
        for category_facts in facts.values() {
            for fact in category_facts {
                if !fact.is_expired() {
                    all.push(MemoryFactSummary::from(fact));
                }
            }
        }
        
        // Sort by category and importance
        all.sort_by(|a, b| {
            a.category.cmp(&b.category)
                .then(b.importance.partial_cmp(&a.importance).unwrap())
        });
        
        all
    }

    /// Delete a fact by ID
    #[allow(dead_code)]
    pub fn delete_fact(&self, fact_id: &str) -> bool {
        let categories_to_save: Vec<FactCategory>;
        let mut found = false;
        
        {
            let mut facts = self.facts.write();
            let mut to_save: Vec<FactCategory> = Vec::new();
            
            for (category, category_facts) in facts.iter_mut() {
                if let Some(pos) = category_facts.iter().position(|f| f.id == fact_id) {
                    category_facts.remove(pos);
                    found = true;
                    to_save.push(category.clone());
                    break;
                }
            }
            
            categories_to_save = to_save;
        }
        
        for category in categories_to_save {
            self.save_category(&category);
        }
        
        found
    }

    /// Clear all facts in a category
    #[allow(dead_code)]
    pub fn clear_category(&self, category: &FactCategory) {
        self.facts.write().insert(category.clone(), Vec::new());
        self.save_category(category);
    }

    /// Clean up expired facts (can be called periodically)
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) {
        let categories_to_save: Vec<FactCategory>;
        
        {
            let mut facts = self.facts.write();
            let mut to_save: Vec<FactCategory> = Vec::new();
            
            for (category, category_facts) in facts.iter_mut() {
                let original_len = category_facts.len();
                category_facts.retain(|f| !f.is_expired());
                if category_facts.len() != original_len {
                    to_save.push(category.clone());
                }
            }
            
            categories_to_save = to_save;
        }
        
        for category in categories_to_save {
            self.save_category(&category);
        }
    }

    /// Count total facts
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        let facts = self.facts.read();
        facts.values().map(|v| v.iter().filter(|f| !f.is_expired()).count()).sum()
    }

    /// Generate a context prompt from relevant memories
    pub fn generate_context_prompt(&self, query: &str, max_facts: usize) -> String {
        let facts = self.search(query);
        if facts.is_empty() {
            return String::new();
        }
        
        let relevant: Vec<_> = facts.into_iter().take(max_facts).collect();
        let lines: Vec<String> = relevant.iter().map(|f| f.to_prompt()).collect();
        
        format!(
            "\n\n## Relevant Memory\n{}\n",
            lines.join("\n")
        )
    }

    /// Generate a context prompt from session memories
    pub fn generate_session_prompt(&self, session_id: &str, max_facts: usize) -> String {
        let facts = self.get_for_session(session_id, max_facts);
        if facts.is_empty() {
            return String::new();
        }
        
        let lines: Vec<String> = facts.iter().map(|f| f.to_prompt()).collect();
        
        format!(
            "\n\n## Session Memory\n{}\n",
            lines.join("\n")
        )
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Clear test memory before each test to avoid stale data
    fn setup_test_memory() {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tiny_claw")
            .join("memory");
        if path.exists() {
            let _ = std::fs::remove_dir_all(&path);
        }
    }

    fn create_test_fact() -> MemoryFact {
        MemoryFact::new(
            "User prefers dark mode".to_string(),
            FactCategory::UserPreference,
            "session1".to_string(),
            0.8,
        )
    }

    #[test]
    fn test_fact_creation() {
        let fact = create_test_fact();
        assert_eq!(fact.content, "User prefers dark mode");
        assert_eq!(fact.category, FactCategory::UserPreference);
        assert!(!fact.is_expired());
        assert!(!fact.keywords.is_empty());
    }

    #[test]
    fn test_fact_matches_query() {
        let fact = create_test_fact();
        assert!(fact.matches_query("dark mode"));
        assert!(fact.matches_query("DARK"));
        assert!(fact.matches_query("prefers"));
        assert!(!fact.matches_query("light mode"));
    }

    #[test]
    fn test_fact_to_prompt() {
        let fact = create_test_fact();
        let prompt = fact.to_prompt();
        assert!(prompt.contains("dark mode"));
        // to_prompt replaces underscores with spaces
        assert!(prompt.contains("user preference"));
    }

    #[test]
    fn test_memory_manager_add_and_search() {
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        manager.add_fact(fact);
        
        let results = manager.search("dark mode");
        assert!(!results.is_empty());
        assert!(results[0].content.contains("dark mode"));
    }

    #[test]
    fn test_memory_manager_search_no_match() {
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        manager.add_fact(fact);
        
        let results = manager.search("light mode");
        assert!(results.is_empty());
    }

    #[test]
    fn test_memory_manager_get_for_session() {
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        manager.add_fact(fact);
        
        let session_facts = manager.get_for_session("session1", 10);
        assert!(!session_facts.is_empty());
        
        let other_session = manager.get_for_session("other_session", 10);
        // Should only have high importance facts
        assert!(other_session.iter().all(|f| f.importance >= 0.8));
    }

    #[test]
    fn test_memory_manager_delete_fact() {
        setup_test_memory();
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        let id = fact.id.clone();
        manager.add_fact(fact);
        
        assert!(manager.delete_fact(&id));
        assert!(manager.search("dark mode").is_empty());
        
        // Deleting non-existent should return false
        assert!(!manager.delete_fact("non-existent-id"));
    }

    #[test]
    fn test_memory_manager_generate_context_prompt() {
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        manager.add_fact(fact);
        
        let prompt = manager.generate_context_prompt("dark mode", 5);
        assert!(!prompt.is_empty());
        assert!(prompt.contains("dark mode"));
        assert!(prompt.contains("Memory"));
    }

    #[test]
    fn test_memory_manager_generate_session_prompt() {
        let manager = MemoryManager::new();
        let fact = create_test_fact();
        manager.add_fact(fact);
        
        let prompt = manager.generate_session_prompt("session1", 5);
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Session Memory"));
    }

    #[test]
    fn test_fact_category_serialization() {
        let cat = FactCategory::Technical;
        let serialized = cat.as_str();
        assert_eq!(serialized, "technical");
        
        let deserialized = FactCategory::from_str("technical");
        assert_eq!(deserialized, FactCategory::Technical);
    }

    #[test]
    fn test_list_all() {
        setup_test_memory();
        let manager = MemoryManager::new();
        
        let fact1 = MemoryFact::new(
            "Fact 1".to_string(),
            FactCategory::UserPreference,
            "session1".to_string(),
            0.5,
        );
        let fact2 = MemoryFact::new(
            "Fact 2".to_string(),
            FactCategory::Technical,
            "session1".to_string(),
            0.9,
        );
        
        manager.add_fact(fact1);
        manager.add_fact(fact2);
        
        let all = manager.list_all();
        assert_eq!(all.len(), 2);
    }
}
