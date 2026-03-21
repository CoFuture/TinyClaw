//! Session history module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "tool")]
    Tool,
}

/// A message in the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    #[allow(dead_code)]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::System,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    #[allow(dead_code)]
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: Some(tool_call_id.into()),
            tool_name: Some(tool_name.into()),
        }
    }
}

/// Session history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionHistory {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionHistory {
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.updated_at = Utc::now();
        self.messages.push(message);
    }

    #[allow(dead_code)]
    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// History manager
pub struct HistoryManager {
    histories: RwLock<HashMap<String, Arc<RwLock<SessionHistory>>>>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            histories: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create history for a session
    pub fn get_or_create(&self, session_id: &str) -> Arc<RwLock<SessionHistory>> {
        let mut histories = self.histories.write();
        if let Some(history) = histories.get(session_id) {
            return history.clone();
        }
        
        let history = Arc::new(RwLock::new(SessionHistory::new(session_id)));
        histories.insert(session_id.to_string(), history.clone());
        history
    }

    /// Add a message to a session's history
    pub fn add_message(&self, session_id: &str, message: Message) {
        let history = self.get_or_create(session_id);
        history.write().add_message(message);
    }

    /// Get history for a session
    pub fn get(&self, session_id: &str) -> Option<Arc<RwLock<SessionHistory>>> {
        self.histories.read().get(session_id).cloned()
    }

    /// List all histories
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<Arc<RwLock<SessionHistory>>> {
        self.histories.read().values().cloned().collect()
    }

    /// Clear history for a session
    #[allow(dead_code)]
    pub fn clear(&self, session_id: &str) {
        if let Some(history) = self.histories.read().get(session_id) {
            history.write().clear();
        }
    }

    /// Remove history for a session
    #[allow(dead_code)]
    pub fn remove(&self, session_id: &str) {
        self.histories.write().remove(session_id);
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_message_tool() {
        let msg = Message::tool("result", "call_123", "exec");
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.content, "result");
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(msg.tool_name, Some("exec".to_string()));
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("test");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{"id": "123", "role": "user", "content": "test", "timestamp": "2024-01-01T00:00:00Z"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "test");
    }

    #[test]
    fn test_session_history_new() {
        let history = SessionHistory::new("session1");
        assert_eq!(history.session_id, "session1");
        assert!(history.messages.is_empty());
    }

    #[test]
    fn test_session_history_add_message() {
        let mut history = SessionHistory::new("session1");
        history.add_message(Message::user("Hello"));
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_session_history_clear() {
        let mut history = SessionHistory::new("session1");
        history.add_message(Message::user("Hello"));
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_history_manager_new() {
        let manager = HistoryManager::new();
        let list = manager.list();
        assert!(list.is_empty());
    }

    #[test]
    fn test_history_manager_get_or_create() {
        let manager = HistoryManager::new();
        let history1 = manager.get_or_create("session1");
        let history2 = manager.get_or_create("session1");
        
        // Should be the same session_id
        let id1 = history1.read().session_id.clone();
        let id2 = history2.read().session_id.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_history_manager_remove() {
        let manager = HistoryManager::new();
        manager.get_or_create("session1");
        manager.remove("session1");
        
        // After remove, get should return None
        let history = manager.get("session1");
        assert!(history.is_none());
    }
}
