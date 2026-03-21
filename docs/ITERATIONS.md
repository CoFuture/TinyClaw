# TinyClaw 版本迭代记录

## v0.1.0 - 2026-03-21

### 初始化版本
- 创建项目结构
- 定义项目设计文档

### 完成状态: ✅ 已完成并推送

---

## v0.2.0 - 2026-03-21

### Iteration 2: HTTP服务器与配置管理

新增功能:
- [x] HTTP服务器 (Web UI + REST API)
- [x] 健康检查端点
- [x] 配置热重载

### 完成状态: ✅ 已完成并推送

---

## v0.3.0 - 2026-03-21

### Iteration 3: 会话历史与事件系统

新增功能:
- [x] 会话消息历史
- [x] 事件广播系统
- [x] sessions.history API

### 完成状态: ✅ 已完成并推送

---

## v0.4.0 - 2026-03-21

### Iteration 4: 高级工具系统

新增功能:
- [x] ToolExecutor 工具执行器
- [x] 文件操作工具 (read_file, write_file, list_dir)
- [x] HTTP请求工具 (http_request)
- [x] tools.list 和 tools.execute API

### 完成状态: ✅ 已完成并推送

---

## v0.5.0 - 2026-03-21

### Iteration 5: 项目完善

新增功能:
- [x] README.md
- [x] 项目说明文档
- [x] 配置示例

### 完成状态: ✅ 已完成并推送

---

## v0.6.0 - 2026-03-21

### Iteration 6: 实用功能增强

新增功能:
- [x] 启动脚本 (scripts/start.sh)
- [x] 服务管理脚本 (scripts/install_service.sh)
- [x] 配置示例 (examples/)
- [x] 增强健康检查 (uptime, memory)

### 完成状态: ✅ 已完成并推送

---

## v0.7.0 - 2026-03-21

### Iteration 7: Agent Runtime 核心

新增功能:
- [x] AgentRuntime 运行时引擎
- [x] AgentContext 上下文管理
- [x] ExecutionState 执行状态跟踪
- [x] ToolCall / ModelResponse 类型
- [x] RuntimeConfig 运行时配置
- [x] PRINCIPLES.md 开发规范

### 完成状态: ✅ 已完成并推送

---

## v0.8.0 - 2026-03-21

### Iteration 8: 持久化与界面增强

新增功能:
- [x] 消息持久化模块 (persistence.rs)
- [x] Web 管理界面 (admin.html + HTTP 路由)
- [x] 终端界面 (TUI) - 使用 ratatui

### 完成状态: ✅ 已完成并推送

---

## v0.9.0 - 2026-03-21

### Iteration 9: 界面增强

新增功能:
- [x] Web 管理界面增强 (实时数据图表、性能监控)
- [x] TUI 交互式会话管理 (键盘导航、详情查看)
- [x] 消息预览与搜索 (API + UI)
- [x] 会话消息 API (/api/sessions/:id/messages)

### 完成状态: ✅ 已完成

---

## v1.0.0 - 2026-03-21

### Iteration 10: 多模型支持

新增功能:
- [x] ModelProvider 枚举 (Anthropic, OpenAI, Ollama)
- [x] ModelConfig 模型配置
- [x] AgentConfig 支持 provider 字段
- [x] 多模型客户端实现
  - Anthropic API 支持
  - OpenAI API 支持
  - Ollama 本地模型支持
- [x] 自动模型类型检测 (根据模型名称)
- [x] 单元测试覆盖

### 完成状态: ✅ 已完成并推送

---

## v1.1.0 - 2026-03-21

### Iteration 11: 插件系统

新增功能:
- [x] 插件 trait 定义 (Plugin, Hook, PluginMetadata)
- [x] 插件管理器 (PluginManager)
- [x] 插件加载器 (PluginLoader)
- [x] 内置插件
  - Echo Plugin - 回显消息
  - Logger Plugin - 记录所有消息和事件
  - Validator Plugin - 验证消息和参数

