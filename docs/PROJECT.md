# TinyClaw 项目说明文档

## 项目概述

**TinyClaw** - OpenClaw 的 Rust 实现子集，一个小而精的生产级 AI Agent Gateway。

### OpenClaw 是什么？

> OpenClaw is a personal AI assistant you run on your own devices.
> It answers you on the channels you already use (WhatsApp, Telegram, Slack, Discord, Google Chat, Signal, iMessage, IRC, Microsoft Teams, Matrix, Feishu, LINE, Mattermost, Nextcloud Talk, Nostr, Synology Chat, Tlon, Twitch, Zalo, WebChat).

**OpenClaw** 是一个运行在你自有设备上的个人 AI 助手，支持多种消息渠道，具备语音和听力能力，可以渲染 Live Canvas。

### TinyClaw 的定位

TinyClaw 是 OpenClaw 的 Rust 实现子集：
- **目标**：小而精，保留核心功能
- **语言**：Rust（高性能、内存安全）
- **架构**：Gateway 控制平面 + Agent Runtime

## OpenClaw 完整功能 vs TinyClaw 子集

| OpenClaw 功能 | TinyClaw 状态 |
|--------------|---------------|
| **Gateway WS 控制平面** | ✅ 已实现 |
| **多渠道收件箱** | ❌ 未实现 (WhatsApp, Telegram 等) |
| **多智能体路由** | ❌ 未实现 |
| **Pi Agent Runtime** | ✅ 已实现 (简化版) |
| **会话模型** (main/isolation) | ✅ 已实现 |
| **工具系统** | ✅ 已实现 |
| **语音支持** | ❌ 未实现 |
| **Live Canvas** | ❌ 未实现 |
| **技能系统** | ❌ 未实现 |
| **伴侣应用** (macOS/iOS/Android) | ❌ 未实现 |
| **Node 节点** | ❌ 未实现 |

## 核心架构

```
┌─────────────────────────────────────────────────────────┐
│                        Client                             │
│         (Web UI / CLI / Application)                     │
└─────────────────────┬───────────────────────────────────┘
                      │ WebSocket / HTTP
                      ▼
┌─────────────────────────────────────────────────────────┐
│                   TinyClaw Core                          │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │  Gateway    │  │    Agent     │  │     HTTP      │ │
│  │  (WebSocket)│  │  (AI + Tools)│  │ (REST + Web)  │ │
│  └─────────────┘  └──────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## 模块结构

### agent/ - AI Agent 模块
| 文件 | 职责 |
|------|------|
| client.rs | AI 模型客户端 (支持 Anthropic/OpenAI/GLM) |
| runtime.rs | Agent 运行时 + 工具调用循环 |
| tools.rs | 工具执行器 (exec/read_file/write_file/list_dir/http) |

### gateway/ - WebSocket 网关
| 文件 | 职责 |
|------|------|
| server.rs | WebSocket 服务器 |
| messages.rs | JSON-RPC 消息处理 |
| protocol.rs | JSON-RPC 2.0 协议定义 |
| session.rs | 会话管理 |
| history.rs | 消息历史 |
| events.rs | 事件系统 |

### http/ - HTTP 服务器
| 文件 | 职责 |
|------|------|
| routes.rs | REST API + Web 管理界面 + 交互式聊天 |

## 完整链路

```
用户消息
    │
    ▼
┌──────────────────────────────────────┐
│  Gateway (WebSocket)                  │
│  JSON-RPC 解析 → agent.turn         │
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│  Agent Runtime                        │
│  发送消息给 AI 模型                  │
│  AI 请求工具调用                      │
│  执行工具 → 返回结果                  │
│  循环直到最终回复                    │
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│  工具执行                             │
│  exec / read_file / write_file       │
│  list_dir / http_request             │
└──────────────────────────────────────┘
```

## 配置说明

```json
{
  "gateway": {
    "bind": "127.0.0.1:18790"
  },
  "agent": {
    "model": "glm-5",
    "api_key": "your-api-key",
    "api_base": "https://open.bigmodel.cn/api/paas",
    "provider": "openai"
  },
  "tools": {
    "execEnabled": true
  },
  "ratelimit": {
    "maxRequests": 60
  }
}
```

## JSON-RPC 方法

| 方法 | 描述 |
|------|------|
| ping | 心跳检测 |
| sessions.list | 列出所有会话 |
| sessions.send | 发送消息到会话 |
| sessions.history | 获取会话历史 |
| agent.turn | AI 对话 (自动工具调用) |
| tools.list | 列出可用工具 |
| tools.execute | 执行工具 |
| status | 服务器状态 |
| shutdown | 关闭服务器 |

## 内置工具

| 工具 | 描述 |
|------|------|
| exec | 执行 Shell 命令 |
| read_file | 读取文件 |
| write_file | 写入文件 |
| list_dir | 列出目录 |
| http_request | HTTP 请求 |

## 技术栈

| 组件 | 技术 |
|------|------|
| 运行时 | Tokio |
| WebSocket | tokio-tungstenite |
| HTTP | Axum |
| 序列化 | Serde JSON |
| 日志 | Tracing |

## 项目状态

| 功能 | 状态 |
|------|------|
| WebSocket Gateway | ✅ |
| HTTP REST API | ✅ |
| 会话管理 | ✅ |
| AI 工具调用循环 | ✅ |
| 多模型支持 | ✅ |
| 内置工具 | ✅ |
| 速率限制 | ✅ |
| Web 管理界面 | ✅ |
| 交互式聊天 UI | ✅ |
| 多渠道 (WhatsApp/Telegram/Discord) | ❌ |
| 语音支持 | ❌ |
| Live Canvas | ❌ |
| 技能系统 | ❌ |

## 设计原则

1. **小而精** - 只实现必要的核心功能
2. **稳定性** - 代码简洁，易于维护
3. **安全性** - Rust 内存安全，最小依赖

## 参考

- [OpenClaw](https://github.com/openclaw/openclaw)
- [OpenClaw Docs](https://docs.openclaw.ai)
- [Tokio](https://tokio.rs)
- [Axum](https://docs.rs/axum)
