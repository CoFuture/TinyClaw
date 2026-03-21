# TinyClaw

<p align="center">
  <img src="assets/tiny_claw.svg" width="200" alt="TinyClaw Logo">
</p>

<p align="center">
  <a href="https://github.com/CoFuture/TinyClaw">
    <img src="https://img.shields.io/badge/version-2.4.0-blue.svg" alt="Version">
  </a>
  <a href="https://github.com/CoFuture/TinyClaw/blob/master/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  </a>
  <a href="https://github.com/CoFuture/TinyClaw">
    <img src="https://img.shields.io/badge/Rust-1.70+-orange.svg" alt="Rust Version">
  </a>
  <a href="https://github.com/CoFuture/TinyClaw/actions">
    <img src="https://github.com/CoFuture/TinyClaw/workflows/CI/badge.svg" alt="CI">
  </a>
</p>

## 简介

TinyClaw 是一个用 Rust 实现的轻量级 AI Agent Gateway，完整支持 AI 对话、工具调用、工具执行全链路闭环。

## 核心特性

| 特性 | 描述 | 状态 |
|------|------|------|
| 🌐 **WebSocket Gateway** | 基于 JSON-RPC 2.0 协议的 WebSocket 服务器 | ✅ |
| 🔌 **HTTP REST API** | 完整的 RESTful API 接口 + Web 管理界面 | ✅ |
| 💬 **会话管理** | 支持多个并发会话，消息历史持久化 | ✅ |
| 🤖 **Agent 工具调用** | AI Agent 自动调用工具完成任务 | ✅ |
| 🔧 **内置工具** | exec, read_file, write_file, list_dir, http_request | ✅ |
| 📊 **指标监控** | 系统指标收集与速率限制 | ✅ |
| 🔌 **插件系统** | 可扩展的插件架构 | ✅ |
| 🛡️ **认证授权** | API Key 认证与权限管理 | ✅ |

## 支持的 AI 模型

| 提供商 | 模型 | 状态 |
|--------|------|------|
| Anthropic | claude-sonnet-4, claude-3.5 等 | ✅ |
| OpenAI | gpt-4, gpt-3.5-turbo 等 | ✅ |
| Ollama | 本地模型 (llama2, etc.) | ✅ |

## 快速开始

### 1. 构建

```bash
cargo build --release
```

### 2. 配置

创建配置文件 `~/.config/tiny_claw/config.json`:

```json
{
  "gateway": {
    "bind": "127.0.0.1:18789",
    "verbose": false
  },
  "agent": {
    "model": "claude-sonnet-4-20250514",
    "apiKey": "your-api-key-here",
    "apiBase": "https://api.anthropic.com"
  },
  "tools": {
    "execEnabled": true
  },
  "ratelimit": {
    "requestsPerMinute": 60
  }
}
```

### 3. 运行

```bash
# 使用默认配置
cargo run --release

# 或指定配置文件
cargo run --release -- --config /path/to/config.json
```

### 4. 访问

- **Web 管理界面**: http://localhost:8080/admin.html
- **WebSocket**: ws://127.0.0.1:18789
- **HTTP API**: http://localhost:8080

## API 使用

### WebSocket 对话 (agent.turn)

```javascript
const ws = new WebSocket('ws://127.0.0.1:18789');

ws.send(JSON.stringify({
  id: "1",
  method: "agent.turn",
  params: {
    message: "列出当前目录的文件"
  }
}));

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log(response);
  // 如果AI调用工具，会自动执行并返回结果
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

# 获取会话列表
curl http://localhost:8080/api/sessions

# 获取会话历史
curl http://localhost:8080/api/sessions/{session_id}/messages

# 执行工具
curl -X POST http://localhost:8080/api/tools/execute \
  -H "Content-Type: application/json" \
  -d '{"name": "exec", "input": {"command": "ls -la"}}'
```

## 可用 JSON-RPC 方法

| 方法 | 描述 |
|------|------|
| `ping` | 心跳检测 |
| `sessions.list` | 列出所有会话 |
| `sessions.send` | 发送消息到会话 |
| `sessions.history` | 获取会话历史 |
| `agent.turn` | 与 AI 对话（自动工具调用） |
| `exec` | 直接执行 Shell 命令 |
| `tools.list` | 列出所有可用工具 |
| `tools.execute` | 执行指定工具 |
| `status` | 获取服务器状态 |
| `shutdown` | 关闭服务器 |

## 可用内置工具

