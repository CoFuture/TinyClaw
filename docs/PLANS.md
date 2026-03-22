# TinyClaw 迭代计划 (PLANS)

> 本文件记录每轮迭代的完成事项与下一步规划
> 每次迭代前阅读以了解长期愿景与当前优先级

---

## 项目愿景

**TinyClaw** - **真正能帮你做事的 AI Agent**，本地运行的智能助手。

> "The AI that actually does things" —— 一个真正能帮你做事的 AI 助手。

### 核心理念（与 OpenClaw 一致）

| 理念 | 说明 |
|------|------|
| **AI as Teammate** | 不是聊天工具，而是能自主工作的"数字员工" |
| **数据主权** | 你的数据、上下文、记忆都保存在本地设备上 |
| **可 hackable** | 完全开源，像 20 年前用 Linux 的感觉，你可以掌控并定制 |

### 长期目标

1. **真正能做事** - 邮件、日历、智能家居、信息查询、文件处理等
2. **24/7 持续运行** - 定时任务、自动执行、主动提醒
3. **多渠道接入** - 支持多种协议接入（WebSocket、HTTP、CLI）
4. **稳定可靠** - 保持 Rust 代码精简、高质量、可维护

### 与 OpenClaw 的关系

TinyClaw 是 OpenClaw 的 **Rust 实现子集**，聚焦于：
- ✅ 核心 Agent Runtime（工具调用循环、上下文管理）
- ✅ WebSocket Gateway（JSON-RPC 协议）
- ✅ HTTP Server + WebUI/TUI
- ✅ 基础工具集（文件、代码、系统）
- ❌ 不做多渠道（20+ 消息平台）- 聚焦核心
- ❌ 不做语音/视觉 - 保持精简

---

## 优先级定义（面向"真正能做事"）

| 优先级 | 领域 | 说明 |
|--------|------|------|
| **P0** | **Agent 自主执行** | 自动规划+执行多步骤任务、任务队列、主动执行 |
| **P0** | **核心工具扩展** | 邮件、天气、信息查询、提醒、日程 |
| **P0** | **定时任务/自动化** | 定时执行、周期任务、主动提醒 |
| **P1** | **多 Session** | 并发会话管理、Session 隔离 |
| **P1** | **交互体验** | WebUI 完善、TUI、实时反馈 |
| **P2** | **稳定性** | 错误处理、重试机制、断路器 |

---

## 迭代历史

### v3.0.0 (已完成 ✅)

**完成事项**:
- **Skill 系统核心** - 全新 Skill 机制，支持将相关工具分组为可复用技能单元
  - `Skill` 结构：name, description, instructions, tool_names, tags
  - `SkillRegistry` - 全局技能注册表，内置 5 个常用技能 (file_ops, code_analysis, system_ops, web_search, diff_compare)
  - `SessionSkillManager` - 会话级技能管理，支持 per-session 启用/禁用
  - 技能指令自动注入系统提示词
  - 默认技能：file_ops 和 code_analysis 自动对新会话启用
- **HTTP API 扩展** - 完整的技能管理 REST API:
  - `GET /api/skills` - 列出所有技能
  - `GET /api/skills/{name}` - 获取单个技能
  - `POST /api/skills` - 创建技能
  - `PUT /api/skills/{name}` - 更新技能
  - `DELETE /api/skills/{name}` - 删除技能
  - `GET /api/sessions/{id}/skills` - 获取会话已启用技能
  - `POST /api/sessions/{id}/skills` - 设置会话技能
  - `PUT /api/sessions/{id}/skills/{name}` - 启用技能
  - `DELETE /api/sessions/{id}/skills/{name}` - 禁用技能
- **Skill Prompt 注入** - 技能指令在运行时自动注入到 Agent 系统提示词
  - `SessionSkillManager` 集成到 `HandlerContext`
  - `Agent::send_message` 支持技能提示词参数
  - Anthropic/OpenAI API 自动注入技能上下文
- **WebUI 技能管理面板** - admin.html 中展示和操作技能
  - 技能列表展示：名称、描述、工具、标签、指令
  - 创建新技能表单
  - 显示默认启用状态
- cargo clippy 0 警告
- cargo test 168 tests

**下一步**: 持久化技能配置、实时反馈

---

### v2.9.0 (已完成 ✅)

