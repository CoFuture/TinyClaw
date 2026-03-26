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

/// A predefined task template that a skill can offer
/// These are reusable task patterns the agent can invoke
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTemplate {
    /// Template identifier (unique within the skill)
    pub name: String,
    /// Human-readable description of what this template does
    pub description: String,
    /// Step-by-step instructions for executing this task
    /// Use {placeholder} syntax for parameters the agent should fill in
    pub steps: String,
    /// Tools required for this template
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Example usage showing how to invoke this template
    #[serde(default)]
    pub example: String,
}

impl SkillTemplate {
    /// Create a new skill template
    pub fn new(name: impl Into<String>, description: impl Into<String>, steps: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            steps: steps.into(),
            required_tools: Vec::new(),
            example: String::new(),
        }
    }

    /// Add required tools to this template
    pub fn with_tools<T: Into<String>>(mut self, tools: impl IntoIterator<Item = T>) -> Self {
        self.required_tools.extend(tools.into_iter().map(|t| t.into()));
        self
    }

    /// Set an example usage
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = example.into();
        self
    }
}

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
    /// Predefined task templates this skill offers
    #[serde(default)]
    pub templates: Vec<SkillTemplate>,
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
            templates: Vec::new(),
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

    /// Add a template to this skill
    pub fn with_template(mut self, template: SkillTemplate) -> Self {
        self.templates.push(template);
        self
    }

    /// Add multiple templates to this skill
    pub fn with_templates(mut self, templates: impl IntoIterator<Item = SkillTemplate>) -> Self {
        self.templates.extend(templates);
        self
    }

    /// Get template by name
    #[allow(dead_code)]
    pub fn get_template(&self, name: &str) -> Option<&SkillTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }

    /// Check if this skill has any templates
    #[allow(dead_code)]
    pub fn has_templates(&self) -> bool {
        !self.templates.is_empty()
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

    #[test]
    fn test_skill_template_new() {
        let template = SkillTemplate::new(
            "read_file_content",
            "Read content from a file",
            "1. Use read_file tool with path parameter\n2. Analyze the content",
        );
        assert_eq!(template.name, "read_file_content");
        assert!(template.required_tools.is_empty());
        assert!(template.example.is_empty());
    }

    #[test]
    fn test_skill_template_with_tools() {
        let template = SkillTemplate::new("test", "Test", "Steps")
            .with_tools(["read_file", "grep"]);
        assert_eq!(template.required_tools.len(), 2);
        assert!(template.required_tools.contains(&"read_file".to_string()));
    }

    #[test]
    fn test_skill_with_templates() {
        let template = SkillTemplate::new("analyze", "Analyze", "1. Read file\n2. Search patterns")
            .with_tools(["read_file", "grep"]);
        
        let skill = Skill::new("code_analysis", "Code Analysis", "Analyze code")
            .with_template(template);
        
        assert!(skill.has_templates());
        assert_eq!(skill.templates.len(), 1);
        assert_eq!(skill.get_template("analyze").unwrap().name, "analyze");
    }

    #[test]
    fn test_skill_get_template_not_found() {
        let skill = Skill::new("test", "Test", "Test");
        assert!(skill.get_template("nonexistent").is_none());
    }

    #[test]
    fn test_skill_no_templates() {
        let skill = Skill::new("test", "Test", "Test");
        assert!(!skill.has_templates());
    }
}
