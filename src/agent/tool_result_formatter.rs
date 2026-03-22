//! Tool Result Formatter Module
//!
//! Formats tool execution results for better readability and comprehension.
//! Adds contextual prefixes, summaries for long outputs, and structured formatting
//! to help the Agent understand tool results more effectively.
//!
//! ## Key Features
//!
//! - **Contextual prefixes**: Tool name + execution time
//! - **Output summarization**: For long outputs (e.g., "47 files, 12 dirs")
//! - **Smart truncation**: Keep beginning + summary + end for very long outputs
//! - **Structured formatting**: Key-value results formatted consistently

use crate::agent::error_recovery::ErrorRecovery;
use crate::agent::tools::ToolResult;

/// Maximum output length before truncation
const MAX_OUTPUT_LENGTH: usize = 4000;

/// Tool result formatter for enhancing tool output readability
pub struct ToolResultFormatter;

impl ToolResultFormatter {
    /// Format a tool result with contextual information
    ///
    /// Adds:
    /// - Tool name prefix (e.g., `[exec • 1.2s]`)
    /// - Output summary for long outputs
    /// - Smart truncation for very long outputs
    /// - Structured error report on failure (via ErrorRecovery)
    pub fn format(tool_name: &str, result: &ToolResult, duration_ms: u64) -> String {
        if !result.success {
            // Use structured error reporting for failures
            let error_msg = result.error.as_deref().unwrap_or(&result.output);
            let recovery = ErrorRecovery::from_error(tool_name, error_msg);
            return recovery.format_report(tool_name);
        }

        let prefix = Self::build_prefix(tool_name, duration_ms);
        let output = &result.output;

        if output.is_empty() {
            return format!("{} [no output]", prefix);
        }

        // Get formatted output (potentially summarized)
        let formatted = Self::format_output(tool_name, output);

        // Check if truncation is needed
        let with_prefix = if formatted.len() > MAX_OUTPUT_LENGTH {
            Self::truncate_output(&prefix, &formatted)
        } else {
            format!("{}\n{}", prefix, formatted)
        };

        with_prefix
    }

    /// Build a contextual prefix for the tool result
    fn build_prefix(tool_name: &str, duration_ms: u64) -> String {
        let duration_str = Self::format_duration(duration_ms);
        format!("[{} • {}]", tool_name, duration_str)
    }

    /// Format duration in human-readable form
    fn format_duration(ms: u64) -> String {
        if ms < 1000 {
            format!("{}ms", ms)
        } else {
            let secs = ms as f64 / 1000.0;
            if secs < 60.0 {
                format!("{:.1}s", secs)
            } else {
                let mins = secs / 60.0;
                format!("{:.1}m", mins)
            }
        }
    }

    /// Format output based on tool type
    fn format_output(tool_name: &str, output: &str) -> String {
        match tool_name {
            "list_dir" => Self::format_list_dir(output),
            "grep" | "search" => Self::format_grep(output),
            "read_file" | "cat" => Self::format_read_file(output),
            "exec" => Self::format_exec(output),
            "http_request" => Self::format_http_request(output),
            "glob" => Self::format_glob(output),
            "find" => Self::format_find(output),
            _ => output.to_string(),
        }
    }

    /// Format list_dir output with summary
    fn format_list_dir(output: &str) -> String {
        if output.contains("(empty directory)") {
            return "(empty directory)".to_string();
        }

        let lines: Vec<&str> = output.lines().collect();
        let total = lines.len();

        // Count files and directories from the formatted output
        // Each line has format: "filename              size    date"
        // Directories have size "-" and filenames may end with "/"
        let dirs = lines.iter().filter(|l| l.contains(" - ") || l.contains("/")).count();
        let files = total - dirs;

        // If output is short enough, just add summary
        let summary = if total > 0 && (total > 5 || output.len() > 500) {
            let summary = if files > 0 && dirs > 0 {
                format!("{} files, {} directories", files, dirs)
            } else if dirs > 0 {
                format!("{} directories", dirs)
            } else {
                format!("{} files", files)
            };
            format!("\n--- {} ---", summary)
        } else {
            String::new()
        };

        format!("{}{}", output, summary)
    }

