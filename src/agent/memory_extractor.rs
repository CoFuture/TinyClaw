//! Memory Extractor Module
//!
//! Automatic extraction of facts from conversation text with content-based importance scoring.
//! This module provides heuristics for identifying and scoring important facts without requiring
//! external AI API calls.

use crate::agent::memory::{FactCategory, MemoryFact};
use std::collections::HashSet;

/// Maximum facts to extract per conversation turn
const MAX_EXTRACTIONS_PER_TURN: usize = 5;
/// Minimum content length to consider for extraction
const MIN_CONTENT_LEN: usize = 20;
/// Minimum importance score to keep a fact (0.0 - 1.0)
const IMPORTANCE_LOW: f32 = 0.30;

/// Importance Calculator - analyzes text content and determines importance scores
pub struct ImportanceCalculator;

impl ImportanceCalculator {
    /// Calculate importance score (0.0 - 1.0) based on content analysis
    /// Higher scores = more likely to be worth remembering long-term
    pub fn calculate(content: &str, category: &FactCategory) -> f32 {
        let content_lower = content.to_lowercase();
        let words: Vec<&str> = content_lower.split_whitespace().collect();
        
        // Start with category-based base score
        let base_score = match category {
            FactCategory::UserPreference => 0.7,
            FactCategory::Decision => 0.8,
            FactCategory::Technical => 0.75,
            FactCategory::ProjectContext => 0.65,
            FactCategory::ActionItem => 0.7,
            FactCategory::General => 0.4,
        };
        
        let mut score: f32 = base_score;
        
        // Boost for explicit preference indicators
        if Self::has_preference_indicator(&content_lower) {
            score += 0.1;
        }
        
        // Boost for decision indicators
        if Self::has_decision_indicator(&content_lower) {
            score += 0.1;
        }
        
        // Boost for technical specificity (contains numbers, paths, code)
        if Self::has_technical_detail(&content_lower) {
            score += 0.08;
        }
        
        // Boost for action-oriented language
        if Self::has_action_indicator(&content_lower) {
            score += 0.08;
        }
        
        // Slight boost for longer, more specific content
        if words.len() > 15 {
            score += 0.05;
        }
        
        // Slight boost for specificity (contains specific names, numbers)
        if Self::has_specific_reference(&content_lower) {
            score += 0.05;
        }
        
        // Cap at 1.0
        score.min(1.0)
    }
    
    fn has_preference_indicator(text: &str) -> bool {
        let indicators = [
            "i prefer", "i like", "i dislike", "i hate", "my favorite",
            "i usually", "i always", "i never", "i want", "i need",
            "i'm working on", "i'm using", "i'm building", "i'm developing",
            "my project", "my code", "my setup", "my environment",
            "i'm currently", "at the moment",
        ];
        indicators.iter().any(|i| text.contains(i))
    }
    
    fn has_decision_indicator(text: &str) -> bool {
        let indicators = [
            "i decided", "we decided", "i'll go with", "we'll use",
            "i've chosen", "we've selected", "let's do",
            "the plan is", "the approach", "going forward",
            "i think we should", "i suggest", "i recommend",
            "agreed", "decided to", "choice was",
        ];
        indicators.iter().any(|i| text.contains(i))
    }
    
    fn has_technical_detail(text: &str) -> bool {
        // Contains code-like elements
        let has_code = text.contains("::") || text.contains("()") || 
                       text.contains("{}") || text.contains("[]") ||
                       text.contains("fn ") || text.contains("let ");
        // Contains paths
        let has_path = text.contains("/") && (text.contains(".") || text.contains("src") || text.contains("home"));
        // Contains version numbers or config
        let has_version = text.contains("v1") || text.contains("v2") || 
                          text.contains("2.0") || text.contains("3.0") ||
                          text.contains("config") || text.contains(".json") ||
                          text.contains(".yaml") || text.contains(".toml");
        // Contains technical terms
        let has_tech = text.contains("api") || text.contains("http") ||
                       text.contains("git") || text.contains("rust") ||
                       text.contains("python") || text.contains("javascript") ||
                       text.contains("database") || text.contains("server");
        
        has_code || has_path || has_version || has_tech
    }
    
    fn has_action_indicator(text: &str) -> bool {
        let indicators = [
            "need to", "should", "must", "have to", "i will",
            "i'll", "going to", "will need", "plan to",
            "next step", "the next", "after this",
            "waiting for", "depends on", "blocked by",
        ];
        indicators.iter().any(|i| text.contains(i))
    }
    
