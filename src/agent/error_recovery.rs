//! Error Recovery Module
//!
//! Provides structured error classification and recovery guidance for tool failures.
//! Helps the agent understand what went wrong and how to potentially recover.
//!
//! Key design:
//! - Errors are classified into categories (NotFound, Permission, InvalidArg, etc.)
//! - Each error includes whether it's retryable and a recovery suggestion
//! - Structured error reports help the model self-correct on subsequent attempts

use serde::{Deserialize, Serialize};

/// Kinds of tool errors - helps the model understand what went wrong
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorKind {
    /// File or resource not found
    NotFound,
    /// Permission denied (file permissions, sudo, etc.)
    PermissionDenied,
    /// Invalid arguments passed to the tool
    InvalidArgument,
    /// Syntax error in user input or generated code
    SyntaxError,
    /// Network connectivity issue
    NetworkError,
    /// Operation timed out
    Timeout,
    /// Tool is unavailable or not registered
    ToolNotFound,
    /// Resource is busy (file locked, process running, etc.)
    ResourceBusy,
    /// Disk space exhausted
    OutOfSpace,
    /// Unknown or uncategorized error
    Unknown,
}

impl ToolErrorKind {
    /// Whether this kind of error is worth retrying
    /// Transient errors (network, timeout) may succeed on retry
    /// Permanent errors (not found, permission) will likely fail again
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ToolErrorKind::NetworkError | ToolErrorKind::Timeout | ToolErrorKind::ResourceBusy
        )
    }

    /// A short label for this error kind
    pub fn label(&self) -> &'static str {
        match self {
            ToolErrorKind::NotFound => "NOT_FOUND",
            ToolErrorKind::PermissionDenied => "PERMISSION_DENIED",
            ToolErrorKind::InvalidArgument => "INVALID_ARGUMENT",
            ToolErrorKind::SyntaxError => "SYNTAX_ERROR",
            ToolErrorKind::NetworkError => "NETWORK_ERROR",
            ToolErrorKind::Timeout => "TIMEOUT",
            ToolErrorKind::ToolNotFound => "TOOL_NOT_FOUND",
            ToolErrorKind::ResourceBusy => "RESOURCE_BUSY",
            ToolErrorKind::OutOfSpace => "OUT_OF_SPACE",
            ToolErrorKind::Unknown => "UNKNOWN",
        }
    }
}

/// Structured error recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecovery {
    /// The error category
    pub kind: ToolErrorKind,
    /// Whether retrying might succeed (for transient errors)
    #[serde(default)]
    pub retryable: bool,
    /// A brief suggestion for how the agent might recover
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// The original error message
    pub message: String,
}

impl ErrorRecovery {
    /// Classify a raw error string into a structured ErrorRecovery
    pub fn from_error(tool_name: &str, error_msg: &str) -> Self {
        let kind = classify_error(error_msg);
        let retryable = kind.is_retryable();
        let suggestion = generate_suggestion(tool_name, kind, error_msg);

        Self {
            kind,
            retryable,
            suggestion,
            message: error_msg.to_string(),
        }
    }

    /// Format as a human-readable error report for the model
    /// This is designed to be both human-readable and machine-parseable
    pub fn format_report(&self, tool_name: &str) -> String {
        let retry_hint = if self.retryable {
            " (may be retryable)"
        } else {
            " (not retryable)"
        };

        let mut report = format!(
            "[TOOL_ERROR] tool={}{}\n  error_kind={}\n  message={}",
            tool_name, retry_hint, self.kind.label(), self.message
        );

        if let Some(ref suggestion) = self.suggestion {
            report.push_str(&format!("\n  suggestion={}", suggestion));
        }

        report
    }
}