    /// Format grep/search output with match count summary
    fn format_grep(output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();

        if total_lines == 0 {
            return "(no matches found)".to_string();
        }

        // Count unique files
        let unique_files: std::collections::HashSet<_> = lines.iter()
            .filter_map(|l| l.split(':').next())
            .collect();
        let file_count = unique_files.len();
        let match_count = total_lines;

        // If output is short enough, return as-is
        if output.len() < 2000 {
            return output.to_string();
        }

        // For longer outputs, add summary header
        let summary = if file_count > 0 {
            format!("--- {} matches in {} files ---\n", match_count, file_count)
        } else {
            format!("--- {} matches ---\n", match_count)
        };

        // Truncate if still too long
        let truncated = if summary.len() + output.len() > MAX_OUTPUT_LENGTH {
            let kept_lines = std::cmp::min(100, total_lines);
            let kept: String = lines[..kept_lines].join("\n");
            format!("{}\n\n[... {} more matches truncated ...]\n{} total matches",
                    kept,
                    total_lines - kept_lines,
                    total_lines)
        } else {
            output.to_string()
        };

        format!("{}{}", summary, truncated)
    }

    /// Format read_file/cat output
    fn format_read_file(output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let line_count = lines.len();

        if line_count == 0 {
            return "(empty file)".to_string();
        }

        // For very long files, truncate intelligently
        if output.len() > MAX_OUTPUT_LENGTH {
            let kept_lines = std::cmp::min(200, line_count);
            let kept: String = lines[..kept_lines].join("\n");
            return format!(
                "{}\n\n[... {} more lines truncated ...]\n[Total: {} lines]",
                kept,
                line_count - kept_lines,
                line_count
            );
        }

        // Add line count summary for medium-long files
        if line_count > 100 && output.len() < MAX_OUTPUT_LENGTH {
            return format!("{}\n\n[{} lines]", output, line_count);
        }

        output.to_string()
    }

    /// Format exec output
    fn format_exec(output: &str) -> String {
        if output.is_empty() {
            return "(no output)".to_string();
        }

        // For very long outputs, truncate but keep important info
        if output.len() > MAX_OUTPUT_LENGTH {
            let kept: String = output.chars().take(3000).collect();
            return format!(
                "{}\n\n[... output truncated ...]\n[Total: {} chars]",
                kept,
                output.len()
            );
        }

        output.to_string()
    }