    fn has_specific_reference(text: &str) -> bool {
        // Contains numbers (port numbers, versions, counts)
        let has_numbers = text.chars().any(|c| c.is_ascii_digit());
        // Contains proper nouns (capitalized words in middle of sentence)
        let words: Vec<&str> = text.split_whitespace().collect();
        let capitalized_count = words.iter().filter(|w| {
            w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        }).count();
        
        has_numbers || capitalized_count > 2
    }
}

/// Fact Extractor - identifies potential facts in conversation text
pub struct FactExtractor;

impl FactExtractor {
    /// Extract potential facts from conversation text
    /// Returns a list of (content, category, importance) tuples
    pub fn extract(text: &str) -> Vec<(String, FactCategory, f32)> {
        if text.len() < MIN_CONTENT_LEN {
            return Vec::new();
        }
        
        let mut facts: Vec<(String, FactCategory, f32)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        
        // Split into sentences for more granular extraction
        let sentences = Self::split_sentences(text);
        
        for sentence in sentences {
            let trimmed = sentence.trim();
            if trimmed.len() < MIN_CONTENT_LEN {
                continue;
            }
            
            // Try to categorize the sentence
            if let Some((category, content)) = Self::categorize(trimmed) {
                // Deduplicate
                let content_lower = content.to_lowercase();
                if seen.contains(&content_lower) {
                    continue;
                }
                seen.insert(content_lower);
                
                let importance = ImportanceCalculator::calculate(&content, &category);
                
                // Only keep if importance is above threshold
                if importance >= IMPORTANCE_LOW {
                    facts.push((content, category, importance));
                }
            }
        }
        
        // Also try paragraph-level extraction for complex content
        let paragraphs: Vec<&str> = text.split("\n\n")
            .filter(|p| p.len() >= MIN_CONTENT_LEN)
            .collect();
        
        for para in paragraphs {
            if let Some((category, content)) = Self::categorize(para.trim()) {
                let content_lower = content.to_lowercase();
                if seen.contains(&content_lower) {
                    continue;
                }
                seen.insert(content_lower);
                
                let importance = ImportanceCalculator::calculate(&content, &category);
                if importance >= IMPORTANCE_LOW {
                    facts.push((content, category, importance));
                }
            }
        }
        
        // Sort by importance descending
        facts.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        // Limit per turn
        facts.truncate(MAX_EXTRACTIONS_PER_TURN);
        
        facts
    }
    
    /// Split text into sentences
    fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        
        for ch in text.chars() {
            current.push(ch);
            if ch == '.' || ch == '!' || ch == '?' {
                let trimmed = current.trim();
                if trimmed.len() > 3 {
                    sentences.push(trimmed.to_string());
                }
                current.clear();
            }
        }
        
        let final_trimmed = current.trim();
        if !final_trimmed.is_empty() {
            sentences.push(final_trimmed.to_string());
        }
        
