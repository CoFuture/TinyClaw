//! Agent Runtime Module
//!
//! Core runtime for agent execution with tool calling loop.

use crate::agent::client::Agent;
use crate::agent::context::{AgentContext, ExecutionState};
use crate::agent::context_manager::{ContextManager, ContextOptions};
use crate::agent::tools::{ToolExecutor, ToolResult};
use crate::common::Result;
use crate::gateway::events::{Event, EventEmitter};
use crate::persistence::HistoryManager;
use crate::gateway::session::SessionManager;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

    /// Execute a complete agent turn with tool calling loop
    pub async fn run_turn(
        &self,
        context: &AgentContext,
        user_message: &str,
    ) -> Result<String> {
        info!("Starting agent turn for session: {}", context.session_id);
        
        // Reset state
        context.reset_turns();
        context.set_state(ExecutionState::Thinking);
        
        // Add user message to history
        context.add_user_message(user_message);
        
        // Main tool calling loop
        loop {
            context.increment_turn();
            
            if context.max_turns_reached() {
                warn!("Max turns reached, stopping loop");
                context.set_state(ExecutionState::Finished);
                return Ok("Maximum turns reached. I need to stop here.".to_string());
            }
            
            // Get response from model
            context.set_state(ExecutionState::Thinking);
            let response = self.get_model_response(context).await?;
            
            // Check if we have text to return
            if let Some(text) = &response.text {
                if text.is_empty() {
                    // No text, check for tool calls
                    if let Some(tool_calls) = &response.tool_calls {
                        if tool_calls.is_empty() {
                            // Empty response with no tools, we're done
                            context.set_state(ExecutionState::Finished);
                            return Ok(text.clone());
                        }
                    }
                } else {
                    // Has text, check if we should stop
                    if response.stop_reason.as_deref() == Some("end_turn") {
                        context.add_assistant_message(text);
                        context.set_state(ExecutionState::Finished);
                        
                        // Emit event
                        if self.config.read().emit_events {
                            if let Some(emitter) = &self.event_emitter {
                                emitter.emit(Event::AssistantText {
                                    session_id: context.session_id.clone(),
                                    text: text.clone(),
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
                    if self.config.read().emit_events {
                        if let Some(emitter) = &self.event_emitter {
                            emitter.emit(Event::AssistantToolUse {
                                session_id: context.session_id.clone(),
                                tool: tool_call.name.clone(),
                                input: tool_call.arguments.clone(),
                            });
                        }
                    }
                    
                    // Execute tool
                    let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await;
                    
                    // Add tool result to history
                    let tool_result_str = match &result {
                        Ok(r) => r.output.clone(),
                        Err(e) => e.to_string(),
                    };
                    
                    context.history_manager.add_message(
                        &context.session_id,
                        crate::types::Message::tool(
                            &tool_result_str,
                            &tool_call.id,
                            &tool_call.name,
                        ),
                    );
                    
                    // Emit tool result event
                    if self.config.read().emit_events {
                        if let Some(emitter) = &self.event_emitter {
                            emitter.emit(Event::ToolResult {
                                session_id: context.session_id.clone(),
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
                    return Ok("I don't have anything more to say.".to_string());
                }
            }
        }
    }

    /// Get response from the model
    async fn get_model_response(&self, context: &AgentContext) -> Result<ModelResponse> {
        // Get conversation history
        let history = context.get_history();
        
        // Apply context management (truncation if needed)
        // Note: we drop the lock guard before any await point
        let truncated_history = {
            let cm = self.context_manager.read();
            if cm.needs_truncation(&history) {
                let truncated = cm.truncate_to_fit(&history);
                tracing::debug!(
                    "Context truncated: {} messages -> {} messages",
                    history.len(),
                    truncated.len()
                );
                truncated
            } else {
                history.clone()
            }
        }; // Lock guard dropped here
        
        // Get last user message for the agent
        let last_message = truncated_history.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        
        // Send to agent (agent handles API formatting internally)
        let response_text = context.agent.send_message(&context.session_id, &last_message, None).await?;
        
        Ok(ModelResponse {
            text: Some(response_text),
            tool_calls: None,
            stop_reason: Some("end_turn".to_string()),
        })
    }

    /// Execute a tool
    async fn execute_tool(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<ToolResult> {
        info!("Executing tool: {} with args: {:?}", tool_name, arguments);
        
        // Add any required fields to arguments
        let args = arguments.clone();
        
        let result = self.tool_executor.execute(tool_name, args).await;
        
        if !result.success {
            warn!("Tool {} failed: {:?}", tool_name, result.error);
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
