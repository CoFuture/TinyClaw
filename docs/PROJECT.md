# TinyClaw 项目说明文档

## 项目概述

TinyClaw 是 OpenClaw (https://github.com/openclaw/openclaw) 的 Rust 实现精简版，旨在提供一个轻量级的 AI Agent Gateway 框架，保留核心功能以便学习和扩展。

## 架构设计

### 核心模块

1. **Gateway (网关层)**
   - WebSocket 服务器处理客户端连接
   - JSON-RPC 2.0 协议解析与消息路由
   - 会话管理

2. **Agent (代理层)**
   - AI 模型客户端 (支持 Anthropic API)
   - 工具执行框架
   - 对话上下文管理

3. **HTTP Server**
   - RESTful API
   - 健康检查
   - 配置管理

4. **工具系统**
   - Exec: Shell 命令执行
   - File: 文件读写操作
   - HTTP: 网络请求

### 数据流

```
Client <--WebSocket--> Gateway <---> Agent <---> AI Model
                  |
                  v
              HTTP Server <---> Config
```

## 技术栈

- **运行时**: Tokio (异步 Runtime)
- **WebSocket**: tokio-tungstenite
- **HTTP**: Axum
- **序列化**: Serde JSON
- **日志**: Tracing
- **并发**: Arc + RwLock (parking_lot)

## 配置说明

### Gateway 配置

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| bind | String | "127.0.0.1:18789" | WebSocket 绑定地址 |
| verbose | bool | false | 详细日志 |
| dataDir | Option<String> | None | 数据目录 |

### Agent 配置

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| model | String | "anthropic/claude-sonnet-4-20250514" | AI 模型 |
| apiKey | Option<String> | None | API 密钥 |
| apiBase | String | "https://api.anthropic.com" | API 地址 |
| workspace | Option<String> | None | 工作目录 |

## API 协议

### JSON-RPC 2.0

请求格式:
```json
{
  "id": "uuid",
  "method": "method.name",
  "params": {}
}
```

响应格式:
```json
{
  "id": "uuid",
  "result": {}
}
```

错误响应:
```json
{
  "id": "uuid",
  "error": {
    "code": "ERROR_CODE",
    "message": "Error message"
  }
}
```

## 事件系统

支持的事件类型:

- `assistant.text` - AI 文本响应
- `assistant.tool_use` - AI 工具调用
- `tool_result` - 工具执行结果
- `session.created` - 会话创建
- `session.ended` - 会话结束
- `error` - 错误事件

## 开发指南

### 添加新工具

1. 在 `src/agent/tools.rs` 中添加工具定义
2. 在 `ToolExecutor::execute` 中添加处理逻辑

### 添加新 API 方法

1. 在 `src/gateway/protocol.rs` 中定义方法常量
2. 在 `src/gateway/messages.rs` 中实现处理函数

## 性能考虑

- 使用 `parking_lot` 的 `RwLock` 提供更快的并发访问
- 使用 `Arc` 实现共享所有权
- 异步 I/O 使用 `Tokio`

## 后续计划

- [ ] 消息持久化 (数据库支持)
- [ ] 更丰富的工具集
- [ ] Web 管理界面
- [ ] 多模型支持
- [ ] 插件系统

## 参考

- OpenClaw: https://github.com/openclaw/openclaw
- Tokio: https://tokio.rs
- Axum: https://docs.rs/axum
