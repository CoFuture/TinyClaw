//! Turn Summary Module
//!
//! Provides concise summaries of agent turns, capturing what was accomplished.
//! Helps users understand what the agent did, especially when multiple tools were called.
//!
//! ## Example Output
//!
//! ```text
//! 📋 Turn Summary
//! ├─ Actions: 3 tools called
//! │  ├─ read_file: Read package.json (142 lines)
//! │  ├─ grep: Found 5 matches for "dependencies"
//! │  └─ exec: Installed 2 new packages
//! ├─ Result: ✓ Task completed successfully
//! └─ Files: Modified package.json, Added node_modules/
//! ```

use serde::{Deserialize, Serialize};

/// A summary of a single tool execution within a turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionSummary {
    /// Tool name
    pub tool_name: String,
    /// Brief description of what the tool did
    pub summary: String,
    /// Whether the tool succeeded
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl ToolExecutionSummary {
    /// Create a new tool execution summary
    pub fn new(tool_name: String, summary: String, success: bool, duration_ms: u64) -> Self {
        Self {
            tool_name,
            summary,
            success,
            duration_ms,
        }
    }

    /// Generate a brief summary from tool name and result
    pub fn from_tool_result(tool_name: &str, output_preview: &str, success: bool, duration_ms: u64) -> Self {
        let summary = Self::generate_summary(tool_name, output_preview, success);
        Self::new(tool_name.to_string(), summary, success, duration_ms)
    }

    /// Generate a human-readable summary based on tool type and output
    fn generate_summary(tool_name: &str, output: &str, success: bool) -> String {
        if !success {
            return "Failed".to_string();
        }

        let output = output.trim();
        if output.is_empty() {
            return match tool_name {
                "exec" => "Command executed".to_string(),
                "read_file" | "cat" => "File read".to_string(),
                "write_file" => "File written".to_string(),
                "list_dir" => "Directory listed".to_string(),
                "grep" | "search" => "Search completed".to_string(),
                "http_request" => "Request completed".to_string(),
                _ => "Completed".to_string(),
            };
        }

        // Extract key information based on tool type
        match tool_name {
            "exec" => {
                // For exec, show first line of output or truncate
                let first_line = output.lines().next().unwrap_or(output);
                if first_line.len() > 60 {
                    first_line[..60].to_string()
                } else {
                    first_line.to_string()
                }
            }
            "read_file" | "cat" => {
                // Show file info - number of lines or size
                let lines = output.lines().count();
                if lines > 1 {
                    format!("{} lines", lines)
                } else if output.len() > 60 {
                    format!("{}...", &output[..57])
                } else {
                    output.to_string()
                }
            }
            "write_file" => {
                // Show file path if present
                let first_line = output.lines().next().unwrap_or(output);
                format!("Wrote to {}", first_line)
            }
            "list_dir" => {
                // Extract counts from summary if present
                if output.contains("files") || output.contains("directories") {
                    // Extract the summary line
                    output.lines().last().map(|l| l.trim().to_string()).unwrap_or_else(|| "Listed directory".to_string())
                } else {
                    "Listed directory".to_string()
                }
            }
            "grep" | "search" => {
                // Show match count
                if output.contains("matches") || output.contains("match") {
                    output.lines().next().map(|l| l.trim().to_string()).unwrap_or_else(|| "Found matches".to_string())
                } else {
                    let lines = output.lines().count();
                    format!("{} matches", lines)
                }
            }
            "http_request" => {
                // Show status or response info
                let first_line = output.lines().next().unwrap_or(output);
                if first_line.len() > 60 {
                    first_line[..60].to_string()
                } else {
                    first_line.to_string()
                }
            }
            _ => {
                // Generic - show first line or truncate
                let first_line = output.lines().next().unwrap_or(output);
                if first_line.len() > 60 {
                    first_line[..60].to_string()
                } else {
                    first_line.to_string()
                }
            }
        }
    }
}

/// A summary of an entire agent turn (renamed to avoid conflict with turn_history::TurnSummary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnSummary {
    /// Session ID
    pub session_id: String,
    /// Turn ID
    pub turn_id: String,
    /// Number of tools called
    pub tool_count: usize,
    /// Summaries of tool executions
    pub tool_summaries: Vec<ToolExecutionSummary>,
    /// Whether the turn was successful
    pub success: bool,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Brief description of what was accomplished
    pub accomplishment: String,
    /// Files or resources mentioned as affected
    pub affected_resources: Vec<String>,
}

