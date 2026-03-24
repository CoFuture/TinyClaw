//! Agent Runtime Module
//!
//! Core runtime for agent execution with tool calling loop.

use crate::agent::client::Agent;
use crate::agent::context::{AgentContext, ExecutionState};
use crate::agent::context_manager::{ContextManager, ContextOptions};
use crate::agent::context_summarizer::{ContextSummarizer, SummarizerConfig};
use crate::agent::tools::{ToolExecutor, ToolResult};
use crate::agent::tool_result_formatter::ToolResultFormatter;
use crate::agent::turn_log::{TurnLog, TurnLogEntry, TurnAction};
use crate::common::Result;
use crate::gateway::events::{Event, EventEmitter};
use crate::persistence::HistoryManager;
use crate::gateway::session::SessionManager;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

/// Tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ToolCall {
    /// Unique ID for the tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments to the tool (JSON)
    pub arguments: serde_json::Value,
}

/// Response from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelResponse {
    /// Text content from the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Tool calls requested by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Whether the conversation is complete
    pub stop_reason: Option<String>,
}

/// Agent runtime configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RuntimeConfig {
    /// Maximum turns in tool loop
    pub max_turns: usize,
    /// Whether to emit events
    pub emit_events: bool,
    /// Tool call timeout in seconds
    pub tool_timeout_secs: u64,
    /// Context manager options
    pub context_options: ContextOptions,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_turns: 10,
            emit_events: true,
            tool_timeout_secs: 30,
            context_options: ContextOptions::default(),
        }
    }
}

/// Agent runtime for executing agent loops
#[allow(dead_code)]
pub struct AgentRuntime {
    config: RwLock<RuntimeConfig>,
    tool_executor: Arc<ToolExecutor>,
    event_emitter: Option<Arc<EventEmitter>>,
    context_manager: RwLock<ContextManager>,
    summarizer: RwLock<Option<Arc<ContextSummarizer>>>,
}

#[allow(dead_code)]
impl AgentRuntime {
    /// Create a new agent runtime
    pub fn new(tool_executor: Arc<ToolExecutor>) -> Self {
        let config = RuntimeConfig::default();
        let context_manager = ContextManager::new(config.context_options.clone());
        Self {
            config: RwLock::new(config),
            tool_executor,
            event_emitter: None,
            context_manager: RwLock::new(context_manager),
            summarizer: RwLock::new(None),
        }
    }

