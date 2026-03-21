//! Persistence module - SQLite-based session history storage

pub mod history;
pub mod sqlite;

pub use history::HistoryManager;