**完成事项**:
- WebUI 增强 - 工具面板：显示可用工具列表及输入 schema，可展开查看详细参数
- WebUI 增强 - 配置编辑器：在线编辑配置，支持保存和重载
- 移除 dead code: `with_full_recovery` 函数 (clippy warning fix)
- 修复测试冲突：hash/wc 测试使用 UUID 生成唯一临时文件名，避免并行测试冲突
- 新增 `/api/tools` HTTP 端点
- cargo clippy 0 警告
- cargo test 154 tests

**下一步**: Skill 机制、实时反馈

---

### v2.8.0 (已完成 ✅)

**完成事项**:
- 结构化错误代码体系 (13种错误类型)
- 错误恢复建议 (ErrorRecovery 结构)
- 智能错误映射
- cargo clippy 0 警告
- cargo test 154 tests

**下一步**: WebUI 增强

---

### v2.7.0 (已完成 ✅)

**完成事项**:
- 工具扩展: cp, mv, rm, cat

**下一步**: 错误处理增强

---

### v2.6.0 (已完成 ✅)

**完成事项**:
- TUI 模块 - 交互式终端界面
- 键盘导航支持
- `--tui` / `-t` 命令行标志

**下一步**: 工具扩展

---

### v2.5.0 (已完成 ✅)

**完成事项**:
- ContextManager 模块 - 上下文管理核心
- Token 估算与上下文截断策略

**下一步**: TUI 完善

---

### v2.4.0 (已完成 ✅)

**完成事项**:
- 会话导出/导入 API
- 活动连接 API
- 工具输入 schema 验证

**下一步**: 上下文管理

---

### v2.0.2 (已完成 ✅)

---

## 当前迭代规划 (v5.6.0)

### 本轮目标
**Agent 韧性增强 (Agent Resilience Enhancement)** - 集成断路器保护 AI API 调用

**计划完成**:
- [x] 断路器集成到 Agent 客户端
  - Agent 结构新增 `circuit_breaker: Arc<CircuitBreaker>`
  - 新增 `execute_protected()` 方法 - 包装 with_retry + 断路器检查
  - 所有 AI API 调用改用 `execute_protected()` (Anthropic/OpenAI/Ollama)
- [x] 指标系统增强
  - SystemMetrics 新增 `circuit_breaker_state` 字段
  - MetricsCollector 新增 `set_circuit_breaker_state()` 方法
  - `/api/metrics` 返回断路器状态
- [x] Gateway JSON-RPC 方法
  - 新增 `agent.circuit_breaker` 方法常量
  - 新增 `handle_agent_circuit_breaker` 处理器
- [x] TUI 支持
  - AppState 新增 `circuit_breaker_state` 字段
  - 新增 `get_circuit_breaker()` 网关客户端方法
  - 新增 `CircuitBreakerState` 事件变体
  - 标题栏显示 AI 熔断状态指示器 (🟢/🟡/🔴)
- [x] WebUI 支持
  - admin.html 新增"AI 熔断状态"显示
  - 根据状态显示不同颜色和图标
- [x] cargo clippy 0 警告
- [x] cargo test 176 tests

---

### v5.3.0 (已完成 ✅)

**完成事项**:
- **Session Turn Cancellation** - 取消正在进行的 Agent Turn
  - Agent 取消机制 (`turn_cancellations` HashMap)
  - Gateway `session.cancel` 方法
  - 事件系统增强 (`TurnCancelled` 事件)
  - TUI 支持 (`:cancel` / `:stop` 命令)
  - WebUI 支持 (取消按钮)
  - Ollama 流式取消支持

---

### v5.2.0 (已完成 ✅)

**完成事项**:
- **WebUI 会话管理增强** - 会话列表优化
  - API 新增字段：`durationSecs`、`lastMessagePreview`、`isActive`
  - 新增"消息"列显示消息数量
  - 新增"时长"列显示相对时间（秒/分钟/小时/天）
  - 新增活动状态指示器（绿色=活跃，灰色=空闲）
  - 支持点击列头排序（ID/标签/类型/消息数/时长/最后活跃）
  - 搜索支持最后消息预览
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: Session Turn Cancellation

---

### v5.1.0 (已完成 ✅)

