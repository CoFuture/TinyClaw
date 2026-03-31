//! Tool Strategy Module
//!
//! Provides intelligent tool usage guidance to help the Agent
//! make better decisions about which tools to use and how to combine them.
//!
//! This module analyzes user intent and suggests optimal tool combinations
//! for common task patterns.

use serde::{Deserialize, Serialize};

/// User intent classification for tool selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserIntent {
    /// User wants to explore or understand a project/codebase
    Explore,
    /// User wants to modify or edit files
    Modify,
    /// User wants to run commands or programs
    Execute,
    /// User wants to search for specific content
    Search,
    /// User wants to read or view content
    Read,
    /// User wants to write or create content
    Write,
    /// User wants to understand code structure
    Analyze,
    /// User wants to compare differences
    Compare,
    /// User wants to make HTTP requests
    Fetch,
    /// User wants to understand system status
    Monitor,
    /// Unclear or mixed intent
    Ambiguous,
}

impl UserIntent {
    /// Classify user intent from message content
    pub fn from_message(message: &str) -> Self {
        let msg_lower = message.to_lowercase();
        let msg_words: Vec<&str> = msg_lower.split_whitespace().collect();
        
        // Helper to check if word boundary matches (for single words, must be whole word)
        fn contains_word(msg_lower: &str, words: &[&str], msg_words: &[&str]) -> usize {
            words.iter().filter(|word| {
                if word.contains(' ') {
                    // Multi-word phrase, use contains
                    msg_lower.contains(*word)
                } else {
                    // Single word, must be whole word match
                    msg_words.contains(word)
                }
            }).count()
        }
        
        // Check for strong indicators first (multi-word phrases first for precision)
        let explore_indicators = ["explore", "find files", "list files", "what's in", "what is in", "folder structure", "project structure", "browse"];
        let modify_indicators = ["edit", "change", "modify", "update", "fix", "add code", "remove code", "replace"];
        let execute_indicators = ["run", "execute", "build", "compile", "test", "start", "stop", "restart", "deploy"];
        let search_indicators = ["search", "find", "grep", "look for", "where is", "locate"];
        let read_indicators = ["read", "view", "show", "display", "cat", "open"];
        let write_indicators = ["write", "create file", "new file", "save", "make file", "mkdir"];
        let analyze_indicators = ["analyze", "review", "understand", "explain", "how does", "what does", "why is", "what is the issue"];
        let compare_indicators = ["compare", "diff", "difference between", "vs", "versus", "changes"];
        let fetch_indicators = ["fetch", "http", "api", "request", "download", "curl", "wget"];
        let monitor_indicators = ["status", "check", "monitor", "health", "metrics", "logs", "processes"];
        
        // Count matches for each intent (using word boundary matching)
        let explore_count = contains_word(&msg_lower, &explore_indicators, &msg_words);
        let modify_count = contains_word(&msg_lower, &modify_indicators, &msg_words);
        let execute_count = contains_word(&msg_lower, &execute_indicators, &msg_words);
        let search_count = contains_word(&msg_lower, &search_indicators, &msg_words);
        let read_count = contains_word(&msg_lower, &read_indicators, &msg_words);
        let write_count = contains_word(&msg_lower, &write_indicators, &msg_words);
        let analyze_count = contains_word(&msg_lower, &analyze_indicators, &msg_words);
        let compare_count = contains_word(&msg_lower, &compare_indicators, &msg_words);
        let fetch_count = contains_word(&msg_lower, &fetch_indicators, &msg_words);
        let monitor_count = contains_word(&msg_lower, &monitor_indicators, &msg_words);
        
        // Find the max count
        let max_count = [
            explore_count, modify_count, execute_count, search_count,
            read_count, write_count, analyze_count, compare_count,
            fetch_count, monitor_count
        ].into_iter().max().unwrap_or(0);
        
        if max_count == 0 {
            return UserIntent::Ambiguous;
        }
        
        // Return the intent with highest count
        if explore_count == max_count { return UserIntent::Explore; }
        if modify_count == max_count { return UserIntent::Modify; }
        if execute_count == max_count { return UserIntent::Execute; }
        if search_count == max_count { return UserIntent::Search; }
        if read_count == max_count { return UserIntent::Read; }
        if write_count == max_count { return UserIntent::Write; }
        if analyze_count == max_count { return UserIntent::Analyze; }
        if compare_count == max_count { return UserIntent::Compare; }
        if fetch_count == max_count { return UserIntent::Fetch; }
        if monitor_count == max_count { return UserIntent::Monitor; }
        
        UserIntent::Ambiguous
    }
    
