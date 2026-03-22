//! TinyClaw - A minimal implementation of OpenClaw in Rust

pub mod agent;
pub mod chat;  // CLI chat client module
pub mod common;
pub mod config;
pub mod gateway;
pub mod http;
pub mod metrics;
pub mod persistence;
pub mod preferences;
pub mod ratelimit;
pub mod tui;
pub mod types;

pub use common::error::Error;
pub use persistence::HistoryManager;
pub use preferences::{PreferencesManager, UserPreferences, UserPreferencesUpdate};
pub use tui::{TuiGatewayClient, TuiGatewayEvent, SessionInfo};
pub use types::{Message, Role, SessionHistory};