**完成事项**:
- **TUI 输入历史导航** - Up/Down 箭头在输入面板中循环浏览历史消息
  - AppState 新增 input_history, input_history_index, input_history_saved
  - 辅助方法: add_to_input_history, input_history_up/down, is_navigating_history
  - Enter 发送前将当前缓冲区添加到历史
  - 键入/Backspace/Ctrl+C/Ctrl+D/Esc 取消历史导航
  - 显示历史位置提示: '↑↓ 3/10'
  - 每个会话历史限制 100 条
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: WebUI 会话管理增强

---

### v5.0.0 (已完成 ✅)

**完成事项**:
- **WebUI 聊天体验增强** - 消息复制、清空聊天、搜索功能
  - 新增 `.chat-toolbar` 布局：包含搜索框和清空按钮
  - 用户/AI 消息 hover 时显示"复制"按钮，点击后显示"已复制"反馈
  - 点击"清空"清除当前会话消息和搜索框
  - `onChatSearchInput()` 实时过滤，搜索结果计数显示
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: TUI 输入历史导航、WebUI 会话管理增强

---

### v4.9.0 (已完成 ✅)

**完成事项**:
- **Agent 执行状态可视化** - TUI 和 WebUI 的 thinking/tool_use 指示器
  - `TuiGatewayEvent::TurnStarted` 和 `TurnThinking` 事件
  - TUI 状态显示增强：thinking 状态跟踪
  - WebUI SSE 实时反馈：`turn.thinking` 和 `assistant.tool_use` 事件
  - CSS 动画：`.thinking-indicator`、`.tool-indicator`、`.thinking-dots`、`.dot-anim`
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: WebUI 聊天体验增强

---

### v4.8.0 (已完成 ✅)

**完成事项**:
- **Session Rename 支持** - 会话重命名功能
  - 新增 `session.rename` 网关方法，支持通过 WebSocket 重命名会话
  - 新增 `PATCH /api/sessions/{id}` HTTP 端点，支持通过 REST API 重命名会话
  - 新增 `SessionManager::rename` 方法，支持更新会话标签
- **TUI 会话重命名** - 命令行界面会话管理增强
  - 新增 `:ren` / `:rename` 命令进入重命名模式
  - 新增 `rename_mode` 状态，输入区域显示"Enter new session name..."
  - 按 Enter 确认新名称，Esc 取消重命名
  - 重命名后自动刷新会话列表
- **TUI 增强**
  - 帮助栏更新：显示 `:ren Rename` 和 `:c Reconnect` 命令
  - 命令解析优化：支持多字符命令（`:rc` reconnects，`:ren` renames）
- **HTTP API 增强**
  - `/api/sessions` 返回每条会话的消息数量 (`messageCount`)
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: WebUI 会话面板增强、更多会话管理功能

---

### v4.7.0 (已完成 ✅)

**完成事项**:
- Agent 流式响应支持 (Ollama)
  - 新增 `send_ollama_streaming` 方法：通过 SSE 逐块接收 AI 响应
  - 新增 `send_message_streaming` 方法：支持流式回调，Ollama 自动使用流式
  - 新增 `AssistantPartial` 事件：流式文本片段实时推送
  - Gateway `handle_agent_turn` 集成流式路径，实时发射 partial 事件
- WebUI 实时流式显示
  - SSE 事件流新增 `assistant.partial` 事件类型
  - 实时文本缓冲区 + 流式消息元素动态更新
  - 流式结束动画：`▊` 闪烁指示器，完成后自动消失
  - CSS 动画增强：streaming 消息透明度 + blink 动画
- SSE 事件过滤增强
  - `AssistantPartial` 加入 session filter 逻辑
  - `AssistantPartial` 加入 event name match
- 依赖更新：reqwest 添加 `stream` feature
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: Session Rename 支持

---

### v4.6.0 (已完成 ✅)

**完成事项**:
- TUI 命令帮助系统增强
  - 结构化命令元数据 (TuiCommandMeta)：名称、别名、描述、分类
  - 命令分类：Session (n/new, d/delete)、Connection (r/reconnect)、Navigation (q/quit, h/help)
  - Tab 补全支持所有命令别名
  - 帮助面板重构：按分类展示命令，带颜色高亮
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: Agent 流式响应、WebUI 实时反馈

---

### v4.5.0 (已完成 ✅)

