//! Streaming response support for real-time AI responses
//! 
//! This module provides Server-Sent Events (SSE) streaming support

use futures_util::stream::Stream;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamingEvent {
    /// Start of streaming response
    #[serde(rename = "start")]
    Start {
        /// Session ID
        session_id: String,
        /// Request ID
        request_id: String,
    },
    /// Chunk of content
    #[serde(rename = "chunk")]
    Chunk {
        /// Content chunk
        content: String,
        /// Whether this is the final chunk
        #[serde(default)]
        done: bool,
    },
    /// Tool call started
    #[serde(rename = "tool_start")]
    ToolStart {
        /// Tool name
        tool: String,
        /// Tool call ID
        call_id: String,
    },
    /// Tool call result
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Tool call ID
        call_id: String,
        /// Result content
        result: String,
    },
    /// End of streaming
    #[serde(rename = "end")]
    End {
        /// Total tokens used
        #[serde(default)]
        tokens: Option<u32>,
    },
    /// Error occurred
    #[serde(rename = "error")]
    Error {
        /// Error message
        message: String,
    },
}

impl StreamingEvent {
    /// Convert event to SSE format
    pub fn to_sse(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Streaming response handle
#[allow(dead_code)]
pub struct StreamingResponse {
    /// Session ID
    pub session_id: String,
    /// Request ID
    pub request_id: String,
    /// Sender for pushing chunks
    sender: Option<mpsc::Sender<StreamingEvent>>,
}

impl StreamingResponse {
    /// Create a new streaming response
    #[allow(dead_code)]
    pub fn new(session_id: String, request_id: String) -> Self {
        Self {
            session_id,
            request_id,
            sender: None,
        }
    }

    /// Set the sender channel
    #[allow(dead_code)]
    pub fn set_sender(&mut self, sender: mpsc::Sender<StreamingEvent>) {
        self.sender = Some(sender);
    }

    /// Send a chunk of content
    #[allow(dead_code)]
    pub async fn send_chunk(&self, content: &str, done: bool) -> Result<(), String> {
        if let Some(ref sender) = self.sender {
            sender
                .send(StreamingEvent::Chunk {
                    content: content.to_string(),
                    done,
                })
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    /// Send tool start event
    #[allow(dead_code)]
    pub async fn send_tool_start(&self, tool: &str, call_id: &str) -> Result<(), String> {
        if let Some(ref sender) = self.sender {
            sender
                .send(StreamingEvent::ToolStart {
                    tool: tool.to_string(),
                    call_id: call_id.to_string(),
                })
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    /// Send tool result event
    #[allow(dead_code)]
    pub async fn send_tool_result(&self, call_id: &str, result: &str) -> Result<(), String> {
        if let Some(ref sender) = self.sender {
            sender
                .send(StreamingEvent::ToolResult {
                    call_id: call_id.to_string(),
                    result: result.to_string(),
                })
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    /// Send end event
    #[allow(dead_code)]
    pub async fn send_end(&self, tokens: Option<u32>) -> Result<(), String> {
        if let Some(ref sender) = self.sender {
            sender
                .send(StreamingEvent::End { tokens })
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    /// Send error event
    #[allow(dead_code)]
    pub async fn send_error(&self, message: &str) -> Result<(), String> {
        if let Some(ref sender) = self.sender {
            sender
                .send(StreamingEvent::Error {
                    message: message.to_string(),
                })
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

/// Streaming manager for handling multiple concurrent streams
#[derive(Default)]
#[allow(dead_code)]
pub struct StreamingManager {
    /// Active streams
    streams: RwLock<HashMap<String, Arc<RwLock<StreamingResponse>>>>,
}

impl StreamingManager {
    /// Create a new streaming manager
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new streaming response
    #[allow(dead_code)]
    pub fn register(&self, request_id: String, response: StreamingResponse) {
        debug!("Registering stream for request: {}", request_id);
        self.streams.write().insert(request_id, Arc::new(RwLock::new(response)));
    }

    /// Get a streaming response by request ID
    #[allow(dead_code)]
    pub fn get(&self, request_id: &str) -> Option<Arc<RwLock<StreamingResponse>>> {
        self.streams.read().get(request_id).cloned()
    }

    /// Remove a streaming response
    #[allow(dead_code)]
    pub fn unregister(&self, request_id: &str) {
        debug!("Unregistering stream for request: {}", request_id);
        self.streams.write().remove(request_id);
    }

    /// Get count of active streams
    #[allow(dead_code)]
    pub fn active_count(&self) -> usize {
        self.streams.read().len()
    }
}

/// Create a streaming event source
#[allow(dead_code)]
pub fn create_event_stream(
    mut receiver: mpsc::Receiver<StreamingEvent>,
) -> Pin<Box<dyn Stream<Item = String> + Send>> {
    Box::pin(async_stream::stream! {
        while let Some(event) = receiver.recv().await {
            let sse = format!("data: {}\n\n", event.to_sse());
            yield sse;
        }
        yield "data: [DONE]\n\n".to_string();
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_event_chunk() {
        let event = StreamingEvent::Chunk {
            content: "Hello".to_string(),
            done: false,
        };
        let sse = event.to_sse();
        assert!(sse.contains("\"type\":\"chunk\""));
        assert!(sse.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_streaming_event_end() {
        let event = StreamingEvent::End { tokens: Some(100) };
        let sse = event.to_sse();
        assert!(sse.contains("\"type\":\"end\""));
        assert!(sse.contains("\"tokens\":100"));
    }

    #[test]
    fn test_streaming_manager() {
        let manager = StreamingManager::new();
        assert_eq!(manager.active_count(), 0);

        let response = StreamingResponse::new("session1".to_string(), "req1".to_string());
        manager.register("req1".to_string(), response);
        assert_eq!(manager.active_count(), 1);

        let retrieved = manager.get("req1");
        assert!(retrieved.is_some());

        manager.unregister("req1");
        assert_eq!(manager.active_count(), 0);
    }
}
