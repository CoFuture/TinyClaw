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

## 迭代计划

### v1.4.0 待实现
- WebSocket 消息队列优化
- 分布式支持 (节点发现、状态同步)
- 插件市场/远程插件加载
- 高级认证 (OAuth, JWT)