**完成事项**:
- **WebUI 配置编辑器增强** - 显著提升配置编辑体验
  - JSON/YAML 格式切换：支持在编辑器和下拉菜单中选择格式，互相转换
  - 语法验证：新增"验证"按钮，实时检查 JSON/YAML 语法正确性
  - 视觉反馈：验证通过显示绿色边框，失败显示红色边框并显示错误详情
  - 一键重置：新增"重置"按钮，可快速恢复默认配置
  - 复制到剪贴板：新增"复制"按钮，便于一键复制配置内容
  - 格式切换提示：编辑器左上角显示当前格式和加载状态
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: Agent 能力提升、交互体验优化

---

## 待办事项池

### Agent 能力增强
- [x] 上下文管理机制 (Context struct + 压缩策略) ✅ v3.7.0

---

## 待办事项池

### Agent 自主执行能力（真正能做事的关键！）
- [x] **任务抽象与执行** ✅ v5.9.0
- [x] **后台任务队列** ✅ v5.9.0
- [x] **定时任务系统** ✅ v6.0.0
- [x] **Agent 主动建议** ✅ v6.2.0
- [x] 上下文管理机制 ✅ v3.7.0
- [x] Skill 机制 ✅ v3.0.0
- [x] 断路器保护 ✅ v5.6.0
- [x] 结构化错误报告 + 自我修正 ✅ v5.7.0

### 核心工具（参考 OpenClaw Skill，依赖外部 API）
> 工具不求多，够用即可。关键是通过 Skill 组合实现复杂能力。

**已有工具（够用）**：
- [x] 文件操作 (read/write/cp/mv/rm/cat/grep/glob/find) ✅
- [x] 代码分析 ✅
- [x] 系统命令执行 ✅
- [x] HTTP 请求 ✅

**可选扩展（按需）**：
- [ ] **天气查询** - 通过 Skill + 外部 API 实现
- [ ] **信息搜索** - 通过 Skill + 外部 API 实现
- [ ] **提醒** - 通过 Skill + 定时任务实现

### 定时任务/自动化
- [x] **定时任务系统** - 支持 cron 风格定时执行 ✅ v6.0.0
- [ ] **主动提醒** - 基于条件的主动通知
- [ ] **周期性检查** - 定期检查并报告

### 交互体验
- [x] WebUI 技能管理面板 ✅ v3.0.0
- [x] Terminal UI (TUI) ✅ v2.6.0
- [x] 实时反馈 (SSE 事件流) ✅ v3.1.0
- [x] TUI Gateway 集成 ✅ v3.2.0
- [x] TUI 多 Session 支持 ✅ v3.3.0
- [x] TUI 会话历史加载 ✅ v3.4.0
- [x] TUI 会话删除功能 ✅ v3.4.0
- [x] WebUI Chat 多会话切换 ✅ v3.5.0
- [x] TUI 消息历史持久化 ✅ v3.6.0
- [x] 命令行客户端 ✅ v3.8.0
- [x] WebUI 聊天 Markdown + 代码高亮 ✅ v4.2.0
- [x] TUI Tab 命令补全 ✅ v4.3.0
- [x] TUI 视觉优化 ✅ v4.4.0
- [x] TUI 命令帮助系统增强 ✅ v4.6.0
- [x] WebUI 聊天消息复制/清空/搜索 ✅ v5.0.0
- [x] TUI 输入历史导航 ✅ v5.1.0
- [x] Session Turn Cancellation ✅ v5.3.0
- [x] Session Rename ✅ v4.8.0
- [x] Agent 主动建议 ✅ v6.2.0

### 稳定性
- [x] 日志优化 (结构化日志) ✅ v4.1.0
- [x] 监控指标 ✅ v3.9.0
- [x] 错误处理增强 ✅ v5.7.0

---

## 历史迭代

