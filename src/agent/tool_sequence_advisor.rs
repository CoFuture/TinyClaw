//! Tool Sequence Advisor Module
//!
//! Provides context-aware tool sequencing recommendations based on:
//! - Current conversation context (task type classification)
//! - Historical tool usage patterns (from tool_pattern_learner)
//! - Skill synergy data (which tools work well together)
//!
//! This advisor helps the agent make better tool selection decisions by
//! analyzing the user's intent and suggesting proven tool sequences.

use std::sync::Arc;
use parking_lot::RwLock;
use crate::agent::tool_pattern_learner::ToolPatternLearner;
use crate::agent::skill_synergy::SkillSynergyAnalyzer;

/// Recommendation confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationConfidence {
    /// High confidence - strong historical evidence
    High,
    /// Medium confidence - some historical evidence
    Medium,
    /// Low confidence - limited or speculative evidence
    Low,
}

impl RecommendationConfidence {
    /// Get display string for prompt formatting
    pub fn as_str(&self) -> &'static str {
        match self {
            RecommendationConfidence::High => "[HIGH]",
            RecommendationConfidence::Medium => "[MED]",
            RecommendationConfidence::Low => "[LOW]",
        }
    }
}

/// A single tool recommendation with reasoning
#[derive(Debug, Clone)]
pub struct ToolRecommendation {
    /// Name of the recommended tool
    pub tool_name: String,
    /// Human-readable reason for this recommendation
    pub reason: String,
    /// Confidence level of this recommendation
    pub confidence: RecommendationConfidence,
    /// Which historical pattern this is based on (if any)
    pub based_on_pattern: Option<String>,
}

/// A sequence recommendation - ordered list of tools to use
#[derive(Debug, Clone)]
pub struct SequenceRecommendation {
    /// Ordered list of tool recommendations
    pub tools: Vec<ToolRecommendation>,
    /// Classified task context (e.g., "explore, modify")
    pub task_context: String,
    /// Description of matched pattern if any
    pub pattern_match_description: String,
}

/// The main advisor struct that combines pattern learning and synergy analysis
pub struct ToolSequenceAdvisor {
    /// Pattern learner for historical tool sequences (wrapped in RwLock for thread-safe access)
    pattern_learner: Arc<RwLock<ToolPatternLearner>>,
    /// Skill synergy analyzer for tool combinations (uses internal locking)
    skill_synergy: Arc<SkillSynergyAnalyzer>,
}

impl ToolSequenceAdvisor {
    /// Create a new tool sequence advisor
    ///
    /// # Arguments
    /// * `pattern_learner` - Shared reference to the tool pattern learner (RwLock wrapped)
    /// * `skill_synergy` - Shared reference to the skill synergy analyzer
    pub fn new(pattern_learner: Arc<RwLock<ToolPatternLearner>>, skill_synergy: Arc<SkillSynergyAnalyzer>) -> Self {
        Self { pattern_learner, skill_synergy }
    }

    /// Classify the task type(s) based on user message content
    ///
    /// Returns a vector of task type identifiers that describe what the user
    /// is trying to accomplish.
    fn classify_task(message: &str) -> Vec<&'static str> {
        let msg_lower = message.to_lowercase();
        let mut types = Vec::new();
        
        // Explore - understanding a codebase or project
        if msg_lower.contains("understand") || msg_lower.contains("explore") 
            || msg_lower.contains("what's in") || msg_lower.contains("find out")
            || msg_lower.contains("investigate") || msg_lower.contains("look at")
            || msg_lower.contains("overview") || msg_lower.contains("introduction") {
            types.push("explore");
        }
        
        // Modify - making changes to files
        if msg_lower.contains("change") || msg_lower.contains("modify") 
            || msg_lower.contains("update") || msg_lower.contains("fix")
            || msg_lower.contains("add") || msg_lower.contains("remove")
            || msg_lower.contains("edit") || msg_lower.contains("refactor")
            || msg_lower.contains("delete") || msg_lower.contains("replace") {
            types.push("modify");
        }
        
