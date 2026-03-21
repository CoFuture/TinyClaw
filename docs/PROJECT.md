# TinyClaw 项目说明文档

## 项目概述

TinyClaw 是一个用 Rust 实现的轻量级 AI Agent Gateway，完整支持 AI 对话、工具调用、工具执行全链路闭环。

**设计目标**: 提供一个生产可用的 AI Agent Gateway，具备 OpenClaw 的核心功能。

## 架构设计

### 核心模块

```
┌─────────────────────────────────────────────────────────┐
│                      Client (Web/CLI)                    │
└─────────────────────┬───────────────────────────────────┘
                      │ WebSocket / HTTP
                      ▼
┌─────────────────────────────────────────────────────────┐
│                   Gateway (WebSocket)                    │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │  Protocol   │  │   Session    │  │    Messages     │  │
│  │  (JSON-RPC) │  │   Manager    │  │    Handler      │  │
│  └─────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│                    Agent Runtime                         │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Client    │  │   Context    │  │     Tools       │  │
│  │ (Anthropic/ │  │   Manager    │  │   Executor      │  │
│  │   OpenAI)   │  │              │  │                 │  │
│  └─────────────┘  └──────────────┘  └─────────────────┘  │
│                          │                               │
│                          ▼                               │
│                  ┌─────────────────┐                     │
│                  │  Tool Calling   │                     │
│                  │     Loop        │                     │
│                  └─────────────────┘                     │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│                    Built-in Tools                         │
│  ┌────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │  exec  │  │read_file │  │write_file│  │http_request│  │
│  └────────┘  └──────────┘  └──────────┘  └───────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 模块职责

| 模块 | 职责 | 文件 |
|------|------|------|
| **Gateway** | WebSocket 服务器，JSON-RPC 协议解析 | gateway/server.rs, protocol.rs, messages.rs |
| **Session** | 会话管理，消息历史 | gateway/session.rs, history.rs |
| **Agent** | AI 模型客户端，工具调用循环 | agent/client.rs, runtime.rs |
| **Tools** | 工具执行器 (exec, read_file, etc.) | agent/tools.rs |
| **HTTP** | REST API，Web 管理界面 | http/routes.rs |
| **Plugins** | 插件系统 | plugins/*.rs |
| **Metrics** | 指标收集 | metrics/*.rs |
| **RateLimit** | 速率限制 | ratelimit/*.rs |

### 数据流

```
用户消息
    │
    ▼
┌──────────────────────────────────────┐
│  Gateway (WebSocket)                  │
│  1. 解析 JSON-RPC 请求                │
│  2. 路由到 agent.turn                │
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│  Agent Runtime                        │
│  1. 发送消息给 AI 模型               │
│  2. 检查模型是否请求工具调用          │
│  3. 如有工具调用，执行工具            │
│  4. 将工具结果返回给模型              │
│  5. 循环直到返回最终文本             │
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│  工具执行 (Tool Executor)             │
│  - exec: 执行 Shell 命令              │
│  - read_file: 读取文件               │
│  - write_file: 写入文件               │
│  - list_dir: 列出目录                │
│  - http_request: HTTP 请求           │
└──────────────────┬───────────────────┘
                   │
                   ▼
              返回结果给用户
```

## 技术栈

| 组件 | 技术 | 用途 |
|------|------|------|
| 运行时 | Tokio | 异步 Runtime |
| WebSocket | tokio-tungstenite | WebSocket 服务器 |
| HTTP | Axum | HTTP 服务器 |
| 序列化 | Serde JSON | JSON 解析 |
| 日志 | Tracing | 日志记录 |
| 并发 | Arc + RwLock | 共享状态 |
| HTTP Client | Reqwest | HTTP 请求 |
| 终端 UI | Ratatui | TUI 界面 |

## 配置说明

### Gateway 配置

```json
{
  "gateway": {
    "bind": "127.0.0.1:18789",
    "verbose": false
  }
}
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| bind | String | "127.0.0.1:18790" | WebSocket 绑定地址 |
| verbose | bool | false | 详细日志 |