| 版本 | 完成事项 |
|------|----------|
| v5.8.0 | Agent Turn Execution Log - 新增 `agent/turn_log.rs`：`TurnAction`/`TurnLogEntry`/`TurnLog`/`TurnLogSummary` 结构；新增 `Event::TurnLogUpdated` 和 `Event::TurnLogCompleted` SSE 事件；Runtime 集成：每个 Turn 创建 TurnLog，工具执行时记录（名称/输入/输出/成功/耗时），Turn 结束时发送 TurnLogCompleted；3个新测试；cargo clippy 0 警告；cargo test 191 tests |
| v5.7.0 | Structured Tool Error Reporting + Agent Self-Correction - 新增 `agent/error_recovery.rs`：ToolErrorKind 枚举(9种错误类型)、ErrorRecovery 结构(包含 retryable 和 suggestion)；集成到 runtime.rs 和 client.rs，工具失败时返回结构化错误报告帮助 Agent 自我修正；12个新测试；cargo clippy 0 警告；cargo test 188 tests |
| v5.6.0 | Agent 韧性增强 - 集成断路器保护 AI API：execute_protected() 包装 retry + 断路器；指标系统增强 (circuit_breaker_state)；Gateway JSON-RPC 方法；TUI 标题栏熔断指示器 (🟢/🟡/🔴)；WebUI 熔断状态显示；cargo clippy 0 警告；cargo test 176 tests |
| v5.5.0 | TUI 视觉增强 - 错误显示红色样式（标题、边框、消息）；标题栏思考状态指示器 "⚙ Thinking..." (黄色+粗体)；cargo clippy 0 警告；cargo test 176 tests |
| v5.4.0 | TUI Bug Fix - 修复 unreachable pattern bug：`:rc` 命令因重复 `KeyCode::Char('c')` 无法到达；合并 `:rc` 检查到单个 'c' 处理；移除 dead `is_turn_active` 方法；cargo clippy 0 警告；cargo test 176 tests |
| v5.3.0 | Session Turn Cancellation - Agent 取消机制：turn_cancellations HashMap、start_turn_cancellation/cancel_turn/is_turn_active 方法；Gateway session.cancel 方法 + 处理器；TurnCancelled 事件；TUI :cancel/:stop 命令；WebUI 取消按钮(思考中显示)；Ollama 流式取消(send_ollama_streaming 支持取消检查)；cargo clippy 0 警告；cargo test 176 tests |
| v5.2.0 | WebUI 会话管理增强 - API新增 durationSecs/lastMessagePreview/isActive 字段；新增"消息数列"显示消息数量；新增"时长"列显示相对时间(秒/分钟/小时/天)；新增活动状态指示器(绿=活跃/灰=空闲)；支持点击列头排序(ID/标签/类型/消息数/时长/最后活跃)；搜索支持最后消息预览；cargo clippy 0 警告；cargo test 176 tests |
| v5.1.0 | TUI 输入历史导航 - Up/Down 箭头在输入面板中循环浏览历史消息；AppState 新增 input_history/input_history_index/input_history_saved；辅助方法: add_to_input_history/input_history_up/down/is_navigating_history；Enter发送前添加历史；键入/Backspace/Ctrl+C/Ctrl+D/Esc 取消导航；历史位置提示 '↑↓ 3/10'；每个会话历史限制 100 条；cargo clippy 0 警告；cargo test 176 tests |
| v5.0.0 | WebUI 聊天体验增强 - 消息复制按钮(hover显示)、清空聊天按钮、实时搜索过滤(显示匹配计数)；CSS 增强：copy按钮动画、hidden-by-search隐藏、search-count显示；cargo clippy 0 警告；cargo test 176 tests |
| v4.9.0 | Agent 执行状态可视化 - TUI 新增 TurnStarted/TurnThinking 事件处理、WebUI SSE 新增 thinking/tool_use 指示器、CSS 动画增强；cargo clippy 0 警告；cargo test 176 tests |
| v4.8.0 | Session Rename 支持 - session.rename 网关方法、PATCH /api/sessions/{id} HTTP 端点、SessionManager::rename 方法；TUI :ren/:rename 命令进入重命名模式、rename_mode 状态管理、Enter 确认/Esc 取消；帮助栏更新显示新命令；/api/sessions 返回 messageCount；cargo clippy 0 警告；cargo test 176 tests |
| v4.7.0 | Agent 流式响应支持 (Ollama) - send_ollama_streaming/send_message_streaming 方法、AssistantPartial 事件；WebUI 实时流式显示 - SSE assistant.partial、流式缓冲区、▊ 闪烁指示器；SSE 事件过滤增强；cargo clippy 0 警告；cargo test 176 tests |
| v4.6.0 | TUI 命令帮助系统增强 - 结构化命令元数据(TuiCommandMeta)：名称、别名、描述、分类；命令分类：Session/Connection/Navigation；Tab补全支持所有命令别名；帮助面板重构：按分类展示命令、颜色高亮；cargo clippy 0 警告；cargo test 176 tests |
| v4.5.0 | WebUI 配置编辑器增强 - JSON/YAML 格式切换、语法验证（绿/红边框反馈）、一键重置默认配置、复制到剪贴板；cargo clippy 0 警告；cargo test 176 tests |
| v4.4.0 | TUI 视觉优化 - 消息面板增强：角色标签颜色区分 (User=绿, Assistant=青, System=黄, Tool=紫)、每条消息显示时间戳 (HH:MM:SS)、多行内容正确缩进；输入区域优化：命令模式标题变化 (:前缀显示为"Command")、字符计数显示；标题栏增强：连接状态和当前 session 名称使用不同颜色区分；cargo clippy 0 警告；cargo test 176 tests |
| v4.3.0 | YAML 配置支持 + TUI Tab 命令补全 - 添加 serde_yaml 依赖，配置文件支持 .yaml/.yml 格式自动检测和加载；TUI 新增 Tab 键命令补全功能，支持 :q/:quit, :r/:reconnect, :n/:new, :d/:delete, :h/:help/:? 等命令补全；Shift+Tab 反向循环；输入普通文本时支持 Session ID 补全；cargo clippy 0 警告；cargo test 176 tests |
| v4.2.0 | WebUI 聊天增强 (Markdown + 代码高亮) - 集成 marked.js 和 highlight.js，支持 Markdown 渲染（标题、列表、代码块、表格、链接等）；用户消息和 AI 回复均支持 Markdown 格式；代码块自动语法高亮；会话详情面板同样支持 Markdown 渲染；cargo clippy 0 警告；cargo test 173 tests |
| v4.1.0 | WebUI 监控面板增强 + 结构化日志 - WebUI 监控面板集成 `/api/metrics` 端点，新增请求速率、响应时间、错误数、WS 连接数等统计；新增图表标签页（请求/响应/错误）；端点统计表格展示 Top 10 API 端点；Agent/Gateway 日志改为 structured fields 格式；cargo clippy 0 警告；cargo test 173 tests |
| v4.0.0 | Skill 持久化 - 自定义技能自动保存到 JSON 文件，重启后自动加载；内置技能保护（无法删除/覆盖）；移除无用的 metrics_middleware dead code；cargo clippy 0 警告；cargo test 173 tests |
| v3.9.0 | 监控指标增强 - HTTP Metrics 中间件、请求 timing 采集、clippy warning 修复 |
| v3.8.0 | 交互体验优化 - HTTP API 会话创建、WebUI 会话管理面板增强、交互式 CLI 聊天客户端 |
| v3.7.0 | Agent 上下文管理修复 - send_message_with_history 实现、历史上下文传递到 AI API、Tool 消息编码修复 |
| v3.6.0 | TUI 消息历史持久化 - SQLite 本地存储、会话恢复、优雅降级 |
| v3.5.0 | WebUI Chat 多会话切换 - 会话下拉选择、sessionKey 传递、历史加载、SSE 实时更新 |
| v3.4.0 | TUI 会话历史加载 + 会话删除 - 自动加载历史、:d 删除会话、防止删除 main |
| v3.3.0 | TUI 多 Session 支持 - agent.spawn 处理器、会话列表同步、:n 新建会话 |
| v3.2.0 | TUI Gateway 集成 - WebSocket 客户端、实时对话、连接状态显示 |
| v3.1.0 | 实时反馈系统 (SSE) + 新事件类型 + WebUI 实时事件面板 |
| v3.0.0 | Skill 系统核心 + 技能注入 + WebUI 技能面板 |
| v2.9.0 | WebUI 增强 (工具面板、配置编辑器)、测试冲突修复 |
| v2.8.0 | 错误处理增强 (13种错误代码、ErrorRecovery) |
| v2.7.0 | 工具扩展 (cp, mv, rm, cat) |
| v2.6.0 | TUI 终端界面 |
| v2.5.0 | 上下文管理 (ContextManager) |
| v2.4.0 | 会话导入/导出 API |
| v2.0.2 | Request ID 追踪、SQLite 会话恢复 |

