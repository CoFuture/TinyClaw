//! Skill Registry Module
//!
//! Manages available skills and provides skill lookup functionality.
//! Includes built-in skills for common use cases.

use crate::agent::skill::Skill;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Skill registry - manages all available skills
pub struct SkillRegistry {
    /// Map of skill name -> Skill
    skills: RwLock<HashMap<String, Skill>>,
}

impl SkillRegistry {
    /// Create a new skill registry with built-in skills
    pub fn new() -> Arc<Self> {
        let registry = Self {
            skills: RwLock::new(HashMap::new()),
        };
        let arc = Arc::new(registry);
        arc.register_builtin_skills();
        arc
    }

    /// Register a skill
    pub fn register(&self, skill: Skill) {
        let name = skill.name.clone();
        self.skills.write().insert(name, skill);
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<Skill> {
        self.skills.read().get(name).cloned()
    }

    /// List all skills
    pub fn list(&self) -> Vec<Skill> {
        self.skills.read().values().cloned().collect()
    }

    /// List skills by tag
    pub fn list_by_tag(&self, tag: &str) -> Vec<Skill> {
        self.skills.read()
            .values()
            .filter(|s| s.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Get skills that use a specific tool
    pub fn get_skills_for_tool(&self, tool_name: &str) -> Vec<Skill> {
        self.skills.read()
            .values()
            .filter(|s| s.has_tool(tool_name))
            .cloned()
            .collect()
    }

    /// Remove a skill
    pub fn unregister(&self, name: &str) -> Option<Skill> {
        self.skills.write().remove(name)
    }

    /// Update a skill (must exist)
    pub fn update(&self, skill: &Skill) -> bool {
        let mut skills = self.skills.write();
        if skills.contains_key(&skill.name) {
            skills.insert(skill.name.clone(), skill.clone());
            true
        } else {
            false
        }
    }

    /// Get count of registered skills
    pub fn count(&self) -> usize {
        self.skills.read().len()
    }

    /// Check if a skill exists
    pub fn exists(&self, name: &str) -> bool {
        self.skills.read().contains_key(name)
    }

    /// Register built-in skills for common use cases
    fn register_builtin_skills(&self) {
        // File Operations skill
        self.register(Skill::new(
            "file_ops",
            "File Operations",
            r#"When the user wants to read, write, copy, move, or delete files, use the appropriate file tool.

Available tools:
- read_file: Read file contents (specify path)
- write_file: Write content to a file (specify path and content)
- cat: Display file contents with options for line numbers
- cp: Copy files or directories
- mv: Move/rename files or directories
- rm: Remove files or directories
- mkdir: Create directories
- stat: Get file/directory metadata

Always verify the file exists before operations. Use absolute paths when possible."#,
        )
        .with_tools(["read_file", "write_file", "cat", "cp", "mv", "rm", "mkdir", "stat"])
        .with_tag("file")
        .with_tag("filesystem")
        .with_default_enabled(true));

        // Code Analysis skill
        self.register(Skill::new(
            "code_analysis",
            "Code Analysis and Search",
            r#"When analyzing code, searching for patterns, or exploring codebases, use these tools.

Available tools:
- grep: Search for text patterns in files
- find: Find files by name or type
- glob: Find files matching a pattern
- tree: Display directory structure
- list_dir: List directory contents
- wc: Count lines, words, characters

Tips:
- Use grep with -r for recursive search
- Use find with -name for filename matching
- Use glob for pattern-based file finding
- tree -a shows hidden files"#,
        )
        .with_tools(["grep", "find", "glob", "tree", "list_dir", "wc"])
        .with_tag("code")
        .with_tag("search")
        .with_default_enabled(true));

        // System Operations skill
        self.register(Skill::new(
            "system_ops",
            "System Operations",
            r#"When executing commands, checking system status, or performing system operations.

Available tools:
- exec: Execute shell commands (respects exec_enabled config)
- which: Find executable location
- env: Get or list environment variables
- chmod: Change file permissions
- hash: Compute file hashes

Warning: Be careful with destructive commands (rm, chmod with dangerous permissions).
Always confirm with the user before executing potentially harmful operations."#,
        )
        .with_tools(["exec", "which", "env", "chmod", "hash"])
        .with_tag("system")
        .with_tag("shell")
        .with_default_enabled(false)); // Disabled by default - exec is dangerous

        // Web Search skill (if http tool is available)
        self.register(Skill::new(
            "web_search",
            "Web and HTTP Operations",
            r#"When the user wants to fetch web content, make HTTP requests, or interact with web services.

Available tools:
- http_request: Make HTTP requests (GET, POST, PUT, DELETE, etc.)
  - Supports custom headers, query parameters, and request body
  - Useful for APIs, web scraping, or checking endpoints

Always inform the user before making requests that modify data."#,
        )
        .with_tools(["http_request"])
        .with_tag("web")
        .with_tag("http")
        .with_default_enabled(false));

        // Diff and Compare skill
        self.register(Skill::new(
            "diff_compare",
            "Diff and Comparison",
            r#"When comparing files, finding differences, or analyzing changes.

Available tools:
- diff: Compare two files and show differences
- hash: Compute file hashes to verify equality

Use diff to see line-by-line differences.
Use hash to quickly check if files are identical."#,
        )
        .with_tools(["diff", "hash"])
        .with_tag("diff")
        .with_tag("comparison")
        .with_default_enabled(false));
    }

    /// Generate system prompt section from active skills
    pub fn generate_skill_prompt(&self, active_skills: &[String]) -> Option<String> {
        if active_skills.is_empty() {
            return None;
        }

        let mut prompt = String::from("\n\n## Active Skills\n\n");
        prompt.push_str("The following skills are available for this conversation:\n\n");

        for skill_name in active_skills {
            if let Some(skill) = self.get(skill_name) {
                prompt.push_str(&format!("### {}\n", skill.name));
                prompt.push_str(&format!("{}\n\n", skill.description));
                prompt.push_str(&format!(
                    "Instructions: {}\n",
                    skill.instructions
                ));
                prompt.push_str(&format!("Tools: {}\n\n", skill.tool_names.join(", ")));
            }
        }

        Some(prompt)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry_new() {
        let registry = SkillRegistry::new();
        // Should have built-in skills
        assert!(registry.count() > 0);
    }

    #[test]
    fn test_skill_registry_register_get() {
        let registry = SkillRegistry::new();
        
        registry.register(Skill::new("test", "Test Skill", "Test instructions"));
        
        let skill = registry.get("test");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().description, "Test Skill");
    }

    #[test]
    fn test_skill_registry_list() {
        let registry = SkillRegistry::new();
        let skills = registry.list();
        assert!(!skills.is_empty());
    }

    #[test]
    fn test_skill_registry_list_by_tag() {
        let registry = SkillRegistry::new();
        let file_skills = registry.list_by_tag("file");
        assert!(!file_skills.is_empty());
        assert!(file_skills.iter().all(|s| s.tags.contains(&"file".to_string())));
    }

    #[test]
    fn test_skill_registry_get_skills_for_tool() {
        let registry = SkillRegistry::new();
        let skills = registry.get_skills_for_tool("read_file");
        assert!(!skills.is_empty());
        assert!(skills.iter().all(|s| s.has_tool("read_file")));
    }

    #[test]
    fn test_skill_registry_generate_prompt() {
        let registry = SkillRegistry::new();
        
        let prompt = registry.generate_skill_prompt(&["file_ops".to_string()]);
        assert!(prompt.is_some());
        let prompt_str = prompt.unwrap();
        assert!(prompt_str.contains("file_ops"));
        assert!(prompt_str.contains("File Operations"));
    }

    #[test]
    fn test_skill_registry_generate_prompt_empty() {
        let registry = SkillRegistry::new();
        let prompt = registry.generate_skill_prompt(&[]);
        assert!(prompt.is_none());
    }

    #[test]
    fn test_skill_registry_update() {
        let registry = SkillRegistry::new();
        
        // Update existing skill
        let mut skill = registry.get("file_ops").unwrap();
        skill.instructions = "Updated instructions".to_string();
        
        let result = registry.update(&skill);
        assert!(result);
        
        let updated = registry.get("file_ops").unwrap();
        assert_eq!(updated.instructions, "Updated instructions");
    }

    #[test]
    fn test_skill_registry_unregister() {
        let registry = SkillRegistry::new();
        
        registry.register(Skill::new("temp", "Temp", "Temp instructions"));
        assert!(registry.exists("temp"));
        
        registry.unregister("temp");
        assert!(!registry.exists("temp"));
    }
}