    /// Get the display name for this intent
    pub fn display_name(&self) -> &'static str {
        match self {
            UserIntent::Explore => "Explore",
            UserIntent::Modify => "Modify",
            UserIntent::Execute => "Execute",
            UserIntent::Search => "Search",
            UserIntent::Read => "Read",
            UserIntent::Write => "Write",
            UserIntent::Analyze => "Analyze",
            UserIntent::Compare => "Compare",
            UserIntent::Fetch => "Fetch",
            UserIntent::Monitor => "Monitor",
            UserIntent::Ambiguous => "Ambiguous",
        }
    }
}

/// A workflow pattern describing a common multi-tool task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    /// Pattern identifier
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Which intents this pattern applies to
    pub intents: Vec<UserIntent>,
    /// Ordered list of tools to use
    pub tools: Vec<String>,
    /// Step-by-step guidance
    pub steps: Vec<String>,
    /// When this pattern is most useful
    pub when_to_use: String,
}

impl WorkflowPattern {
    /// Create a new workflow pattern
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            intents: Vec::new(),
            tools: Vec::new(),
            steps: Vec::new(),
            when_to_use: String::new(),
        }
    }
    
    /// Add intents this pattern applies to
    pub fn with_intents(mut self, intents: impl IntoIterator<Item = UserIntent>) -> Self {
        self.intents.extend(intents);
        self
    }
    
    /// Add tools in order
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = &'static str>) -> Self {
        self.tools.extend(tools.into_iter().map(|s| s.to_string()));
        self
    }
    
    /// Add step guidance
    pub fn with_steps(mut self, steps: impl IntoIterator<Item = &'static str>) -> Self {
        self.steps.extend(steps.into_iter().map(|s| s.to_string()));
        self
    }
    
    /// Set when to use this pattern
    pub fn when_to_use(mut self, hint: &str) -> Self {
        self.when_to_use = hint.to_string();
        self
    }
}

/// Tool selection guidance for a specific intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGuidance {
    /// The primary intent
    pub intent: UserIntent,
    /// Recommended tools in order of preference
    pub recommended_tools: Vec<String>,
    /// Tips for using these tools effectively
    pub tips: Vec<String>,
    /// Common pitfalls to avoid
    pub pitfalls: Vec<String>,
}

