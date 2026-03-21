//! Skill Module
//!
//! Skills bundle related tools with metadata (description, instructions)
//! to provide the AI with context-aware guidance on when and how to use tools.
//!
//! Unlike tools which perform actions, skills provide:
//! - Human-readable descriptions of what the skill does
//! - Instructions for when to use the skill
//! - Grouping of related tools

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A skill that bundles related tools with usage guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique skill identifier
    pub name: String,
    /// Human-readable description of what this skill does
    pub description: String,
    /// Instructions for the AI on when and how to use this skill
    /// This gets injected into the system prompt when the skill is active
    pub instructions: String,
    /// List of tool names that this skill uses
    pub tool_names: Vec<String>,
    /// Whether this skill is enabled by default
    #[serde(default)]
    pub enabled_by_default: bool,
    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Skill {
    /// Create a new skill
    pub fn new(name: impl Into<String>, description: impl Into<String>, instructions: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            instructions: instructions.into(),
            tool_names: Vec::new(),
            enabled_by_default: false,
            tags: Vec::new(),
        }
    }

    /// Add a tool name to this skill
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_names.push(tool_name.into());
        self
    }

    /// Add multiple tool names to this skill
    pub fn with_tools<T: Into<String>>(mut self, tool_names: impl IntoIterator<Item = T>) -> Self {
        self.tool_names.extend(tool_names.into_iter().map(|t| t.into()));
        self
    }

    /// Set enabled by default
    pub fn with_default_enabled(mut self, enabled: bool) -> Self {
        self.enabled_by_default = enabled;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn with_tags<T: Into<String>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Check if this skill uses a specific tool
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.tool_names.iter().any(|t| t == tool_name)
    }

    /// Get the tool names as a set for fast lookup
    pub fn tool_names_set(&self) -> HashSet<&str> {
        self.tool_names.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_new() {
        let skill = Skill::new(
            "file_ops",
            "File Operations",
            "Use file tools when the user wants to read, write, or manage files.",
        );
        assert_eq!(skill.name, "file_ops");
        assert!(skill.tool_names.is_empty());
        assert!(!skill.enabled_by_default);
    }

    #[test]
    fn test_skill_with_tools() {
        let skill = Skill::new("test", "Test", "Test instructions")
            .with_tool("read_file")
            .with_tool("write_file");
        
        assert_eq!(skill.tool_names.len(), 2);
        assert!(skill.has_tool("read_file"));
        assert!(skill.has_tool("write_file"));
        assert!(!skill.has_tool("exec"));
    }

    #[test]
    fn test_skill_with_tags() {
        let skill = Skill::new("code", "Code Skills", "Instructions")
            .with_tag("programming")
            .with_tag("development");
        
        assert_eq!(skill.tags.len(), 2);
    }
}
