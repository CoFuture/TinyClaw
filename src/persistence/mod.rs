//! Persistence module - SQLite-based session history storage

pub mod history;
pub mod sqlite;

pub use history::HistoryManager;
// SqliteStore is part of public API for advanced use cases
#[allow(unused)]
pub use sqlite::SqliteStore;
