//! AI Model client with multi-provider support

use crate::agent::context_summarizer::{ContextSummary, SummarizerConfig, SummaryHistoryEntry, SummaryHistoryManager, SummaryHistoryStats};
use crate::agent::retry::{with_retry, CircuitBreaker, CircuitState, RetrySettings};
use uuid::Uuid;
use crate::agent::tools::{Tool, ToolExecutor, ToolResult};
use crate::common::{Error, Result};
use crate::config::{AgentConfig, ModelProvider};
use crate::gateway::events::EventEmitter;
use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{debug, error, info};

/// Anthropic API request
#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Anthropic API message
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
struct Message {
    role: String,
    content: String,
}

/// Anthropic API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    #[serde(rename = "usage")]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    #[serde(rename = "input_tokens")]
    input_tokens: u32,
    #[serde(rename = "output_tokens")]
    output_tokens: u32,
}

/// OpenAI API request
#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// OpenAI API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    completion_tokens: u32,
}

/// Ollama API request
#[derive(Debug, Serialize, Clone)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Ollama API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    response: String,
}

/// Ollama streaming SSE response line
#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    /// Text chunk
    response: String,
    /// Whether this is the final chunk
    done: bool,
}

/// Tool execution record for tracking during a turn (alias for turn_history::ToolExecution)
pub use crate::agent::turn_history::ToolExecution;

/// Pending action plan waiting for user confirmation
#[derive(Debug)]
#[allow(dead_code)]
struct PendingActionPlan {
    /// Unique plan ID
    plan_id: String,
    /// Tools planned for execution
    tools: Vec<crate::gateway::events::ToolCallPreview>,
    /// Channel to send confirmation response (wrapped in Mutex for shared access)
    response_tx: Mutex<Option<oneshot::Sender<bool>>>,
}

/// Re-export TokenUsage from turn_history for use in the agent client
pub use crate::agent::turn_history::TokenUsage;

/// Agent client for AI model interaction
pub struct Agent {
    config: Arc<RwLock<AgentConfig>>,
    retry_config: RetrySettings,
    http_client: reqwest::Client,
    tool_executor: Arc<ToolExecutor>,
    /// Active turn cancellation channels (session_id -> sender)
    turn_cancellations: RwLock<HashMap<String, broadcast::Sender<()>>>,
    /// Circuit breaker for AI API calls (prevents cascading failures)
    circuit_breaker: Arc<CircuitBreaker>,
    /// Event emitter for tool and turn events (optional, for gateway integration)
    event_emitter: Option<Arc<EventEmitter>>,
    /// Current session key for tool event emission (set during send_message_streaming)
    current_session_key: RwLock<Option<String>>,
    /// Tool executions recorded during the last send_message_with_history call
    tool_executions: Arc<RwLock<Vec<ToolExecution>>>,
    /// Token usage recorded during the last send_message call
    token_usage: Arc<RwLock<Option<TokenUsage>>>,
    /// Pending action plan waiting for user confirmation (session_key -> plan)
    pending_action_plan: RwLock<Option<PendingActionPlan>>,
    /// Summarizer configuration
    summarizer_config: RwLock<SummarizerConfig>,
    /// Summary history manager for tracking summarization events
    summary_history: Arc<SummaryHistoryManager>,
}

/// History message for passing conversation context to API methods
#[derive(Debug, Clone)]
struct HistoryMessage {
    role: String,
    content: String,
}

impl From<&(String, String)> for HistoryMessage {
    fn from(tuple: &(String, String)) -> Self {
        Self {
            role: tuple.0.clone(),
            content: tuple.1.clone(),
        }
    }
}

impl From<&crate::types::Message> for HistoryMessage {
    fn from(msg: &crate::types::Message) -> Self {
        match msg.role {
            crate::types::Role::User => Self {
                role: "user".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::Assistant => Self {
                role: "assistant".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::System => Self {
                role: "system".to_string(),
                content: msg.content.clone(),
            },
            crate::types::Role::Tool => {
                // Tool messages: encode tool_call_id and tool_name in content
                // Format: "[TOOL:tool_name:id] content"
                let name = msg.tool_name.as_deref().unwrap_or("unknown");
                let id = msg.tool_call_id.as_deref().unwrap_or("unknown");
                let encoded = format!("[TOOL:{}:{}] {}", name, id, msg.content);
                Self {
                    role: "user".to_string(), // Tool results as user role
                    content: encoded,
                }
            }
        }
    }
}

impl HistoryMessage {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "role": self.role,
            "content": self.content
        })
    }
}

impl Agent {
    pub fn new(config: Arc<RwLock<AgentConfig>>) -> Self {
        Self {
            config,
            retry_config: RetrySettings::default(),
            http_client: reqwest::Client::new(),
            tool_executor: Arc::new(ToolExecutor::new()),
            turn_cancellations: RwLock::new(HashMap::new()),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            event_emitter: None,
            current_session_key: RwLock::new(None),
            tool_executions: Arc::new(RwLock::new(Vec::new())),
            token_usage: Arc::new(RwLock::new(None)),
            pending_action_plan: RwLock::new(None),
            summarizer_config: RwLock::new(SummarizerConfig::default()),
            summary_history: Arc::new(SummaryHistoryManager::new()),
        }
    }

    /// Create Agent with custom retry settings
    #[allow(dead_code)]
    pub fn with_retry(config: Arc<RwLock<AgentConfig>>, retry: RetrySettings) -> Self {
        Self {
            config,
            retry_config: retry,
            http_client: reqwest::Client::new(),
            tool_executor: Arc::new(ToolExecutor::new()),
            turn_cancellations: RwLock::new(HashMap::new()),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            event_emitter: None,
            current_session_key: RwLock::new(None),
            tool_executions: Arc::new(RwLock::new(Vec::new())),
            token_usage: Arc::new(RwLock::new(None)),
            pending_action_plan: RwLock::new(None),
            summarizer_config: RwLock::new(SummarizerConfig::default()),
            summary_history: Arc::new(SummaryHistoryManager::new()),
        }
    }