/// Classify an error message into a ToolErrorKind
fn classify_error(error_msg: &str) -> ToolErrorKind {
    let msg_lower = error_msg.to_lowercase();

    // Not found patterns
    if msg_lower.contains("no such file")
        || msg_lower.contains("not found")
        || msg_lower.contains("doesn't exist")
        || msg_lower.contains("does not exist")
        || msg_lower.contains("ENOENT")
        || msg_lower.contains("error 2") // Linux ENOENT
    {
        return ToolErrorKind::NotFound;
    }

    // Permission denied patterns
    if msg_lower.contains("permission denied")
        || msg_lower.contains("access denied")
        || msg_lower.contains("eacces")
        || msg_lower.contains("error 13") // Linux EACCES
        || msg_lower.contains("operation not permitted")
        || msg_lower.contains("sudo:")
        || msg_lower.contains("must be root")
    {
        return ToolErrorKind::PermissionDenied;
    }

    // Invalid argument patterns
    if msg_lower.contains("invalid argument")
        || msg_lower.contains("invalid option")
        || msg_lower.contains("expected ")
        || msg_lower.contains("missing required")
        || msg_lower.contains("invalid path")
        || msg_lower.contains("einval")
        || msg_lower.contains("error 22") // Linux EINVAL
        || msg_lower.contains("unrecognized")
        || msg_lower.contains("unexpected token")
        || msg_lower.contains("parse error")
    {
        return ToolErrorKind::InvalidArgument;
    }

    // Syntax error patterns
    if msg_lower.contains("syntax error")
        || msg_lower.contains("unexpected end")
        || msg_lower.contains("unterminated")
        || msg_lower.contains("parse error")
        || msg_lower.contains("unexpected token")
    {
        return ToolErrorKind::SyntaxError;
    }

    // Network error patterns
    if msg_lower.contains("connection refused")
        || msg_lower.contains("connection reset")
        || msg_lower.contains("connection timeout")
        || msg_lower.contains("network error")
        || msg_lower.contains("name or service not known")
        || msg_lower.contains("no route to host")
        || msg_lower.contains("dns")
        || msg_lower.contains("temporary failure")
        || msg_lower.contains("eai_noname")
        || msg_lower.contains("ssl")
        || msg_lower.contains("tls")
    {
        return ToolErrorKind::NetworkError;
    }

    // Timeout patterns
    if msg_lower.contains("timed out")
        || msg_lower.contains("timeout")
        || msg_lower.contains("etimedout")
        || msg_lower.contains("deadline exceeded")
    {
        return ToolErrorKind::Timeout;
    }

    // Resource busy patterns
    if msg_lower.contains("resource busy")
        || msg_lower.contains("file is busy")
        || msg_lower.contains("device or resource busy")
        || msg_lower.contains("etext file busy")
        || msg_lower.contains("ebusy")
        || msg_lower.contains("process")
        || msg_lower.contains("locked")
    {
        return ToolErrorKind::ResourceBusy;
    }

    // Out of space patterns
    if msg_lower.contains("no space left")
        || msg_lower.contains("disk full")
        || msg_lower.contains("out of space")
        || msg_lower.contains("enospc")
        || msg_lower.contains("error 28")
        || msg_lower.contains("quota exceeded")
    {
        return ToolErrorKind::OutOfSpace;
    }

    // Tool not found patterns
    if msg_lower.contains("tool not found")
        || msg_lower.contains("unknown tool")
        || msg_lower.contains("executor error")
    {
        return ToolErrorKind::ToolNotFound;
    }

    ToolErrorKind::Unknown
}