    /// Set the event emitter
    pub fn with_event_emitter(mut self, emitter: Arc<EventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Update configuration
    pub fn set_config(&self, config: RuntimeConfig) {
        let context_manager = ContextManager::new(config.context_options.clone());
        *self.context_manager.write() = context_manager;
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn get_config(&self) -> RuntimeConfig {
        self.config.read().clone()
    }

    /// Set the context summarizer for AI-powered context compression
    /// When configured, the runtime will use AI summarization instead of simple
    /// truncation when context exceeds token limits
    pub fn set_context_summarizer(&self, agent: Arc<Agent>) {
        let config = SummarizerConfig::default();
        let summarizer = ContextSummarizer::with_agent(agent);
        *self.summarizer.write() = Some(Arc::new(summarizer));
        info!("Context summarizer enabled with config: min_msgs={}, threshold={} tokens",
              config.min_messages, config.token_threshold);
    }

    /// Check if context summarizer is enabled
    pub fn has_summarizer(&self) -> bool {
        self.summarizer.read().is_some()
    }

    /// Execute a complete agent turn with tool calling loop
    pub async fn run_turn(
        &self,
        context: &AgentContext,
        user_message: &str,
    ) -> Result<String> {
        info!(session_id = %context.session_id, "Starting agent turn");
        
        // Reset state
        context.reset_turns();
        context.set_state(ExecutionState::Thinking);
        
        // Add user message to history
        context.add_user_message(user_message);
        
        // Start turn execution log
        let mut turn_log = TurnLog::new();
        let emit_events = self.config.read().emit_events;
        let session_id = context.session_id.clone();

        // Helper to emit TurnLogUpdated events
        let emit_log_update = |entry: TurnLogEntry| {
            if emit_events {
                if let Some(emitter) = &self.event_emitter {
                    emitter.emit(Event::TurnLogUpdated {
                        session_id: session_id.clone(),
                        entry,
                    });
                }
            }
        };

        // Main tool calling loop
        loop {
            context.increment_turn();
            
            if context.max_turns_reached() {
                warn!(session_id = %context.session_id, "Max turns reached, stopping loop");
                context.set_state(ExecutionState::Finished);

                // Emit completed log
                if emit_events {
                    if let Some(emitter) = &self.event_emitter {
                        emitter.emit(Event::TurnLogCompleted {
                            session_id: session_id.clone(),
                            summary: turn_log.summary(),
                        });
                    }
                }

                return Ok("Maximum turns reached. I need to stop here.".to_string());
            }
            
            // Get response from model
            context.set_state(ExecutionState::Thinking);
            let response_start = Instant::now();
            let response = self.get_model_response(context).await?;
            let response_duration_ms = response_start.elapsed().as_millis() as u64;
            
            // Check if we have text to return
            if let Some(text) = &response.text {
                if text.is_empty() {
                    // No text, check for tool calls
                    if let Some(tool_calls) = &response.tool_calls {
                        if tool_calls.is_empty() {
                            // Empty response with no tools, we're done
                            context.set_state(ExecutionState::Finished);

                            if emit_events {
                                if let Some(emitter) = &self.event_emitter {
                                    emitter.emit(Event::TurnLogCompleted {
                                        session_id: session_id.clone(),
                                        summary: turn_log.summary(),
                                    });
                                }
                            }

                            return Ok(text.clone());
                        }
                    }
                } else {
                    // Has text, check if we should stop
                    if response.stop_reason.as_deref() == Some("end_turn") {
                        context.add_assistant_message(text);
                        context.set_state(ExecutionState::Finished);
                        
                        // Record response in turn log
                        turn_log.record_response(text, response_duration_ms);
                        emit_log_update(TurnLogEntry {
                            offset_ms: 0,
                            action: TurnAction::Response {
                                preview: if text.len() > 120 {
                                    format!("{}...", &text[..120])
                                } else {
                                    text.clone()
                                },
                                duration_ms: response_duration_ms,
                            },
                        });

                        // Emit event
                        if emit_events {
                            if let Some(emitter) = &self.event_emitter {
                                emitter.emit(Event::AssistantText {
                                    session_id: session_id.clone(),
                                    text: text.clone(),
                                });
                            }
                        }

                        // Emit turn log completed
                        if emit_events {
                            if let Some(emitter) = &self.event_emitter {
                                emitter.emit(Event::TurnLogCompleted {
                                    session_id: session_id.clone(),
                                    summary: turn_log.summary(),
                                });
                            }
                        }
                        
                        return Ok(text.clone());
                    }
                }
            }
            
            // Process tool calls
            if let Some(tool_calls) = &response.tool_calls {
                for tool_call in tool_calls {
                    context.set_state(ExecutionState::UsingTool { 
                        tool: tool_call.name.clone() 
                    });
                    
                    // Emit tool use event
                    if emit_events {
                        if let Some(emitter) = &self.event_emitter {
                            emitter.emit(Event::AssistantToolUse {
                                session_id: session_id.clone(),
                                tool: tool_call.name.clone(),
                                input: tool_call.arguments.clone(),
                            });
                        }
                    }
                    
                    // Execute tool with timing
                    let tool_start = Instant::now();
                    let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await;
                    let tool_duration_ms = tool_start.elapsed().as_millis() as u64;
                    
                    // Format tool result - use ToolResultFormatter for enhanced readability
                    let (tool_result_str, tool_success) = match &result {
                        Ok(r) => (ToolResultFormatter::format(&tool_call.name, r, tool_duration_ms), r.success),
                        Err(e) => (format!("Tool execution error: {}", e), false),
                    };
                    
                    context.history_manager.add_message(
                        &context.session_id,
                        crate::types::Message::tool(
                            &tool_result_str,
                            &tool_call.id,
                            &tool_call.name,
                        ),
                    );
                    
                    // Record in turn log
                    turn_log.record_tool(
                        &tool_call.name,
                        tool_call.arguments.clone(),
                        &tool_result_str,
                        tool_success,
                        tool_duration_ms,
                    );

                    // Emit turn log update
                    let log_entry = TurnLogEntry {
                        offset_ms: tool_start.elapsed().as_millis() as u64,
                        action: TurnAction::Tool {
                            name: tool_call.name.clone(),
                            input: tool_call.arguments.clone(),
                            output_preview: if tool_result_str.len() > 200 {
                                format!("{}...", &tool_result_str[..200])
                            } else {
                                tool_result_str.clone()
                            },
                            success: tool_success,
                            duration_ms: tool_duration_ms,
                        },
                    };
                    emit_log_update(log_entry);
                    
                    // Emit tool result event
                    if emit_events {
                        if let Some(emitter) = &self.event_emitter {
                            emitter.emit(Event::ToolResult {
                                session_id: session_id.clone(),
                                tool_call_id: tool_call.id.clone(),
                                output: tool_result_str,
                            });
                        }
                    }
                }
            } else {
                // No tool calls and no text, we're done
                if response.text.is_none() || response.text.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    context.set_state(ExecutionState::Finished);

                    if emit_events {
                        if let Some(emitter) = &self.event_emitter {
                            emitter.emit(Event::TurnLogCompleted {
                                session_id: session_id.clone(),
                                summary: turn_log.summary(),
                            });
                        }
                    }

                    return Ok("I don't have anything more to say.".to_string());
                }
            }
        }
    }

    /// Get response from the model
    async fn get_model_response(&self, context: &AgentContext) -> Result<ModelResponse> {
        // Get conversation history (includes the current user message already added by run_turn)
        let history = context.get_history();

        // Step 1: Do all synchronous checks first and collect what we need
        // Check if truncation is needed
        let needs_truncation = {
            let cm = self.context_manager.read();
            cm.needs_truncation(&history)
        }; // Lock dropped here

        // Check if summarization is available and should be used
        let summarizer_opt = if needs_truncation {
            let summarizer_guard = self.summarizer.read();
            if let Some(summarizer) = summarizer_guard.as_ref() {
                let estimated_tokens = ContextManager::estimate_messages_tokens(&history);
                if summarizer.should_summarize(&history, estimated_tokens) {
                    Some(Arc::clone(summarizer))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }; // Lock dropped here

        // Step 2: Apply context management (truncation or summarization)
        let (truncated_history, current_message) = if !needs_truncation {
            // No truncation needed, use history as-is
            let prev_history: Vec<_> = history.iter()
                .take(history.len().saturating_sub(1))
                .map(|m| {
                    let role = match m.role {
                        crate::types::Role::User => "user",
                        crate::types::Role::Assistant => "assistant",
                        crate::types::Role::System => "system",
                        crate::types::Role::Tool => "tool",
                    };
                    (role.to_string(), m.content.clone())
                })
                .collect();
            let current = history.last().map(|m| m.content.clone()).unwrap_or_default();
            (prev_history, current)
        } else if let Some(summarizer) = summarizer_opt {
            // Try AI summarization (no locks held at this point)
            let keep_recent = 5;
            match summarizer.summarize_messages(&history, keep_recent).await {
                Ok(Some(summary)) => {
                    info!(
                        "Context summarized: {} messages -> summary ({} tokens vs {} original, {:.1}% compression)",
                        summary.messages_summarized,
                        summary.summary_tokens,
                        summary.original_tokens,
                        summary.compression_ratio() * 100.0
                    );
                    // Build history with summary message
                    let summary_msg = ContextSummarizer::create_summary_message(&summary);
                    let recent_msgs: Vec<_> = history.iter()
                        .skip(history.len().saturating_sub(keep_recent))
                        .cloned()
                        .collect();
                    
                    // Format for response
                    let mut all_msgs = vec![summary_msg];
                    all_msgs.extend(recent_msgs);
                    
                    let current = all_msgs.last().map(|m| m.content.clone()).unwrap_or_default();
                    let prev_history: Vec<_> = all_msgs.iter()
                        .take(all_msgs.len().saturating_sub(1))
                        .map(|m| {
                            let role = match m.role {
                                crate::types::Role::User => "user",
                                crate::types::Role::Assistant => "assistant",
                                crate::types::Role::System => "system",
                                crate::types::Role::Tool => "tool",
                            };
                            (role.to_string(), m.content.clone())
                        })
                        .collect();
                    (prev_history, current)
                }
                Ok(None) => {
                    tracing::debug!("Summarization returned None, using fallback truncation");
                    self.apply_fallback_truncation(&history)
                }
                Err(e) => {
                    warn!("Context summarization failed: {}, falling back to truncation", e);
                    self.apply_fallback_truncation(&history)
                }
            }
        } else {
            // No summarization available, use normal truncation
            self.apply_fallback_truncation(&history)
        };

        // Send to agent with conversation history
        let response_text = context.agent.send_message_with_history(
            &context.session_id,
            &current_message,
            &truncated_history,
            None
        ).await?;

        Ok(ModelResponse {
            text: Some(response_text),
            tool_calls: None,
            stop_reason: Some("end_turn".to_string()),
        })
    }

    /// Apply fallback truncation when summarization is not available or fails
    fn apply_fallback_truncation(&self, history: &[crate::types::Message]) -> (Vec<(String, String)>, String) {
        let cm = self.context_manager.read();
        let truncated = cm.truncate_to_fit(history);
        tracing::debug!(
            "Context truncated: {} messages -> {} messages",
            history.len(),
            truncated.len()
        );
        let current = truncated.last().map(|m| m.content.clone()).unwrap_or_default();
        let prev_history: Vec<_> = truncated.iter()
            .take(truncated.len().saturating_sub(1))
            .map(|m| {
                let role = match m.role {
                    crate::types::Role::User => "user",
                    crate::types::Role::Assistant => "assistant",
                    crate::types::Role::System => "system",
                    crate::types::Role::Tool => "tool",
                };
                (role.to_string(), m.content.clone())
            })
            .collect();
        (prev_history, current)
    }

    /// Execute a tool
    async fn execute_tool(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<ToolResult> {
        info!(tool_name = %tool_name, args = ?arguments, "Executing tool");
        
        // Add any required fields to arguments
        let args = arguments.clone();
        
        let result = self.tool_executor.execute(tool_name, args).await;
        
        if !result.success {
            warn!(tool_name = %tool_name, error = ?result.error, "Tool execution failed");
        }
        
        Ok(result)
    }

    /// Create an agent context for a session
    pub fn create_context(
        &self,
        session_id: String,
        history_manager: Arc<HistoryManager>,
        session_manager: Arc<SessionManager>,
        agent: Arc<Agent>,
    ) -> AgentContext {
        AgentContext::new(
            session_id,
            history_manager,
            session_manager,
            Arc::new(ToolExecutor::new()),
            agent,
        )
    }
}

impl Default for AgentRuntime {
    fn default() -> Self {
        Self::new(Arc::new(ToolExecutor::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_turns, 10);
        assert_eq!(config.tool_timeout_secs, 30);
        assert!(config.emit_events);
    }

    #[test]
    fn test_runtime_with_context_manager() {
        let runtime = AgentRuntime::new(Arc::new(ToolExecutor::new()));
        let config = runtime.get_config();
        assert_eq!(config.max_turns, 10);
    }
}