---

## 当前迭代规划 (v6.3.0)

### 本轮目标
**Agent 能力增强 + 交互体验优化**

**计划完成**:
- [ ] **会话上下文持久化** - 跨会话记住用户偏好
- [ ] **TUI 增强** - 交互式 TUI 完善

**下一步**: 邮件/日历集成、多模态支持

---

## 迭代历史

### v6.2.0 (已完成 ✅)

**完成事项**:
- **Agent 主动建议 (Proactive Suggestions)** - 基于上下文主动给用户建议
  - 新增 `agent/suggestion.rs`：`Suggestion`、`SuggestionType`、`SuggestionEngine` 结构
  - 支持 5 种建议类型：FollowUp、Action、Information、Reminder、Task
  - 关键词检测：file、error、bug、code、task、meeting 等
  - 会话级建议引擎状态管理（跨 turn 保持上下文）
  - 基于置信度排序，最多返回 3 条建议
- **Gateway 事件集成**
  - 新增 `suggestion.generated` SSE 事件类型
  - HandlerContext 集成 suggestion_engines 状态
  - 在 `handle_agent_turn` 结束后自动生成并推送建议
- **HTTP SSE 事件过滤**
  - 更新 `http/routes.rs` 支持 `SuggestionGenerated` 事件
  - 支持按 session_id 过滤建议事件