impl AgentTurnSummary {
    /// Create a new turn summary
    pub fn new(
        session_id: String,
        turn_id: String,
        tool_count: usize,
        tool_summaries: Vec<ToolExecutionSummary>,
        success: bool,
        total_duration_ms: u64,
    ) -> Self {
        let accomplishment = Self::generate_accomplishment(&tool_summaries, success);
        let affected_resources = Self::extract_resources(&tool_summaries);
        
        Self {
            session_id,
            turn_id,
            tool_count,
            tool_summaries,
            success,
            total_duration_ms,
            accomplishment,
            affected_resources,
        }
    }

    /// Generate a brief description of what was accomplished
    fn generate_accomplishment(summaries: &[ToolExecutionSummary], success: bool) -> String {
        if summaries.is_empty() {
            if success {
                "Provided a response".to_string()
            } else {
                "Failed to complete request".to_string()
            }
        } else {
            let successful_count = summaries.iter().filter(|s| s.success).count();
            let total_count = summaries.len();

            if total_count == 1 {
                let tool = &summaries[0].tool_name;
                let summary = &summaries[0].summary;
                if summaries[0].success {
                    match tool.as_str() {
                        "read_file" | "cat" => format!("Read: {}", summary),
                        "write_file" => format!("Wrote: {}", summary),
                        "exec" => format!("Executed: {}", summary),
                        "grep" | "search" => format!("Searched: {}", summary),
                        "list_dir" => format!("Listed: {}", summary),
                        "http_request" => format!("Requested: {}", summary),
                        _ => format!("Completed: {}", summary),
                    }
                } else {
                    "Action failed".to_string()
                }
            } else {
                let tool_names: Vec<_> = summaries.iter()
                    .map(|s| s.tool_name.as_str())
                    .collect();
                let unique_tools: Vec<_> = tool_names.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
                
                if successful_count == total_count {
                    format!("Completed {} actions ({})", total_count, unique_tools.join(", "))
                } else {
                    format!("Partial: {}/{} succeeded", successful_count, total_count)
                }
            }
        }
    }

    /// Extract affected resources (files, directories, URLs) from tool summaries
    fn extract_resources(summaries: &[ToolExecutionSummary]) -> Vec<String> {
        let mut resources = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for summary in summaries {
            let text = &summary.summary;
            
            // Look for file paths (simple heuristic: contains / or ends with common extensions)
            for part in text.split_whitespace() {
                if part.starts_with('/') || part.starts_with("./") || part.starts_with("../") {
                    let normalized = part.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');
                    if !normalized.is_empty() && !seen.contains(normalized) {
                        seen.insert(normalized.to_string());
                        resources.push(normalized.to_string());
                    }
                }
                // Check for common file extensions
                if part.ends_with(".rs") || part.ends_with(".js") || part.ends_with(".ts") 
                    || part.ends_with(".json") || part.ends_with(".md") || part.ends_with(".txt")
                    || part.ends_with(".yaml") || part.ends_with(".yml") || part.ends_with(".toml")
                    || part.ends_with(".html") || part.ends_with(".css") || part.ends_with(".py")
                {
                    let normalized = part.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');
                    if !normalized.is_empty() && !seen.contains(normalized) {
                        seen.insert(normalized.to_string());
                        resources.push(normalized.to_string());
                    }
                }
            }
        }

        resources
    }

    /// Format the summary for display
    #[allow(dead_code)]
    pub fn to_display_string(&self) -> String {
        let mut lines = Vec::new();
        
        lines.push("📋 Turn Summary".to_string());
        lines.push(format!("├─ Actions: {} tool{}", self.tool_count, if self.tool_count == 1 { "" } else { "s" }));
        
        for (i, tool) in self.tool_summaries.iter().enumerate() {
            let prefix = if i == self.tool_summaries.len() - 1 { "└─" } else { "│  ├─" };
            lines.push(format!("{} {}: {} [{}ms]", prefix, tool.tool_name, tool.summary, tool.duration_ms));
        }
        
        lines.push(format!("├─ Result: {}", if self.success { "✓ Success" } else { "✗ Failed" }));
        
        if !self.affected_resources.is_empty() {
            lines.push(format!("└─ Resources: {}", self.affected_resources.join(", ")));
        }
        
        lines.join("\n")
    }
}

