//! TinyClaw 插件系统
//!
//! 插件系统允许扩展 TinyClaw 的功能，包括：
//! - 自定义消息处理器
//! - 自定义认证提供者

pub mod loader;
pub mod manager;
pub mod traits;

pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use traits::{Hook, Plugin, PluginMetadata};