impl ToolGuidance {
    /// Get guidance for a specific intent
    pub fn for_intent(intent: UserIntent) -> Option<Self> {
        match intent {
            UserIntent::Explore => Some(Self {
                intent,
                recommended_tools: vec!["glob".to_string(), "list_dir".to_string(), "read_file".to_string()],
                tips: vec![
                    "Start with glob to find relevant files".to_string(),
                    "Use list_dir to understand directory structure".to_string(),
                    "Read key files to understand the codebase".to_string(),
                ],
                pitfalls: vec![
                    "Don't read every file - focus on relevant ones".to_string(),
                    "Use patterns like **/*.rs to narrow down".to_string(),
                ],
            }),
            UserIntent::Modify => Some(Self {
                intent,
                recommended_tools: vec!["read_file".to_string(), "grep".to_string(), "edit_file".to_string()],
                tips: vec![
                    "Always read the file before editing".to_string(),
                    "Use grep to find the exact location to modify".to_string(),
                    "Make small, focused changes".to_string(),
                ],
                pitfalls: vec![
                    "Don't edit without reading first".to_string(),
                    "Avoid making multiple unrelated changes at once".to_string(),
                ],
            }),
            UserIntent::Execute => Some(Self {
                intent,
                recommended_tools: vec!["exec".to_string()],
                tips: vec![
                    "Use appropriate timeouts for long-running commands".to_string(),
                    "Check the command syntax before running".to_string(),
                    "Use 'cd' to change to the right directory first".to_string(),
                ],
                pitfalls: vec![
                    "Don't run commands you don't understand".to_string(),
                    "Be careful with rm, chmod, dd and other destructive commands".to_string(),
                ],
            }),
            UserIntent::Search => Some(Self {
                intent,
                recommended_tools: vec!["grep".to_string(), "glob".to_string()],
                tips: vec![
                    "Use regex patterns for complex searches".to_string(),
                    "Start broad, then narrow down".to_string(),
                    "Use case_sensitive: false for more results".to_string(),
                ],
                pitfalls: vec![
                    "Don't use overly broad patterns that return too many results".to_string(),
                    "Remember to escape special regex characters".to_string(),
                ],
            }),
            UserIntent::Read => Some(Self {
                intent,
                recommended_tools: vec!["read_file".to_string()],
                tips: vec![
                    "Use max_bytes for large files".to_string(),
                    "Paths support ~ for home directory".to_string(),
                ],
                pitfalls: vec![
                    "Don't try to read binary files".to_string(),
                ],
            }),
            UserIntent::Write => Some(Self {
                intent,
                recommended_tools: vec!["write_file".to_string(), "exec".to_string()],
                tips: vec![
                    "Use mkdir to create directories if needed".to_string(),
                    "Backup existing files before overwriting".to_string(),
                    "Use exec with echo or cat to verify write success".to_string(),
                ],
                pitfalls: vec![
                    "write_file will overwrite without confirmation".to_string(),
                    "Don't forget to use proper line endings".to_string(),
                ],
            }),
            UserIntent::Analyze => Some(Self {
                intent,
                recommended_tools: vec!["read_file".to_string(), "grep".to_string(), "exec".to_string()],
                tips: vec![
                    "Read the main entry points first".to_string(),
                    "Use grep to find usage patterns".to_string(),
                    "Run analysis tools if available (e.g., cargo check)".to_string(),
                ],
                pitfalls: vec![
                    "Don't assume - verify with actual code".to_string(),
                    "Context matters - consider the broader codebase".to_string(),
                ],
            }),
            UserIntent::Compare => Some(Self {
                intent,
                recommended_tools: vec!["read_file".to_string(), "exec".to_string()],
                tips: vec![
                    "Read both versions of the content".to_string(),
                    "Use git diff for file comparisons".to_string(),
                    "Look for meaningful differences, not just line counts".to_string(),
                ],
                pitfalls: vec![
                    "Don't ignore subtle semantic differences".to_string(),
                ],
            }),
            UserIntent::Fetch => Some(Self {
                intent,
                recommended_tools: vec!["http_request".to_string()],
                tips: vec![
                    "Use GET for reading data, POST for creating".to_string(),
                    "Set appropriate headers (Content-Type, etc.)".to_string(),
                    "Handle JSON responses appropriately".to_string(),
                ],
                pitfalls: vec![
                    "Don't send sensitive data without encryption".to_string(),
                    "Remember to handle HTTP error responses".to_string(),
                ],
            }),
            UserIntent::Monitor => Some(Self {
                intent,
                recommended_tools: vec!["exec".to_string()],
                tips: vec![
                    "Use appropriate system commands (ps, top, htop)".to_string(),
                    "Check logs in /var/log or similar locations".to_string(),
                    "Use curl to check service health endpoints".to_string(),
                ],
                pitfalls: vec![
                    "Don't overwhelm the system with monitoring queries".to_string(),
                ],
            }),
            UserIntent::Ambiguous => None,
        }
    }
}

/// Tool Strategy engine - provides tool usage guidance
pub struct ToolStrategy {
    /// Registered workflow patterns
    patterns: Vec<WorkflowPattern>,
}

impl ToolStrategy {
    /// Create a new tool strategy engine with default patterns
    pub fn new() -> Self {
        let patterns = Self::default_patterns();
        Self { patterns }
    }
    
    /// Get the primary user intent from a message
    pub fn classify_intent(&self, message: &str) -> UserIntent {
        UserIntent::from_message(message)
    }
    
    /// Get tool guidance for a specific intent
    pub fn get_guidance(&self, intent: UserIntent) -> Option<ToolGuidance> {
        ToolGuidance::for_intent(intent)
    }
    