### Agent 配置

```json
{
  "agent": {
    "model": "claude-sonnet-4-20250514",
    "apiKey": "your-api-key",
    "apiBase": "https://api.anthropic.com",
    "provider": "anthropic"
  }
}
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| model | String | "claude-sonnet-4-20250514" | AI 模型 |
| apiKey | Option<String> | None | API 密钥 |
| apiBase | String | "https://api.anthropic.com" | API 地址 |
| provider | Option<String> | None | 提供商 (anthropic/openai/ollama) |

### 工具配置

```json
{
  "tools": {
    "execEnabled": true
  }
}
```

### 速率限制配置

```json
{
  "ratelimit": {
    "maxRequests": 60,
    "windowSeconds": 60
  }
}
```

## JSON-RPC 协议

### 请求格式

```json
{
  "id": "unique-id",
  "method": "agent.turn",
  "params": {
    "message": "用户消息",
    "sessionKey": "可选会话ID"
  }
}
```

### 响应格式

```json
{
  "id": "unique-id",
  "result": {
    "text": "AI 响应文本"
  }
}
```

### 错误格式

```json
{
  "id": "unique-id",
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

## 事件系统

客户端可以订阅以下事件:

| 事件 | 描述 | 数据 |
|------|------|------|
| `assistant.text` | AI 文本响应 | `{text: string}` |
| `assistant.tool_use` | AI 工具调用 | `{tool: string, input: object}` |
| `tool_result` | 工具执行结果 | `{toolCallId: string, result: object}` |
| `session.ended` | 会话结束 | `{sessionId: string}` |
| `error` | 错误事件 | `{code: string, message: string}` |

## 已实现功能

| 功能 | 版本 | 状态 |
|------|------|------|
| WebSocket Gateway | v0.1.0 | ✅ |
| HTTP REST API | v0.2.0 | ✅ |
| 会话管理 | v0.3.0 | ✅ |
| 消息历史 | v0.3.0 | ✅ |
| 工具系统 | v0.4.0 | ✅ |
| Agent 工具调用循环 | v1.5.0 | ✅ |
| 多模型支持 | v1.0.0 | ✅ |
| Web 管理界面 | v0.8.0 | ✅ |
| 指标监控 | v1.3.0 | ✅ |
| 速率限制 | v1.3.0 | ✅ |
| 插件系统 | v1.1.0 | ✅ |
| 认证授权 | v1.2.0 | ✅ |
| 消息队列 | v1.4.0 | ✅ |
| 测试体系 | v1.6.0 | ✅ |

## 待实现功能

| 功能 | 优先级 | 描述 |
|------|--------|------|
| 流式响应 | 高 | 支持 Server-Sent Events |
| 消息持久化 | 中 | 数据库支持 |
| Channel 集成 | 中 | Telegram, Discord 等 |
| Token 管理 | 中 | 使用量统计 |
| OAuth/JWT | 低 | 高级认证 |

## 开发指南

### 添加新工具

1. 在 `src/agent/tools.rs` 的 `ToolExecutor::new()` 中添加工具定义
2. 在 `execute()` 方法中添加处理逻辑

```rust
// 添加工具定义
tools.insert("my_tool".to_string(), Tool {
    name: "my_tool".to_string(),
    description: "My custom tool".to_string(),
    input_schema: serde_json::json!({...}),
});

// 添加执行逻辑
"my_tool" => self.execute_my_tool(input).await,
```

### 添加新 API 方法

1. 在 `src/gateway/protocol.rs` 的 `methods` 模块中添加常量
2. 在 `src/gateway/messages.rs` 中实现处理函数

### 运行测试

```bash
cargo test              # 运行所有测试
cargo tarpaulin --out Json  # 运行覆盖率测试
cargo clippy           # 检查代码规范
```

## 参考

- [OpenClaw](https://github.com/openclaw/openclaw)
- [Tokio](https://tokio.rs)
- [Axum](https://docs.rs/axum)