- **代码质量**
  - cargo clippy 0 警告
  - cargo test 237 tests (新增 11 个建议引擎测试)

**下一步**: 会话上下文持久化、TUI 增强

---

### v6.1.0 (已完成 ✅)

**完成事项**:
- **HTTP REST API 定时任务管理** - 完整的 CRUD API
  - `GET /api/scheduled` - 列出所有定时任务
  - `POST /api/scheduled` - 创建定时任务（支持 cron 和 interval）
  - `GET /api/scheduled/{id}` - 获取单个任务
  - `POST /api/scheduled/{id}/pause` - 暂停任务
  - `POST /api/scheduled/{id}/resume` - 恢复任务
  - `POST /api/scheduled/{id}/enable` - 启用任务
  - `POST /api/scheduled/{id}/disable` - 禁用任务
  - `POST /api/scheduled/{id}/fire` - 立即触发任务
  - `DELETE /api/scheduled/{id}` - 删除任务
- **WebUI 定时任务面板** - admin.html 中完整的任务管理界面
  - 任务列表展示：名称、状态、类型、下次执行时间、运行次数
  - 状态徽章：运行中/已暂停/已禁用、Cron/间隔
  - 操作按钮：暂停/恢复、启用/禁用、立即执行、删除
  - 创建任务弹窗：支持 cron 和间隔两种类型
  - 实时刷新：与数据面板同步刷新
- **HttpState 增强** - 集成 Scheduler 到 HTTP 状态
- **代码质量修复** - 修复 clippy redundant closure 警告
- 226 tests passing

**未完成/待续**:
- 主动提醒系统（需要推送渠道集成）
- 任务执行结果通知（需要更完善的通知机制）

### v6.0.0 (已完成 ✅)

**完成事项**:
- **定时任务触发系统** - 让 Agent 能够定时自动执行任务
  - 新增 `agent/scheduled_task.rs`：`ScheduledTask`、`ScheduleType`、`ScheduledTaskSummary` 结构
  - 支持两种调度类型：Cron 表达式（6字段格式）和固定间隔（秒）
  - `ScheduledTask` 方法：创建、暂停、恢复、启用、禁用、删除
- **Scheduler 调度器**
  - 新增 `agent/scheduler.rs`：`Scheduler` 结构管理所有定时任务
  - 后台轮询循环，每秒检查是否有任务到期
  - 到期时自动创建并执行后台任务（使用 TaskManager）
- **Gateway JSON-RPC 方法**
  - 新增：`scheduled.create`、`scheduled.list`、`scheduled.get`
  - 新增：`scheduled.pause`、`scheduled.resume`、`scheduled.delete`
  - 新增：`scheduled.enable`、`scheduled.disable`、`scheduled.fire_now`
- **事件系统增强**
  - 新增事件：`scheduled.created`、`scheduled.fired`、`scheduled.failed`、`scheduled.updated`、`scheduled.deleted`
  - SSE 事件过滤和映射更新
- **HandlerContext 集成**
  - Scheduler 集成到 HandlerContext
  - WebSocket 和 HTTP 服务器初始化更新
- 完整测试覆盖：226 tests
- cargo clippy 0 警告（仅 dead_code 警告）

**下一步**: 主动提醒 + 任务组合执行

---

### v5.9.0 (已完成 ✅)

**完成事项**:
- **Agent 自主任务执行能力** - 后台任务系统基础设施
  - 新增 `agent/task.rs`：`Task`、`TaskState`、`TaskStep`、`TaskSummary` 结构
  - `TaskState` 枚举：Pending/Running/Completed/Failed/Cancelled
  - `TaskStep` 结构：步骤描述、结果、成功状态、时间戳
  - `TaskSummary`：用于列表展示的任务摘要
