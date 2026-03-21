//! Advanced tools module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::fs;
use tokio::time::timeout;
use tracing::info;
use chrono::{DateTime, Local};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Tool executor
#[derive(Default)]
pub struct ToolExecutor {
    tools: HashMap<String, Tool>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        let mut tools = HashMap::new();

        // Register exec tool
        tools.insert(
            "exec".to_string(),
            Tool {
                name: "exec".to_string(),
                description: "Execute a shell command".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to execute"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Timeout in milliseconds"
                        }
                    },
                    "required": ["command"]
                }),
            },
        );

        // Register read_file tool
        tools.insert(
            "read_file".to_string(),
            Tool {
                name: "read_file".to_string(),
                description: "Read contents of a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        },
                        "max_bytes": {
                            "type": "number",
                            "description": "Maximum bytes to read"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register write_file tool
        tools.insert(
            "write_file".to_string(),
            Tool {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
        );

        // Register list_dir tool
        tools.insert(
            "list_dir".to_string(),
            Tool {
                name: "list_dir".to_string(),
                description: "List contents of a directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory (defaults to current directory)"
                        }
                    }
                }),
            },
        );

        // Register http_request tool
        tools.insert(
            "http_request".to_string(),
            Tool {
                name: "http_request".to_string(),
                description: "Make an HTTP request".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to request"
                        },
                        "method": {
                            "type": "string",
                            "description": "HTTP method (GET, POST, etc.)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Request headers"
                        },
                        "body": {
                            "type": "string",
                            "description": "Request body"
                        }
                    },
                    "required": ["url"]
                }),
            },
        );

        // Register glob tool
        tools.insert(
            "glob".to_string(),
            Tool {
                name: "glob".to_string(),
                description: "Find files matching a glob pattern".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern (e.g., '**/*.rs', 'src/**/*.ts')"
                        },
                        "root": {
                            "type": "string",
                            "description": "Root directory to search from (default: current directory)"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
        );

        // Register grep tool
        tools.insert(
            "grep".to_string(),
            Tool {
                name: "grep".to_string(),
                description: "Search for text patterns in files".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Pattern or regex to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory or file to search in"
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "Whether search is case-sensitive (default: true)"
                        },
                        "regex": {
                            "type": "boolean",
                            "description": "Treat pattern as regex (default: false)"
                        },
                        "max_results": {
                            "type": "number",
                            "description": "Maximum number of results to return (default: 100)"
                        }
                    },
                    "required": ["pattern", "path"]
                }),
            },
        );

        // Register sed_file tool (replace lines in a file)
        tools.insert(
            "sed_file".to_string(),
            Tool {
                name: "sed_file".to_string(),
                description: "Replace specific lines in a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to edit"
                        },
                        "old_text": {
                            "type": "string",
                            "description": "Text to search for (first occurrence will be replaced)"
                        },
                        "new_text": {
                            "type": "string",
                            "description": "Text to replace with"
                        },
                        "line_number": {
                            "type": "number",
                            "description": "Specific line number to replace (1-indexed, takes precedence over old_text)"
                        }
                    },
                    "required": ["path", "new_text"]
                }),
            },
        );

        // Register which tool (find executable in PATH)
        tools.insert(
            "which".to_string(),
            Tool {
                name: "which".to_string(),
                description: "Find an executable in the system PATH".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Name of the executable to find"
                        }
                    },
                    "required": ["command"]
                }),
            },
        );

        // Register mkdir tool (create directories)
        tools.insert(
            "mkdir".to_string(),
            Tool {
                name: "mkdir".to_string(),
                description: "Create one or more directories".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory to create (supports ~ and $VAR)"
                        },
                        "parents": {
                            "type": "boolean",
                            "description": "Create parent directories as needed (default: true)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register stat_file tool (get file metadata)
        tools.insert(
            "stat_file".to_string(),
            Tool {
                name: "stat_file".to_string(),
                description: "Get file or directory metadata (size, modified time, permissions)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file or directory (supports ~ and $VAR)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register find tool (find files by name)
        tools.insert(
            "find".to_string(),
            Tool {
                name: "find".to_string(),
                description: "Find files and directories by name pattern".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "File/directory name to search for (supports * wildcards)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Root directory to search from (default: current directory)"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum directory depth to search (default: 10)"
                        },
                        "type": {
                            "type": "string",
                            "description": "Filter by type: 'f' (files), 'd' (directories), 'a' (all, default)"
                        }
                    },
                    "required": ["name"]
                }),
            },
        );

        // Register tail tool (read last N lines of a file)
        tools.insert(
            "tail".to_string(),
            Tool {
                name: "tail".to_string(),
                description: "Read the last N lines of a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file (supports ~ and $VAR)"
                        },
                        "lines": {
                            "type": "number",
                            "description": "Number of lines to read from the end (default: 10)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register batch_execute tool (execute multiple tools)
        tools.insert(
            "batch_execute".to_string(),
            Tool {
                name: "batch_execute".to_string(),
                description: "Execute multiple tools in sequence".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "description": "Array of tool calls, each with 'name' and 'input' fields"
                        }
                    },
                    "required": ["tools"]
                }),
            },
        );

        // Register env tool (get/set environment variables)
        tools.insert(
            "env".to_string(),
            Tool {
                name: "env".to_string(),
                description: "Get or set environment variables".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Environment variable name (omit to list all)"
                        },
                        "value": {
                            "type": "string",
                            "description": "Value to set (omit to get value, set to empty string to unset)"
                        }
                    }
                }),
            },
        );

        // Register diff tool (compare two files)
        tools.insert(
            "diff".to_string(),
            Tool {
                name: "diff".to_string(),
                description: "Compare two files and show differences".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path1": {
                            "type": "string",
                            "description": "First file path (supports ~ and $VAR)"
                        },
                        "path2": {
                            "type": "string",
                            "description": "Second file path (supports ~ and $VAR)"
                        }
                    },
                    "required": ["path1", "path2"]
                }),
            },
        );

        // Register cp tool (copy file)
        tools.insert(
            "cp".to_string(),
            Tool {
                name: "cp".to_string(),
                description: "Copy a file to a new location".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source file path (supports ~ and $VAR)"
                        },
                        "dest": {
                            "type": "string",
                            "description": "Destination file path (supports ~ and $VAR)"
                        }
                    },
                    "required": ["source", "dest"]
                }),
            },
        );

        // Register mv tool (move file)
        tools.insert(
            "mv".to_string(),
            Tool {
                name: "mv".to_string(),
                description: "Move a file to a new location (rename)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source file path (supports ~ and $VAR)"
                        },
                        "dest": {
                            "type": "string",
                            "description": "Destination file path (supports ~ and $VAR)"
                        }
                    },
                    "required": ["source", "dest"]
                }),
            },
        );

        // Register rm tool (remove file)
        tools.insert(
            "rm".to_string(),
            Tool {
                name: "rm".to_string(),
                description: "Remove a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to remove (supports ~ and $VAR)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register cat tool (concatenate files)
        tools.insert(
            "cat".to_string(),
            Tool {
                name: "cat".to_string(),
                description: "Read and concatenate multiple files".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "List of file paths to read (supports ~ and $VAR)"
                        },
                        "show_line_numbers": {
                            "type": "boolean",
                            "description": "Whether to show line numbers (default: false)"
                        }
                    },
                    "required": ["paths"]
                }),
            },
        );

        // Register tree tool (directory tree visualization)
        tools.insert(
            "tree".to_string(),
            Tool {
                name: "tree".to_string(),
                description: "Display directory tree structure".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root directory path (supports ~ and $VAR, default: .)"
                        },
                        "depth": {
                            "type": "number",
                            "description": "Maximum depth to traverse (default: 3)"
                        },
                        "show_hidden": {
                            "type": "boolean",
                            "description": "Whether to show hidden files (default: false)"
                        }
                    },
                    "required": []
                }),
            },
        );

        // Register chmod tool (change file permissions)
        tools.insert(
            "chmod".to_string(),
            Tool {
                name: "chmod".to_string(),
                description: "Change file permissions".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File or directory path (supports ~ and $VAR)"
                        },
                        "mode": {
                            "type": "string",
                            "description": "Permission mode in octal (e.g., '755') or symbolic (e.g., '+x')"
                        }
                    },
                    "required": ["path", "mode"]
                }),
            },
        );

        // Register hash tool (compute file hash)
        tools.insert(
            "hash".to_string(),
            Tool {
                name: "hash".to_string(),
                description: "Compute file hash/checksum".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to hash (supports ~ and $VAR)"
                        },
                        "algorithm": {
                            "type": "string",
                            "description": "Hash algorithm: sha256, sha512, sha1, md5 (default: sha256)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        // Register wc tool (word/line/char count)
        tools.insert(
            "wc".to_string(),
            Tool {
                name: "wc".to_string(),
                description: "Count lines, words, and characters in a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path (supports ~ and $VAR)"
                        },
                        "bytes": {
                            "type": "boolean",
                            "description": "Show byte count (default: true)"
                        },
                        "chars": {
                            "type": "boolean",
                            "description": "Show character count (default: false)"
                        },
                        "lines": {
                            "type": "boolean",
                            "description": "Show line count (default: true)"
                        },
                        "words": {
                            "type": "boolean",
                            "description": "Show word count (default: false)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        );

        Self { tools }
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    /// Get a tool by name
    #[allow(dead_code)]
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// Execute a tool with schema validation
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> ToolResult {
        // First validate input against schema
        if let Some(validation_error) = self.validate_input(name, &input) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(validation_error),
            };
        }

        match name {
            "exec" => self.execute_exec(input).await,
            "read_file" => self.execute_read_file(input).await,
            "write_file" => self.execute_write_file(input).await,
            "list_dir" => self.execute_list_dir(input).await,
            "http_request" => self.execute_http_request(input).await,
            "glob" => self.execute_glob(input).await,
            "grep" => self.execute_grep(input).await,
            "sed_file" => self.execute_sed_file(input).await,
            "which" => self.execute_which(input).await,
            "mkdir" => self.execute_mkdir(input).await,
            "stat_file" => self.execute_stat_file(input).await,
            "find" => self.execute_find(input).await,
            "tail" => self.execute_tail(input).await,
            "batch_execute" => self.execute_batch_execute(input).await,
            "env" => self.execute_env(input).await,
            "diff" => self.execute_diff(input).await,
            "cp" => self.execute_cp(input).await,
            "mv" => self.execute_mv(input).await,
            "rm" => self.execute_rm(input).await,
            "cat" => self.execute_cat(input).await,
            "tree" => self.execute_tree(input).await,
            "chmod" => self.execute_chmod(input).await,
            "hash" => self.execute_hash(input).await,
            "wc" => self.execute_wc(input).await,
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", name)),
            },
        }
    }

    /// Validate tool input against its schema
    fn validate_input(&self, name: &str, input: &serde_json::Value) -> Option<String> {
        let tool = self.tools.get(name)?;

        // Get the input_schema
        let schema = tool.input_schema.get("properties")?.as_object()?;

        // Get required fields
        let required = tool.input_schema.get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        // Check each required field is present
        let input_obj = input.as_object()?;

        for field in required {
            if !input_obj.contains_key(field) {
                return Some(format!("Missing required field '{}' for tool '{}'", field, name));
            }
        }

        // Type validation for present fields
        for (field, expected_type) in schema {
            if let Some(value) = input_obj.get(field) {
                if let Some(type_str) = expected_type.get("type").and_then(|t| t.as_str()) {
                    let valid = match type_str {
                        "string" => value.is_string(),
                        "number" => value.is_number(),
                        "boolean" => value.is_boolean(),
                        "object" => value.is_object(),
                        "array" => value.is_array(),
                        "null" => value.is_null(),
                        _ => true, // Unknown type, skip validation
                    };
                    if !valid {
                        return Some(format!(
                            "Invalid type for field '{}': expected {}, got {:?}",
                            field, type_str, value
                        ));
                    }
                }
            }
        }

        None
    }

    /// Normalize a path by expanding ~ to home directory and $VAR env vars.
    fn normalize_path(path: &str) -> String {
        if path.is_empty() {
            return path.to_string();
        }
        // Expand ~ to home directory
        let expanded = if path.starts_with("~/") || path == "~" {
            if let Some(home) = dirs::home_dir() {
                if path == "~" {
                    home.to_string_lossy().to_string()
                } else {
                    format!("{}{}", home.to_string_lossy(), &path[1..])
                }
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };
        // Expand environment variables ($VAR or ${VAR} forms)
        Self::expand_env_vars(&expanded)
    }

    /// Expand environment variables in a string ($VAR and ${VAR} forms).
    fn expand_env_vars(s: &str) -> String {
        let mut result = s.to_string();
        // Handle ${VAR} form
        while let Some(rel_start) = result.find("${") {
            let start = rel_start;
            if let Some(rel_end) = result[start..].find('}') {
                let close_pos = start + rel_end;
                let var_name = &result[start + 2..close_pos];
                let replacement = std::env::var(var_name).unwrap_or_default();
                result = format!("{}{}{}", &result[..start], replacement, &result[close_pos + 1..]);
            } else {
                break;
            }
        }
        // Handle $VAR form (alphanumeric and underscore)
        let mut i = 0;
        let bytes = result.as_bytes();
        let mut output = String::new();
        while i < bytes.len() {
            if bytes[i] == b'$' && i + 1 < bytes.len() && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                    end += 1;
                }
                let var_name = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                output.push_str(&std::env::var(var_name).unwrap_or_default());
                i = end;
            } else {
                output.push(bytes[i] as char);
                i += 1;
            }
        }
        output
    }

    /// Execute a tool with timeout (in milliseconds)
    /// Set timeout_ms to None for no timeout, or Some(ms) for custom timeout
    /// Default timeout is 30 seconds if not specified
    #[allow(dead_code)]
    pub async fn execute_with_timeout(&self, name: &str, input: serde_json::Value, timeout_ms: Option<u64>) -> ToolResult {
        if let Some(ms) = timeout_ms {
            let result = timeout(Duration::from_millis(ms), self.execute(name, input)).await;
            match result {
                Ok(tool_result) => tool_result,
                Err(_) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Tool execution timed out after {}ms", ms)),
                },
            }
        } else {
            // No timeout
            self.execute(name, input).await
        }
    }

    /// Execute the exec tool
    async fn execute_exec(&self, input: serde_json::Value) -> ToolResult {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if command.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("command is required".to_string()),
            };
        }

        let timeout_ms = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000); // Default 30 second timeout

        info!("Executing command: {} (timeout: {}ms)", command, timeout_ms);

        let command = command.to_string();
        let future = async {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
        };

        let output = timeout(Duration::from_millis(timeout_ms), future).await;

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                ToolResult {
                    success: output.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                }
            }
            Ok(Err(e)) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
            Err(_) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Command timed out after {}ms", timeout_ms)),
            },
        }
    }

    /// Execute the read_file tool
    async fn execute_read_file(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("Reading file: {}", path);

        match fs::read_to_string(&path).await {
            Ok(content) => ToolResult {
                success: true,
                output: content,
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Execute the write_file tool
    async fn execute_write_file(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("Writing file: {}", path);

        match fs::write(&path, content).await {
            Ok(()) => ToolResult {
                success: true,
                output: format!("File written successfully: {}", path),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Format a file size in bytes to a human-readable string.
    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if bytes >= GB {
            format!("{:.1}G", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1}M", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1}K", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }

    /// Execute the list_dir tool
    async fn execute_list_dir(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let path = Self::normalize_path(path);
        info!("Listing directory: {}", path);

        match fs::read_dir(&path).await {
            Ok(mut entries) => {
                let mut results = Vec::new();
                while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let metadata = entry.metadata().await.ok();
                    
                    let file_type = metadata.as_ref().map(|m| {
                        if m.is_dir() {
                            "dir"
                        } else if m.is_file() {
                            "file"
                        } else if m.is_symlink() {
                            "symlink"
                        } else {
                            "unknown"
                        }
                    }).unwrap_or("unknown");

                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    
                    let modified = metadata.and_then(|m| {
                        m.modified().ok()
                    }).map(|t| {
                        let datetime: DateTime<Local> = t.into();
                        datetime.format("%Y-%m-%d %H:%M").to_string()
                    }).unwrap_or_else(|| "-".to_string());

                    // Format size
                    let size_str = if file_type == "dir" || size == 0 {
                        "-".to_string()
                    } else {
                        Self::format_size(size)
                    };

                    results.push(format!("{:<40} {:>8}  {}", file_name, size_str, modified));
                }
                
                if results.is_empty() {
                    return ToolResult {
                        success: true,
                        output: "(empty directory)".to_string(),
                        error: None,
                    };
                }
                
                // Sort: dirs first, then files, both alphabetically
                results.sort_by(|a, b| {
                    let a_is_dir = a.contains("dir");
                    let b_is_dir = b.contains("dir");
                    if a_is_dir != b_is_dir {
                        b_is_dir.cmp(&a_is_dir)
                    } else {
                        a.cmp(b)
                    }
                });
                
                ToolResult {
                    success: true,
                    output: results.join("\n"),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Execute the http_request tool
    async fn execute_http_request(&self, input: serde_json::Value) -> ToolResult {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if url.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("url is required".to_string()),
            };
        }

        let method = input
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        info!("Making HTTP {} request to: {}", method, url);

        let client = reqwest::Client::new();
        
        let request_builder = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported method: {}", method)),
                };
            }
        };

        // Add headers if provided
        if let Some(headers) = input.get("headers").and_then(|v| v.as_object()) {
            for (_key, value) in headers {
                if let Some(_value_str) = value.as_str() {
                    // Skip invalid headers
                }
            }
        }

        // Add body if provided
        let request_builder = if let Some(body) = input.get("body").and_then(|v| v.as_str()) {
            request_builder.body(body.to_string())
        } else {
            request_builder
        };

        match request_builder.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let success = response.status().is_success();
                let body = response.text().await.unwrap_or_default();
                
                ToolResult {
                    success,
                    output: format!("Status: {}\n\n{}", status, body),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Convert a simple glob pattern to a regex
    fn pattern_to_regex(pattern: &str) -> Result<regex::Regex, String> {
        let mut regex_str = String::from("^");
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            match chars[i] {
                '*' => {
                    // ** matches everything including /
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        regex_str.push_str(".*");
                        i += 2;
                    } else {
                        regex_str.push_str("[^/]*");
                        i += 1;
                    }
                }
                '?' => {
                    regex_str.push('.');
                    i += 1;
                }
                '.' | '+' | '^' | '$' | '(' | ')' | '|' | '[' | ']' | '{' | '}' => {
                    regex_str.push('\\');
                    regex_str.push(chars[i]);
                    i += 1;
                }
                c => {
                    regex_str.push(c);
                    i += 1;
                }
            }
        }
        regex_str.push('$');
        
        regex::Regex::new(&regex_str)
            .map_err(|e| format!("Invalid pattern: {}", e))
    }

    /// Execute the glob tool
    async fn execute_glob(&self, input: serde_json::Value) -> ToolResult {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if pattern.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("pattern is required".to_string()),
            };
        }

        let root = input
            .get("root")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        info!("Glob pattern '{}' in root '{}'", pattern, root);

        // Use walkdir for glob-like matching
        let is_recursive = pattern.contains("**");

        let mut results = Vec::new();

        if is_recursive {
            // Handle ** patterns recursively
            match glob::glob(&format!("{}/{}", root, pattern)) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        results.push(entry.display().to_string());
                    }
                }
                Err(e) => {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Glob error: {}", e)),
                    };
                }
            }
        } else {
            // Simple non-recursive case - use walkdir with max depth
            let walker = walkdir::WalkDir::new(root).max_depth(if pattern.contains('*') { 10 } else { 1 });
            let re: regex::Regex = match Self::pattern_to_regex(pattern) {
                Ok(r) => r,
                Err(e) => {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    }
                }
            };
            
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let file_name = entry.file_name().to_string_lossy();
                if re.is_match(&file_name) {
                    results.push(entry.path().display().to_string());
                }
            }
        }

        results.sort();
        results.dedup();

        ToolResult {
            success: true,
            output: if results.is_empty() {
                "(no matches)".to_string()
            } else {
                results.join("\n")
            },
            error: None,
        }
    }

    /// Execute the grep tool
    async fn execute_grep(&self, input: serde_json::Value) -> ToolResult {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        if pattern.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("pattern is required".to_string()),
            };
        }

        let case_sensitive = input
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let _is_regex = input
            .get("regex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let max_results = input
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        info!("Grep pattern '{}' in '{}' (case_sensitive={})", pattern, path, case_sensitive);

        let mut matches = Vec::new();
        let walker = walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in walker {
            if matches.len() >= max_results {
                break;
            }

            let file_path = entry.path();
            
            // Skip binary files and very large files
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() > 1_000_000 {
                    continue; // Skip files > 1MB
                }
            }

            // Try to read as text
            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                let (search_content, actual_pattern): (&str, String);
                
                if case_sensitive {
                    actual_pattern = pattern.to_string();
                    search_content = &content;
                } else {
                    actual_pattern = pattern.to_lowercase();
                    let lower = content.to_lowercase();
                    search_content = Box::leak(lower.into_boxed_str());
                }

                for (line_num, line) in search_content.lines().enumerate() {
                    if line.contains(&actual_pattern) {
                        let original_line = content.lines().nth(line_num).unwrap_or("");
                        matches.push(format!(
                            "{}:{}: {}",
                            file_path.display(),
                            line_num + 1,
                            original_line
                        ));
                        if matches.len() >= max_results {
                            break;
                        }
                    }
                }
            }
        }

        ToolResult {
            success: true,
            output: if matches.is_empty() {
                "(no matches)".to_string()
            } else {
                matches.join("\n")
            },
            error: None,
        }
    }

    /// Execute the sed_file tool - replace lines in a file
    async fn execute_sed_file(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");

        let new_text = input
            .get("new_text")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        let line_number = input
            .get("line_number")
            .and_then(|v: &serde_json::Value| v.as_u64())
            .map(|n| n as usize);

        let old_text = input
            .get("old_text")
            .and_then(|v: &serde_json::Value| v.as_str());

        info!("sed_file: path='{}', line_number={:?}, old_text={:?}, new_text has {} chars",
            path, line_number, old_text.as_ref().map(|s| &s[..s.len().min(50)]), new_text.len());

        // Read the file
        let content = match fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file: {}", e)),
                };
            }
        };

        let new_content = if let Some(lineno) = line_number {
            // Replace by line number (1-indexed)
            let mut lines: Vec<&str> = content.lines().collect();
            if lineno == 0 || lineno > lines.len() {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Line number {} out of range (file has {} lines)", lineno, lines.len())),
                };
            }
            lines[lineno - 1] = new_text;
            lines.join("\n")
        } else if let Some(needle) = old_text {
            // Replace first occurrence of old_text
            if let Some(pos) = content.find(needle) {
                let mut new_content = content.clone();
                new_content.replace_range(pos..pos + needle.len(), new_text);
                new_content
            } else {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Text '{}' not found in file", &needle[..needle.len().min(50)])),
                };
            }
        } else {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Either 'line_number' or 'old_text' must be provided".to_string()),
            };
        };

        // Write back
        if let Err(e) = fs::write(&path, &new_content).await {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to write file: {}", e)),
            };
        }

        ToolResult {
            success: true,
            output: "File updated successfully".to_string(),
            error: None,
        }
    }

    /// Execute the which tool - find executable in PATH
    async fn execute_which(&self, input: serde_json::Value) -> ToolResult {
        let command = input
            .get("command")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");

        if command.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("command is required".to_string()),
            };
        }

        info!("which: finding '{}' in PATH", command);

        // Use std::env::split_paths to search PATH
        let path_var = std::env::var("PATH").unwrap_or_default();
        let candidates: Vec<std::path::PathBuf> = std::env::split_paths(&path_var).collect();

        for dir in candidates {
            let candidate = dir.join(command);
            // Check if file exists and is executable
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = candidate.metadata() {
                        let mode = metadata.permissions().mode();
                        if mode & 0o111 != 0 {
                            return ToolResult {
                                success: true,
                                output: candidate.display().to_string(),
                                error: None,
                            };
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    return ToolResult {
                        success: true,
                        output: candidate.display().to_string(),
                        error: None,
                    };
                }
            }
        }

        // Not found - return success=false
        ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("'{}' not found in PATH", command)),
        }
    }

    /// Execute the mkdir tool - create directories
    async fn execute_mkdir(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        let parents = input
            .get("parents")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        info!("mkdir: path='{}', parents={}", path, parents);

        if parents {
            match fs::create_dir_all(&path).await {
                Ok(()) => ToolResult {
                    success: true,
                    output: format!("Directory created: {}", path),
                    error: None,
                },
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                },
            }
        } else {
            match fs::create_dir(&path).await {
                Ok(()) => ToolResult {
                    success: true,
                    output: format!("Directory created: {}", path),
                    error: None,
                },
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                },
            }
        }
    }

    /// Execute the find tool - find files/directories by name pattern
    async fn execute_find(&self, input: serde_json::Value) -> ToolResult {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if name.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("name is required".to_string()),
            };
        }

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let max_depth = input
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let type_filter = input
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("a");

        let path = Self::normalize_path(path);
        info!("find: name='{}' in '{}' (max_depth={}, type={})", name, path, max_depth, type_filter);

        // Convert name pattern to regex (* -> .*, ? -> .)
        let name_pattern = name
            .replace('*', ".*")
            .replace('?', ".");

        let regex_match = format!("^{}$", name_pattern);
        let re = match regex::Regex::new(&regex_match) {
            Ok(r) => r,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid name pattern '{}': {}", name, e)),
                };
            }
        };

        let mut results: Vec<String> = Vec::new();
        let walker = walkdir::WalkDir::new(&path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            let file_name = entry.file_name().to_string_lossy();
            if re.is_match(&file_name) {
                let entry_path = entry.path();
                let is_dir = entry.file_type().is_dir();

                // Apply type filter
                match type_filter {
                    "f" if !is_dir => results.push(entry_path.display().to_string()),
                    "d" if is_dir => results.push(entry_path.display().to_string()),
                    "a" => results.push(entry_path.display().to_string()),
                    _ => {}
                }
            }
        }

        results.sort();

        ToolResult {
            success: true,
            output: if results.is_empty() {
                "(no matches)".to_string()
            } else {
                results.join("\n")
            },
            error: None,
        }
    }

    /// Execute the tail tool - read last N lines of a file
    async fn execute_tail(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let num_lines = input
            .get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let path = Self::normalize_path(path);
        info!("tail: reading {} lines from '{}'", num_lines, path);

        let content = match fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                };
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        let start_idx = if lines.len() > num_lines {
            lines.len() - num_lines
        } else {
            0
        };

        let tail: String = lines[start_idx..].join("\n");

        ToolResult {
            success: true,
            output: tail,
            error: None,
        }
    }

    /// Execute the stat_file tool - get file/directory metadata
    async fn execute_stat_file(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("stat_file: path='{}'", path);

        let metadata = match fs::metadata(&path).await {
            Ok(m) => m,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                };
            }
        };

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "unknown"
        };

        let size = metadata.len();
        let size_str = if metadata.is_dir() { "-".to_string() } else { Self::format_size(size) };

        let modified: String = metadata.modified()
            .ok()
            .map(|t| {
                let datetime: DateTime<Local> = t.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        let created: String = metadata.created()
            .ok()
            .map(|t| {
                let datetime: DateTime<Local> = t.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        #[cfg(unix)]
        let perms = {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", metadata.permissions().mode() & 0o777)
        };
        #[cfg(not(unix))]
        let perms = "n/a".to_string();

        let output = format!(
            "Path:     {}\n\
             Type:     {}\n\
             Size:     {}\n\
             Modified: {}\n\
             Created:  {}\n\
             Permissions: {}",
            path, file_type, size_str, modified, created, perms
        );

        ToolResult {
            success: true,
            output,
            error: None,
        }
    }

    /// Execute the batch_execute tool - execute multiple tools in sequence
    async fn execute_batch_execute(&self, input: serde_json::Value) -> ToolResult {
        let empty: Vec<serde_json::Value> = Vec::new();
        let tools = input
            .get("tools")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty);

        if tools.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("tools array is required".to_string()),
            };
        }

        info!("batch_execute: executing {} tools", tools.len());

        let mut results: Vec<serde_json::Value> = Vec::new();
        let mut all_success = true;

        for (i, tool_call) in tools.iter().enumerate() {
            let tool_name = tool_call
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tool_input = tool_call.get("input").cloned().unwrap_or(serde_json::Value::Null);

            // Execute each tool inline (avoid recursion)
            let result = self.execute_single(tool_name, tool_input).await;
            
            if !result.success {
                all_success = false;
            }

            results.push(serde_json::json!({
                "index": i,
                "tool": tool_name,
                "success": result.success,
                "output": result.output,
                "error": result.error
            }));
        }

        let output = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());

        ToolResult {
            success: all_success,
            output,
            error: if all_success { None } else { Some("Some tools failed".to_string()) },
        }
    }

    /// Execute a single tool (internal, non-recursive)
    async fn execute_single(&self, name: &str, input: serde_json::Value) -> ToolResult {
        match name {
            "exec" => self.execute_exec(input).await,
            "read_file" => self.execute_read_file(input).await,
            "write_file" => self.execute_write_file(input).await,
            "list_dir" => self.execute_list_dir(input).await,
            "http_request" => self.execute_http_request(input).await,
            "glob" => self.execute_glob(input).await,
            "grep" => self.execute_grep(input).await,
            "sed_file" => self.execute_sed_file(input).await,
            "which" => self.execute_which(input).await,
            "mkdir" => self.execute_mkdir(input).await,
            "stat_file" => self.execute_stat_file(input).await,
            "find" => self.execute_find(input).await,
            "tail" => self.execute_tail(input).await,
            "env" => self.execute_env(input).await,
            "diff" => self.execute_diff(input).await,
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", name)),
            },
        }
    }

    /// Execute the env tool - get or set environment variables
    async fn execute_env(&self, input: serde_json::Value) -> ToolResult {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let value = input
            .get("value")
            .and_then(|v| v.as_str());

        if name.is_empty() {
            // List all environment variables
            let env_vars: Vec<String> = std::env::vars()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            
            return ToolResult {
                success: true,
                output: env_vars.join("\n"),
                error: None,
            };
        }

        match value {
            Some("") => {
                // Unset the variable
                std::env::remove_var(name);
                info!("env: unset {}", name);
                ToolResult {
                    success: true,
                    output: format!("Unset {}", name),
                    error: None,
                }
            }
            Some(v) => {
                // Set the variable
                std::env::set_var(name, v);
                info!("env: set {} = {}", name, v);
                ToolResult {
                    success: true,
                    output: format!("Set {} = {}", name, v),
                    error: None,
                }
            }
            None => {
                // Get the variable
                let value = std::env::var(name).unwrap_or_else(|_| "(not set)".to_string());
                info!("env: get {} = {}", name, value);
                ToolResult {
                    success: true,
                    output: value,
                    error: None,
                }
            }
        }
    }

    /// Execute the diff tool - compare two files
    async fn execute_diff(&self, input: serde_json::Value) -> ToolResult {
        let path1 = input
            .get("path1")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path2 = input
            .get("path2")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path1.is_empty() || path2.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path1 and path2 are required".to_string()),
            };
        }

        let path1 = Self::normalize_path(path1);
        let path2 = Self::normalize_path(path2);

        info!("diff: {} vs {}", path1, path2);

        // Read both files
        let content1 = match fs::read_to_string(&path1).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read {}: {}", path1, e)),
                };
            }
        };

        let content2 = match fs::read_to_string(&path2).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read {}: {}", path2, e)),
                };
            }
        };

        if content1 == content2 {
            return ToolResult {
                success: true,
                output: "Files are identical".to_string(),
                error: None,
            };
        }

        // Simple line-by-line diff
        let lines1: Vec<&str> = content1.lines().collect();
        let lines2: Vec<&str> = content2.lines().collect();

        let mut diff_output = String::new();
        let max_lines = lines1.len().max(lines2.len());

        for i in 0..max_lines {
            let l1 = lines1.get(i).copied();
            let l2 = lines2.get(i).copied();

            match (l1, l2) {
                (Some(a), Some(b)) if a == b => {
                    diff_output.push_str(&format!("  {}\n", a));
                }
                (Some(a), Some(b)) => {
                    diff_output.push_str(&format!("- {}\n", a));
                    diff_output.push_str(&format!("+ {}\n", b));
                }
                (Some(a), None) => {
                    diff_output.push_str(&format!("- {}\n", a));
                }
                (None, Some(b)) => {
                    diff_output.push_str(&format!("+ {}\n", b));
                }
                (None, None) => {}
            }
        }

        ToolResult {
            success: true,
            output: diff_output,
            error: None,
        }
    }

    /// Execute the cp tool - copy a file
    async fn execute_cp(&self, input: serde_json::Value) -> ToolResult {
        let source = input
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let dest = input
            .get("dest")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if source.is_empty() || dest.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("source and dest are required".to_string()),
            };
        }

        let source = Self::normalize_path(source);
        let dest = Self::normalize_path(dest);

        info!("cp: {} -> {}", source, dest);

        // Check if source exists
        if tokio::fs::metadata(&source).await.is_err() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source file does not exist: {}", source)),
            };
        }

        // Copy the file
        match tokio::fs::copy(&source, &dest).await {
            Ok(bytes) => ToolResult {
                success: true,
                output: format!("Copied {} bytes from {} to {}", bytes, source, dest),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to copy {} to {}: {}", source, dest, e)),
            },
        }
    }

    /// Execute the mv tool - move/rename a file
    async fn execute_mv(&self, input: serde_json::Value) -> ToolResult {
        let source = input
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let dest = input
            .get("dest")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if source.is_empty() || dest.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("source and dest are required".to_string()),
            };
        }

        let source = Self::normalize_path(source);
        let dest = Self::normalize_path(dest);

        info!("mv: {} -> {}", source, dest);

        // Check if source exists
        if tokio::fs::metadata(&source).await.is_err() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source file does not exist: {}", source)),
            };
        }

        // Move the file
        match tokio::fs::rename(&source, &dest).await {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Moved {} to {}", source, dest),
                error: None,
            },
            Err(e) => {
                // If rename fails (e.g., cross-filesystem), try copy+delete
                if let Ok(bytes) = tokio::fs::copy(&source, &dest).await {
                    if tokio::fs::remove_file(&source).await.is_ok() {
                        return ToolResult {
                            success: true,
                            output: format!("Moved {} to {} (copy+delete fallback, {} bytes)", source, dest, bytes),
                            error: None,
                        };
                    }
                }
                ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to move {} to {}: {}", source, dest, e)),
                }
            }
        }
    }

    /// Execute the rm tool - remove a file
    async fn execute_rm(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);

        info!("rm: {}", path);

        // Check if file exists
        match tokio::fs::metadata(&path).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("{} is a directory, use rmdir instead", path)),
                    };
                }
            }
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("File does not exist: {}", e)),
                };
            }
        }

        // Remove the file
        match tokio::fs::remove_file(&path).await {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Removed {}", path),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to remove {}: {}", path, e)),
            },
        }
    }

    /// Execute the cat tool - read and concatenate multiple files
    async fn execute_cat(&self, input: serde_json::Value) -> ToolResult {
        let paths_json = input
            .get("paths")
            .and_then(|v| v.as_array())
            .cloned();

        let show_line_numbers = input
            .get("show_line_numbers")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let paths = match paths_json {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("paths array is required and must not be empty".to_string()),
                };
            }
        };

        let mut results = Vec::new();
        let mut has_error = false;

        for (idx, path_val) in paths.iter().enumerate() {
            let path = match path_val.as_str() {
                Some(s) => Self::normalize_path(s),
                None => {
                    results.push(format!("[{}] Error: Invalid path", idx + 1));
                    has_error = true;
                    continue;
                }
            };

            info!("cat: {}", path);

            match fs::read_to_string(&path).await {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    if show_line_numbers {
                        let numbered: Vec<String> = lines
                            .iter()
                            .enumerate()
                            .map(|(i, l)| format!("{:6}  {}", i + 1, l))
                            .collect();
                        results.push(format!("==> {} <==\n{}", path, numbered.join("\n")));
                    } else {
                        results.push(format!("==> {} <==\n{}", path, content));
                    }
                }
                Err(e) => {
                    results.push(format!("[{}] Error reading {}: {}", idx + 1, path, e));
                    has_error = true;
                }
            }
        }

        ToolResult {
            success: !has_error,
            output: results.join("\n"),
            error: if has_error { Some("Some files could not be read".to_string()) } else { None },
        }
    }

    /// Execute the tree tool - display directory tree
    async fn execute_tree(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let depth = input
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;
        let show_hidden = input
            .get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = Self::normalize_path(path);
        info!("tree: {} (depth={})", path, depth);

        let mut output = String::new();
        output.push_str(&format!("{}\n", path));

        fn build_tree(
            dir_path: &std::path::Path,
            prefix: &str,
            depth: usize,
            max_depth: usize,
            show_hidden: bool,
            output: &mut String,
        ) -> std::io::Result<()> {
            if depth >= max_depth {
                return Ok(());
            }

            let entries: Vec<_> = std::fs::read_dir(dir_path)?
                .filter_map(|e| e.ok())
                .filter(|e| show_hidden || !e.file_name().to_string_lossy().starts_with('.'))
                .collect();

            let total = entries.len();
            for (i, entry) in entries.iter().enumerate() {
                let is_last = i == total - 1;
                let file_name: String = entry.file_name().to_string_lossy().into_owned();
                let entry_path = entry.path();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

                let branch = if is_last { "└── " } else { "├── " };
                let next_prefix = if is_last { "    " } else { "│   " };

                output.push_str(prefix);
                output.push_str(branch);
                output.push_str(&file_name);

                if is_dir {
                    output.push('/');
                }
                output.push('\n');

                if is_dir {
                    build_tree(
                        &entry_path,
                        &format!("{}{}", prefix, next_prefix),
                        depth + 1,
                        max_depth,
                        show_hidden,
                        output,
                    )?;
                }
            }
            Ok(())
        }

        match std::fs::metadata(&path) {
            Ok(meta) => {
                if !meta.is_dir() {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("{} is not a directory", path)),
                    };
                }
            }
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Cannot access {}: {}", path, e)),
                };
            }
        }

        if let Err(e) = build_tree(std::path::Path::new(&path), "", 0, depth, show_hidden, &mut output) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Error building tree: {}", e)),
            };
        }

        ToolResult {
            success: true,
            output,
            error: None,
        }
    }

    /// Execute the chmod tool - change file permissions
    async fn execute_chmod(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mode_str = input
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() || mode_str.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path and mode are required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("chmod: {} {}", mode_str, path);

        // Parse the mode - support both octal (755) and symbolic (+x)
        let new_mode = if mode_str.chars().all(|c| c.is_ascii_digit()) {
            // Octal mode like "755" or "644"
            u32::from_str_radix(mode_str, 8).ok()
        } else {
            // Symbolic mode - for simplicity, we handle a subset: +x, -x, +r, -r, +w, -w
            // Try to get current permissions and modify them
            std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.permissions().mode().into())
                .map(|current| {
                    let mut mode = current;
                    for part in mode_str.split(',') {
                        let part = part.trim();
                        if part.len() >= 2 {
                            let op = part.chars().nth(0).unwrap();
                            let mut perms = part[1..].chars().filter(|c| !c.is_ascii_digit()).collect::<String>();
                            // Handle +x, -x patterns
                            if perms.is_empty() && part.len() >= 2 && part[1..].chars().all(|c| c == 'x' || c == 'r' || c == 'w') {
                                perms = part[1..].to_string();
                            }
                            for p in perms.chars() {
                                let flag = match p {
                                    'r' => 0o444,
                                    'w' => 0o222,
                                    'x' => 0o111,
                                    'u' => (mode >> 6) & 0o777,
                                    'g' => (mode >> 3) & 0o777,
                                    'o' => mode & 0o777,
                                    'a' => 0o777,
                                    _ => continue,
                                };
                                match op {
                                    '+' => mode |= flag,
                                    '-' => mode &= !flag,
                                    '=' => mode = (mode & !0o777) | flag,
                                    _ => {}
                                }
                            }
                        }
                    }
                    mode
                })
        };

        let new_mode = match new_mode {
            Some(m) => m,
            None => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid mode: {}", mode_str)),
                };
            }
        };

        // Apply the permission change
        match std::fs::metadata(&path) {
            Ok(meta) => {
                let mut perms = meta.permissions();
                perms.set_mode(new_mode);
                if let Err(e) = std::fs::set_permissions(&path, perms) {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to change permissions: {}", e)),
                    };
                }
            }
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Cannot access {}: {}", path, e)),
                };
            }
        }

        ToolResult {
            success: true,
            output: format!("Changed permissions of {} to {:o}", path, new_mode),
            error: None,
        }
    }

    /// Execute the hash tool - compute file hash/checksum
    async fn execute_hash(&self, input: serde_json::Value) -> ToolResult {

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let algorithm = input
            .get("algorithm")
            .and_then(|v| v.as_str())
            .unwrap_or("sha256");

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("hash ({}): {}", algorithm, path);

        // Check if file exists and is readable
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(m) => m,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Cannot access {}: {}", path, e)),
                };
            }
        };

        if metadata.is_dir() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("{} is a directory, not a file", path)),
            };
        }

        // Read file and compute hash
        let content = match tokio::fs::read(&path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read {}: {}", path, e)),
                };
            }
        };

        let hash_hex = match algorithm.to_lowercase().as_str() {
            "sha256" => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            }
            "sha512" => {
                use sha2::{Sha512, Digest};
                let mut hasher = Sha512::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            }
            "sha1" => {
                use sha1::{Sha1, Digest};
                let mut hasher = Sha1::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            }
            "md5" => {
                let digest = md5::compute(&content);
                format!("{:x}", digest)
            }
            _ => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported algorithm: {}. Supported: sha256, sha512, sha1, md5", algorithm)),
                };
            }
        };

        ToolResult {
            success: true,
            output: format!("{}  {}", hash_hex, path),
            error: None,
        }
    }

    /// Execute the wc tool - count lines, words, characters
    async fn execute_wc(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let show_bytes = input
            .get("bytes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let show_chars = input
            .get("chars")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let show_lines = input
            .get("lines")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let show_words = input
            .get("words")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // If no specific flags, show all
        let show_all = ![show_bytes, show_chars, show_lines, show_words]
            .contains(&true);

        if path.is_empty() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("path is required".to_string()),
            };
        }

        let path = Self::normalize_path(path);
        info!("wc: {}", path);

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read {}: {}", path, e)),
                };
            }
        };

        let mut output = String::new();
        let line_count = content.lines().count();
        let word_count = content.split_whitespace().count();
        let byte_count = content.len();
        let char_count = content.chars().count();

        // Default: show lines, words, bytes (like unix wc)
        if show_all || show_lines {
            output.push_str(&format!("{} ", line_count));
        }
        if show_all || show_words {
            output.push_str(&format!("{} ", word_count));
        }
        if show_bytes || (show_all && !show_chars) {
            output.push_str(&format!("{} ", byte_count));
        }
        if show_chars {
            output.push_str(&format!("{} ", char_count));
        }
        output.push_str(&path);

        ToolResult {
            success: true,
            output,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_executor_new() {
        let executor = ToolExecutor::new();
        let tools = executor.list_tools();
        assert!(!tools.is_empty());
        assert_eq!(tools.len(), 24); // exec, read_file, write_file, list_dir, http_request, glob, grep, sed_file, which, mkdir, stat_file, find, tail, batch_execute, env, diff, cp, mv, rm, cat, tree, chmod, hash, wc
    }

    #[test]
    fn test_tool_executor_get_tool() {
        let executor = ToolExecutor::new();
        let exec_tool = executor.get_tool("exec");
        assert!(exec_tool.is_some());
        assert_eq!(exec_tool.unwrap().name, "exec");
    }

    #[test]
    fn test_tool_executor_get_nonexistent_tool() {
        let executor = ToolExecutor::new();
        let tool = executor.get_tool("nonexistent");
        assert!(tool.is_none());
    }

    #[tokio::test]
    async fn test_execute_exec_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("exec", serde_json::json!({
            "command": "echo hello"
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_exec_failure() {
        let executor = ToolExecutor::new();
        let result = executor.execute("exec", serde_json::json!({
            "command": "exit 1"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_exec_empty_command() {
        let executor = ToolExecutor::new();
        let result = executor.execute("exec", serde_json::json!({
            "command": ""
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_read_file_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("read_file", serde_json::json!({
            "path": "Cargo.toml"
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("[package]"));
    }

    #[tokio::test]
    async fn test_execute_read_file_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("read_file", serde_json::json!({
            "path": "/nonexistent/file.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_read_file_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("read_file", serde_json::json!({})).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_write_file_success() {
        let executor = ToolExecutor::new();
        let test_path = "/tmp/tiny_claw_test.txt";
        let result = executor.execute("write_file", serde_json::json!({
            "path": test_path,
            "content": "test content"
        })).await;
        
        assert!(result.success);
        
        // Cleanup
        let _ = tokio::fs::remove_file(test_path).await;
    }

    #[tokio::test]
    async fn test_execute_write_file_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("write_file", serde_json::json!({
            "content": "test"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_list_dir_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("list_dir", serde_json::json!({
            "path": "."
        })).await;
        
        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_execute_list_dir_default_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("list_dir", serde_json::json!({})).await;
        
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_http_request_missing_url() {
        let executor = ToolExecutor::new();
        let result = executor.execute("http_request", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_http_request_unsupported_method() {
        let executor = ToolExecutor::new();
        let result = executor.execute("http_request", serde_json::json!({
            "url": "http://example.com",
            "method": "INVALID"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let executor = ToolExecutor::new();
        let result = executor.execute("unknown_tool", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            success: true,
            output: "test output".to_string(),
            error: None,
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("test output"));
    }

    #[test]
    fn test_tool_result_with_error() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("error message".to_string()),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("error"));
    }

    #[tokio::test]
    async fn test_execute_glob_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("glob", serde_json::json!({
            "pattern": "**/*.toml",
            "root": "."
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Cargo.toml"));
    }

    #[tokio::test]
    async fn test_execute_glob_no_matches() {
        let executor = ToolExecutor::new();
        let result = executor.execute("glob", serde_json::json!({
            "pattern": "**/*.nonexistent-extension-xyz",
            "root": "."
        })).await;
        
        assert!(result.success);
        assert_eq!(result.output, "(no matches)");
    }

    #[tokio::test]
    async fn test_execute_glob_missing_pattern() {
        let executor = ToolExecutor::new();
        let result = executor.execute("glob", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_grep_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("grep", serde_json::json!({
            "pattern": "package",
            "path": "."
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("package"));
    }

    #[tokio::test]
    async fn test_execute_grep_no_matches() {
        let executor = ToolExecutor::new();
        // Use a pattern that won't match itself - search a specific file
        let result = executor.execute("grep", serde_json::json!({
            "pattern": "__tiny_claw_nonexistent_unique_marker_12345__",
            "path": "Cargo.toml"
        })).await;
        
        assert!(result.success);
        assert_eq!(result.output, "(no matches)");
    }

    #[tokio::test]
    async fn test_execute_grep_missing_pattern() {
        let executor = ToolExecutor::new();
        let result = executor.execute("grep", serde_json::json!({
            "path": "."
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_grep_case_insensitive() {
        let executor = ToolExecutor::new();
        let result = executor.execute("grep", serde_json::json!({
            "pattern": "PACKAGE",
            "path": "Cargo.toml",
            "case_sensitive": false
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("package") || result.output.contains("PACKAGE"));
    }

    #[test]
    fn test_pattern_to_regex() {
        // Test simple pattern
        let re = ToolExecutor::pattern_to_regex("*.rs").unwrap();
        assert!(re.is_match("main.rs"));
        assert!(re.is_match("lib.rs"));
        assert!(!re.is_match("main.js"));
        
        // Test glob with dots
        let re = ToolExecutor::pattern_to_regex("*.toml").unwrap();
        assert!(re.is_match("Cargo.toml"));
        assert!(!re.is_match("Cargo.tomll"));
    }

    #[tokio::test]
    async fn test_execute_sed_file_by_line_number() {
        let executor = ToolExecutor::new();
        // Create a temp file
        let temp_path = "/tmp/tinyclaw_test_sed.txt";
        tokio::fs::write(temp_path, "line1\nline2\nline3\n").await.unwrap();
        
        let result = executor.execute("sed_file", serde_json::json!({
            "path": temp_path,
            "new_text": "replaced",
            "line_number": 2
        })).await;
        
        assert!(result.success);
        assert_eq!(result.output, "File updated successfully");
        
        // Verify the change
        let content = tokio::fs::read_to_string(temp_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[1], "replaced");
        
        // Cleanup
        tokio::fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_sed_file_by_old_text() {
        let executor = ToolExecutor::new();
        let temp_path = "/tmp/tinyclaw_test_sed2.txt";
        tokio::fs::write(temp_path, "hello world\nfoo bar\n").await.unwrap();
        
        let result = executor.execute("sed_file", serde_json::json!({
            "path": temp_path,
            "old_text": "world",
            "new_text": "universe"
        })).await;
        
        assert!(result.success);
        
        let content = tokio::fs::read_to_string(temp_path).await.unwrap();
        assert!(content.contains("universe"));
        assert!(!content.contains("world"));
        
        tokio::fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_sed_file_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("sed_file", serde_json::json!({
            "new_text": "something"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_which_rustc() {
        let executor = ToolExecutor::new();
        let result = executor.execute("which", serde_json::json!({
            "command": "rustc"
        })).await;
        
        // rustc should be in PATH on this system
        assert!(result.success);
        assert!(!result.output.is_empty() && result.output != "(not found)");
    }

    #[tokio::test]
    async fn test_execute_which_nonexistent() {
        let executor = ToolExecutor::new();
        let result = executor.execute("which", serde_json::json!({
            "command": "__nonexistent_command_tinyclaw_xyz123__"
        })).await;
        
        // Tool returns success=false when not found (semantically correct)
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_which_missing_command() {
        let executor = ToolExecutor::new();
        let result = executor.execute("which", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_mkdir_success() {
        let executor = ToolExecutor::new();
        let test_path = "/tmp/tiny_claw_test_mkdir/subdir";
        let result = executor.execute("mkdir", serde_json::json!({
            "path": test_path,
            "parents": true
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("created"));
        
        // Cleanup
        let _ = tokio::fs::remove_dir_all("/tmp/tiny_claw_test_mkdir").await;
    }

    #[tokio::test]
    async fn test_execute_mkdir_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("mkdir", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_stat_file_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("stat_file", serde_json::json!({
            "path": "Cargo.toml"
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Type:"));
        assert!(result.output.contains("Size:"));
        assert!(result.output.contains("Modified:"));
    }

    #[tokio::test]
    async fn test_execute_stat_file_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("stat_file", serde_json::json!({
            "path": "/nonexistent/file_tinyclaw_xyz.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_stat_file_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("stat_file", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_normalize_path_tilde() {
        let _executor = ToolExecutor::new();
        // Write to home dir first
        if let Some(home) = dirs::home_dir() {
            let expanded = ToolExecutor::normalize_path("~/tiny_claw_test.txt");
            assert!(expanded.starts_with(&*home.to_string_lossy()));
            assert!(expanded.ends_with("tiny_claw_test.txt"));
        }
    }

    #[tokio::test]
    async fn test_normalize_path_env_var() {
        // Can't easily test env vars without modifying the environment
        // This is tested implicitly through path tools
    }

    #[tokio::test]
    async fn test_execute_read_file_with_tilde_path() {
        let executor = ToolExecutor::new();
        // Create a file in home dir first
        if let Some(home) = dirs::home_dir() {
            let test_file = home.join(".tiny_claw_test_file");
            tokio::fs::write(&test_file, "test content").await.unwrap();
            
            let result = executor.execute("read_file", serde_json::json!({
                "path": format!("~/{}", test_file.file_name().unwrap().to_string_lossy())
            })).await;
            
            assert!(result.success);
            assert!(result.output.contains("test content"));
            
            // Cleanup
            let _ = tokio::fs::remove_file(&test_file).await;
        }
    }

    #[test]
    fn test_expand_env_vars() {
        // Test ${VAR} form
        std::env::set_var("TINY_CLAW_TEST_VAR", "test_value");
        let result = ToolExecutor::expand_env_vars("prefix_${TINY_CLAW_TEST_VAR}_suffix");
        assert_eq!(result, "prefix_test_value_suffix");
        std::env::remove_var("TINY_CLAW_TEST_VAR");
        
        // Test $VAR form
        std::env::set_var("TINY_CLAW_TEST_VAR2", "hello");
        let result = ToolExecutor::expand_env_vars("prefix$TINY_CLAW_TEST_VAR2");
        assert_eq!(result, "prefixhello");
        std::env::remove_var("TINY_CLAW_TEST_VAR2");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(ToolExecutor::format_size(0), "0B");
        assert_eq!(ToolExecutor::format_size(500), "500B");
        assert_eq!(ToolExecutor::format_size(1024), "1.0K");
        assert_eq!(ToolExecutor::format_size(1536), "1.5K");
        assert_eq!(ToolExecutor::format_size(1048576), "1.0M");
        assert_eq!(ToolExecutor::format_size(1073741824), "1.0G");
    }

    #[tokio::test]
    async fn test_execute_batch_execute_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("batch_execute", serde_json::json!({
            "tools": [
                {"name": "exec", "input": {"command": "echo hello"}},
                {"name": "exec", "input": {"command": "echo world"}}
            ]
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("hello"));
        assert!(result.output.contains("world"));
    }

    #[tokio::test]
    async fn test_execute_batch_execute_empty() {
        let executor = ToolExecutor::new();
        let result = executor.execute("batch_execute", serde_json::json!({
            "tools": []
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_env_get() {
        let executor = ToolExecutor::new();
        let result = executor.execute("env", serde_json::json!({
            "name": "PATH"
        })).await;
        
        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_execute_env_list_all() {
        let executor = ToolExecutor::new();
        let result = executor.execute("env", serde_json::json!({})).await;
        
        assert!(result.success);
        assert!(result.output.contains("="));
    }

    #[tokio::test]
    async fn test_execute_diff_identical() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_diff_test.txt";
        tokio::fs::write(path, "hello\nworld\n").await.unwrap();
        
        let result = executor.execute("diff", serde_json::json!({
            "path1": path,
            "path2": path
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("identical"));
        
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_diff_different() {
        let executor = ToolExecutor::new();
        let path1 = "/tmp/tinyclaw_diff1.txt";
        let path2 = "/tmp/tinyclaw_diff2.txt";
        tokio::fs::write(path1, "hello\nworld\n").await.unwrap();
        tokio::fs::write(path2, "hello\nrust\n").await.unwrap();
        
        let result = executor.execute("diff", serde_json::json!({
            "path1": path1,
            "path2": path2
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("-"));
        assert!(result.output.contains("+"));
        
        tokio::fs::remove_file(path1).await.unwrap();
        tokio::fs::remove_file(path2).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_diff_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("diff", serde_json::json!({
            "path1": "/nonexistent1.txt",
            "path2": "/nonexistent2.txt"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_cp_success() {
        let executor = ToolExecutor::new();
        let source = "/tmp/tinyclaw_cp_source.txt";
        let dest = "/tmp/tinyclaw_cp_dest.txt";
        
        tokio::fs::write(source, "test content").await.unwrap();
        
        let result = executor.execute("cp", serde_json::json!({
            "source": source,
            "dest": dest
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Copied"));
        assert!(tokio::fs::read_to_string(dest).await.unwrap() == "test content");
        
        // Cleanup
        tokio::fs::remove_file(source).await.unwrap();
        tokio::fs::remove_file(dest).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_cp_missing_source() {
        let executor = ToolExecutor::new();
        let result = executor.execute("cp", serde_json::json!({
            "dest": "/tmp/dest.txt"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_cp_source_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("cp", serde_json::json!({
            "source": "/nonexistent_file_tinyclaw.txt",
            "dest": "/tmp/dest.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_mv_success() {
        let executor = ToolExecutor::new();
        let source = "/tmp/tinyclaw_mv_source.txt";
        let dest = "/tmp/tinyclaw_mv_dest.txt";
        
        tokio::fs::write(source, "test content").await.unwrap();
        
        let result = executor.execute("mv", serde_json::json!({
            "source": source,
            "dest": dest
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Moved"));
        assert!(!tokio::fs::metadata(source).await.is_ok()); // source gone
        assert!(tokio::fs::read_to_string(dest).await.unwrap() == "test content");
        
        // Cleanup
        let _ = tokio::fs::remove_file(dest).await;
    }

    #[tokio::test]
    async fn test_execute_mv_missing_source() {
        let executor = ToolExecutor::new();
        let result = executor.execute("mv", serde_json::json!({
            "dest": "/tmp/dest.txt"
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_mv_source_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("mv", serde_json::json!({
            "source": "/nonexistent_file_tinyclaw.txt",
            "dest": "/tmp/dest.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_rm_success() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_rm_test.txt";
        
        tokio::fs::write(path, "test content").await.unwrap();
        
        let result = executor.execute("rm", serde_json::json!({
            "path": path
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Removed"));
        assert!(!tokio::fs::metadata(path).await.is_ok()); // file gone
        
        // No cleanup needed
    }

    #[tokio::test]
    async fn test_execute_rm_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("rm", serde_json::json!({})).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_rm_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("rm", serde_json::json!({
            "path": "/nonexistent_file_tinyclaw.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_cat_single_file() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_cat_test.txt";
        
        tokio::fs::write(path, "line1\nline2\nline3").await.unwrap();
        
        let result = executor.execute("cat", serde_json::json!({
            "paths": [path]
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("line1"));
        assert!(result.output.contains("line2"));
        assert!(result.output.contains("line3"));
        
        // Cleanup
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_cat_multiple_files() {
        let executor = ToolExecutor::new();
        let path1 = "/tmp/tinyclaw_cat_test1.txt";
        let path2 = "/tmp/tinyclaw_cat_test2.txt";
        
        tokio::fs::write(path1, "content1").await.unwrap();
        tokio::fs::write(path2, "content2").await.unwrap();
        
        let result = executor.execute("cat", serde_json::json!({
            "paths": [path1, path2]
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("content1"));
        assert!(result.output.contains("content2"));
        
        // Cleanup
        tokio::fs::remove_file(path1).await.unwrap();
        tokio::fs::remove_file(path2).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_cat_with_line_numbers() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_cat_linenum.txt";
        
        tokio::fs::write(path, "line1\nline2\nline3").await.unwrap();
        
        let result = executor.execute("cat", serde_json::json!({
            "paths": [path],
            "show_line_numbers": true
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("1  line1"));
        assert!(result.output.contains("2  line2"));
        assert!(result.output.contains("3  line3"));
        
        // Cleanup
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_cat_missing_paths() {
        let executor = ToolExecutor::new();
        let result = executor.execute("cat", serde_json::json!({
            "paths": []
        })).await;
        
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_cat_partial_failure() {
        let executor = ToolExecutor::new();
        let path1 = "/tmp/tinyclaw_cat_ok.txt";
        let path2 = "/tmp/nonexistent_tinyclaw.txt";
        
        tokio::fs::write(path1, "ok content").await.unwrap();
        
        let result = executor.execute("cat", serde_json::json!({
            "paths": [path1, path2]
        })).await;
        
        // Should succeed overall but include error for missing file
        assert!(result.output.contains("ok content"));
        assert!(result.output.contains("Error"));
        
        // Cleanup
        tokio::fs::remove_file(path1).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_find_by_name() {
        let executor = ToolExecutor::new();
        let result = executor.execute("find", serde_json::json!({
            "name": "Cargo.toml",
            "path": "."
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("Cargo.toml"));
    }

    #[tokio::test]
    async fn test_execute_find_with_wildcard() {
        let executor = ToolExecutor::new();
        let result = executor.execute("find", serde_json::json!({
            "name": "*.rs",
            "path": "src"
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains(".rs") || result.output == "(no matches)");
    }

    #[tokio::test]
    async fn test_execute_find_no_matches() {
        let executor = ToolExecutor::new();
        let result = executor.execute("find", serde_json::json!({
            "name": "__nonexistent_file_tinyclaw_xyz__",
            "path": "."
        })).await;
        
        assert!(result.success);
        assert_eq!(result.output, "(no matches)");
    }

    #[tokio::test]
    async fn test_execute_find_missing_name() {
        let executor = ToolExecutor::new();
        let result = executor.execute("find", serde_json::json!({
            "path": "."
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_find_type_filter() {
        let executor = ToolExecutor::new();
        let result = executor.execute("find", serde_json::json!({
            "name": "src",
            "path": ".",
            "type": "d"
        })).await;
        
        assert!(result.success);
        // Should find the src directory
        assert!(result.output.contains("src") || result.output == "(no matches)");
    }

    #[tokio::test]
    async fn test_execute_tail_success() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_tail_test.txt";
        tokio::fs::write(path, "line1\nline2\nline3\nline4\nline5\n").await.unwrap();
        
        let result = executor.execute("tail", serde_json::json!({
            "path": path,
            "lines": 3
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("line3"));
        assert!(result.output.contains("line5"));
        assert!(!result.output.contains("line1"));
        
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_tail_default_lines() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_tail_test2.txt";
        let content: String = (1..=20).map(|i| format!("line{}\n", i)).collect();
        tokio::fs::write(path, content).await.unwrap();
        
        let result = executor.execute("tail", serde_json::json!({
            "path": path
        })).await;
        
        assert!(result.success);
        // Default is 10 lines, so should get lines 11-20
        assert!(result.output.contains("line11"));
        assert!(result.output.contains("line20"));
        
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_tail_file_too_short() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tinyclaw_tail_test3.txt";
        tokio::fs::write(path, "short\n").await.unwrap();
        
        let result = executor.execute("tail", serde_json::json!({
            "path": path,
            "lines": 100
        })).await;
        
        assert!(result.success);
        assert!(result.output.contains("short"));
        
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_tail_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("tail", serde_json::json!({})).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_tail_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("tail", serde_json::json!({
            "path": "/nonexistent/file_tinyclaw.txt"
        })).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    // ============ tree tool tests ============

    #[tokio::test]
    async fn test_execute_tree_success() {
        let executor = ToolExecutor::new();
        let result = executor.execute("tree", serde_json::json!({
            "path": "/tmp",
            "depth": 1
        })).await;

        assert!(result.success);
        assert!(result.output.contains("/tmp"));
    }

    #[tokio::test]
    async fn test_execute_tree_default_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("tree", serde_json::json!({})).await;

        // Should use "." as default path
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_tree_not_directory() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tiny_claw_tree_file.txt";
        tokio::fs::write(path, "content").await.unwrap();

        let result = executor.execute("tree", serde_json::json!({
            "path": path
        })).await;

        assert!(!result.success);
        assert!(result.error.is_some());

        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_tree_show_hidden() {
        let executor = ToolExecutor::new();
        let result = executor.execute("tree", serde_json::json!({
            "path": "/tmp",
            "depth": 1,
            "show_hidden": true
        })).await;

        assert!(result.success);
    }

    // ============ chmod tool tests ============

    #[tokio::test]
    async fn test_execute_chmod_success() {
        let executor = ToolExecutor::new();
        let path = "/tmp/tiny_claw_chmod_test.txt";
        tokio::fs::write(path, "content").await.unwrap();

        let result = executor.execute("chmod", serde_json::json!({
            "path": path,
            "mode": "755"
        })).await;

        assert!(result.success);
        assert!(result.output.contains("755"));

        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_chmod_missing_args() {
        let executor = ToolExecutor::new();
        let result = executor.execute("chmod", serde_json::json!({
            "path": "/tmp/test.txt"
        })).await;

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_chmod_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("chmod", serde_json::json!({
            "path": "/tmp/nonexistent_tinyclaw_chmod.txt",
            "mode": "755"
        })).await;

        assert!(!result.success);
    }

    // ============ hash tool tests ============

    #[tokio::test]
    async fn test_execute_hash_sha256_success() {
        let executor = ToolExecutor::new();
        let path = format!("/tmp/tiny_claw_hash_test_{}.txt", uuid::Uuid::new_v4());
        tokio::fs::write(&path, "hello world").await.unwrap();

        let result = executor.execute("hash", serde_json::json!({
            "path": &path,
            "algorithm": "sha256"
        })).await;

        assert!(result.success);
        // Verify the output contains a 64-char hex hash and the file path
        let hash_part = result.output.split_whitespace().next().unwrap_or("");
        assert_eq!(hash_part.len(), 64, "SHA256 should be 64 hex chars");
        assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()), "Should be valid hex");
        assert!(result.output.contains(&path));

        tokio::fs::remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_hash_default_algorithm() {
        let executor = ToolExecutor::new();
        let path = format!("/tmp/tiny_claw_hash_test_{}.txt", uuid::Uuid::new_v4());
        tokio::fs::write(&path, "test").await.unwrap();

        let result = executor.execute("hash", serde_json::json!({
            "path": &path
        })).await;

        assert!(result.success); // defaults to sha256
        tokio::fs::remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_hash_unsupported_algorithm() {
        let executor = ToolExecutor::new();
        let path = format!("/tmp/tiny_claw_hash_test_{}.txt", uuid::Uuid::new_v4());
        tokio::fs::write(&path, "test").await.unwrap();

        let result = executor.execute("hash", serde_json::json!({
            "path": &path,
            "algorithm": "blake3"
        })).await;

        assert!(!result.success);
        assert!(result.error.is_some());

        // Cleanup
        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn test_execute_hash_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("hash", serde_json::json!({
            "path": "/tmp/nonexistent_tinyclaw_hash.txt"
        })).await;

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_hash_on_directory() {
        let executor = ToolExecutor::new();
        let result = executor.execute("hash", serde_json::json!({
            "path": "/tmp"
        })).await;

        assert!(!result.success);
    }

    // ============ wc tool tests ============

    #[tokio::test]
    async fn test_execute_wc_default() {
        let executor = ToolExecutor::new();
        let path = format!("/tmp/tiny_claw_wc_test_{}.txt", uuid::Uuid::new_v4());
        tokio::fs::write(&path, "hello world\nsecond line\nthird line").await.unwrap();

        let result = executor.execute("wc", serde_json::json!({
            "path": &path
        })).await;

        assert!(result.success);
        assert!(result.output.contains("3")); // 3 lines
        assert!(result.output.contains(&path));

        tokio::fs::remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_wc_specific_flags() {
        let executor = ToolExecutor::new();
        let path = format!("/tmp/tiny_claw_wc_test_{}.txt", uuid::Uuid::new_v4());
        tokio::fs::write(&path, "hello world\nsecond line").await.unwrap();

        let result = executor.execute("wc", serde_json::json!({
            "path": &path,
            "lines": true,
            "words": true
        })).await;

        // When specific flags are set (not using defaults), show_all becomes false
        // show_bytes defaults to true, so it will also be shown
        // Output format: "<lines> <words> <bytes> <path>"
        assert!(result.success, "wc failed: {:?}", result.error);
        // Contains line count
        assert!(result.output.contains("2"));
        // Contains word count  
        assert!(result.output.contains("4"));

        tokio::fs::remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_wc_missing_path() {
        let executor = ToolExecutor::new();
        let result = executor.execute("wc", serde_json::json!({})).await;

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_execute_wc_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute("wc", serde_json::json!({
            "path": "/tmp/nonexistent_tinyclaw_wc.txt"
        })).await;

        assert!(!result.success);
    }
}
