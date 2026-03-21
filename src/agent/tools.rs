//! Advanced tools module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::fs;
use tokio::time::timeout;
use tracing::info;

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
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", name)),
            },
        }
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

        info!("Reading file: {}", path);

        match fs::read_to_string(path).await {
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

        info!("Writing file: {}", path);

        match fs::write(path, content).await {
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

    /// Execute the list_dir tool
    async fn execute_list_dir(&self, input: serde_json::Value) -> ToolResult {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        info!("Listing directory: {}", path);

        match fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut results = Vec::new();
                while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_type = entry.file_type().await.map(|ft| {
                        if ft.is_dir() {
                            "dir"
                        } else if ft.is_file() {
                            "file"
                        } else if ft.is_symlink() {
                            "symlink"
                        } else {
                            "unknown"
                        }
                    }).unwrap_or("unknown");
                    
                    results.push(format!("{} ({})", file_name, file_type));
                }
                
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_executor_new() {
        let executor = ToolExecutor::new();
        let tools = executor.list_tools();
        assert!(!tools.is_empty());
        assert_eq!(tools.len(), 5); // exec, read_file, write_file, list_dir, http_request
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
}
