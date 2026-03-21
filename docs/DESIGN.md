# TinyClaw 系统设计

## 概述

TinyClaw 是一个轻量级 AI Agent Gateway，实现从对话到工具执行的完整闭环。

它是 OpenClaw 的 Rust 实现子集，参考了 OpenClaw 的核心架构设计。

## OpenClaw 背景

> "OpenClaw is a personal AI assistant you run on your own devices.
> It answers you on the channels you already use."

OpenClaw 是一个跨平台的个人 AI 助手，支持：
- **多渠道**：WhatsApp, Telegram, Slack, Discord, Signal, iMessage 等
- **多设备**：macOS, iOS, Android
- **语音**：支持语音唤醒和对话
- **视觉**：Live Canvas 可视化工作区

OpenClaw 的核心架构：
- **Gateway**：WS 控制平面，管理会话、渠道、工具、事件
- **Agent Runtime**：Pi agent runtime，支持工具流和块流
- **Session Model**：main 用于直接聊天，group isolation 用于群组隔离

## 设计原则

1. **最小化** - 只实现必要的功能
2. **稳定性** - 代码简洁，易于维护
3. **安全性** - Rust 内存安全

## 系统架构

```
┌──────────────────────────────────────────────────────────────┐
│                        Client Applications                     │
│         (Web UI / CLI / Application / Other Agent)           │
└───────────────────────────┬──────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
┌─────────────────────────┐   ┌─────────────────────────┐
│   WebSocket Gateway      │   │      HTTP Server          │
│   (JSON-RPC 2.0)       │   │   (REST + Admin UI)      │
│   ws://:18790           │   │   http://:8080          │
└───────────┬───────────────┘   └───────────┬─────────────┘
            │                               │
            └───────────────┬───────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                    Handler / Router                           │
│   - JSON-RPC 协议解析                                       │
│   - 消息路由到 agent.turn                                  │
└─────────────────────────────┬────────────────────────────────┘
                            │
┌────────────────────────────┼────────────────────────────────┐
│                            │                                │
│                            ▼                                ▼
┌───────────────────┐ ┌───────────────┐ ┌───────────────────┐
│   Session Manager  │ │    Agent      │ │   Tools Executor  │
│                   │ │   Runtime     │ │                   │
│ session.rs        │ │ runtime.rs    │ │   tools.rs       │
│ history.rs        │ │ client.rs     │ │                   │
└───────────────────┘ └───────────────┘ └───────────────────┘
                            │
                            ▼
                    ┌───────────────────┐
                    │   AI Provider      │
                    │  OpenAI/GLM/Claude │
                    └───────────────────┘
```

## 核心模块

### 1. Gateway (WebSocket)

**职责**: 处理客户端连接，JSON-RPC 协议解析

**关键文件**:
- `server.rs` - WebSocket 服务器
- `protocol.rs` - JSON-RPC 2.0 定义
- `messages.rs` - 消息处理
- `session.rs` - 会话管理

### 2. Agent (AI 代理)

**职责**: 与 AI 模型交互，工具调用循环

**关键文件**:
- `runtime.rs` - Agent 运行时
- `client.rs` - AI 模型客户端
- `tools.rs` - 工具执行器

### 3. HTTP Server

**职责**: REST API 和 Web 管理界面（含交互式聊天）

**关键文件**:
- `routes.rs` - HTTP 路由

## 数据流

```
用户消息
    │
    ▼
Gateway: 解析 JSON-RPC
    │
    ▼
消息路由: agent.turn
    │
    ▼
Agent Runtime:
  1. 发送消息给 AI
  2. 检查响应是否有 tool_calls
  3. 执行工具
  4. 将结果返回给 AI
  5. 循环直到最终回复
    │
    ▼
返回结果给客户端
```

## 与 OpenClaw 的差异

| 特性 | OpenClaw | TinyClaw |
|------|----------|----------|
| 语言 | TypeScript/Node.js | Rust |
| 渠道 | 20+ 消息渠道 | 仅 WebSocket |
| 语音 | 支持 | 不支持 |
| Canvas | 支持 | 不支持 |
| 技能 | 支持 | 不支持 |
| 复杂度 | 高 | 低 |
| 性能 | 中 | 高 |
| 内存安全 | 依赖 Node.js | 原生 Rust |

## 安全性

1. **速率限制** - 防止滥用
2. **输入验证** - JSON-RPC 参数验证
3. **命令限制** - exec 工具限制
4. **无硬编码** - 密钥在配置中

## 性能

1. **异步 I/O** - Tokio 异步运行时
2. **并发控制** - Arc + RwLock
3. **最小依赖** - Rust 编译优化