    /// Get the current summarizer configuration
    pub fn get_summarizer_config(&self) -> SummarizerConfig {
        self.summarizer_config.read().clone()
    }

    /// Update the summarizer configuration
    pub fn update_summarizer_config(&self, min_messages: Option<usize>, token_threshold: Option<usize>, enabled: Option<bool>) -> SummarizerConfig {
        let mut config = self.summarizer_config.write();
        config.update(min_messages, token_threshold, enabled);
        info!("Summarizer config updated: min_msgs={}, threshold={}, enabled={}",
              config.min_messages, config.token_threshold, config.enabled);
        config.clone()
    }

    /// Record a summary event to history
    pub fn record_summary(&self, session_id: &str, summary: &ContextSummary) {
        self.summary_history.record(session_id, summary);
    }

    /// Summarize conversation history and record to history.
    /// This is called by the gateway to summarize accumulated context.
    pub async fn summarize_and_record(&self, session_id: &str, messages: &[(String, String)]) -> Result<ContextSummary> {
        // Check if summarization is enabled using config getters
        let (enabled, min_messages, token_threshold) = {
            let config = self.summarizer_config.read();
            (config.is_enabled(), config.min_messages(), config.token_threshold())
        };

        if !enabled {
            return Err(Error::Agent("Summarization is disabled".into()));
        }
        if messages.len() < min_messages {
            return Err(Error::Agent(format!("Not enough messages to summarize: {} < {}", messages.len(), min_messages)));
        }

        // Estimate tokens and check threshold
        let estimated_tokens: usize = messages.iter().map(|(_, c)| c.len() / 4).sum();
        if estimated_tokens < token_threshold {
            return Err(Error::Agent(format!("Token threshold not reached: {} < {}", estimated_tokens, token_threshold)));
        }

        // Format messages for summarization
        let conversation_text = messages
            .iter()
            .map(|(role, content)| format!("[{}]: {}", role, content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Estimate original tokens (~4 chars per token)
        let original_tokens = conversation_text.len() / 4;

        // Build summarization prompt
        let summary_prompt = format!(
            r#"Please summarize the following conversation history. Your summary should:

1. Preserve ALL important decisions made
2. Note any user preferences or requirements mentioned
3. Track the progression of topics discussed
4. Remember key information that might be needed later

Format your response as:

TOPICS: [list main topics, comma-separated]
DECISIONS: [list key decisions, one per line, starting with -]
TOOLS: [list tools/commands used, comma-separated]

SUMMARY:
[Write a concise narrative summary of the conversation, focusing on information that would be important for continuing this conversation. Include specific details like file names, code snippets discussed, error messages encountered, etc.]

---

CONVERSATION TO SUMMARIZE:

{}"#,
            conversation_text
        );

        // Generate summary using AI
        let summary_text = self.summarize_content(&summary_prompt).await?;

        // Parse structured info from summary text
        let (topics, decisions, tools) = self.parse_summary_structured_info(&summary_text);

        // Calculate summary tokens
        let summary_tokens = summary_text.len() / 4;

        // Create context summary
        let summary = ContextSummary::new(
            summary_text,
            messages.len(),
            original_tokens,
            topics,
            decisions,
            tools,
        );

        // Record to history
        self.record_summary(session_id, &summary);

        info!(
            session_id = session_id,
            messages_summarized = messages.len(),
            original_tokens = original_tokens,
            summary_tokens = summary_tokens,
            compression = format!("{:.1}%", summary.compression_ratio() * 100.0),
            "Session history summarized and recorded"
        );

        Ok(summary)
    }

    /// Parse structured info (topics, decisions, tools) from summary text
    fn parse_summary_structured_info(&self, summary_text: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut topics = Vec::new();
        let mut decisions = Vec::new();
        let mut tools = Vec::new();

        for line in summary_text.lines() {
            let line = line.trim();

            if line.starts_with("TOPICS:") {
                let t = line.strip_prefix("TOPICS:").unwrap_or("");
                topics = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            if line.starts_with("- ") {
                decisions.push(line.strip_prefix("- ").unwrap_or(line).to_string());
            }
            if line.starts_with("TOOLS:") {
                let t = line.strip_prefix("TOOLS:").unwrap_or("");
                tools = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
        }

        // Also extract tool names mentioned in the summary if not already captured
        let tool_keywords = ["read_file", "write_file", "exec", "http_request", "list_dir",
                            "grep", "find", "glob", "cp", "mv", "rm", "cat", "mkdir", "touch"];
        for tool in tool_keywords {
            if summary_text.contains(tool) && !tools.contains(&tool.to_string()) {
                tools.push(tool.to_string());
            }
        }

        (topics, decisions, tools)
    }

    /// Get summary history statistics
    pub fn get_summary_stats(&self) -> SummaryHistoryStats {
        self.summary_history.stats()
    }

    /// Get recent summary history entries
    pub fn get_summary_history(&self, limit: usize) -> Vec<SummaryHistoryEntry> {
        self.summary_history.recent(limit)
    }

    /// Get summary history for a specific session
    pub fn get_session_summary_history(&self, session_id: &str) -> Vec<SummaryHistoryEntry> {
        self.summary_history.for_session(session_id)
    }

    /// Set an action plan waiting for confirmation and return a receiver to wait on.
    /// Returns the plan_id and a oneshot receiver.
    fn set_pending_action_plan(&self, plan_id: String, tools: Vec<crate::gateway::events::ToolCallPreview>) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        let plan = PendingActionPlan {
            plan_id,
            tools,
            response_tx: Mutex::new(Some(tx)),
        };
        *self.pending_action_plan.write() = Some(plan);
        rx
    }

    /// Clear the pending action plan (called when turn ends)
    fn clear_pending_action_plan(&self) {
        *self.pending_action_plan.write() = None;
    }

    /// Get the plan_id of the current pending action plan, if any
    #[allow(dead_code)]
    fn get_pending_plan_id(&self) -> Option<String> {
        self.pending_action_plan.read().as_ref().map(|p| p.plan_id.clone())
    }

    /// Take and clear the tool executions recorded during the last turn.
    /// Returns the collected tool executions and clears the buffer.
    #[allow(dead_code)]
    pub fn take_tool_executions(&self) -> Vec<ToolExecution> {
        let mut executions = self.tool_executions.write();
        let result = executions.clone();
        executions.clear();
        result
    }

    /// Take and clear the token usage recorded during the last turn.
    /// Returns the token usage if available, and clears the buffer.
    #[allow(dead_code)]
    pub fn take_token_usage(&self) -> Option<TokenUsage> {
        self.token_usage.write().take()
    }

    /// Set token usage (called internally after API calls)
    fn set_token_usage(&self, usage: Option<TokenUsage>) {
        *self.token_usage.write() = usage;
    }

    /// Record a tool execution during message processing.
    fn record_tool_execution(&self, name: String, input: serde_json::Value, output: String, success: bool, duration_ms: u64) {
        let preview = if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output
        };
        self.tool_executions.write().push(ToolExecution {
            name,
            input,
            output_preview: preview,
            success,
            duration_ms,
        });
    }

    /// Set the event emitter for tool and turn events
    pub fn with_event_emitter(mut self, emitter: Arc<EventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Set the current session key for tool event tracking
    fn set_session_key(&self, session_key: Option<&str>) {
        *self.current_session_key.write() = session_key.map(String::from);
    }

    /// Get the current session key
    fn get_session_key(&self) -> Option<String> {
        self.current_session_key.read().clone()
    }

    /// Emit a tool use event if event emitter is configured
    fn emit_tool_use(&self, tool: &str, input: serde_json::Value) {
        if let Some(emitter) = &self.event_emitter {
            let session_id = self.get_session_key().unwrap_or_else(|| "unknown".to_string());
            emitter.emit(crate::gateway::events::Event::AssistantToolUse {
                session_id,
                tool: tool.to_string(),
                input,
            });
        }
    }

    /// Emit an action plan preview event showing all planned tool calls
    #[allow(dead_code)]
    fn emit_action_plan_preview(&self, tools: Vec<crate::gateway::events::ToolCallPreview>) {
        if let Some(emitter) = &self.event_emitter {
            let session_id = self.get_session_key().unwrap_or_else(|| "unknown".to_string());
            emitter.emit(crate::gateway::events::Event::ActionPlanPreview {
                session_id,
                tools,
            });
        }
    }

    /// Emit a tool result event if event emitter is configured
    fn emit_tool_result(&self, tool_call_id: &str, output: &str, _success: bool) {
        if let Some(emitter) = &self.event_emitter {
            let session_id = self.get_session_key().unwrap_or_else(|| "unknown".to_string());
            emitter.emit(crate::gateway::events::Event::ToolResult {
                session_id,
                tool_call_id: tool_call_id.to_string(),
                output: output.to_string(),
            });
        }
    }

    /// Start a new turn cancellation channel for a session.
    /// Returns a receiver that will be notified when cancel is called.
    /// If a turn is already active, returns None.
    pub fn start_turn_cancellation(&self, session_key: &str) -> Option<broadcast::Receiver<()>> {
        let mut cancellations = self.turn_cancellations.write();
        // Check if a turn is already active
        if cancellations.contains_key(session_key) {
            return None;
        }
        let (tx, rx) = broadcast::channel(1);
        cancellations.insert(session_key.to_string(), tx);
        Some(rx)
    }

    /// Cancel an ongoing turn for a session.
    /// Returns true if a turn was cancelled, false if no turn was active.
    pub fn cancel_turn(&self, session_key: &str) -> bool {
        let mut cancellations = self.turn_cancellations.write();
        if let Some(tx) = cancellations.remove(session_key) {
            // Drop the sender to notify the receiver
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    /// Clean up cancellation channel after turn ends.
    fn cleanup_turn_cancellation(&self, session_key: &str) {
        let mut cancellations = self.turn_cancellations.write();
        cancellations.remove(session_key);
    }

    /// Confirm or deny a pending action plan.
    /// Returns true if the plan was found and confirmation was sent, false otherwise.
    /// The `confirmed` parameter indicates whether to execute (true) or cancel (false) the plan.
    pub fn confirm_action(&self, session_key: &str, plan_id: &str, confirmed: bool) -> bool {
        let pending = self.pending_action_plan.read();
        if let Some(ref plan) = *pending {
            if plan.plan_id == plan_id {
                // Try to take the sender from the mutex and send confirmation
                if let Ok(mut guard) = plan.response_tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(confirmed);
                        info!(
                            session_id = %session_key,
                            plan_id = %plan_id,
                            confirmed = confirmed,
                            "Action plan confirmation sent to agent"
                        );
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Update retry settings
    #[allow(dead_code)]
    pub fn set_retry_settings(&mut self, retry: RetrySettings) {
        self.retry_config = retry;
    }

    /// Get list of available tools
    #[allow(dead_code)]
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_executor.list_tools()
    }

    /// Get current circuit breaker state
    pub fn circuit_breaker_state(&self) -> CircuitState {
        self.circuit_breaker.state()
    }

    /// Execute an HTTP request with circuit breaker protection + retry logic.
    /// Returns error if circuit is open (fast-fail).
    async fn execute_protected<F, Fut>(&self, f: F) -> std::result::Result<reqwest::Response, Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<reqwest::Response, Error>>,
    {
        // Check circuit breaker first
        if !self.circuit_breaker.is_allowed() {
            return Err(Error::Network(
                "AI provider circuit breaker is open - service unavailable".into(),
            ));
        }

        let cb = self.circuit_breaker.clone();
        let result = with_retry(&self.retry_config, f).await;

        match result {
            Ok(response) => {
                cb.record_success();
                Ok(response)
            }
            Err(e) => {
                cb.record_failure();
                Err(e)
            }
        }
    }

    /// Send a message with conversation history and get a response (with tool calling loop)
///
/// # Arguments
/// * `session_key` - Session identifier (unused, kept for API compatibility)
/// * `message` - User message to send
/// * `history` - Conversation history to prepend (role, content tuples)
/// * `skill_prompt` - Optional skill instructions to inject into system prompt
pub async fn send_message_with_history(
    &self,
    _session_key: &str,
    message: &str,
    history: &[(String, String)],
    skill_prompt: Option<&str>,
) -> Result<String> {
        let config = self.config.read().clone();

        // Determine provider based on model name or explicit config
        let provider = config.provider.clone().unwrap_or_else(|| {
            if config.model.starts_with("claude-") || config.model.starts_with("anthropic/") {
                ModelProvider::Anthropic
            } else if config.model.starts_with("gpt-") || config.model.starts_with("openai/") {
                ModelProvider::OpenAI
            } else {
                ModelProvider::Ollama
            }
        });

        // Build messages: prepend history, then add current user message
        let mut messages: Vec<serde_json::Value> = history
            .iter()
            .map(HistoryMessage::from)
            .map(|h| h.to_json())
            .collect();
        
        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        // Maximum tool call iterations
        let max_turns = 10;
        let tools = self.tool_executor.list_tools();

        for turn in 0..max_turns {
            debug!("Tool loop turn {}", turn + 1);

            match provider {
                ModelProvider::Anthropic => {
                    let response = self.send_anthropic_with_tools(&config, &messages, &tools, skill_prompt).await?;
                    
                    // Extract token usage from response (only on final turn to avoid overwriting)
                    if !response["usage"]["input_tokens"].is_null() {
                        let input_tokens = response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
                        let output_tokens = response["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
                        self.set_token_usage(Some(TokenUsage {
                            input_tokens,
                            output_tokens,
                            total_tokens: input_tokens + output_tokens,
                        }));
                    }
                    
                    // Extract text content
                    let text_content: Option<String> = response["content"]
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .filter_map(|c| c.get("text").and_then(|t| t.as_str().map(String::from)))
                                .next()
                        });

                    // Check for tool_use in response
                    let tool_calls = response["content"]
                        .as_array()
                        .map(|arr| {
                            arr.iter().filter_map(|c| c.get("tool_use").cloned()).collect::<Vec<_>>()
                        })
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);

                    if tool_calls {
                        // Get tool_use blocks
                        let tool_use_blocks = response["content"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|c| c.get("tool_use").is_some())
                            .filter_map(|c| c.get("tool_use").cloned())
                            .collect::<Vec<_>>();

                        // Add assistant message with tool use
                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": tool_use_blocks
                        }));

                        // Emit action plan preview (shows all planned tools before execution)
                        let preview_tools: Vec<_> = tool_use_blocks.iter().filter_map(|tb| {
                            Some(crate::gateway::events::ToolCallPreview {
                                id: tb["id"].as_str()?.to_string(),
                                name: tb["name"].as_str()?.to_string(),
                                input: tb["input"].clone(),
                            })
                        }).collect();
                        
                        // If tools are planned, wait for user confirmation before executing
                        let confirmed = if !preview_tools.is_empty() {
                            let plan_id = Uuid::new_v4().to_string();
                            let confirm_rx = self.set_pending_action_plan(plan_id.clone(), preview_tools.clone());
                            
                            // Emit confirmation request event
                            let session_id = self.get_session_key().unwrap_or_else(|| "unknown".to_string());
                            if let Some(emitter) = &self.event_emitter {
                                emitter.emit(crate::gateway::events::Event::ActionPlanConfirm {
                                    session_id,
                                    plan_id,
                                    tools: preview_tools,
                                });
                            }
                            
                            // Wait for confirmation (60 second timeout)
                            // If timeout or error, treat as cancelled
                            match timeout(Duration::from_secs(60), confirm_rx).await {
                                Ok(Ok(true)) => true,  // User confirmed
                                Ok(Ok(false)) => false, // User cancelled
                                _ => false, // Timeout or error
                            }
                        } else {
                            true  // No tools, no confirmation needed
                        };
                        
                        // Clear pending action plan after confirmation received
                        self.clear_pending_action_plan();
                        
                        // Execute tools only if confirmed
                        if !confirmed {
                            // User denied or timeout - return action denied error
                            return Err(Error::ActionDenied);
                        }
                        
                        // Execute tools and add results
                        for tool_block in &tool_use_blocks {
                            let tool_name = tool_block["name"].as_str().unwrap_or("");
                            let tool_input = tool_block["input"].clone();
                            
                            // Emit tool use event
                            self.emit_tool_use(tool_name, tool_input.clone());
                            
                            // Execute tool (with retry for transient errors)
                            let tool_start = Instant::now();
                            let (result, retries) = self.tool_executor.execute_with_retry(tool_name, tool_input.clone()).await;
                            let tool_duration_ms = tool_start.elapsed().as_millis() as u64;
                            
                            // Log if retries occurred
                            if retries > 0 {
                                debug!(
                                    tool = %tool_name,
                                    retries = retries,
                                    "Tool execution succeeded after {} retries", retries
                                );
                            }
                            
                            // Format tool result - use structured error report on failure
                            let content = format_tool_result_content(tool_name, &result);
                            
                            // Record tool execution for turn history
                            self.record_tool_execution(
                                tool_name.to_string(),
                                tool_input,
                                content.clone(),
                                result.success,
                                tool_duration_ms,
                            );
                            
                            // Emit tool result event
                            let tool_call_id = tool_block["id"].as_str().unwrap_or("");
                            self.emit_tool_result(tool_call_id, &content, result.success);
                            
                            // Record tool timing
                            debug!(
                                tool = %tool_name,
                                duration_ms = tool_duration_ms,
                                success = result.success,
                                "Tool execution completed"
                            );
                            
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "content": content,
                                "tool_use_id": tool_block["id"]
                            }));
                        }
                    } else {
                        // No tool calls, return the text response
                        return Ok(text_content.unwrap_or_default());
                    }
                }
                ModelProvider::OpenAI => {
                    let response = self.send_openai_with_tools(&config, &messages, &tools, skill_prompt).await?;
                    
                    // Extract token usage from response (only on final turn)
                    if !response["usage"]["prompt_tokens"].is_null() {
                        let input_tokens = response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
                        let output_tokens = response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;
                        self.set_token_usage(Some(TokenUsage {
                            input_tokens,
                            output_tokens,
                            total_tokens: input_tokens + output_tokens,
                        }));
                    }
                    
                    // Check for tool_calls in response
                    let has_tool_calls = response["choices"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("tool_calls"))
                        .is_some();

                    if has_tool_calls {
                        let choices = response["choices"].as_array().unwrap();
                        let message_obj = &choices[0]["message"];
                        
                        // Add assistant message
                        messages.push(message_obj.clone());

                        let tool_calls = message_obj["tool_calls"].as_array().unwrap();
                        
                        // Emit action plan preview (shows all planned tools before execution)
                        let preview_tools: Vec<_> = tool_calls.iter().filter_map(|tc| {
                            let name = tc["function"]["name"].as_str()?;
                            let args: serde_json::Value = serde_json::from_str(
                                tc["function"]["arguments"].as_str().unwrap_or("{}")
                            ).unwrap_or(serde_json::json!({}));
                            Some(crate::gateway::events::ToolCallPreview {
                                id: tc["id"].as_str()?.to_string(),
                                name: name.to_string(),
                                input: args,
                            })
                        }).collect();
                        
                        // If tools are planned, wait for user confirmation before executing
                        let confirmed = if !preview_tools.is_empty() {
                            let plan_id = Uuid::new_v4().to_string();
                            let confirm_rx = self.set_pending_action_plan(plan_id.clone(), preview_tools.clone());
                            
                            // Emit confirmation request event
                            let session_id = self.get_session_key().unwrap_or_else(|| "unknown".to_string());
                            if let Some(emitter) = &self.event_emitter {
                                emitter.emit(crate::gateway::events::Event::ActionPlanConfirm {
                                    session_id,
                                    plan_id,
                                    tools: preview_tools,
                                });
                            }
                            
                            // Wait for confirmation (60 second timeout)
                            match timeout(Duration::from_secs(60), confirm_rx).await {
                                Ok(Ok(true)) => true,
                                Ok(Ok(false)) => false,
                                _ => false,
                            }
                        } else {
                            true
                        };
                        
                        // Clear pending action plan after confirmation received
                        self.clear_pending_action_plan();
                        
                        // Execute tools only if confirmed
                        if !confirmed {
                            return Err(Error::ActionDenied);
                        }
                        
                        // Execute tools and add results
                        for tool_call in tool_calls {
                            let tool_name = tool_call["function"]["name"].as_str().unwrap_or("");
                            let args: serde_json::Value = serde_json::from_str(
                                tool_call["function"]["arguments"].as_str().unwrap_or("{}")
                            ).unwrap_or(serde_json::json!({}));
                            
                            // Emit tool use event
                            self.emit_tool_use(tool_name, args.clone());
                            
                            // Execute tool (with retry for transient errors)
                            let tool_start = Instant::now();
                            let (result, retries) = self.tool_executor.execute_with_retry(tool_name, args.clone()).await;
                            let tool_duration_ms = tool_start.elapsed().as_millis() as u64;
                            
                            // Log if retries occurred
                            if retries > 0 {
                                debug!(
                                    tool = %tool_name,
                                    retries = retries,
                                    "Tool execution succeeded after {} retries", retries
                                );
                            }
                            
                            // Format tool result - use structured error report on failure
                            let content = format_tool_result_content(tool_name, &result);
                            
                            // Record tool execution for turn history
                            self.record_tool_execution(
                                tool_name.to_string(),
                                args,
                                content.clone(),
                                result.success,
                                tool_duration_ms,
                            );
                            
                            // Emit tool result event
                            let tool_call_id = tool_call["id"].as_str().unwrap_or("");
                            self.emit_tool_result(tool_call_id, &content, result.success);
                            
                            // Record tool timing
                            debug!(
                                tool = %tool_name,
                                duration_ms = tool_duration_ms,
                                success = result.success,
                                "Tool execution completed"
                            );
                            
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "content": content,
                                "tool_call_id": tool_call["id"]
                            }));
                        }
                    } else {
                        // No tool calls, return the text response
                        let text = response["choices"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|c| c.get("message"))
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                            .map(String::from)
                            .unwrap_or_default();
                        return Ok(text);
                    }
                }
                ModelProvider::Ollama => {
                    // Ollama doesn't support tools or token usage tracking
                    self.set_token_usage(None);
                    return self.send_ollama(&config, message).await;
                }
            }
        }

        Ok("Max tool call iterations reached".to_string())
    }

    /// Send message to Anthropic API with tools
    async fn send_anthropic_with_tools(
        &self,
        config: &AgentConfig,
        messages: &[serde_json::Value],
        tools: &[Tool],
        skill_prompt: Option<&str>,
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // Build base system prompt
        let base_system = "You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You have access to tools to help you answer questions. You should introduce yourself as TinyClaw powered by GLM-5 when asked.";
        let system = if let Some(skills) = skill_prompt {
            format!("{}\n\n{}", base_system, skills)
        } else {
            base_system.to_string()
        };

        // Convert tools to Anthropic format
        let anthropic_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema
                })
            })
            .collect();

        // Build request with tools
        let request = serde_json::json!({
            "model": model,
            "max_tokens": 1024,
            "messages": messages,
            "system": system,
            "tools": anthropic_tools
        });

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base = api_base.clone();
            let api_key = api_key.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/messages", api_base))
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        // Parse response as JSON value
        let response: serde_json::Value = result.json().await?;

        debug!("Anthropic tool response received");

        Ok(response)
    }

    /// Send message to OpenAI API with tools
    async fn send_openai_with_tools(
        &self,
        config: &AgentConfig,
        messages: &[serde_json::Value],
        tools: &[Tool],
        skill_prompt: Option<&str>,
    ) -> Result<serde_json::Value> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // Build system message with skill context
        let base_system = "You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You have access to tools to help you answer questions. You should introduce yourself as TinyClaw powered by GLM-5 when asked.";
        let system_content = if let Some(skills) = skill_prompt {
            format!("{}\n\n{}", base_system, skills)
        } else {
            base_system.to_string()
        };

        // Build messages with system prompt prepended
        let mut all_messages = vec![serde_json::json!({
            "role": "system",
            "content": system_content
        })];
        all_messages.extend(messages.iter().cloned());

        // Convert tools to OpenAI format
        let openai_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    }
                })
            })
            .collect();

        // Build request with tools
        let request = serde_json::json!({
            "model": model,
            "messages": all_messages,
            "max_tokens": 1024,
            "tools": openai_tools
        });

        // Determine API version path based on model
        // GLM models use v4, others use v1
        let api_version = if model.starts_with("glm") {
            "v4"
        } else {
            "v1"
        };

        let api_base_clone = api_base.clone();
        let api_version_clone = api_version.to_string();
        let api_key_clone = api_key.clone();

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_version_clone = api_version_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/{}/chat/completions", api_base_clone, api_version_clone))
                    .header("Authorization", format!("Bearer {}", api_key_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: serde_json::Value = result.json().await?;

        debug!("OpenAI tool response received");

        Ok(response)
    }

    /// Send message to Anthropic API
    #[allow(dead_code)]
    async fn send_anthropic(&self, config: &AgentConfig, message: &str) -> Result<String> {
        // Check if API key is set
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to Anthropic {}: {}", model, message);

        // Build request
        let request = AnthropicRequest {
            model,
            max_tokens: 1024,
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            system: Some("You are TinyClaw, an AI assistant powered by GLM-5 (Zhipu AI). You should introduce yourself as TinyClaw powered by GLM-5 when asked.".to_string()),
        };

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let api_base_clone = api_base.clone();
        let api_key_clone = api_key.clone();
        let response = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/messages", api_base_clone))
                    .header("x-api-key", api_key_clone)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        // Parse response
        let response: AnthropicResponse = response.json().await?;

        // Extract text from response
        let text = response
            .content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("");

        debug!("Anthropic response: {}", text);

        Ok(text)
    }

    /// Send message to OpenAI API
    #[allow(dead_code)]
    async fn send_openai(&self, config: &AgentConfig, message: &str) -> Result<String> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }

        let api_key = config.api_key.clone().unwrap();
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        info!("Sending message to OpenAI {}: {}", model, message);

        let request = OpenAIRequest {
            model: model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(1024),
        };

        // Determine API version path based on model
        let api_version = if model.starts_with("glm") {
            "v4"
        } else {
            "v1"
        };

        let api_base_clone = api_base.clone();
        let api_version_clone = api_version.to_string();
        let api_key_clone = api_key.clone();

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base_clone = api_base_clone.clone();
            let api_version_clone = api_version_clone.clone();
            let api_key_clone = api_key_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/{}/chat/completions", api_base_clone, api_version_clone))
                    .header("Authorization", format!("Bearer {}", api_key_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OpenAIResponse = result.json().await?;

        let text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        debug!("OpenAI response: {}", text);

        Ok(text)
    }

    /// Send message to Ollama API
    async fn send_ollama(&self, config: &AgentConfig, message: &str) -> Result<String> {
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // For Ollama, default to localhost if no base URL
        let base_url = if api_base.is_empty() || api_base == "https://api.anthropic.com" {
            "http://localhost:11434".to_string()
        } else {
            api_base
        };

        info!("Sending message to Ollama {}: {}", model, message);

        let request = OllamaRequest {
            model,
            prompt: message.to_string(),
            stream: false,
        };

        let base_url_clone = base_url.clone();

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let result = self.execute_protected(|| {
            let http_client = http_client.clone();
            let base_url_clone = base_url_clone.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/api/generate", base_url_clone))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            error!("API error: {} - {}", status, body);
            return Err(Error::Agent(format!("API error: {} - {}", status, body)));
        }

        let response: OllamaResponse = result.json().await?;

        debug!("Ollama response: {}", response.response);

        Ok(response.response)
    }

    /// Send message to Ollama API with streaming support.
    /// Calls the callback for each text chunk as it arrives via SSE.
    async fn send_ollama_streaming<F>(&self, config: &AgentConfig, message: &str, mut on_chunk: F, mut cancel_rx: broadcast::Receiver<()>) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let model = config.model.clone();
        let api_base = config.api_base.clone();

        // For Ollama, default to localhost if no base URL
        let base_url = if api_base.is_empty() || api_base == "https://api.anthropic.com" {
            "http://localhost:11434".to_string()
        } else {
            api_base
        };

        info!("Sending streaming message to Ollama {}: {}", model, message);

        let request = serde_json::json!({
            "model": model,
            "prompt": message,
            "stream": true
        });

        // Use circuit breaker + retry for the initial HTTP request
        let client = reqwest::Client::new();
        let resp = self.execute_protected(|| {
            let client = client.clone();
            let base_url = base_url.clone();
            let request = request.clone();
            async move {
                client
                    .post(format!("{}/api/generate", base_url))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Ollama streaming error: {} - {}", status, body);
            return Err(Error::Agent(format!("Ollama error: {} - {}", status, body)));
        }

        // Read SSE lines and process each chunk
        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = Vec::new();
        let check_interval = Duration::from_millis(100);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        // Spawn a task to watch for cancellation
        let cancelled_flag = cancelled_clone;
        tokio::spawn(async move {
            let _ = cancel_rx.recv().await;
            cancelled_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        
        while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            // Use timeout to periodically check for cancellation
            let result = timeout(check_interval, stream.next()).await;
            
            match result {
                Ok(Some(chunk_result)) => {
                    let chunk = match chunk_result {
                        Ok(c) => c,
                        Err(e) => {
                            debug!("Stream error: {}", e);
                            break;
                        }
                    };
                    buffer.extend_from_slice(&chunk);
                    
                    // Process complete lines (each SSE line starts with "data: ")
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line_str = String::from_utf8_lossy(&line);
                        let line_str = line_str.trim();
                        
                        if let Some(json_str) = line_str.strip_prefix("data: ") {
                            if let Ok(stream_resp) = serde_json::from_str::<OllamaStreamResponse>(json_str) {
                                if !stream_resp.response.is_empty() {
                                    full_response.push_str(&stream_resp.response);
                                    on_chunk(stream_resp.response);
                                }
                                if stream_resp.done {
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended
                    break;
                }
                Err(_) => {
                    // Timeout - check if cancelled and continue
                    continue;
                }
            }
        }

        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            debug!("Ollama streaming cancelled: {} chars accumulated", full_response.len());
            return Err(Error::Cancelled);
        }

        debug!("Ollama streaming complete: {} chars", full_response.len());
        Ok(full_response)
    }

    /// Send a message with streaming partial text support.
    /// Calls the callback for each text chunk as it arrives (for streaming providers like Ollama).
    /// 
    /// # Arguments
    /// * `session_key` - Session identifier
    /// * `message` - User message to send
    /// * `history` - Conversation history
    /// * `skill_prompt` - Optional skill instructions
    /// * `on_partial` - Callback invoked with each text chunk as it arrives
    pub async fn send_message_streaming(
        &self,
        session_key: &str,
        message: &str,
        history: &[(String, String)],
        skill_prompt: Option<&str>,
        on_partial: impl FnMut(String) + Send + 'static,
    ) -> Result<String> {
        // Set session key for tool event tracking
        self.set_session_key(Some(session_key));

        // Start cancellation tracking for this turn
        let cancel_rx = match self.start_turn_cancellation(session_key) {
            Some(rx) => rx,
            None => {
                self.set_session_key(None);
                return Err(Error::Agent(format!(
                    "A turn is already in progress for session '{}'",
                    session_key
                )));
            }
        };

        let config = self.config.read().clone();

        // Determine provider
        let provider = config.provider.clone().unwrap_or_else(|| {
            if config.model.starts_with("claude-") || config.model.starts_with("anthropic/") {
                ModelProvider::Anthropic
            } else if config.model.starts_with("gpt-") || config.model.starts_with("openai/") {
                ModelProvider::OpenAI
            } else {
                ModelProvider::Ollama
            }
        });

        // For non-streaming providers (Anthropic, OpenAI), fall back to regular method
        // Ollama supports native streaming
        let result = if provider == ModelProvider::Ollama {
            // Build prompt from history + current message
            let mut prompt = String::new();
            for (role, content) in history {
                prompt.push_str(&format!("{}: {}\n", role, content));
            }
            prompt.push_str(&format!("user: {}\nassistant:", message));
            
            self.send_ollama_streaming(&config, &prompt, on_partial, cancel_rx).await
        } else {
            // Fall back to non-streaming for other providers
            // Release the cancellation channel since we won't use it for non-streaming
            self.cleanup_turn_cancellation(session_key);
            self.send_message_with_history(session_key, message, history, skill_prompt).await
        };

        // Clear session key after turn completes
        self.set_session_key(None);

        // Clean up cancellation channel after turn ends
        self.cleanup_turn_cancellation(session_key);
        result
    }

    /// Send a message and get a response (with tool calling loop)
    /// For backward compatibility: uses empty history. Use send_message_with_history for context.
    ///
    /// # Arguments
    /// * `session_key` - Session identifier (unused, kept for API compatibility)
    /// * `message` - User message to send
    /// * `skill_prompt` - Optional skill instructions to inject into system prompt
    pub async fn send_message(&self, session_key: &str, message: &str, skill_prompt: Option<&str>) -> Result<String> {
        self.send_message_with_history(session_key, message, &[], skill_prompt).await
    }

    /// Summarize content using the AI without tool calling.
    /// Used internally for context compression.
    ///
    /// This is a lightweight call that doesn't trigger tool execution or
    /// turn tracking - it's just for generating summaries.
    pub async fn summarize_content(&self, content: &str) -> Result<String> {
        let config = self.config.read().clone();

        // Determine provider
        let provider = config.provider.clone().unwrap_or_else(|| {
            if config.model.starts_with("claude-") || config.model.starts_with("anthropic/") {
                ModelProvider::Anthropic
            } else if config.model.starts_with("gpt-") || config.model.starts_with("openai/") {
                ModelProvider::OpenAI
            } else {
                ModelProvider::Ollama
            }
        });

        // Build a simple request without tools
        let system_prompt = "You are a helpful assistant that creates concise, accurate summaries. Focus on preserving key information, decisions, and context that would be important for continuing a conversation.";

        match provider {
            ModelProvider::Anthropic => {
                self.summarize_anthropic(&config, content, system_prompt).await
            }
            ModelProvider::OpenAI => {
                self.summarize_openai(&config, content, system_prompt).await
            }
            ModelProvider::Ollama => {
                self.summarize_ollama(&config, content, system_prompt).await
            }
        }
    }

    /// Summarize using Anthropic API
    async fn summarize_anthropic(&self, config: &AgentConfig, content: &str, system_prompt: &str) -> Result<String> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }
        let api_key = config.api_key.clone().unwrap();
        
        let request = serde_json::json!({
            "model": config.model.trim_start_matches("anthropic/"),
            "max_tokens": 2000,
            "system": system_prompt,
            "messages": [{
                "role": "user",
                "content": content
            }]
        });

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let api_base = config.api_base.clone();
        let response = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base = api_base.clone();
            let api_key = api_key.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/messages", api_base))
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .timeout(std::time::Duration::from_secs(60))
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        let json: serde_json::Value = response.json().await.map_err(|e| Error::Network(e.to_string()))?;
        
        // Extract text from response
        let text = json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    /// Summarize using OpenAI API
    async fn summarize_openai(&self, config: &AgentConfig, content: &str, system_prompt: &str) -> Result<String> {
        if config.api_key.is_none() {
            return Err(Error::Agent("API key not configured".into()));
        }
        let api_key = config.api_key.clone().unwrap();
        
        let request = serde_json::json!({
            "model": config.model.trim_start_matches("openai/"),
            "max_tokens": 2000,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": content}
            ]
        });

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let api_base = config.api_base.clone();
        let response = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base = api_base.clone();
            let api_key = api_key.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/v1/chat/completions", api_base))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("content-type", "application/json")
                    .json(&request)
                    .timeout(std::time::Duration::from_secs(60))
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        let json: serde_json::Value = response.json().await.map_err(|e| Error::Network(e.to_string()))?;

        let text = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    /// Summarize using Ollama API
    async fn summarize_ollama(&self, config: &AgentConfig, content: &str, system_prompt: &str) -> Result<String> {
        let full_prompt = format!("{}\n\n{}", system_prompt, content);

        let request = serde_json::json!({
            "model": config.model.trim_start_matches("ollama/"),
            "prompt": full_prompt,
            "stream": false
        });

        // Use circuit breaker + retry wrapper for the HTTP request
        let http_client = self.http_client.clone();
        let api_base = config.api_base.clone();
        let response = self.execute_protected(|| {
            let http_client = http_client.clone();
            let api_base = api_base.clone();
            let request = request.clone();
            async move {
                http_client
                    .post(format!("{}/api/generate", api_base))
                    .header("content-type", "application/json")
                    .json(&request)
                    .timeout(std::time::Duration::from_secs(120))
                    .send()
                    .await
                    .map_err(|e| Error::Network(e.to_string()))
            }
        }).await?;

        let json: serde_json::Value = response.json().await.map_err(|e| Error::Network(e.to_string()))?;

        let text = json["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

/// Format a ToolResult for inclusion in a tool result message.
/// On success: returns the output string.
/// On failure: returns a structured error report with error kind, retryability, and suggestion.
fn format_tool_result_content(tool_name: &str, result: &ToolResult) -> String {
    use crate::agent::error_recovery::ErrorRecovery;

    if result.success {
        result.output.clone()
    } else {
        // Use structured error reporting for failures
        let error_msg = result.error.as_deref().unwrap_or(&result.output);
        let recovery = ErrorRecovery::from_error(tool_name, error_msg);
        recovery.format_report(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelConfig;

    #[test]
    fn test_model_provider_display_name() {
        assert_eq!(ModelProvider::Anthropic.display_name(), "Anthropic");
        assert_eq!(ModelProvider::OpenAI.display_name(), "OpenAI");
        assert_eq!(ModelProvider::Ollama.display_name(), "Ollama");
    }

    #[test]
    fn test_agent_default_config() {
        let agent_config = AgentConfig::default();
        assert_eq!(agent_config.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(agent_config.api_base, "https://api.anthropic.com");
        assert!(agent_config.provider.is_none());
    }

    #[test]
    fn test_model_config_default() {
        let model_config = ModelConfig::default();
        assert_eq!(model_config.provider, ModelProvider::Anthropic);
        assert_eq!(model_config.max_tokens, 1024);
    }
}
