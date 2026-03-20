# TinyClaw

<p align="center">
  <img src="assets/tiny_claw.svg" width="200" alt="TinyClaw Logo">
</p>

<p align="center">
  <a href="https://github.com/CoFuture/TinyClaw">
    <img src="https://img.shields.io/badge/version-1.0.0-blue.svg" alt="Version">
  </a>
  <a href="https://github.com/CoFuture/TinyClaw/blob/master/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  </a>
  <a href="https://github.com/CoFuture/TinyClaw">
    <img src="https://img.shields.io/badge/Rust-1.70+-orange.svg" alt="Rust Version">
  </a>
</p>

## 简介

TinyClaw 是一个用 Rust 实现的 OpenClaw 最小化版本，旨在还原 OpenClaw 的核心架构，提供一个轻量级、可扩展的 AI Agent Gateway。

## 特性

- 🌐 **WebSocket Gateway** - 基于 JSON-RPC 2.0 协议的 WebSocket 服务器
- 🔌 **HTTP REST API** - 完整的 RESTful API 接口
- 💬 **会话管理** - 支持多个并发会话
- 📝 **消息历史** - 会话消息历史记录
- 🔧 **工具系统** - 多种内置工具支持
  - `exec` - 执行 Shell 命令
  - `read_file` - 读取文件
  - `write_file` - 写入文件
  - `list_dir` - 列出目录
  - `http_request` - HTTP 请求
- 📊 **事件系统** - 事件广播机制
- ⚙️ **配置管理** - 运行时配置更新

## 快速开始

### 构建

```bash
cargo build --release
```

### 运行

```bash
# 使用默认配置
cargo run --release

# 或指定配置文件
cargo run --release -- --config /path/to/config.json
```

### 配置

创建配置文件 `~/.config/tiny_claw/config.json`:

```json
{
  "gateway": {
    "bind": "127.0.0.1:18789",
    "verbose": false,
    "dataDir": null
  },
  "agent": {
    "model": "anthropic/claude-sonnet-4-20250514",
    "apiKey": "your-api-key-here",
    "apiBase": "https://api.anthropic.com",
    "workspace": null
  },
  "tools": {
    "execEnabled": true
  }
}
```

## API 使用

### WebSocket 连接

```javascript
const ws = new WebSocket('ws://127.0.0.1:18789');

// 发送消息
ws.send(JSON.stringify({
  id: "1",
  method: "agent.turn",
  params: {
    message: "Hello!"
  }
}));

// 接收响应
ws.onmessage = (event) => {
  console.log(JSON.parse(event.data));
};
```

### HTTP API

```bash
# 健康检查
curl http://localhost:8080/health

# 获取状态
curl http://localhost:8080/api/status

# 列出工具
curl http://localhost:8080/api/tools

# 执行命令
curl -X POST http://localhost:8080/api/exec \
  -H "Content-Type: application/json" \
  -d '{"command": "ls -la"}'
```

## 可用方法

| 方法 | 描述 |
|------|------|
| `ping` | 心跳检测 |
| `sessions.list` | 列出所有会话 |
| `sessions.send` | 发送消息到会话 |
| `sessions.history` | 获取会话历史 |
| `agent.turn` | 与 AI 对话 |
| `exec` | 执行 Shell 命令 |
| `tools.list` | 列出所有工具 |
| `tools.execute` | 执行工具 |
| `status` | 获取服务器状态 |
| `shutdown` | 关闭服务器 |

## 项目结构

```
tiny_claw/
├── src/
│   ├── agent/        # AI Agent 模块
│   ├── common/       # 通用工具
│   ├── config/       # 配置管理
│   ├── gateway/      # WebSocket 网关
│   └── http/         # HTTP 服务器
├── docs/             # 文档
└── Cargo.toml       # 项目配置
```

## 迭代版本

- **v0.1.0** - 初始版本，基础 Gateway
- **v0.2.0** - 添加 HTTP REST API
- **v0.3.0** - 会话历史与事件系统
- **v0.4.0** - 高级工具系统

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