    /// Get relevant workflow patterns for an intent
    pub fn get_patterns(&self, intent: UserIntent) -> Vec<&WorkflowPattern> {
        self.patterns
            .iter()
            .filter(|p| p.intents.contains(&intent))
            .collect()
    }
    
    /// Generate a comprehensive strategy prompt for the agent
    pub fn generate_strategy_prompt(&self, user_message: &str) -> String {
        let intent = self.classify_intent(user_message);
        
        if intent == UserIntent::Ambiguous {
            return String::from("## Tool Usage Guidance\n\nWhen the user's intent is unclear, start by asking clarifying questions or use read_file/list_dir to explore the environment before deciding which tools to use.\n");
        }
        
        let mut prompt = format!("## Tool Usage Guidance (Intent: {})\n\n", intent.display_name());
        
        // Add guidance for this intent
        if let Some(guidance) = self.get_guidance(intent) {
            prompt.push_str("### Recommended Tools\n");
            for (i, tool) in guidance.recommended_tools.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, tool));
            }
            
            if !guidance.tips.is_empty() {
                prompt.push_str("\n### Tips\n");
                for tip in &guidance.tips {
                    prompt.push_str(&format!("- {}\n", tip));
                }
            }
            
            if !guidance.pitfalls.is_empty() {
                prompt.push_str("\n### Pitfalls to Avoid\n");
                for pitfall in &guidance.pitfalls {
                    prompt.push_str(&format!("- {}\n", pitfall));
                }
            }
        }
        
        // Add relevant workflow patterns
        let patterns = self.get_patterns(intent);
        if !patterns.is_empty() {
            prompt.push_str("\n### Common Workflows\n");
            for pattern in patterns {
                prompt.push_str(&format!("\n**{}**: {}\n", pattern.name, pattern.description));
                if !pattern.tools.is_empty() {
                    prompt.push_str(&format!("Tools: {}\n", pattern.tools.join(" → ")));
                }
                if !pattern.steps.is_empty() {
                    prompt.push_str("Steps:\n");
                    for (i, step) in pattern.steps.iter().enumerate() {
                        prompt.push_str(&format!("{}. {}\n", i + 1, step));
                    }
                }
            }
        }
        
        prompt
    }
    
    /// Default workflow patterns
    fn default_patterns() -> Vec<WorkflowPattern> {
        vec![
            WorkflowPattern::new("explore_and_read", "Explore a codebase then read key files")
                .with_intents([UserIntent::Explore, UserIntent::Read])
                .with_tools(["glob", "read_file"])
                .with_steps([
                    "Use glob to find relevant files with patterns like **/*.rs",
                    "Read the main entry points (main.rs, lib.rs, index.js)",
                    "Explore subdirectories for specific functionality",
                ])
                .when_to_use("When exploring a new project or understanding structure"),
            
            WorkflowPattern::new("find_and_modify", "Find specific code then make changes")
                .with_intents([UserIntent::Modify, UserIntent::Search])
                .with_tools(["grep", "read_file", "edit_file"])
                .with_steps([
                    "Use grep to locate the exact code to modify",
                    "Read the file to understand the context",
                    "Make the modification with edit_file",
                    "Verify the change with read_file or grep",
                ])
                .when_to_use("When you need to find and change specific code"),
            
            WorkflowPattern::new("build_and_run", "Build a project then execute it")
                .with_intents([UserIntent::Execute])
                .with_tools(["exec"])
                .with_steps([
                    "Build the project (cargo build, npm build, etc.)",
                    "Fix any build errors",
                    "Run the compiled program or script",
                ])
                .when_to_use("When compiling and running code"),
            
            WorkflowPattern::new("backup_and_write", "Backup existing file then write new content")
                .with_intents([UserIntent::Write])
                .with_tools(["exec", "write_file", "read_file"])
                .with_steps([
                    "Check if file exists with read_file",
                    "Backup with exec (cp or git backup)",
                    "Write the new content",
                    "Verify with read_file",
                ])
                .when_to_use("When creating or overwriting important files"),
            
            WorkflowPattern::new("analyze_code", "Analyze code structure and behavior")
                .with_intents([UserIntent::Analyze])
                .with_tools(["glob", "read_file", "grep"])
                .with_steps([
                    "Identify the main entry points",
                    "Read key source files",
                    "Use grep to find usage patterns",
                    "Trace the execution flow",
                ])
                .when_to_use("When trying to understand how code works"),
            
            WorkflowPattern::new("research_api", "Research an API by fetching and analyzing")
                .with_intents([UserIntent::Fetch, UserIntent::Analyze])
                .with_tools(["http_request", "read_file"])
                .with_steps([
                    "Make HTTP request to fetch data",
                    "Parse and analyze the response",
                    "Store useful information",
                ])
                .when_to_use("When researching APIs or external services"),
        ]
    }
}

