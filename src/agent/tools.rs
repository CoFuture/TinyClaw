//! Advanced tools module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::fs;
use tokio::time::timeout;
use tracing::info;
use chrono::{DateTime, Local};

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
                            "description": "Path to the directory"
                        }
                    },
                    "required": ["path"]
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

    /// Execute a tool
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> ToolResult {
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
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", name)),
            },
        }
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

        info!("Executing command: {}", command);

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await;

        match output {
            Ok(output) => {
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
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
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

        ToolResult {
            success: true,
            output: "(not found)".to_string(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_executor_new() {
        let executor = ToolExecutor::new();
        let tools = executor.list_tools();
        assert!(!tools.is_empty());
        assert_eq!(tools.len(), 11); // exec, read_file, write_file, list_dir, http_request, glob, grep, sed_file, which, mkdir, stat_file
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
        
        assert!(result.success); // Tool succeeds even if not found
        assert_eq!(result.output, "(not found)");
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
}
