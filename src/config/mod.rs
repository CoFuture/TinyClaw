//! Configuration module

pub mod schema;

pub use schema::{
    load_config, default_config_path, Config, AgentConfig, ModelProvider,
};
