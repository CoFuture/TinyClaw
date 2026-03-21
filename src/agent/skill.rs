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
            .with_tools(["read_file", "write_file"]);
        
        assert_eq!(skill.tool_names.len(), 2);
        assert!(skill.tool_names.contains(&"read_file".to_string()));
        assert!(skill.tool_names.contains(&"write_file".to_string()));
        assert!(!skill.tool_names.contains(&"exec".to_string()));
    }

    #[test]
    fn test_skill_with_tags() {
        let skill = Skill::new("code", "Code Skills", "Instructions")
            .with_tag("programming")
            .with_tag("development");
        
        assert_eq!(skill.tags.len(), 2);
    }
}