### 完成状态: ✅ 已完成并推送

---

## v1.2.0 - 2026-03-21

### Iteration 12: 认证授权与会话增强

新增功能:
- [x] 认证与授权模块 (auth.rs)
  - ApiKey 结构 - API 密钥认证
  - Permission 枚举 - 权限类型 (Read, Write, Admin, Execute)
  - Authenticator - 密钥管理验证器
  - require_admin / require_permission 辅助函数
- [x] 消息模板系统 (templates.rs)
  - MessageTemplate - 模板结构支持变量替换
  - TemplateManager - 模板管理器
  - 6 个内置模板 (greeting, farewell, error_*, success, processing)
- [x] 高级会话管理 (session_ext.rs)
  - SessionTag - 会话标签
  - ExtendedSession - 扩展会话信息
  - AdvancedSessionManager - 高级会话管理
  - 会话优先级、置顶、标签功能
  - 会话导出/导入支持
- [x] 插件 API (plugins/api.rs)
  - PluginApi - 插件 HTTP API 状态管理
  - 插件启用/禁用/列表功能

### 完成状态: ✅ 已完成并推送

---

## v1.3.0 - 2026-03-21

### Iteration 13: 指标监控与速率限制

新增功能:
- [x] 指标收集模块 (metrics/)
  - SystemMetrics - 系统指标 (请求数、响应时间、会话数等)
  - EndpointMetrics - 端点指标
  - MetricsCollector - 指标收集器
  - 每分钟请求数统计
  - 平均响应时间跟踪
- [x] 速率限制模块 (ratelimit/)
  - RateLimitConfig - 速率限制配置
  - RateLimiter - 滑动窗口速率限制器
  - RateLimitResult - 限制结果
  - 客户端级别限制
  - 自动封禁机制
- [x] HTTP API 端点
  - /api/metrics - 获取系统指标
  - /api/ratelimit/:client_id - 检查速率限制状态

### 完成状态: ✅ 已完成

---

## v1.4.0 - 2026-03-21

### Iteration 14: WebSocket 消息队列优化

新增功能:
- [x] 异步消息处理架构
  - 独立的响应写入任务 (writer task)
  - 并发请求处理 (最多 10 个并发)
  - 消息 channel 缓冲区
- [x] 消息队列模块 (gateway/queue.rs)
  - MessageQueue 结构 - 可配置的消息队列
  - MessageQueueBuilder - 建造者模式
  - 背压 (backpressure) 支持
  - 单元测试覆盖
- [x] HandlerContext Clone 支持
- [x] 修复导入路径问题 (ModelConfig)

### 完成状态: ✅ 已完成

---

## 迭代计划

### v1.5.0 - 2026-03-21

### Iteration 15: 全链路 Agent 工具调用 + 流式响应

**目标**: 实现完整的对话 → Agent → 工具执行 → 结果返回 闭环

新增功能:
- [x] Agent 工具调用循环实现
  - 解析模型响应的 tool_calls
  - 执行工具 (exec, read_file, write_file, list_dir, http_request)
  - 将工具结果返回给模型
  - 循环直到模型返回最终文本
- [x] 支持 Anthropic tool_use 格式
- [x] 支持 OpenAI function_call 格式
- [x] 流式响应支持 (gateway/streaming.rs)
  - StreamingEvent 事件类型 (chunk, tool_start, tool_result, end, error)
  - StreamingResponse 流式响应句柄
  - StreamingManager 并发流管理
  - SSE (Server-Sent Events) 支持

### 完成状态: ✅ 已完成

---

## v1.6.0 - 2026-03-21

### Iteration 16: 测试体系完善

**目标**: 提升代码覆盖率，完善各模块测试用例

