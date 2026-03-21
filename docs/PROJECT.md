# TinyClaw 项目说明文档

## 项目概述

**TinyClaw** - OpenClaw 的 Rust 实现子集，一个小而精的生产级 AI Agent Gateway。

**设计目标**: 保留 OpenClaw 核心功能，实现一个稳定可用的 AI Agent 产品。

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
| routes.rs | REST API + Web 管理界面 |

### ratelimit/ - 速率限制
| 文件 | 职责 |
|------|------|
| limiter.rs | 令牌桶速率限制 |

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
| 插件系统 | ❌ (已移除冗余) |
| TUI | ❌ (已移除冗余) |

## 参考

- [OpenClaw](https://github.com/openclaw/openclaw)
- [Tokio](https://tokio.rs)
- [Axum](https://docs.rs/axum)
