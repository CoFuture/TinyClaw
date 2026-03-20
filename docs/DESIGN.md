# TinyClaw 项目设计

## 概述

TinyClaw是OpenClaw的最小化Rust实现，保留核心功能以便学习和扩展。

## 核心架构

### 1. Gateway (网关层)
- **WebSocket服务器**: 处理所有客户端连接
- **会话管理**: 管理多个活动会话
- **消息路由**: 将消息路由到正确的会话/处理程序

### 2. Agent (代理层)
- **对话管理**: 管理与AI的对话上下文
- **工具调用**: 执行各种工具（exec等）
- **模型集成**: 调用外部AI模型API

### 3. Message (消息层)
- **消息解析**: 解析JSON格式的消息
- **消息验证**: 验证消息格式和必需字段

### 4. Tools (工具层)
- **Exec工具**: 执行shell命令

## 模块设计

```
tiny_claw/
├── src/
│   ├── main.rs           # 入口点
│   ├── lib.rs            # 库入口
│   ├── gateway/
│   │   ├── mod.rs       # 网关模块
│   │   ├── server.rs    # WebSocket服务器
│   │   ├── session.rs   # 会话管理
│   │   ├── protocol.rs  # 协议定义
│   │   └── messages.rs  # 消息处理
│   ├── agent/
│   │   ├── mod.rs       # Agent模块
│   │   ├── client.rs    # AI模型客户端
│   │   └── tools.rs     # 工具注册与执行
│   ├── config/
│   │   ├── mod.rs       # 配置模块
│   │   └── schema.rs    # 配置结构
│   └── common/
│       ├── mod.rs       # 通用模块
│       ├── error.rs     # 错误类型
│       └── logging.rs   # 日志设置
```

## 协议设计

### 客户端->网关消息

```json
{
  "id": "uuid",
  "method": "agent.turn" | "sessions.list" | "sessions.send" | "exec",
  "params": {
    ...
  }
}
```

### 网关->客户端消息

```json
{
  "id": "uuid",
  "result": { ... } | "error": { "code": "", "message": "" }
}
```

## 迭代计划

### Iteration 1: 基础Gateway
- [x] 项目初始化
- [ ] WebSocket服务器
- [ ] 基本的协议解析
- [ ] 日志系统
- [ ] 配置加载

### Iteration 2: 会话管理
- [ ] 会话创建/销毁
- [ ] 会话状态管理

### Iteration 3: Agent集成
- [ ] AI模型客户端
- [ ] 基本的对话功能
- [ ] 工具执行

### Iteration 4: 工具系统
- [ ] Exec工具实现
- [ ] 工具注册机制