        // Execute - running commands
        if msg_lower.contains("run") || msg_lower.contains("execute")
            || msg_lower.contains("start") || msg_lower.contains("build")
            || msg_lower.contains("compile") || msg_lower.contains("test")
            || msg_lower.contains("launch") || msg_lower.contains("install") {
            types.push("execute");
        }
        
        // Search - finding specific content
        if msg_lower.contains("find") || msg_lower.contains("search")
            || msg_lower.contains("grep") || msg_lower.contains("look for")
            || msg_lower.contains("where is") || msg_lower.contains("locate")
            || msg_lower.contains("find all") {
            types.push("search");
        }
        
        // Read - examining files
        if msg_lower.contains("read") || msg_lower.contains("show")
            || msg_lower.contains("display") || msg_lower.contains("view")
            || msg_lower.contains("cat") || msg_lower.contains("content of")
            || msg_lower.contains("tell me about") {
            types.push("read");
        }
        
        // Write - creating new files
        if msg_lower.contains("create") || msg_lower.contains("write")
            || msg_lower.contains("new file") || msg_lower.contains("generate")
            || msg_lower.contains("make a file") || msg_lower.contains("produce") {
            types.push("write");
        }
        
        // Analyze - understanding code structure
        if msg_lower.contains("analyze") || msg_lower.contains("understand structure")
            || msg_lower.contains("architecture") || msg_lower.contains("dependencies")
            || msg_lower.contains("deep dive") || msg_lower.contains("examine") {
            types.push("analyze");
        }
        
        // Compare - comparing files or versions
        if msg_lower.contains("compare") || msg_lower.contains("diff")
            || msg_lower.contains("difference between") || msg_lower.contains("versus")
            || msg_lower.contains("contrast") {
            types.push("compare");
        }
        
        // Default to general if no specific type detected
        if types.is_empty() {
            types.push("general");
        }
        