impl Default for ToolStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_explore() {
        assert_eq!(UserIntent::from_message("explore the codebase"), UserIntent::Explore);
        assert_eq!(UserIntent::from_message("find files in the project"), UserIntent::Explore);
        assert_eq!(UserIntent::from_message("what's in this folder"), UserIntent::Explore);
    }

    #[test]
    fn test_intent_modify() {
        assert_eq!(UserIntent::from_message("edit the config file"), UserIntent::Modify);
        assert_eq!(UserIntent::from_message("change the port number"), UserIntent::Modify);
        assert_eq!(UserIntent::from_message("fix the bug in main.rs"), UserIntent::Modify);
    }

    #[test]
    fn test_intent_execute() {
        assert_eq!(UserIntent::from_message("run the tests"), UserIntent::Execute);
        assert_eq!(UserIntent::from_message("build the project"), UserIntent::Execute);
        assert_eq!(UserIntent::from_message("deploy the application"), UserIntent::Execute);
    }

    #[test]
    fn test_intent_search() {
        assert_eq!(UserIntent::from_message("search for function foo"), UserIntent::Search);
        assert_eq!(UserIntent::from_message("find where error is handled"), UserIntent::Search);
        assert_eq!(UserIntent::from_message("grep for TODO"), UserIntent::Search);
    }

    #[test]
    fn test_intent_write() {
        assert_eq!(UserIntent::from_message("create a new file"), UserIntent::Write);
        assert_eq!(UserIntent::from_message("write the configuration"), UserIntent::Write);
        assert_eq!(UserIntent::from_message("save the changes"), UserIntent::Write);
    }

    #[test]
    fn test_intent_analyze() {
        assert_eq!(UserIntent::from_message("analyze the code"), UserIntent::Analyze);
        assert_eq!(UserIntent::from_message("explain how it works"), UserIntent::Analyze);
        assert_eq!(UserIntent::from_message("review this function"), UserIntent::Analyze);
    }

    #[test]
    fn test_intent_ambiguous() {
        assert_eq!(UserIntent::from_message("hello there"), UserIntent::Ambiguous);
        assert_eq!(UserIntent::from_message("what about that"), UserIntent::Ambiguous);
    }

    #[test]
    fn test_tool_guidance_for_intent() {
        let guidance = ToolGuidance::for_intent(UserIntent::Explore);
        assert!(guidance.is_some());
        let g = guidance.unwrap();
        assert!(g.recommended_tools.contains(&"glob".to_string()));
    }

    #[test]
    fn test_tool_strategy_classify() {
        let strategy = ToolStrategy::new();
        assert_eq!(strategy.classify_intent("run cargo build"), UserIntent::Execute);
        assert_eq!(strategy.classify_intent("find the main function"), UserIntent::Search);
    }

    #[test]
    fn test_tool_strategy_patterns() {
        let strategy = ToolStrategy::new();
        let patterns = strategy.get_patterns(UserIntent::Explore);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_generate_strategy_prompt() {
        let strategy = ToolStrategy::new();
        let prompt = strategy.generate_strategy_prompt("explore the project structure");
        assert!(prompt.contains("Explore"));
        assert!(prompt.contains("glob"));
    }

    #[test]
    fn test_generate_strategy_prompt_ambiguous() {
        let strategy = ToolStrategy::new();
        let prompt = strategy.generate_strategy_prompt("hello");
        assert!(prompt.contains("clarifying"));
    }
}
