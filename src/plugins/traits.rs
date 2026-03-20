//! 插件 trait 定义

use crate::common::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件唯一标识
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件描述
    pub description: String,
    /// 插件作者
    pub author: String,
}

/// 插件钩子类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Hook {
    /// 消息预处理钩子
    PreMessage { session_id: String, message: String },
    /// 消息后处理钩子
    PostMessage { session_id: String, message: String },
    /// 会话创建钩子
    SessionCreated { session_id: String },
    /// 会话关闭钩子
    SessionClosed { session_id: String },
    /// 工具执行前钩子
    PreToolExecute { tool_name: String },
    /// 工具执行后钩子
    PostToolExecute { tool_name: String, success: bool },
}

/// 插件 trait
///
/// 所有插件必须实现这个 trait
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 获取插件元数据
    fn metadata(&self) -> PluginMetadata;

    /// 初始化插件
    ///
    /// 在插件加载时调用，用于初始化插件状态
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    /// 关闭插件
    ///
    /// 在插件卸载时调用，用于清理资源
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// 处理钩子
    ///
    /// 当对应事件发生时调用
    async fn on_hook(&self, _hook: Hook) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }
}

/// 内置插件 IDs
pub mod builtin {
    pub const ECHO_PLUGIN: &str = "builtin.echo";
    pub const LOGGER_PLUGIN: &str = "builtin.logger";
    pub const VALIDATOR_PLUGIN: &str = "builtin.validator";
}