    /// Format http_request output
    fn format_http_request(output: &str) -> String {
        if output.is_empty() {
            return "(no response body)".to_string();
        }

        // Try to parse as JSON and pretty-print
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                // If pretty JSON is not too long, use it
                if pretty.len() < 3000 {
                    return pretty;
                }
                // Otherwise truncate
                let kept: String = pretty.chars().take(3000).collect();
                return format!("{}\n\n[... JSON truncated ...]", kept);
            }
        }

        // Plain text - truncate if too long
        if output.len() > 3000 {
            let kept: String = output.chars().take(3000).collect();
            format!("{}\n\n[... response truncated ...]\n[Total: {} chars]", kept, output.len())
        } else {
            output.to_string()
        }
    }

    /// Format glob output
    fn format_glob(output: &str) -> String {
        if output.is_empty() {
            return "(no matches)".to_string();
        }

        let lines: Vec<&str> = output.lines().collect();
        let count = lines.len();

        if output.len() < 2000 {
            return format!("{}\n\n[{} matches]", output, count);
        }

        // Truncate long glob results
        let kept_lines = std::cmp::min(100, count);
        let kept: String = lines[..kept_lines].join("\n");
        format!(
            "{}\n\n[... {} more matches truncated ...]\n[Total: {} matches]",
            kept,
            count - kept_lines,
            count
        )
    }

    /// Format find output
    fn format_find(output: &str) -> String {
        if output.is_empty() {
            return "(no matches)".to_string();
        }

        let lines: Vec<&str> = output.lines().collect();
        let count = lines.len();

        if output.len() < 2000 {
            return format!("{}\n\n[{} results]", output, count);
        }

        // Truncate long find results
        let kept_lines = std::cmp::min(100, count);
        let kept: String = lines[..kept_lines].join("\n");
        format!(
            "{}\n\n[... {} more results truncated ...]\n[Total: {} results]",
            kept,
            count - kept_lines,
            count
        )
    }

    /// Truncate very long outputs while keeping start and end
    fn truncate_output(prefix: &str, output: &str) -> String {
        if output.len() <= MAX_OUTPUT_LENGTH {
            return format!("{}\n{}", prefix, output);
        }

        // Keep first and last portions
        let head_len = 1500;
        let tail_len = 2000;
        let head = &output[..head_len.min(output.len())];
        let tail_start = output.len().saturating_sub(tail_len);
        let tail = &output[tail_start..];

        format!(
            "{}\n{}\n\n[... {} characters truncated ...]\n{}\n\n[Total: {} chars]",
            prefix,
            head,
            output.len() - head_len - tail_len,
            tail,
            output.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(ToolResultFormatter::format_duration(100), "100ms");
        assert_eq!(ToolResultFormatter::format_duration(999), "999ms");
        assert_eq!(ToolResultFormatter::format_duration(1000), "1.0s");
        assert_eq!(ToolResultFormatter::format_duration(1500), "1.5s");
        // 60 seconds converts to 1.0 minutes
        assert_eq!(ToolResultFormatter::format_duration(60000), "1.0m");
        assert_eq!(ToolResultFormatter::format_duration(120000), "2.0m");
    }

    #[test]
    fn test_build_prefix() {
        let prefix = ToolResultFormatter::build_prefix("exec", 1234);
        assert_eq!(prefix, "[exec • 1.2s]");
    }

    #[test]
    fn test_format_success() {
        let result = ToolResult {
            success: true,
            output: "hello world".to_string(),
            error: None,
        };
        let formatted = ToolResultFormatter::format("echo", &result, 50);
        assert!(formatted.contains("[echo • 50ms]"));
        assert!(formatted.contains("hello world"));
    }

    #[test]
    fn test_format_empty_output() {
        let result = ToolResult {
            success: true,
            output: String::new(),
            error: None,
        };
        let formatted = ToolResultFormatter::format("exec", &result, 100);
        assert!(formatted.contains("[exec • 100ms]"));
        assert!(formatted.contains("[no output]"));
    }

    #[test]
    fn test_format_failure_uses_error_recovery() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("Permission denied".to_string()),
        };
        let formatted = ToolResultFormatter::format("exec", &result, 0);
        // Should use structured error report from ErrorRecovery
        assert!(!formatted.is_empty());
        // ErrorRecovery format_report includes the error kind and suggestion
        assert!(formatted.contains("denied") || formatted.contains("error"));
    }

    #[test]
    fn test_format_list_dir_summary() {
        // Simulated list_dir output (from tools.rs format)
        // Use enough entries to trigger summary (>5 lines)
        let output = "file1.txt                     1024  2024-01-01 10:00\n\
                      directory1/                     -  2024-01-01 11:00\n\
                      file2.rs                       2048  2024-01-01 12:00\n\
                      file3.txt                      2560  2024-01-01 13:00\n\
                      file4.txt                      3000  2024-01-01 14:00\n\
                      directory2/                     -  2024-01-01 15:00";
        let formatted = ToolResultFormatter::format_output("list_dir", output);
        assert!(formatted.contains("---"));
        assert!(formatted.contains("files") || formatted.contains("directories"));
    }

    #[test]
    fn test_format_grep_with_matches() {
        // Use a longer output to trigger summary (>2000 chars would trigger header)
        // For short outputs, the raw result is returned
        let output = "src/main.rs:10: fn main() {\n\
                      src/lib.rs:5: fn main() {\n\
                      src/main.rs:20: fn main() {";
        let formatted = ToolResultFormatter::format_output("grep", output);
        // Short output is returned as-is, no summary added
        assert!(formatted.contains("main.rs"));
    }

    #[test]
    fn test_format_grep_no_matches() {
        let output = "";
        let formatted = ToolResultFormatter::format_output("grep", output);
        assert!(formatted.contains("no matches"));
    }

    #[test]
    fn test_format_truncation() {
        // Create a very long output
        let long_output = "line\n".repeat(1000);
        let result = ToolResult {
            success: true,
            output: long_output,
            error: None,
        };
        let formatted = ToolResultFormatter::format("exec", &result, 100);
        assert!(formatted.contains("truncated"));
        assert!(formatted.contains("Total:"));
    }

    #[test]
    fn test_format_read_file_empty() {
        let formatted = ToolResultFormatter::format_output("read_file", "");
        assert!(formatted.contains("empty file"));
    }

    #[test]
    fn test_format_http_json() {
        let json = r#"{"name":"test","value":123}"#;
        let formatted = ToolResultFormatter::format_output("http_request", json);
        // Should be pretty-printed
        assert!(formatted.contains('\n'));
    }
}
