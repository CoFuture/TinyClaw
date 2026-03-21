# TinyClaw 开发原则 (PRINCIPLES)

## 项目定位

**TinyClaw** - OpenClaw 的 Rust 实现子集，一个**小而精**的生产级 AI Agent Gateway。

### 核心目标
1. **稳定可用** - 核心功能完整可运行
2. **运行时安全** - 最小依赖，内存安全
3. **代码精简** - 无冗余，易维护

### 设计原则
- ✅ 需要的才加
- ❌ 不要为"可能用到"预实现
- ❌ 不要过度工程化

---

## 核心模块架构

### 已实现 (必须保留)

```
TinyClaw 核心:
├── agent/           # AI Agent
│   ├── client.rs   # 多模型客户端 (Anthropic/OpenAI/GLM)
│   ├── runtime.rs  # Agent 运行时 + 工具调用循环
│   └── tools.rs    # 工具执行器 (exec/read_file/write_file/list_dir/http)
│
├── gateway/         # WebSocket 网关
│   ├── server.rs   # WebSocket 服务器
│   ├── messages.rs # 消息处理
│   ├── protocol.rs # JSON-RPC 协议
│   ├── session.rs  # 会话管理
│   ├── history.rs  # 消息历史
│   └── events.rs   # 事件系统
│
├── http/            # HTTP 服务器
│   └── routes.rs   # REST API + Web 管理界面
│
├── config/          # 配置管理
│   └── schema.rs   # 配置结构
│
└── ratelimit/      # 速率限制
    └── limiter.rs  # 速率限制器
```

### 已删除的冗余模块

| 模块 | 原因 |
|------|------|
| plugins/ | 未使用，~500行 |
| tui/ | 未集成，~420行 |
| gateway/auth.rs | 未使用 |
| gateway/persistence.rs | 未使用 |
| gateway/session_ext.rs | 未使用 |
| gateway/streaming.rs | 未使用 |
| gateway/queue.rs | 未使用 |
| gateway/templates.rs | 未使用 |

---

## 开发规范

### 每次迭代前
1. 阅读本文档
2. 检查当前代码：`cargo clippy`
3. 确认目标：只做必要的改动

### 添加新功能前
1. 确认功能是**必须**的
2. 确认没有现有实现可以复用
3. 确认测试覆盖

### 代码质量
1. `cargo clippy` 无警告
2. `cargo build` 通过
3. `cargo test` 全部通过
4. **覆盖率 ≥ 60%** (核心模块 ≥ 70%)

---

## 版本规范

### 版本号
- `主版本.次版本.修订号`
- 稳定版只做 bug 修复
- 新功能在次版本

### 提交格式
```
<type>: <描述>

[type]: feat|fix|docs|refactor|test|chore
```

---

## 测试规范

### 必须测试
- 单元测试：每个公共函数
- 集成测试：模块交互

### 覆盖率目标
- 总体 ≥ 60%
- 核心模块 (client/messages/session) ≥ 70%

### 运行测试
```bash
cargo test              # 测试
cargo tarpaulin --out Json  # 覆盖率
```

---

## 安全规范

1. **不硬编码密钥** - 使用环境变量或配置
2. **输入验证** - 验证所有外部输入
3. **命令注入防护** - exec 工具限制命令
4. **速率限制** - 防止滥用

---

## 文件结构

```
tiny_claw/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── agent/          # AI Agent (核心)
│   ├── gateway/        # WebSocket 网关 (核心)
│   ├── http/          # HTTP 服务器 (核心)
│   ├── config/        # 配置
│   ├── common/        # 通用工具
│   └── ratelimit/     # 速率限制
├── examples/
│   └── admin.html     # Web 管理界面
├── docs/
└── Cargo.toml
```
