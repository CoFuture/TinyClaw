//! Authentication and authorization

use crate::common::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// API Key authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub name: String,
    pub permissions: Vec<Permission>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

/// Permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Permission {
    Read,
    Write,
    Admin,
    Execute,
}

#[allow(dead_code)]
impl ApiKey {
    pub fn new(id: impl Into<String>, key: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            name: name.into(),
            permissions: vec![Permission::Read, Permission::Write],
            created_at: chrono::Utc::now(),
            expires_at: None,
            enabled: true,
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_expiry(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.enabled && self.expires_at.is_none_or(|exp| exp > chrono::Utc::now())
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }
}

/// Authenticator
#[allow(dead_code)]
pub struct Authenticator {
    keys: std::collections::HashMap<String, ApiKey>,
}

#[allow(dead_code)]
impl Authenticator {
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    /// Add an API key
    pub fn add_key(&mut self, api_key: ApiKey) {
        self.keys.insert(api_key.key.clone(), api_key);
    }

    /// Remove an API key
    pub fn remove_key(&mut self, key: &str) -> Option<ApiKey> {
        self.keys.remove(key)
    }

    /// Validate a key
    pub fn validate(&self, key: &str) -> Option<&ApiKey> {
        self.keys.get(key).filter(|k| k.is_valid())
    }

    /// List all keys (without secret)
    pub fn list_keys(&self) -> Vec<serde_json::Value> {
        self.keys.values()
            .map(|k| {
                serde_json::json!({
                    "id": k.id,
                    "name": k.name,
                    "permissions": k.permissions,
                    "createdAt": k.created_at.to_rfc3339(),
                    "expiresAt": k.expires_at.map(|e| e.to_rfc3339()),
                    "enabled": k.enabled,
                })
            })
            .collect()
    }
}

impl Default for Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

/// Authorization helper
#[allow(dead_code)]
pub fn require_admin(api_key: &ApiKey) -> Result<()> {
    if api_key.has_permission(&Permission::Admin) {
        Ok(())
    } else {
        Err(Error::Auth("Admin permission required".into()))
    }
}

#[allow(dead_code)]
pub fn require_permission(api_key: &ApiKey, permission: Permission) -> Result<()> {
    if api_key.has_permission(&permission) {
        Ok(())
    } else {
        Err(Error::Auth(format!("Permission {:?} required", permission)))
    }
}