- **TaskManager 后台任务管理**
  - 新增 `agent/task_manager.rs`
  - 任务创建、列表、查询、启动、取消、删除
  - 异步执行支持，任务状态追踪
  - 任务计数统计
- **Gateway API 扩展**
  - 新增 JSON-RPC 方法：`task.create`、`task.list`、`task.get`、`task.start`、`task.cancel`、`task.remove`
  - 协议常量定义在 `protocol.rs`
- **事件系统增强**
  - 新增事件类型：`task.created`、`task.started`、`task.progress`、`task.completed`、`task.failed`、`task.cancelled`
  - SSE 事件过滤和名称映射更新
- **HandlerContext 集成**
  - TaskManager 集成到 HandlerContext
  - 主程序和 WebSocket 服务器初始化更新
- **完整测试覆盖**
  - 16 个新测试（Task 和 TaskManager）
  - 总测试数：207 tests
- cargo clippy 0 警告

**下一步**: 任务步骤解析、步骤执行追踪

---

## 迭代历史

### v5.7.0 (已完成 ✅)

**完成事项**:
- **Structured Tool Error Reporting** - 工具执行失败时返回结构化错误报告
  - 新增 `agent/error_recovery.rs`
  - `ToolErrorKind` 枚举：9种错误类型 (NotFound, PermissionDenied, InvalidArgument, etc.)
  - `ErrorRecovery` 结构：包含 kind, retryable, suggestion, message
  - 集成到 `runtime.rs` 和 `client.rs`：工具失败时返回结构化报告
  - 帮助 Agent 理解错误原因并自我修正
- **cargo clippy 0 警告**
- **cargo test 188 tests** (新增 12 个错误恢复测试)

**下一步**: Agent Turn 执行日志、WebUI/TUI 状态可视化

---

### v5.6.0 (已完成 ✅)

**完成事项**:
- **Agent 韧性增强** - 集成断路器保护 AI API 调用
  - Agent 结构新增 `circuit_breaker: Arc<CircuitBreaker>`
  - 新增 `execute_protected()` 方法 - 包装 with_retry + 断路器检查
  - 所有 AI API 调用改用 `execute_protected()` (Anthropic/OpenAI/Ollama)
- **指标系统增强**
  - SystemMetrics 新增 `circuit_breaker_state` 字段
  - MetricsCollector 新增 `set_circuit_breaker_state()` 方法
  - `/api/metrics` 返回断路器状态
- **Gateway JSON-RPC 方法**
  - 新增 `agent.circuit_breaker` 方法常量
  - 新增 `handle_agent_circuit_breaker` 处理器
- **TUI 支持**
  - AppState 新增 `circuit_breaker_state` 字段
  - 新增 `get_circuit_breaker()` 网关客户端方法
  - 新增 `CircuitBreakerState` 事件变体
  - 标题栏显示 AI 熔断状态指示器 (🟢/🟡/🔴)
- **WebUI 支持**
  - admin.html 新增"AI 熔断状态"显示
  - 根据状态显示不同颜色和图标
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: Structured Tool Error Reporting、Agent Self-Correction

---

### v5.5.0 (已完成 ✅)

**完成事项**:
- **TUI 错误显示增强** - 增强 TUI 错误提示视觉效果
  - 错误标题使用红色 + 粗体样式
  - 错误边框使用红色
  - 错误消息使用浅红色显示
  - 提示文字使用深灰色
- **TUI 思考状态指示器** - 在标题栏显示思考状态
  - 当 agent 正在思考时显示 "⚙ Thinking..." (黄色 + 粗体)
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: WebUI 增强、Agent 错误处理

---

### v5.4.0 (已完成 ✅)

**完成事项**:
- **TUI Bug Fix** - 修复 TUI unreachable pattern bug
  - 之前的 `KeyCode::Char('c')` handler（`:rc` reconnect）因重复匹配永远无法到达
  - 将 `:rc` 检查合并到单个 'c' 处理逻辑中，现在正确处理 `:c` + `:rc` 序列
- **Code Cleanup** - 移除 dead code `is_turn_active` 方法（未被使用）
- cargo clippy 0 警告
- cargo test 176 tests

**下一步**: TUI 错误显示增强、思考状态指示器

---

### v5.3.0 (已完成 ✅)
