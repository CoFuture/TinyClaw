# TinyClaw 开发原则 (PRINCIPLES)

## 项目定位

**TinyClaw** - **真正能帮你做事的 AI Agent**，本地运行的智能助手。

> "The AI that actually does things"

### 核心理念（与 OpenClaw 一致）
- **AI as Teammate** - 不是聊天工具，而是能自主工作的"数字员工"
- **数据主权** - 你的数据保存在本地设备上
- **可 hackable** - 完全开源可定制

### 核心目标
1. **真正能做事** - 邮件、日历、智能家居、信息查询等
2. **24/7 运行** - 定时任务、自动执行、主动提醒
3. **稳定可靠** - Rust 内存安全，代码精简
4. **交互友好** - 用户体验优先

### 设计原则
- ✅ 需要的才加（但要聚焦"能做事"的核心工具）
- ❌ 不要为"可能用到"预实现
- ❌ 不要过度工程化
- ✅ 交互体验优先于功能堆砌
- ⚠️ **新增工具要能让 Agent 真正做事**，不只是技术演示

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

### 迭代优先级（面向"真正能做事"）

| 优先级 | 功能 | 说明 |
|--------|------|------|
| **P0** | **Agent 自主执行** | 自动规划+执行多步骤任务、任务队列 |
| **P0** | **核心工具** | 天气、信息搜索、提醒、定时任务 |
| **P0** | **定时任务系统** | cron 风格定时执行、主动提醒 |
| P1 | **多 Session** | 并发会话管理、Session 隔离 |
| P1 | **WebUI 完善** | 管理员界面功能增强、状态可视化 |
| P1 | **TUI 完善** | 命令行交互体验 |
| P2 | **实时反馈** | 执行进度、流式输出展示 |

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

### 每次迭代前 (必须执行)
1. **阅读本文档 (PRINCIPLES.md)** - 了解项目原则和开发规范
2. **阅读 PLANS.md** - 了解长期愿景、优先级定义、当前迭代规划
3. **思考并确定本轮迭代目标**：
   - 根据 PLANS.md 中的优先级和待办事项
   - 结合项目现状（哪些功能最需要完善）
   - 选择 1-2 个核心功能作为本轮目标
4. **制定下一轮迭代计划**：将下一轮要实现的目标写入 PLANS.md 的"当前迭代规划"章节
5. 检查当前代码：`cargo clippy`

### 每次迭代后 (必须执行)
1. **更新 PLANS.md**：
   - 将本轮完成事项记录到"迭代历史"章节
   - 将下一轮迭代目标写入"当前迭代规划"章节
   - 更新待办事项池（标记已完成项）
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