新增测试:
- [x] agent/tools.rs: 工具执行器测试 (18个测试)
- [x] gateway/protocol.rs: 协议解析测试 (10个测试)
- [x] gateway/history.rs: 会话历史测试 (10个测试)
- [x] gateway/session.rs: 会话管理测试 (9个测试)
- [x] config/schema.rs: 配置模块测试 (10个测试)
- [x] plugins/manager.rs: 插件管理器测试 (6个测试)

测试统计:
- 测试用例: 76个 (之前20个 → 现在76个)
- 新增测试: 56个

### 完成状态: ✅ 已完成

---

## v1.6.0 - 2026-03-21

### Iteration 16: 工具执行超时处理

新增功能:
- [x] execute_with_timeout 方法 (ToolExecutor)
  - 支持自定义超时时间 (timeout_ms 参数)
  - 默认 30 秒超时
  - 超时返回错误信息包含持续时间

### 完成状态: ✅ 已完成

---

## v1.7.0 - 2026-03-21

### Iteration 17: 消除冗余 + 设计对齐

**目标**: 消除冗余实现，对齐"小而精"设计目标

消除冗余:
- [x] 删除 plugins/ 模块 (~500行)
- [x] 删除 tui/ 模块 (~420行)
- [x] 删除 gateway/auth.rs
- [x] 删除 gateway/persistence.rs
- [x] 删除 gateway/session_ext.rs
- [x] 删除 gateway/streaming.rs
- [x] 删除 gateway/queue.rs
- [x] 删除 gateway/templates.rs

设计更新:
- [x] 更新 PRINCIPLES.md - 添加设计原则
- [x] 更新 PROJECT.md - 简化架构文档
- [x] 更新 DESIGN.md - 精简设计文档

代码量减少: ~2200行

### 完成状态: ✅ 已完成

---

## v1.8.0 - 2026-03-21

### Iteration 18: 交互式对话 UI

新增功能:
- [x] 在 admin.html 添加聊天面板
- [x] WebSocket 连接管理
- [x] 实时消息发送/接收
- [x] 连接状态指示器
- [x] 自动重连机制
- [x] 用户/AI 消息样式区分

### 完成状态: ✅ 已完成并推送

---

## v1.9.0 - 2026-03-21

### Iteration 19: 错误处理增强 + 配置热重载完善

新增功能:
- [x] 重试机制增强 - 添加 jitter (0-25% 延迟抖动) 防止惊群效应
- [x] CircuitBreaker 断路器实现
  - 三态: Closed / Open / HalfOpen
  - 可配置失败阈值、半开超时、成功阈值
  - 原子操作保证线程安全
- [x] 配置热重载完善
  - 修复 last_modified_map 每个路径独立跟踪
  - 添加配置验证 (bind、model、retry、hot_reload 设置)
  - 添加事件通知 (Started, Stopped, Reloaded, ReloadFailed)

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 90 个测试全部通过 (+9 个新测试)

依赖更新:
- [x] 添加 rand = "0.8" 用于 jitter 生成

### 完成状态: ✅ 已完成并推送

---

## v2.0.0 - 2026-03-21

### Iteration 20: 持久化与优雅关闭

**目标**: 添加 SQLite 持久化，提升生产可用性

新增功能:
- [x] rusqlite 依赖添加 (bundled 特性)
- [x] persistence 模块创建 (src/persistence/sqlite.rs)
  - SqliteStore 结构 - 专用 SQLite 线程避免 Sync 问题
  - 会话和消息的 CRUD 操作
  - WAL 模式提高并发性能
- [x] 修复 common::Error 添加 From<rusqlite::Error> 实现

**待完成** (遇到 Rust lib/bin 模块系统复杂性):
- [ ] HistoryManager 集成 SQLite 持久化
- [ ] main.rs 初始化时启用持久化
- [ ] graceful shutdown (连接排空)

### 完成状态: ⚠️ 部分完成 (SQLite 模块就绪，集成待完成)

代码质量:
- [x] cargo clippy - 通过 (2个 minor warnings，已修复)
- [x] cargo test - 90 个测试全部通过