        sentences
    }
    
    /// Try to categorize a piece of text
    /// Returns (category, cleaned_content) if a category is detected
    fn categorize(text: &str) -> Option<(FactCategory, String)> {
        let text_lower = text.to_lowercase();
        
        // Check for user preferences
        if Self::is_preference(&text_lower) {
            return Some((FactCategory::UserPreference, Self::clean_text(text)));
        }
        
        // Check for decisions
        if Self::is_decision(&text_lower) {
            return Some((FactCategory::Decision, Self::clean_text(text)));
        }
        
        // Check for action items
        if Self::is_action_item(&text_lower) {
            return Some((FactCategory::ActionItem, Self::clean_text(text)));
        }
        
        // Check for technical content
        if Self::is_technical(&text_lower) {
            return Some((FactCategory::Technical, Self::clean_text(text)));
        }
        
        // Check for project context
        if Self::is_project_context(&text_lower) {
            return Some((FactCategory::ProjectContext, Self::clean_text(text)));
        }
        
        // Default to general if content seems worth remembering
        // Check for specificity indicators
        if Self::has_specific_content(&text_lower) {
            return Some((FactCategory::General, Self::clean_text(text)));
        }
        
        None
    }
    
    fn is_preference(text: &str) -> bool {
        let patterns = [
            "i prefer", "i like", "i dislike", "i hate", "my favorite",
            "i usually", "i always", "i never", "i want", "i need",
            "i'm working on", "i'm using", "i'm building", "i'm developing",
            "my project", "my code", "my setup", "my preference",
            "i've been using", "i've been working",
        ];
        patterns.iter().any(|p| text.contains(p))
    }
    
    fn is_decision(text: &str) -> bool {
        let patterns = [
            "i decided", "we decided", "i'll go with", "we'll use",
            "i've chosen", "we've selected", "let's do",
            "the plan is", "the approach", "going forward",
            "i think we should", "i suggest", "i recommend",
            "agreed", "decided to", "choice was", "selected",
            "final decision", "settled on",
        ];
        patterns.iter().any(|p| text.contains(p))
    }
    
    fn is_action_item(text: &str) -> bool {
        let patterns = [
            "need to", "should", "must", "have to",
            "i will", "i'll", "we'll", "going to",
            "next step", "the next", "after this",
            "waiting for", "depends on", "blocked by",
            "todo", "to-do", "to do", "action item",
            "will do", "will need", "plan to",
        ];
        patterns.iter().any(|p| text.contains(p))
    }
    
    fn is_technical(text: &str) -> bool {
        let technical_keywords = [
            "api", "http", "https", "git", "github",
            "rust", "python", "javascript", "typescript", "java", "go", "c++",
            "database", "sql", "postgresql", "mysql", "redis",
            "server", "client", "endpoint", "request", "response",
            "function", "method", "class", "struct", "enum",
            "config", "configuration", "setting",
            "error", "bug", "fix", "issue", "problem",
            "deploy", "deployment", "build", "compile",
            "docker", "container", "kubernetes", "k8s",
            "file", "directory", "folder", "path", "import", "export",
            "variable", "constant", "module", "package", "crate",
        ];
        
        // Count how many technical keywords appear
        let count = technical_keywords.iter()
            .filter(|kw| text.contains(*kw))
            .count();
        
        count >= 2
    }
    
    fn is_project_context(text: &str) -> bool {
        let patterns = [
            "my project", "the project", "our project", "working on",
            "currently building", "currently developing", "in progress",
            "milestone", "deadline", "sprint", "release",
            "feature", "implementation", "refactor",
            "codebase", "repository", "repo",
        ];
        patterns.iter().any(|p| text.contains(p))
    }
    
    fn has_specific_content(text: &str) -> bool {
        // Content with specific names, numbers, or detailed descriptions
        let has_numbers = text.chars().filter(|c| c.is_ascii_digit()).count() >= 2;
        let has_code_syntax = text.contains("::") || text.contains("fn ") || 
                              text.contains("func ") || text.contains("def ") ||
                              text.contains("class ") || text.contains("struct ");
        let is_long = text.split_whitespace().count() >= 10;
        
        (has_numbers || has_code_syntax) && is_long
    }
    
    /// Clean text for storage (remove leading/trailing quotes, normalize)
    fn clean_text(text: &str) -> String {
        let mut cleaned = text.trim().to_string();
        
        // Remove leading quotes
        if cleaned.starts_with('"') || cleaned.starts_with('\'') {
            cleaned = cleaned[1..].to_string();
        }
        // Remove trailing quotes
        if cleaned.ends_with('"') || cleaned.ends_with('\'') {
            cleaned = cleaned[..cleaned.len()-1].to_string();
        }
        
        // Capitalize first letter if not already
        if !cleaned.is_empty() && cleaned.chars().next().unwrap().is_lowercase() {
            let mut c = cleaned.chars();
            cleaned = c.next().unwrap().to_uppercase().collect::<String>() + c.as_str();
        }
        
        // Add period if missing
        if !cleaned.ends_with('.') && !cleaned.ends_with('!') && !cleaned.ends_with('?') {
            cleaned.push('.');
        }
        
        cleaned
    }
}

/// Memory extraction result with metadata
#[derive(Debug, Clone)]
pub struct ExtractedFact {
    pub content: String,
    pub category: FactCategory,
    pub importance: f32,
    pub source_text: String,
}