| 工具 | 描述 | 示例 |
|------|------|------|
| `exec` | 执行 Shell 命令 | `{"command": "ls -la"}` |
| `read_file` | 读取文件内容 | `{"path": "/tmp/test.txt"}` |
| `write_file` | 写入文件内容 | `{"path": "/tmp/test.txt", "content": "hello"}` |
| `list_dir` | 列出目录内容 | `{"path": "."}` |
| `http_request` | 发起 HTTP 请求 | `{"url": "https://api.example.com", "method": "GET"}` |
| `glob` | 按模式匹配文件 | `{"pattern": "**/*.rs"}` |
| `grep` | 搜索文件内容 | `{"pattern": "fn ", "path": "."}` |
| `sed_file` | 替换文件内容 | `{"path": "a.txt", "old_text": "foo", "new_text": "bar"}` |
| `which` | 查找可执行文件 | `{"command": "rustc"}` |
| `mkdir` | 创建目录 | `{"path": "/tmp/test_dir"}` |
| `stat_file` | 获取文件元数据 | `{"path": "/tmp/test.txt"}` |
| `find` | 按名称查找文件 | `{"name": "*.rs", "path": "."}` |
| `tail` | 读取文件末尾行 | `{"path": "/tmp/test.txt", "lines": 10}` |
| `batch_execute` | 批量执行工具 | `{"tools": [{"name": "exec", "input": {"command": "ls"}}]}` |
| `env` | 环境变量管理 | `{"name": "PATH"}` |
| `diff` | 文件对比 | `{"path1": "a.txt", "path2": "b.txt"}` |

## 项目结构

```
TinyClaw/
├── src/
│   ├── agent/           # AI Agent 模块
│   │   ├── client.rs    # AI 模型客户端 (多提供商支持)
│   │   ├── context.rs   # Agent 上下文
│   │   ├── runtime.rs   # Agent 运行时
│   │   └── tools.rs     # 工具执行器
│   ├── gateway/          # WebSocket 网关
│   │   ├── messages.rs  # 消息处理
│   │   ├── protocol.rs  # JSON-RPC 协议
│   │   ├── session.rs  # 会话管理
│   │   ├── history.rs  # 消息历史
│   │   ├── events.rs   # 事件系统
│   │   ├── queue.rs    # 消息队列
│   │   ├── templates.rs # 消息模板
│   │   └── server.rs   # WebSocket 服务器
│   ├── http/            # HTTP 服务器
│   │   └── routes.rs   # HTTP 路由
│   ├── config/          # 配置管理
│   ├── plugins/         # 插件系统
│   ├── metrics/         # 指标收集
│   ├── ratelimit/      # 速率限制
│   └── tui/             # 终端界面
├── examples/
│   └── admin.html       # Web 管理界面
├── docs/
│   ├── DESIGN.md       # 设计文档
│   ├── PROJECT.md      # 项目说明
│   ├── PRINCIPLES.md  # 开发规范
│   └── ITERATIONS.md  # 迭代记录
└── Cargo.toml
```

## 版本历史

See [docs/ITERATIONS.md](docs/ITERATIONS.md) for detailed version history.

- **v2.4.0** - 会话导入导出 + 连接状态API + 工具schema验证
- **v2.3.0** - 工具增强 (find, tail, exec 超时修复)
- **v2.2.0** - 批量执行 + env/diff 工具
- **v2.1.0** - 增强文件工具 + 路径规范化
- **v2.0.3** - sed_file 和 which 工具
- **v2.0.2** - Request ID 追踪 + 会话恢复
- **v2.0.1** - SQLite 持久化 + 优雅关闭
- **v2.0.0** - 持久化与优雅关闭
- **v1.9.0** - 错误处理增强 + 断路器
- **v1.8.0** - 交互式对话 UI
- **v1.7.0** - 消除冗余 + 设计对齐
- **v1.6.0** - 测试体系完善 (76 个测试用例)
- **v1.5.0** - 全链路 Agent 工具调用
- **v1.4.0** - WebSocket 消息队列优化
- **v1.3.0** - 指标监控与速率限制
- **v1.0.0** - 多模型支持

## 测试

```bash
# 运行所有测试
cargo test

# 运行测试覆盖率
cargo tarpaulin --out Json

# 查看 clippy 警告
cargo clippy
```

## 设计理念

TinyClaw 旨在实现一个完整可用的 AI Agent Gateway，核心设计原则：

1. **轻量级** - 最小依赖，Rust 实现高性能
2. **可扩展** - 插件系统支持自定义扩展
3. **全链路** - 从对话到工具执行完整闭环
4. **生产可用** - 速率限制、认证授权、指标监控

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