/// Generate a turn summary from turn record data
pub fn generate_turn_summary(
    session_id: &str,
    turn_id: &str,
    tool_executions: &[(String, String, bool, u64)], // (tool_name, output_preview, success, duration_ms)
    success: bool,
    total_duration_ms: u64,
) -> AgentTurnSummary {
    let tool_summaries: Vec<ToolExecutionSummary> = tool_executions
        .iter()
        .map(|(name, output, succ, dur)| {
            ToolExecutionSummary::from_tool_result(name, output, *succ, *dur)
        })
        .collect();

    AgentTurnSummary::new(
        session_id.to_string(),
        turn_id.to_string(),
        tool_executions.len(),
        tool_summaries,
        success,
        total_duration_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_execution_summary_from_result() {
        let summary = ToolExecutionSummary::from_tool_result(
            "read_file",
            "package.json content here...",
            true,
            150,
        );
        
        assert_eq!(summary.tool_name, "read_file");
        assert!(summary.success);
        assert_eq!(summary.duration_ms, 150);
    }

    #[test]
    fn test_tool_execution_summary_failed() {
        let summary = ToolExecutionSummary::from_tool_result(
            "exec",
            "Permission denied",
            false,
            50,
        );
        
        assert!(!summary.success);
        assert_eq!(summary.summary, "Failed");
    }

    #[test]
    fn test_turn_summary_single_tool() {
        let tool_summaries = vec![
            ToolExecutionSummary::from_tool_result("read_file", "142 lines of content", true, 100),
        ];
        
        let summary = AgentTurnSummary::new(
            "test-session".to_string(),
            "turn-1".to_string(),
            1,
            tool_summaries,
            true,
            100,
        );
        
        assert_eq!(summary.tool_count, 1);
        assert!(summary.success);
        assert!(summary.accomplishment.contains("Read"));
    }

    #[test]
    fn test_turn_summary_multiple_tools() {
        let tool_summaries = vec![
            ToolExecutionSummary::from_tool_result("read_file", "package.json", true, 50),
            ToolExecutionSummary::from_tool_result("exec", "npm install completed", true, 2000),
            ToolExecutionSummary::from_tool_result("grep", "5 matches found", true, 100),
        ];
        
        let summary = AgentTurnSummary::new(
            "test-session".to_string(),
            "turn-1".to_string(),
            3,
            tool_summaries,
            true,
            2150,
        );
        
        assert_eq!(summary.tool_count, 3);
        assert!(summary.success);
        assert!(summary.accomplishment.contains("3 actions"));
    }

    #[test]
    fn test_turn_summary_extracts_resources() {
        let tool_summaries = vec![
            ToolExecutionSummary::from_tool_result("read_file", "/path/to/package.json content", true, 50),
            ToolExecutionSummary::from_tool_result("exec", "Installing dependencies...", true, 2000),
        ];
        
        let summary = AgentTurnSummary::new(
            "test-session".to_string(),
            "turn-1".to_string(),
            2,
            tool_summaries,
            true,
            2050,
        );
        
        assert!(!summary.affected_resources.is_empty());
        assert!(summary.affected_resources.iter().any(|r| r.contains("package.json")));
    }

    #[test]
    fn test_turn_summary_no_tools() {
        let summary = AgentTurnSummary::new(
            "test-session".to_string(),
            "turn-1".to_string(),
            0,
            vec![],
            true,
            100,
        );
        
        assert_eq!(summary.tool_count, 0);
        assert!(summary.accomplishment.contains("Provided a response"));
    }

    #[test]
    fn test_generate_turn_summary() {
        let tool_executions = vec![
            ("read_file".to_string(), "file.rs content".to_string(), true, 100u64),
            ("grep".to_string(), "Found 3 matches".to_string(), true, 50u64),
        ];
        
        let summary = generate_turn_summary(
            "session-1",
            "turn-1",
            &tool_executions,
            true,
            150,
        );
        
        assert_eq!(summary.session_id, "session-1");
        assert_eq!(summary.turn_id, "turn-1");
        assert_eq!(summary.tool_count, 2);
        assert!(summary.success);
    }

    #[test]
    fn test_display_string() {
        let tool_summaries = vec![
            ToolExecutionSummary::from_tool_result("read_file", "100 lines", true, 100),
        ];
        
        let summary = AgentTurnSummary::new(
            "test".to_string(),
            "turn-1".to_string(),
            1,
            tool_summaries,
            true,
            100,
        );
        
        let display = summary.to_display_string();
        assert!(display.contains("📋 Turn Summary"));
        assert!(display.contains("read_file"));
        assert!(display.contains("✓ Success"));
    }
}