/// Generate a recovery suggestion based on error kind and context
fn generate_suggestion(_tool_name: &str, kind: ToolErrorKind, error_msg: &str) -> Option<String> {
    match kind {
        ToolErrorKind::NotFound => {
            // Check if it's a command vs file
            if error_msg.contains("no such file") && (error_msg.contains("/") || error_msg.contains(".")) {
                Some("Verify the file path is correct. Check for typos or missing directory components.".to_string())
            } else {
                Some("Check if the command is installed. Try using the full path or installing the required tool.".to_string())
            }
        }
        ToolErrorKind::PermissionDenied => {
            Some("Check file permissions with 'ls -la'. Use 'chmod' to fix permissions, or run with appropriate privileges.".to_string())
        }
        ToolErrorKind::InvalidArgument => {
            Some("Review the tool's input schema. Check that all required arguments are provided with correct types.".to_string())
        }
        ToolErrorKind::SyntaxError => {
            Some("Check the input syntax. For shell commands, verify quoting and escaping. For file content, check for balanced brackets and quotes.".to_string())
        }
        ToolErrorKind::NetworkError => {
            Some("This may be a transient network issue. A retry might succeed. Check your network connection.".to_string())
        }
        ToolErrorKind::Timeout => {
            Some("The operation took too long. Try increasing the timeout or simplifying the operation.".to_string())
        }
        ToolErrorKind::ToolNotFound => {
            Some("Ensure the tool is registered and available in the current context.".to_string())
        }
        ToolErrorKind::ResourceBusy => {
            Some("Wait briefly and retry. The resource may become available shortly.".to_string())
        }
        ToolErrorKind::OutOfSpace => {
            Some("Free up disk space by removing unnecessary files, or use a different location with more space.".to_string())
        }
        ToolErrorKind::Unknown => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_labels() {
        assert_eq!(ToolErrorKind::NotFound.label(), "NOT_FOUND");
        assert_eq!(ToolErrorKind::PermissionDenied.label(), "PERMISSION_DENIED");
        assert_eq!(ToolErrorKind::InvalidArgument.label(), "INVALID_ARGUMENT");
        assert_eq!(ToolErrorKind::Unknown.label(), "UNKNOWN");
    }

    #[test]
    fn test_retryable_kinds() {
        assert!(ToolErrorKind::NetworkError.is_retryable());
        assert!(ToolErrorKind::Timeout.is_retryable());
        assert!(ToolErrorKind::ResourceBusy.is_retryable());
        assert!(!ToolErrorKind::NotFound.is_retryable());
        assert!(!ToolErrorKind::PermissionDenied.is_retryable());
    }

    #[test]
    fn test_classify_not_found() {
        let err = "Error: ENOENT: no such file or directory, open '/path/to/file'";
        assert_eq!(classify_error(err), ToolErrorKind::NotFound);

        let err2 = "file not found";
        assert_eq!(classify_error(err2), ToolErrorKind::NotFound);
    }

    #[test]
    fn test_classify_permission() {
        let err = "Permission denied: /path/to/file";
        assert_eq!(classify_error(err), ToolErrorKind::PermissionDenied);

        let err2 = "Error: EACCES: permission denied";
        assert_eq!(classify_error(err2), ToolErrorKind::PermissionDenied);
    }

    #[test]
    fn test_classify_network() {
        let err = "Connection refused";
        assert_eq!(classify_error(err), ToolErrorKind::NetworkError);

        let err2 = "name or service not known";
        assert_eq!(classify_error(err2), ToolErrorKind::NetworkError);
    }

    #[test]
    fn test_classify_timeout() {
        let err = "Operation timed out";
        assert_eq!(classify_error(err), ToolErrorKind::Timeout);

        let err2 = "ETIMEDOUT";
        assert_eq!(classify_error(err2), ToolErrorKind::Timeout);
    }

    #[test]
    fn test_error_recovery_from_error() {
        let recovery = ErrorRecovery::from_error("exec", "Error: ENOENT: no such file");
        assert_eq!(recovery.kind, ToolErrorKind::NotFound);
        assert!(!recovery.retryable);
        assert!(recovery.suggestion.is_some());
    }

    #[test]
    fn test_error_recovery_format() {
        let recovery = ErrorRecovery::from_error("exec", "Error: ENOENT: no such file");
        let report = recovery.format_report("exec");
        assert!(report.contains("TOOL_ERROR"));
        assert!(report.contains("NOT_FOUND"));
        assert!(report.contains("no such file"));
        assert!(report.contains("suggestion="));
    }

    #[test]
    fn test_network_error_is_retryable() {
        let recovery = ErrorRecovery::from_error("http", "Connection reset by peer");
        assert!(recovery.retryable);
        assert!(recovery.suggestion.is_some());
    }

    #[test]
    fn test_unknown_error_not_retryable() {
        let recovery = ErrorRecovery::from_error("unknown", "Something went wrong");
        assert!(!recovery.retryable);
        assert!(recovery.suggestion.is_none());
    }

    #[test]
    fn test_out_of_space_classification() {
        let err = "Error: ENOSPC: no space left on device";
        assert_eq!(classify_error(err), ToolErrorKind::OutOfSpace);
    }

    #[test]
    fn test_resource_busy_classification() {
        let err = "Error: EBUSY: resource is busy";
        assert_eq!(classify_error(err), ToolErrorKind::ResourceBusy);
    }
}
