//! Skill Registry Module
//!
//! Manages available skills and provides skill lookup functionality.
//! Includes built-in skills for common use cases.
//! Supports persistence of custom skills to a JSON file.

use crate::agent::skill::{Skill, SkillTemplate};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Built-in skill names - these are never persisted to disk
const BUILT_IN_SKILLS: &[&str] = &[
    "file_ops",
    "code_analysis",
    "system_ops",
    "web_search",
    "diff_compare",
];

/// Check if a skill name is a built-in
fn is_builtin_skill(name: &str) -> bool {
    BUILT_IN_SKILLS.contains(&name)
}

/// Skill registry - manages all available skills
pub struct SkillRegistry {
    /// Map of skill name -> Skill
    skills: RwLock<HashMap<String, Skill>>,
    /// Path to persist custom skills (None = no persistence)
    persist_path: RwLock<Option<String>>,
}

impl SkillRegistry {
    /// Create a new skill registry with built-in skills (no persistence)
    pub fn new() -> Arc<Self> {
        let registry = Self {
            skills: RwLock::new(HashMap::new()),
            persist_path: RwLock::new(None),
        };
        let arc = Arc::new(registry);
        arc.register_builtin_skills();
        arc
    }

    /// Create a new skill registry with persistence enabled
    /// Loads custom skills from the given path and saves to it on changes
    pub fn new_with_persistence(persist_path: &str) -> Arc<Self> {
        let registry = Self {
            skills: RwLock::new(HashMap::new()),
            persist_path: RwLock::new(Some(persist_path.to_string())),
        };
        let arc = Arc::new(registry);

        // Register built-in skills first
        arc.register_builtin_skills();

        // Then load any custom skills from file (won't overwrite built-ins)
        if let Err(e) = arc.load_custom_skills() {
            warn!("Failed to load custom skills from {}: {}", persist_path, e);
        }

        info!("Skill registry initialized with persistence: {}", persist_path);
        arc
    }

    /// Register a skill (auto-persists if persistence is enabled)
    pub fn register(&self, skill: Skill) {
        let name = skill.name.clone();
        self.skills.write().insert(name, skill);
        self.persist();
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<Skill> {
        self.skills.read().get(name).cloned()
    }

    /// List all skills
    pub fn list(&self) -> Vec<Skill> {
        self.skills.read().values().cloned().collect()
    }

    /// Remove a skill (won't remove built-in skills)
    pub fn unregister(&self, name: &str) -> Option<Skill> {
        if is_builtin_skill(name) {
            warn!("Cannot unregister built-in skill: {}", name);
            return None;
        }
        let removed = self.skills.write().remove(name);
        if removed.is_some() {
            self.persist();
        }
        removed
    }

    /// Update a skill (must exist)
    pub fn update(&self, skill: &Skill) -> bool {
        if is_builtin_skill(&skill.name) {
            // Allow updating built-in skills but still persist
            let mut skills = self.skills.write();
            skills.insert(skill.name.clone(), skill.clone());
            drop(skills);
            self.persist();
            return true;
        }
        let mut skills = self.skills.write();
        if skills.contains_key(&skill.name) {
            skills.insert(skill.name.clone(), skill.clone());
            drop(skills);
            self.persist();
            true
        } else {
            false
        }
    }

    /// Check if a skill exists
    pub fn exists(&self, name: &str) -> bool {
        self.skills.read().contains_key(name)
    }

    /// Manually trigger persistence (save custom skills to disk)
    pub fn persist(&self) {
        let path = self.persist_path.read().clone();
        if let Some(path) = path {
            if let Err(e) = self.save_custom_skills_to_file(&path) {
                warn!("Failed to persist skills to {}: {}", path, e);
            }
        }
    }

    /// Save custom skills to JSON file (excludes built-in skills)
    fn save_custom_skills_to_file(&self, path: &str) -> std::io::Result<()> {
        let custom_skills: Vec<Skill> = self
            .skills
            .read()
            .values()
            .filter(|skill| !is_builtin_skill(&skill.name))
            .cloned()
            .collect();

        let json = serde_json::to_string_pretty(&custom_skills)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, json)?;
        info!("Saved {} custom skills to {}", custom_skills.len(), path);
        Ok(())
    }

