# TinyClaw 项目设计

## 概述

TinyClaw 是一个用 Rust 实现的轻量级 AI Agent Gateway，旨在提供完整可用的 AI Agent 核心功能，包括对话、工具调用、工具执行全链路闭环。

## 核心设计原则

1. **轻量级** - 最小依赖，Rust 实现高性能
2. **可扩展** - 插件系统支持自定义扩展
3. **全链路** - 从对话到工具执行完整闭环
4. **生产可用** - 速率限制、认证授权、指标监控

## 核心架构

### 1. Gateway (网关层)
- **WebSocket 服务器**: 处理所有客户端连接，基于 JSON-RPC 2.0 协议
- **会话管理**: 管理多个活动会话，支持 Main/Isolated/Channel 三种类型
- **消息路由**: 将消息路由到正确的处理程序
- **消息队列**: 支持背压的异步消息处理

### 2. Agent (代理层)
- **对话管理**: 管理与 AI 的对话上下文和历史
- **工具调用循环**: 解析 AI 响应中的工具调用，执行并返回结果
- **多模型支持**: Anthropic/OpenAI/Ollama 三种提供商

### 3. Message (消息层)
- **消息解析**: 解析 JSON-RPC 2.0 格式的消息
- **消息历史**: 会话消息持久化和检索
- **事件系统**: 事件广播机制

### 4. Tools (工具层)
- **工具注册**: 内置工具自动注册
- **工具执行**: 异步工具执行框架
- **工具类型**: exec, read_file, write_file, list_dir, http_request

## 系统架构图

```
┌──────────────────────────────────────────────────────────────┐
│                        Client Applications                     │
│   (Web UI, CLI, 其他 Agent)                                  │
└───────────────────────────┬──────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
┌─────────────────────────┐   ┌─────────────────────────┐
│   WebSocket Gateway      │   │      HTTP Server        │
│   (JSON-RPC 2.0)         │   │   (REST API + Admin)    │
│   ws://:18790            │   │   http://:8080         │
└───────────┬───────────────┘   └───────────┬─────────────┘
            │                               │
            └───────────────┬───────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                    Handler / Router                           │
│   - protocol.rs: 方法路由                                     │
│   - messages.rs: 消息处理                                     │
└─────────────────────────────┬────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            │                 │                 │
            ▼                 ▼                 ▼
┌───────────────────┐ ┌───────────────┐ ┌───────────────────┐
│   Session Manager  │ │    Agent      │ │   Tools Executor   │
│   - session.rs    │ │   Runtime     │ │   - tools.rs      │
│   - history.rs    │ │   - client.rs │ │                   │
│                   │ │   - context.rs│ │                   │
└───────────────────┘ └───────┬───────┘ └───────────────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │   AI Provider     │
                    │  Anthropic/OpenAI │
                    │     Ollama        │
                    └───────────────────┘
```

## 模块设计

```
TinyClaw/
├── src/
│   ├── main.rs                # 入口点
│   ├── lib.rs                 # 库入口
│   │
│   ├── gateway/               # WebSocket 网关
│   │   ├── server.rs         # WebSocket 服务器
│   │   ├── protocol.rs       # JSON-RPC 2.0 协议定义
│   │   ├── messages.rs       # 消息处理逻辑
│   │   ├── session.rs        # 会话管理
│   │   ├── history.rs        # 消息历史
│   │   ├── events.rs         # 事件广播
│   │   ├── queue.rs          # 消息队列
│   │   ├── templates.rs      # 消息模板
│   │   ├── auth.rs           # 认证授权
│   │   ├── persistence.rs    # 持久化
│   │   ├── session_ext.rs    # 扩展会话
│   │   └── mod.rs
│   │
│   ├── agent/                 # AI Agent
│   │   ├── client.rs         # AI 模型客户端 (多提供商)
│   │   ├── runtime.rs        # Agent 运行时
│   │   ├── context.rs        # 上下文管理
│   │   ├── tools.rs          # 工具执行器
│   │   └── mod.rs
│   │
│   ├── http/                  # HTTP 服务器
│   │   ├── routes.rs         # HTTP 路由
│   │   └── mod.rs
│   │
│   ├── config/                # 配置管理
│   │   ├── schema.rs         # 配置结构
│   │   └── mod.rs
│   │
│   ├── plugins/              # 插件系统
│   │   ├── traits.rs         # 插件 trait
│   │   ├── manager.rs        # 插件管理器
│   │   ├── loader.rs         # 插件加载器
│   │   ├── api.rs            # 插件 API
│   │   └── mod.rs
│   │
│   ├── metrics/              # 指标收集
│   │   ├── collector.rs      # 指标收集器
│   │   └── mod.rs
│   │
│   ├── ratelimit/            # 速率限制
│   │   ├── limiter.rs        # 速率限制器
│   │   └── mod.rs
│   │
│   ├── tui/                  # 终端界面
│   │   ├── ui.rs            # TUI 实现
│   │   ├── app.rs           # TUI 应用
│   │   └── mod.rs
│   │
│   └── common/               # 通用模块
│       ├── error.rs         # 错误类型
│       ├── logging.rs       # 日志设置
│       └── mod.rs
│
├── examples/                 # 示例
│   ├── admin.html           # Web 管理界面
│   └── config.json          # 配置示例
│
├── docs/                     # 文档
│   ├── DESIGN.md            # 本文档
│   ├── PROJECT.md          # 项目说明
│   ├── PRINCIPLES.md       # 开发规范
│   └── ITERATIONS.md       # 迭代记录
│
└── Cargo.toml               # 项目配置
```

## 协议设计

### JSON-RPC 2.0

#### 客户端 -> 网关请求

```json
{
  "id": "uuid-string",
  "method": "agent.turn",
  "params": {
    "message": "用户消息",
    "sessionKey": "可选会话ID"
  }
}
```

#### 网关 -> 客户端响应

成功:
```json
{
  "id": "uuid-string",
  "result": {
    "text": "AI 响应文本"
  }
}
```

失败:
```json
{
  "id": "uuid-string",
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

## 设计决策

### 1. 为什么选择 Rust?
- 性能: 编译时优化，无 GC 停顿
- 安全性: 内存安全，线程安全
- 异步: Tokio 提供成熟的异步生态

### 2. 为什么选择 JSON-RPC 2.0?
- 简单: 易于实现和调试
- 通用: 广泛支持
- 无状态: 适合分布式

### 3. 为什么分离 WebSocket 和 HTTP?
- WebSocket: 实时双向通信，适合 AI 对话
- HTTP: 简单请求，适合管理操作

### 4. 为什么需要工具调用循环?
- 允许 AI 执行实际操作
- 支持复杂任务分解
- 实现真正的 AI Agent

## 安全性考虑

1. **API Key 认证**: 所有 API 调用需要有效的 API Key
2. **权限控制**: 不同操作需要不同权限级别
3. **输入验证**: 所有输入都经过验证
4. **命令注入防护**: exec 工具执行受限命令

## 性能优化

1. **消息队列**: 背压机制防止过载
2. **并发控制**: 最大并发连接数限制
3. **速率限制**: 防止滥用
4. **异步 I/O**: Tokio 提供高效异步处理