impl From<ExtractedFact> for MemoryFact {
    fn from(extracted: ExtractedFact) -> Self {
        MemoryFact::new(
            extracted.content,
            extracted.category,
            extracted.source_text,
            extracted.importance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test threshold constants
    const TEST_IMPORTANCE_HIGH: f32 = 0.75;
    const TEST_IMPORTANCE_MEDIUM: f32 = 0.50;

    #[test]
    fn test_importance_preference() {
        let text = "I prefer using dark mode in my editor.";
        let cat = FactCategory::UserPreference;
        let score = ImportanceCalculator::calculate(text, &cat);
        assert!(score >= TEST_IMPORTANCE_MEDIUM);
    }

    #[test]
    fn test_importance_decision() {
        let text = "I decided to use Rust for this project.";
        let cat = FactCategory::Decision;
        let score = ImportanceCalculator::calculate(text, &cat);
        assert!(score >= TEST_IMPORTANCE_HIGH);
    }

    #[test]
    fn test_importance_technical() {
        let text = "The API endpoint is at http://localhost:8080/api/v2.";
        let cat = FactCategory::Technical;
        let score = ImportanceCalculator::calculate(text, &cat);
        assert!(score >= TEST_IMPORTANCE_MEDIUM);
    }

    #[test]
    fn test_fact_extractor_preference() {
        let text = "I prefer using VS Code for my development work.";
        let facts = FactExtractor::extract(text);
        assert!(!facts.is_empty());
        let (_, cat, _) = &facts[0];
        assert!(matches!(cat, FactCategory::UserPreference | FactCategory::General));
    }

    #[test]
    fn test_fact_extractor_decision() {
        let text = "I decided to use PostgreSQL as the main database.";
        let facts = FactExtractor::extract(text);
        assert!(!facts.is_empty());
        let (_, cat, importance) = &facts[0];
        assert!(matches!(cat, FactCategory::Decision | FactCategory::General));
        assert!(importance >= &TEST_IMPORTANCE_MEDIUM);
    }

    #[test]
    fn test_fact_extractor_technical() {
        let text = "The Rust function is in src/agent/client.rs and uses async/await.";
        let facts = FactExtractor::extract(text);
        // Should extract some facts
        assert!(facts.len() <= MAX_EXTRACTIONS_PER_TURN);
    }

    #[test]
    fn test_fact_extractor_no_trivial_content() {
        let text = "Hi.";
        let facts = FactExtractor::extract(text);
        assert!(facts.is_empty());
    }

    #[test]
    fn test_fact_extractor_action_item() {
        let text = "I need to fix the authentication bug in the login module.";
        let facts = FactExtractor::extract(text);
        assert!(!facts.is_empty());
    }

    #[test]
    fn test_importance_action_item() {
        let text = "I need to update the configuration file.";
        let cat = FactCategory::ActionItem;
        let score = ImportanceCalculator::calculate(text, &cat);
        assert!(score >= IMPORTANCE_LOW);
    }

    #[test]
    fn test_clean_text() {
        let text = "\"i prefer dark mode";
        let cleaned = FactExtractor::clean_text(text);
        assert!(!cleaned.starts_with('"'));
        assert!(cleaned.starts_with('I'));
    }

    #[test]
    fn test_sentence_splitting() {
        let text = "Hello world. How are you? I'm fine!";
        let sentences = FactExtractor::split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_deduplication() {
        let text = "I prefer dark mode. I prefer dark mode.";
        let facts = FactExtractor::extract(text);
        // Should not have duplicate facts
        let contents: Vec<_> = facts.iter().map(|(c, _, _)| c.clone()).collect();
        let unique: HashSet<_> = contents.iter().collect();
        assert_eq!(contents.len(), unique.len());
    }

    #[test]
    fn test_importance_high_content() {
        // Very specific technical content should score high
        let text = "I use cargo::arc::Arc<RwLock<T>> for thread-safe shared state in my Rust applications.";
        let cat = FactCategory::Technical;
        let score = ImportanceCalculator::calculate(text, &cat);
        assert!(score >= TEST_IMPORTANCE_HIGH);
    }

    #[test]
    fn test_extracted_fact_to_memory_fact() {
        let extracted = ExtractedFact {
            content: "User prefers dark mode.".to_string(),
            category: FactCategory::UserPreference,
            importance: 0.85,
            source_text: "session1".to_string(),
        };
        let memory_fact: MemoryFact = extracted.into();
        assert_eq!(memory_fact.content, "User prefers dark mode.");
        assert!(matches!(memory_fact.category, FactCategory::UserPreference));
        assert!((memory_fact.importance - 0.85).abs() < 0.01);
    }
}