    /// Load custom skills from JSON file
    fn load_custom_skills(&self) -> std::io::Result<()> {
        let path = self.persist_path.read().clone();
        let Some(path) = path else {
            return Ok(());
        };

        let path = Path::new(&path);
        if !path.exists() {
            info!("No custom skills file found at {:?}, skipping load", path);
            return Ok(());
        }

        let json = fs::read_to_string(path)?;
        let skills: Vec<Skill> = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut count = 0;
        for skill in skills {
            if is_builtin_skill(&skill.name) {
                warn!(
                    "Custom skills file contains built-in skill '{}', skipping",
                    skill.name
                );
                continue;
            }
            self.skills.write().insert(skill.name.clone(), skill);
            count += 1;
        }

        info!("Loaded {} custom skills from {:?}", count, path);
        Ok(())
    }

    /// Register built-in skills for common use cases
    fn register_builtin_skills(&self) {
        // File Operations skill
        self.register_builtin(Skill::new(
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
        .with_default_enabled(true)
        .with_template(SkillTemplate::new(
            "explore_project",
            "Explore a project directory structure",
            r#"1. Use list_dir to see the top-level contents
2. Use tree to get a full directory view
3. Identify key files (README, package.json, Cargo.toml, etc.)
4. Read relevant config files to understand the project"#,
        )
        .with_tools(["list_dir", "tree"])
        .with_example("Explore the ~/projects/myapp directory"))
        .with_template(SkillTemplate::new(
            "edit_file",
            "Safely edit a file",
            r#"1. Use read_file to view current content
2. Prepare the new content
3. Use write_file to save changes
4. Verify the changes were applied correctly"#,
        )
        .with_tools(["read_file", "write_file"])
        .with_example("Edit config.json to update the port setting")));

        // Code Analysis skill
        self.register_builtin(Skill::new(
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
        .with_default_enabled(true)
        .with_template(SkillTemplate::new(
            "find_usage",
            "Find where a function/variable is used",
            r#"1. Use grep with -r to search for the identifier
2. Analyze the results to understand usage patterns
3. Report the locations and context"#,
        )
        .with_tools(["grep"])
        .with_example("Find all usages of 'calculateTotal' function"))
        .with_template(SkillTemplate::new(
            "analyze_structure",
            "Analyze code structure in a directory",
            r#"1. Use tree to see the directory layout
2. Use glob to find source files by extension
3. Use wc to count lines of code per file
4. Summarize the architecture and key components"#,
        )
        .with_tools(["tree", "glob", "wc"])
        .with_example("Analyze the src directory structure")));

        // System Operations skill
        self.register_builtin(Skill::new(
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
        self.register_builtin(Skill::new(
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
        self.register_builtin(Skill::new(
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

    /// Register a built-in skill without triggering persistence
    fn register_builtin(&self, skill: Skill) {
        self.skills.write().insert(skill.name.clone(), skill);
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            persist_path: RwLock::new(None),
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
        assert!(!registry.list().is_empty());
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

    #[test]
    fn test_is_builtin_skill() {
        assert!(is_builtin_skill("file_ops"));
        assert!(is_builtin_skill("code_analysis"));
        assert!(!is_builtin_skill("custom_skill"));
        assert!(!is_builtin_skill("temp"));
    }

    #[test]
    fn test_cannot_unregister_builtin() {
        let registry = SkillRegistry::new();
        let initial_count = registry.list().len();

        // Try to unregister a built-in skill
        let removed = registry.unregister("file_ops");
        assert!(removed.is_none());

        // Count should be unchanged
        assert_eq!(registry.list().len(), initial_count);
    }

    #[test]
    fn test_custom_skill_persists() {
        let registry = SkillRegistry::new();
        registry.register(Skill::new("custom1", "Custom 1", "Instructions"));

        // Custom skill should exist
        assert!(registry.exists("custom1"));
        // Built-in should still exist
        assert!(registry.exists("file_ops"));
    }
}
