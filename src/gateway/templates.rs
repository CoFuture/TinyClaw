//! Message templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// Message template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MessageTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template content (supports {{variable}} placeholders)
    pub content: String,
    /// Description
    pub description: String,
    /// Tags for categorization
    pub tags: Vec<String>,
}

#[allow(dead_code)]
impl MessageTemplate {
    pub fn new(id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            content: content.into(),
            description: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Render the template with variables
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.content.clone();
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Template manager
#[allow(dead_code)]
pub struct TemplateManager {
    templates: RwLock<HashMap<String, MessageTemplate>>,
}

#[allow(dead_code)]
impl TemplateManager {
    pub fn new() -> Self {
        let manager = Self {
            templates: RwLock::new(HashMap::new()),
        };
        manager.init_builtin_templates();
        manager
    }

    fn init_builtin_templates(&self) {
        let builtin = vec![
            MessageTemplate::new("greeting", "Greeting", "Hello! {{name}}. How can I help you today?")
                .with_description("Default greeting message")
                .with_tags(vec!["greeting".to_string()]),
            MessageTemplate::new("farewell", "Farewell", "Goodbye, {{name}}! Have a great day!")
                .with_description("Default farewell message")
                .with_tags(vec!["greeting".to_string()]),
            MessageTemplate::new("error_generic", "Generic Error", "Something went wrong. Please try again later.")
                .with_description("Generic error message")
                .with_tags(vec!["error".to_string()]),
            MessageTemplate::new("error_permission", "Permission Error", "You don't have permission to perform this action.")
                .with_description("Permission denied message")
                .with_tags(vec!["error".to_string(), "auth".to_string()]),
            MessageTemplate::new("success", "Success", "Operation completed successfully!")
                .with_description("Success message")
                .with_tags(vec!["status".to_string()]),
            MessageTemplate::new("processing", "Processing", "I'm processing your request. Please wait...")
                .with_description("Processing message")
                .with_tags(vec!["status".to_string()]),
        ];

        let mut templates = self.templates.write();
        for template in builtin {
            templates.insert(template.id.clone(), template);
        }
    }

    /// Add a template
    pub fn add(&self, template: MessageTemplate) {
        self.templates.write().insert(template.id.clone(), template);
    }

    /// Remove a template
    pub fn remove(&self, id: &str) -> Option<MessageTemplate> {
        self.templates.write().remove(id)
    }

    /// Get a template by ID
    pub fn get(&self, id: &str) -> Option<MessageTemplate> {
        self.templates.read().get(id).cloned()
    }

    /// List all templates
    pub fn list(&self) -> Vec<MessageTemplate> {
        self.templates.read().values().cloned().collect()
    }

    /// Find templates by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<MessageTemplate> {
        self.templates.read()
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Render a template
    pub fn render(&self, id: &str, variables: &HashMap<String, String>) -> Option<String> {
        self.templates.read()
            .get(id)
            .map(|t| t.render(variables))
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_render() {
        let template = MessageTemplate::new("test", "Test", "Hello, {{name}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        
        assert_eq!(template.render(&vars), "Hello, World!");
    }

    #[test]
    fn test_template_manager() {
        let manager = TemplateManager::new();
        
        assert!(manager.get("greeting").is_some());
        assert!(manager.get("nonexistent").is_none());
        
        let _greeting = manager.get("greeting").unwrap();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        assert_eq!(manager.render("greeting", &vars), Some("Hello! Alice. How can I help you today?".to_string()));
    }
}