        types
    }

    /// Find matching patterns from historical data
    ///
    /// Returns scored pattern matches based on task type relevance and
    /// historical success rates.
    fn find_matching_patterns(&self, task_types: &[&str], _message: &str) -> Vec<(String, f32)> {
        let mut matches = Vec::new();
        let learner = self.pattern_learner.read();
        let analysis = learner.get_analysis();
        
        for pattern in &analysis.learned_patterns {
            // Score pattern relevance
            let mut score = 0.0;
            
            // Check success rate weight (higher success = higher score)
            score += pattern.success_rate * 2.0;
            
            // Check usage count (more usage = more confidence)
            if pattern.usage_count >= 5 {
                score += 0.3;
            } else if pattern.usage_count >= 3 {
                score += 0.1;
            }
            
            // Check if pattern tools match task types
            let pattern_str = pattern.tools.join(" ").to_lowercase();
            for task_type in task_types {
                match *task_type {
                    "explore" | "analyze" => {
                        if pattern_str.contains("read") || pattern_str.contains("list") {
                            score += 0.5;
                        }
                    }
                    "modify" => {
                        if pattern_str.contains("read") && pattern_str.contains("write") {
                            score += 0.5;
                        }
                    }
                    "execute" => {
                        if pattern_str.contains("exec") || pattern_str.contains("run") {
                            score += 0.5;
                        }
                    }
                    "search" => {
                        if pattern_str.contains("grep") || pattern_str.contains("find") {
                            score += 0.5;
                        }
                    }
                    "write" => {
                        if pattern_str.contains("write") || pattern_str.contains("create") {
                            score += 0.5;
                        }
                    }
                    _ => {}
                }
            }
            
            // Only include patterns with positive scores
            if score > 0.0 {
                matches.push((pattern.tools.join(" -> "), score));
            }
        }
        
        // Sort by score descending and take top 3
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(3);
        matches
    }

    /// Generate tool recommendations based on task context
    ///
    /// Analyzes historical tool statistics and task type to suggest
    /// appropriate tools with confidence levels.
    fn recommend_tools_for_task(&self, task_types: &[&str], _message: &str) -> Vec<ToolRecommendation> {
        let mut recommendations = Vec::new();
        let learner = self.pattern_learner.read();
        let analysis = learner.get_analysis();
        
        // Collect tools with sufficient usage
        let mut sorted_tools: Vec<_> = analysis.tool_stats.values()
            .filter(|s| s.usage_count >= 2)
            .collect();
        sorted_tools.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap_or(std::cmp::Ordering::Equal));
        
        for task_type in task_types {
            match *task_type {
                "explore" => {
                    // For explore tasks, recommend list_dir and read_file
                    if let Some(stats) = analysis.tool_stats.get("list_dir") {
                        if stats.usage_count >= 2 {
                            recommendations.push(ToolRecommendation {
                                tool_name: "list_dir".to_string(),
                                reason: format!("Good starting point ({}% success)", (stats.success_rate * 100.0) as u32),
                                confidence: if stats.usage_count >= 5 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                                based_on_pattern: None,
                            });
                        }
                    }
                    if let Some(stats) = analysis.tool_stats.get("read_file") {
                        if stats.usage_count >= 2 {
                            recommendations.push(ToolRecommendation {
                                tool_name: "read_file".to_string(),
                                reason: format!("Essential for understanding content ({}% success)", (stats.success_rate * 100.0) as u32),
                                confidence: if stats.usage_count >= 5 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                                based_on_pattern: None,
                            });
                        }
                    }
                }
                "modify" => {
                    // For modify tasks, recommend read then write
                    if let Some(_stats) = analysis.tool_stats.get("read_file") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "read_file".to_string(),
                            reason: "Always read before modifying".to_string(),
                            confidence: RecommendationConfidence::High,
                            based_on_pattern: None,
                        });
                    }
                    if let Some(stats) = analysis.tool_stats.get("write_file") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "write_file".to_string(),
                            reason: format!("For making changes ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 3 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                            based_on_pattern: None,
                        });
                    }
                }
                "execute" => {
                    if let Some(stats) = analysis.tool_stats.get("exec") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "exec".to_string(),
                            reason: format!("Run commands ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 5 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                            based_on_pattern: None,
                        });
                    }
                }
                "search" => {
                    if let Some(stats) = analysis.tool_stats.get("grep") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "grep".to_string(),
                            reason: format!("Search file contents ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 5 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                            based_on_pattern: None,
                        });
                    }
                    if let Some(stats) = analysis.tool_stats.get("find") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "find".to_string(),
                            reason: format!("Find files by name ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 3 { RecommendationConfidence::Medium } else { RecommendationConfidence::Low },
                            based_on_pattern: None,
                        });
                    }
                }
                "analyze" => {
                    if let Some(_stats) = analysis.tool_stats.get("read_file") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "read_file".to_string(),
                            reason: "Foundation for analysis".to_string(),
                            confidence: RecommendationConfidence::High,
                            based_on_pattern: None,
                        });
                    }
                    if let Some(stats) = analysis.tool_stats.get("exec") {
                        if stats.usage_count >= 3 {
                            recommendations.push(ToolRecommendation {
                                tool_name: "exec".to_string(),
                                reason: format!("Run analysis commands ({}% success)", (stats.success_rate * 100.0) as u32),
                                confidence: RecommendationConfidence::Medium,
                                based_on_pattern: None,
                            });
                        }
                    }
                }
                "write" => {
                    if let Some(stats) = analysis.tool_stats.get("write_file") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "write_file".to_string(),
                            reason: format!("Create or update files ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 3 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                            based_on_pattern: None,
                        });
                    }
                }
                "compare" => {
                    if let Some(stats) = analysis.tool_stats.get("diff") {
                        recommendations.push(ToolRecommendation {
                            tool_name: "diff".to_string(),
                            reason: format!("Compare file differences ({}% success)", (stats.success_rate * 100.0) as u32),
                            confidence: if stats.usage_count >= 2 { RecommendationConfidence::Medium } else { RecommendationConfidence::Low },
                            based_on_pattern: None,
                        });
                    }
                }
                _ => {
                    // For general tasks, recommend the most reliable tools
                    if let Some(top) = sorted_tools.first() {
                        recommendations.push(ToolRecommendation {
                            tool_name: top.name.clone(),
                            reason: "Most reliable tool based on history".to_string(),
                            confidence: if top.usage_count >= 5 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                            based_on_pattern: None,
                        });
                    }
                }
            }
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        recommendations.retain(|r| seen.insert(r.tool_name.clone()));
        recommendations.truncate(5);
        recommendations
    }

    /// Get synergy-based tool suggestions (tools that work well together)
    ///
    /// Uses skill synergy analysis to find tools that historically
    /// co-occur successfully with the recommended primary tools.
    fn get_synergy_suggestions(&self, primary_tools: &[String]) -> Vec<ToolRecommendation> {
        let mut suggestions = Vec::new();
        
        for tool in primary_tools {
            // Find tools that synergy well with this tool using get_top_synergies_for_skill
            let synergies = self.skill_synergy.get_top_synergies_for_skill(tool, 2);
            
            for synergy in synergies.iter().take(2) {
                // Determine the other tool in this pair
                let other_tool = if synergy.skill_a == *tool {
                    synergy.skill_b.clone()
                } else {
                    synergy.skill_a.clone()
                };
                
                // Only suggest if score is positive (actual synergy, not anti-synergy)
                if synergy.score > 0.0 {
                    suggestions.push(ToolRecommendation {
                        tool_name: other_tool.clone(),
                        reason: format!("Works well with {} (synergy: {:.0}%)", tool, synergy.score * 100.0),
                        confidence: if synergy.score > 0.6 { RecommendationConfidence::High } else { RecommendationConfidence::Medium },
                        based_on_pattern: Some(format!("{}+{}", tool, other_tool)),
                    });
                }
            }
        }
        
        // Remove duplicates
        let mut seen = std::collections::HashSet::new();
        suggestions.retain(|r| seen.insert(r.tool_name.clone()));
        suggestions.truncate(3);
        suggestions
    }

    /// Generate a complete sequence recommendation for a message
    ///
    /// Combines task classification, pattern matching, tool recommendations,
    /// and synergy analysis into a comprehensive recommendation.
    pub fn get_recommendation(&self, message: &str) -> Option<SequenceRecommendation> {
        let task_types = Self::classify_task(message);
        
        // Get task-specific recommendations
        let tool_recs = self.recommend_tools_for_task(&task_types, message);
        if tool_recs.is_empty() {
            return None;
        }
        
        // Get matching patterns
        let patterns = self.find_matching_patterns(&task_types, message);
        
        // Get synergy suggestions
        let primary_tools: Vec<String> = tool_recs.iter().map(|r| r.tool_name.clone()).collect();
        let synergy_recs = self.get_synergy_suggestions(&primary_tools);
        
        // Combine all recommendations (preserve order, avoid duplicates)
        let mut all_tools = tool_recs;
        for syn in synergy_recs {
            if !all_tools.iter().any(|t| t.tool_name == syn.tool_name) {
                all_tools.push(syn);
            }
        }
        
        Some(SequenceRecommendation {
            tools: all_tools,
            task_context: task_types.join(", "),
            pattern_match_description: if !patterns.is_empty() {
                format!("Matches pattern: {} ({:.0}% confidence)", patterns[0].0, patterns[0].1 * 100.0)
            } else {
                String::new()
            },
        })
    }

    /// Generate a formatted prompt section for the agent context
    ///
    /// Creates a human-readable guidance section that can be injected
    /// into the agent's context prompt to inform tool selection.
    pub fn generate_prompt_section(&self, message: &str) -> String {
        if let Some(rec) = self.get_recommendation(message) {
            let mut section = String::from("## Tool Selection Guidance\n\n");
            section.push_str(&format!("Based on your current task ({})", rec.task_context));
            if !rec.pattern_match_description.is_empty() {
                section.push_str(&format!(" — {}\n", rec.pattern_match_description));
            } else {
                section.push('\n');
            }
            section.push_str("\nSuggested tool approach:\n");
            
            for (i, tool) in rec.tools.iter().enumerate() {
                section.push_str(&format!(
                    "{}. {} {} — {}\n",
                    i + 1,
                    tool.confidence.as_str(),
                    tool.tool_name,
                    tool.reason
                ));
            }
            
            section
        } else {
            String::new()
        }
    }
}

