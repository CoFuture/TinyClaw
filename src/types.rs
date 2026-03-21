//! Shared types used across multiple modules
//! This module exists to avoid circular dependencies between gateway and persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