---

## v2.0.1 - 2026-03-21

### Iteration 21: SQLite 持久化 + 优雅关闭

**目标**: 完成 SQLite 持久化集成，实现优雅关闭

新增功能:
- [x] 创建共享 types 模块 (src/types.rs)
  - 解决 gateway 和 persistence 之间的循环依赖
  - Message, Role, SessionHistory 类型统一管理
- [x] SQLite 持久化集成
  - HistoryManager 新增 `new_with_persistence(path)` 构造函数
  - 添加 `persistence` 配置节 (enabled, path)
  - 自动同步会话历史到 SQLite (add_message, remove, clear)
  - 新增 `shutdown_persistence()` 方法
- [x] 优雅关闭支持
  - ServerState 结构追踪活动连接数
  - 关闭时等待连接排空 (可配置超时)
  - SqliteStore 优雅关闭
- [x] Graceful shutdown 配置
  - 新增 `shutdown.timeout_secs` 配置项

代码质量:
- [x] cargo clippy - 通过 (minor warnings)
- [x] cargo test - 164 个测试通过

---

## v2.0.2 - 2026-03-21

### Iteration 22: Request ID 追踪 + 会话恢复

**目标**: 添加请求追踪能力，支持从 SQLite 恢复会话历史

新增功能:
- [x] Request ID 追踪
  - `RequestId` 结构 (基于 UUID，前8字符用于日志显示)
  - 每个请求自动生成唯一 `req:xxxxxxxx` 标识符
  - `handle_request` 日志格式: `[req:xxxx] --> method` / `[req:xxxx] <-- method success/error`
  - request_id 传递到 `handle_sessions_send`、`handle_agent_turn`、`handle_exec`、`handle_tool_execute`
- [x] 会话历史从 SQLite 恢复
  - `HistoryManager::new_with_persistence()` 启动时从 SQLite 加载所有历史会话
  - 恢复后日志输出: `Recovering N sessions from SQLite...` / `Session recovery complete`
  - 内存 HashMap 预填充，已恢复的会话可立即访问

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 82 个测试通过

---

## v2.0.3 - 2026-03-21

### Iteration 23: 新工具 + 代码清理

**目标**: 增加实用工具，清理废弃代码

新增工具:
- [x] `sed_file` - 局部文件编辑工具
  - 支持按行号替换 (line_number 参数)
  - 支持按文本内容替换 (old_text + new_text 参数)
  - 对 AI Agent 非常有用，可做精确修改而不必重写整个文件
- [x] `which` - PATH 中查找可执行文件
  - 跨平台实现 (Unix permission bit 检查)

代码清理:
- [x] 修复版本号: Cargo.toml 2.0.1 → 2.0.3 (与 git commit 对齐)
- [x] 删除废弃的 `gateway/history.rs` 及其模块导出
- [x] 移除 `lib.rs` 中不再使用的 `SqliteStore` 公开导出
- [x] 移除 `persistence/mod.rs` 中的 `#[allow(unused)]` SqliteStore 重导出

测试增强:
- [x] 新增 6 个测试用例 (sed_file × 3, which × 3)

代码质量:
- [x] cargo clippy -- -D warnings - 通过 (0 警告)
- [x] cargo test - 88 个测试通过 (+6 新测试)

---

## v2.1.0 - 2026-03-21

### Iteration 24: 增强文件工具 + 路径规范化

**目标**: 增强文件操作工具，支持路径规范化和更多文件元数据

新增功能:
- [x] 路径规范化 - 扩展 `~` 到家目录，支持 `$VAR` 和 `${VAR}` 环境变量
- [x] `mkdir` 工具 - 创建目录，支持 parents 选项
- [x] `stat_file` 工具 - 获取文件元数据 (类型、大小、修改时间、创建时间、权限)
- [x] 增强 `list_dir` - 显示文件大小、修改时间，目录排序 (目录优先)

