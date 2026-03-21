# TinyClaw 开发原则 (PRINCIPLES)

## 项目定位

**TinyClaw** - OpenClaw 的 Rust 实现子集，一个**小而精**的生产级 AI Agent Gateway。

### 核心目标
1. **稳定可用** - 核心功能完整可运行
2. **运行时安全** - 最小依赖，内存安全
3. **代码精简** - 无冗余，易维护
4. **交互友好** - 用户体验优先

### 设计原则
- ✅ 需要的才加
- ❌ 不要为"可能用到"预实现
- ❌ 不要过度工程化
- ✅ 交互体验优先于功能堆砌

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

### 交互体验优先级

当前重点打磨方向（优先于新功能开发）：

| 优先级 | 功能 | 说明 |
|--------|------|------|
| P0 | **WebUI 完善** | 管理员界面功能增强、状态可视化 |
| P0 | **Terminal UI (TUI)** | 命令行交互体验、实时状态展示 |
| P0 | **多 Session 支持** | 并发会话管理、Session 隔离与切换 |
| P1 | **多轮对话优化** | 上下文保持、会话历史管理 |
| P1 | **实时反馈** | 执行进度、流式输出展示 |

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

## 迭代策略

### 核心原则
每轮迭代应包含：
- **基础修复**: bug 修复、代码质量提升、clippy 警告消除
- **核心功能**: 1-2 个主要功能的新增或重大完善

### 避免
- ❌ 仅因极小改动（<10行）就触发 commit/push
- ❌ 连续多轮只有 tiny fix，缺乏实质功能进展
- ❌ 为赶版本号而拆分功能

### 触发 commit 的条件
满足以下任一条件时进行 commit：
1. 累计代码改动 ≥ 50 行（不含测试）
2. 新增/重构 ≥ 1 个核心功能模块
3. 完成 ≥ 1 个交互体验改进
4. 版本号需要更新（功能里程碑）

### 迭代周期
- 每 30 分钟自动检查（由 OpenClaw cron 驱动）
- 每次迭代应有明确的**功能目标**
- 功能完成后统一 commit + push

---

## 开发规范

### 每次迭代前
1. **阅读本文档 (PRINCIPLES.md)**
2. **阅读 PLANS.md** - 了解上一次的迭代规划和长期愿景
3. 检查当前代码：`cargo clippy`
4. 确认本轮迭代目标：基础修复 + 核心功能

### 每次迭代后 (必须执行)
1. 更新 **PLANS.md** - 记录本轮完成事项与下一步规划
2. `git add .` - 暂存所有更改
3. `git commit -m "..."` - 提交更改（符合上述触发条件时）
4. **`git push origin master`** - 推送到 GitHub

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
- 修订号：交互体验改进、小 bug 修复
- 次版本：核心功能新增、重大重构
- 主版本：架构变更、兼容性破坏

### 版本发布时机
- 每次 commit 时评估是否需要更新版本号
- 累计多个小功能或完成一个里程碑功能 → 提升次版本
- 仅修复体验问题或小 bug → 可仅更新修订号

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