impl Default for ToolSequenceAdvisor {
    fn default() -> Self {
        Self::new(
            Arc::new(parking_lot::RwLock::new(ToolPatternLearner::new())),
            Arc::new(SkillSynergyAnalyzer::new()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_task_explore() {
        let types = ToolSequenceAdvisor::classify_task("I want to understand the project structure");
        assert!(types.contains(&"explore"));
    }
    
    #[test]
    fn test_classify_task_modify() {
        let types = ToolSequenceAdvisor::classify_task("Please fix the bug in main.rs");
        assert!(types.contains(&"modify"));
    }
    
    #[test]
    fn test_classify_task_execute() {
        let types = ToolSequenceAdvisor::classify_task("Run the test suite");
        assert!(types.contains(&"execute"));
    }
    
    #[test]
    fn test_classify_task_search() {
        let types = ToolSequenceAdvisor::classify_task("Find all uses of the function");
        assert!(types.contains(&"search"));
    }
    
    #[test]
    fn test_classify_task_multiple() {
        let types = ToolSequenceAdvisor::classify_task("I need to read the file and fix the bug");
        assert!(types.contains(&"read"));
        assert!(types.contains(&"modify"));
    }
    
    #[test]
    fn test_classify_task_general_default() {
        let types = ToolSequenceAdvisor::classify_task("Hello there");
        assert!(types.contains(&"general"));
        assert_eq!(types.len(), 1);
    }
    
    #[test]
    fn test_confidence_display() {
        assert_eq!(RecommendationConfidence::High.as_str(), "[HIGH]");
        assert_eq!(RecommendationConfidence::Medium.as_str(), "[MED]");
        assert_eq!(RecommendationConfidence::Low.as_str(), "[LOW]");
    }
    
    #[test]
    fn test_generate_prompt_section_empty() {
        let learner = Arc::new(RwLock::new(ToolPatternLearner::new()));
        let synergy = Arc::new(SkillSynergyAnalyzer::new());
        let advisor = ToolSequenceAdvisor::new(learner, synergy);
        
        // With no historical data, should return empty string
        let section = advisor.generate_prompt_section("Hello");
        // Will be empty because there are no tool_stats in a fresh learner
        assert!(section.is_empty());
    }
    
    #[test]
    fn test_recommendation_structure() {
        let rec = ToolRecommendation {
            tool_name: "read_file".to_string(),
            reason: "Test reason".to_string(),
            confidence: RecommendationConfidence::High,
            based_on_pattern: Some("pattern1".to_string()),
        };
        
        assert_eq!(rec.tool_name, "read_file");
        assert_eq!(rec.reason, "Test reason");
        assert_eq!(rec.confidence, RecommendationConfidence::High);
        assert!(rec.based_on_pattern.is_some());
    }
    
    #[test]
    fn test_sequence_recommendation() {
        let seq = SequenceRecommendation {
            tools: vec![
                ToolRecommendation {
                    tool_name: "read_file".to_string(),
                    reason: "First read".to_string(),
                    confidence: RecommendationConfidence::High,
                    based_on_pattern: None,
                },
                ToolRecommendation {
                    tool_name: "write_file".to_string(),
                    reason: "Then write".to_string(),
                    confidence: RecommendationConfidence::Medium,
                    based_on_pattern: None,
                },
            ],
            task_context: "modify".to_string(),
            pattern_match_description: "Test pattern".to_string(),
        };
        
        assert_eq!(seq.tools.len(), 2);
        assert_eq!(seq.task_context, "modify");
        assert!(!seq.pattern_match_description.is_empty());
    }
    
    #[test]
    fn test_default_advisor() {
        let advisor = ToolSequenceAdvisor::default();
        // Default should create fresh learner and analyzer
        let section = advisor.generate_prompt_section("test");
        // Should be empty with no data
        assert!(section.is_empty());
    }
}