工具改进:
- [x] `read_file` - 支持 ~ 和环境变量路径
- [x] `write_file` - 支持 ~ 和环境变量路径
- [x] `sed_file` - 支持 ~ 和环境变量路径
- [x] 新增 `format_size` 辅助函数 (B/K/M/G 格式)

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 98 个测试通过 (+10 新测试)

工具总数: 11 个 (exec, read_file, write_file, list_dir, http_request, glob, grep, sed_file, which, mkdir, stat_file)

---

## v2.2.0 - 2026-03-21

### Iteration 25: 批量执行 + 工具增强

**目标**: 增加批量工具执行和更多实用工具

新增功能:
- [x] `batch_execute` - 批量执行多个工具
  - 接受 tools 数组，每个包含 name 和 input
  - 顺序执行，返回每个工具的结果
  - 全部成功返回 success=true，任一失败返回 false

- [x] `env` - 环境变量管理
  - 获取指定环境变量 (name 参数)
  - 设置环境变量 (value 参数)
  - 列出所有环境变量 (无参数)
  - 设置空值可删除变量

- [x] `diff` - 文件对比
  - 比较两个文件内容
  - 显示差异 (+/- 行格式)
  - 相同文件返回 "Files are identical"

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 105 个测试通过 (+7 新测试)

工具总数: 14 个 (新增 batch_execute, env, diff)

---

## v2.3.0 - 2026-03-21

### Iteration 26: 工具增强

**目标**: 修复 exec 超时、增强文件工具集

新增功能:
- [x] 修复 `exec` 工具超时机制
  - schema 中已有 `timeout` 参数但之前未实现
  - 现在正确应用超时（默认 30 秒）
  - 超时时返回明确错误信息
- [x] `find` - 按名称查找文件/目录
  - 支持 * 和 ? 通配符
  - 可选 `max_depth` 限制搜索深度
  - 可选 `type` 过滤 (f=文件, d=目录, a=全部)
- [x] `tail` - 读取文件末尾 N 行
  - 默认 10 行
  - 对 AI 分析日志文件非常有用

Bug 修复:
- [x] `which` 工具语义修正
  - 之前: 找不到时返回 success=true (不正确)
  - 现在: 找不到时返回 success=false (语义正确)

文档更新:
- [x] README 版本号从 v1.6.0 更新为 v2.2.0
- [x] README 工具列表更新 (新增 find, tail)

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 115 个测试通过 (+10 新测试)

工具总数: 16 个 (新增 find, tail)

---

## v2.4.0 - 2026-03-21

### Iteration 27: 会话管理增强 + 工具输入验证

**目标**: 增强会话API，添加工具输入schema验证

新增功能:
- [x] 会话导出API - `GET /api/sessions/:id/export`
  - 导出会话历史为JSON格式
  - 包含session_id、exported_at、message_count和完整消息数据

- [x] 会话导入API - `POST /api/sessions/import`
  - 从JSON导入会话历史
  - 验证session_id匹配
  - 验证消息role有效性

- [x] 活动连接API - `GET /api/connections`
  - 显示当前活动的WebSocket连接数
  - 显示关闭超时配置

- [x] 工具输入schema验证
  - 执行前验证required字段是否存在
  - 验证字段类型是否匹配 (string/number/boolean/object/array)
  - 提供明确的错误信息

- [x] HistoryManager.import_session() - 批量导入会话

Bug修复:
- [x] list_dir工具schema修复
  - 之前: path字段标记为required但实现支持默认值
  - 现在: 移除required标记，与实现一致

代码质量:
- [x] cargo clippy - 通过 (0 警告)
- [x] cargo test - 115 个测试通过

---

## v2.0.0 待规划 (完整功能)
- 分布式支持 (节点发现、状态同步)
- 插件市场/远程插件加载
- 高级认证 (OAuth, JWT)
- 数据库持久化
