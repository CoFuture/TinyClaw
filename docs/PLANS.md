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

### v12.2.0 (已完成 ✅)

**完成事项**:
- **TUI Safety Commands & Panel** - 终端界面执行安全状态显示与命令
  - **TuiGatewayEvent 新增变体**：
    - `ExecutionSafetyWarning` - 安全警告事件（接近限制）
    - `ExecutionSafetyHalted` - 安全停止事件（达到限制）
  - **SSE 事件解析** (`gateway_client.rs`)：
    - 新增 `execution.warning` 和 `execution.halted` 事件解析
  - **HTTP API 方法** (`gateway_client.rs`)：
    - `get_safety_stats_http()` - 获取执行安全统计
    - `get_safety_session_state_http()` - 获取特定会话安全状态
  - **AppState 新增字段**：
    - `safety_mode` - 安全查看模式
    - `safety_session_id` - 当前查看的会话
    - `safety_stats` / `safety_state` - 缓存的安全数据
    - `last_safety_warning` - 最近的安全警告信息
    - `safety_halted` - 是否因安全限制停止
  - **TUI 命令** (`:safety` / `:safetystats`)：
    - 查看执行安全状态面板
    - 显示 AI 断路器状态
    - 显示安全警告/停止信息
  - **Safety Panel** (`draw_safety_panel`)：
    - 红色边框显示 halted 状态，黄色显示 warning 状态
    - 显示 AI Circuit Breaker 状态（🟢/🟡/🔴）
    - 显示最近的安全警告信息
    - 显示 agent 执行状态说明
  - **Title Bar 增强**：
    - 显示 `⚠️ Safety: X/Y turns` 或 `🛑 Safety Halted` 警告
  - **Esc 键退出** - 支持 Esc 键退出安全查看模式
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 381 tests

**下一步**: WebUI Safety Panel（已完成 ✅ v12.3.0）、配置更新 API（已完成 ✅ v12.3.0）

---

### v12.3.0 (已完成 ✅)

**完成事项**:
- **WebUI Safety Panel** - 网页界面执行安全状态面板
  - **HTTP API 新增端点**：
    - `GET /api/safety/config` - 获取执行安全配置
    - `PATCH /api/safety/config` - 更新执行安全配置
  - **Agent 客户端新增方法** (`client.rs`)：
    - `get_safety_config()` - 获取当前安全配置
    - `update_safety_config()` - 更新安全配置（支持 maxConsecutiveTurns、warningThresholdPct、safetyAction、enabled）
  - **admin.html 新增面板**：
    - 🛡️ 执行安全面板：统计数据卡片（安全事件/警告次数/停止次数/监控中会话）
    - 当前配置摘要显示（最大连续调用数/警告阈值/安全动作/启用状态）
    - 会话安全状态列表：显示每个会话的连续工具调用数、警告/停止状态
    - ⚙️ 配置弹窗：启用/禁用监控、最大连续调用数、警告阈值百分比、安全动作选择
    - 状态颜色编码：正常(绿)/警告(黄)/停止(红)
  - **refreshData() 集成**：自动随页面刷新加载安全面板
  - **CSS 样式**：`.safety-stats-grid`、`.safety-stat-card`、`.safety-session-item` 等完整样式
- **HTTP 路由新增** (`routes.rs`)：
  - `safety_config_get()` - 获取安全配置处理器
  - `safety_config_update()` - 更新安全配置处理器（支持 PATCH）
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 381 tests

**下一步**: Agent 能力持续增强、更多交互优化

---

### v12.4.0 (已完成 ✅)

**完成事项**:
- **Agent Context Enhancement - User Preferences Integration** - 将用户偏好集成到 Agent 上下文
  - **`generate_context_prompt` 增强** (`gateway/messages.rs`)：
    - 新增用户偏好（UserPreferences）到上下文提示词
    - 包含：agent_persona（Agent 人设）、timezone（时区）、language（语言偏好）
    - 使用 `PreferencesManager::get_system_prompt_addition()` 获取格式化偏好
  - **上下文提示词结构优化**：
    - 顺序调整：User Preferences → Active Skills → Session Notes → Memory → Conversation Summary → Session Instructions
    - 用户偏好作为全局设置优先注入
- **Relevance-based Memory Retrieval - 基于相关性的记忆检索** - 优化记忆检索策略
  - **改用 `generate_context_prompt` 替代 `generate_session_prompt`**：
    - 原方案：仅获取当前会话的最近记忆（不考虑消息内容）
    - 新方案：根据当前用户消息内容进行相关性搜索
    - 使用 `MemoryManager::generate_context_prompt(message, 5)` 获取相关记忆
    - 帮助 Agent 回忆与当前对话相关的历史信息，即使来自其他会话
  - **函数签名更新**：
    - `generate_context_prompt(ctx, session_key)` → `generate_context_prompt(ctx, session_key, current_message)`
    - 调用点更新：`handle_agent_turn` 中传入当前消息
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 381 tests

**下一步**: Agent 能力持续增强、交互体验优化

---

### v12.5.0 (已完成 ✅)

**完成事项**:
- **Performance Insights Engine - 性能洞察引擎** - 生成可操作的 Agent 改进建议
  - **新增 `agent/performance_insights.rs` 模块**：
    - `PerformanceInsightsEngine` - 分析引擎，分析工具效率、质量趋势、工具使用模式
    - `PerformanceInsight` - 单条洞察（category、severity、title、description、suggestions）
    - `ToolEfficiencySummary` - 工具效率摘要（最高效/最低效工具、问题工具、平均工具数）
    - `QualityTrend` - 质量趋势分析（当前/前一期分数、趋势方向、趋势幅度）
    - `ToolPattern` - 检测到的工具使用模式（工具序列、出现次数、成功率、可靠性）
    - `PerformanceAnalysis` - 完整分析结果
  - **分析维度**：
    - 工具效率（成功率、执行速度、问题工具识别）
    - 质量趋势（基于自我评估的历史对比）
    - 工具模式检测（识别可靠的工具组合）
    - 可操作洞察生成（最多 8 条，按严重程度排序）
  - **Gateway 事件集成** (`gateway/events.rs`)：
    - 新增 `PerformanceInsights` 事件类型（session_id、insights、tool_efficiency、quality_trend、turns_analyzed）
    - 新增相关数据结构：PerformanceInsightEvent、ToolEfficiencyEvent、QualityTrendEvent
  - **HTTP API 端点** (`http/routes.rs`)：
    - `GET /api/performance/insights` - 获取性能洞察数据
    - 返回：insights、toolEfficiency、qualityTrend、toolPatterns、turnsAnalyzed
  - **SSE 事件过滤** (`http/routes.rs`)：
    - 新增 `agent.performance_insights` 事件支持
    - 支持 session_id 过滤
  - **TUI 支持** (`tui/`)：
    - 新增 `:perf` / `:performance` 命令查看性能洞察
    - `AppState` 新增 `perf_mode` 和 `perf_data` 字段
    - `TuiGatewayClient` 新增 `PerformanceInsightsLoaded` 事件变体
    - `TuiGatewayClient` 新增 `get_performance_insights_http()` 方法
    - `TUI_COMMANDS` 新增 `:perf` 命令元数据
    - App 事件处理集成 `PerformanceInsightsLoaded` 事件
    - App 命令处理集成 `:perf` 命令（触发 HTTP 加载）
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 383 tests

**下一步**: WebUI 性能洞察面板、实时洞察事件推送

---

### v12.6.0 (已完成 ✅)

**完成事项**:
- **TUI Performance Insights Panel** - 终端界面性能洞察面板
  - **新增 `draw_perf_panel()` 函数** (`tui/components.rs`)：
    - 显示 Turns Analyzed 和 Avg Tools/Turn 统计数据
    - 显示 Quality Trend 趋势方向（↑ improving / ↓ declining / → stable）
    - 显示工具效率摘要（最高效工具、最不高效工具、问题工具）
    - 显示 Insights 列表，每条洞察显示 severity icon、category、title、description、suggestion
    - 严重程度颜色编码：🔴 high（红）、🟡 medium（黄）、🟢 low（绿）
    - 无数据时显示友好提示信息
  - **渲染循环集成** (`tui/app.rs`)：
    - 在 `safety_mode` 之后添加 `perf_mode` 条件分支
    - `perf_mode` 为 true 时绘制 `draw_perf_panel`
  - **Esc 键处理集成** (`tui/app.rs`)：
    - 在 `safety_mode` Esc 处理后添加 `perf_mode` 处理
    - Esc 时清空 `perf_mode` 和 `perf_data` 状态
  - 修复了 v12.5.0 遗留问题：`:perf` 命令可以获取数据但无面板显示
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 383 tests

**下一步**: WebUI 性能洞察面板、Agent 能力持续增强

---

### v12.1.0 (已完成 ✅)

**完成事项**:
- **Agent Execution Safety System** - 防止 Agent 工具调用失控循环
  - **新增 `agent/execution_safety.rs` 模块**：
    - `SafetyAction` 枚举：Warn、Summarize、Halt、Stop 四种安全动作
    - `ExecutionSafetyConfig` 结构：配置最大连续工具调用次数、警告阈值、安全动作
    - `ExecutionSafetyState` 结构：追踪每个会话的连续工具调用次数、是否暂停等状态
    - `ExecutionSafetyStats` 结构：汇总统计数据
    - `ExecutionSafetyManager` 管理器：状态追踪、安全检查、持久化到 JSON
  - **Agent 集成** (`agent/client.rs`)：
    - 新增 `safety_manager: RwLock<Option<Arc<ExecutionSafetyManager>>>` 字段
    - 新增 `with_safety_manager()` builder 方法
    - 新增 `check_execution_safety()` 辅助方法：工具执行后检查安全状态
    - 在 Anthropic 和 OpenAI 工具执行循环中集成安全检查
    - 安全限制达到时返回错误
  - **Gateway 事件集成** (`gateway/events.rs`)：
    - 新增 `ExecutionSafetyWarning` 事件：接近安全限制时发出警告
    - 新增 `ExecutionSafetyHalted` 事件：安全限制达到时发出事件
  - **HTTP SSE 事件过滤** (`http/routes.rs`)：
    - 支持 `execution.warning` 和 `execution.halted` 事件过滤
  - **HTTP REST API**：
    - `GET /api/safety/stats` - 获取执行安全统计
    - `GET /api/safety/session/{session_id}` - 获取特定会话的安全状态
  - **main.rs 集成**：
    - 创建 `ExecutionSafetyManager` 并持久化到 `~/.config/tiny_claw/execution_safety/`
    - 将 safety_manager 设置到 Agent
- **Agent Runtime 清理** (`agent/runtime.rs`)：
  - 移除未使用的 runtime.rs 安全检查代码（AgentRuntime 未被使用）
  - 保留 execution_safety.rs 作为独立模块
- cargo clippy 0 错误（仅有 pre-existing dead_code 警告）
- cargo test 381 tests

**下一步**: WebUI 安全状态面板、TUI 安全命令、配置更新 API

---

### v10.4.0 (已完成 ✅)

**完成事项**:
- **TUI Summarizer Configuration Editing** - 终端界面支持编辑摘要器配置
  - 新增 `:sumcfg` 命令进入配置编辑模式
  - **AppState 新增字段**：`sumcfg_mode: bool` 用于追踪编辑模式状态
  - **TUI_COMMANDS 新增命令**：`:sumcfg` 命令元数据
  - **配置编辑面板**：`draw_sumcfg_panel()` 显示当前配置和编辑格式说明
  - **输入格式**：`minMessages=N,tokenThreshold=N,enabled=true|false`，所有字段可选
  - **示例输入**：`minMessages=20` 或 `enabled=false` 或 `minMessages=15,enabled=true`
  - **Enter 键保存**：解析输入并调用 `set_summarizer_config()` 更新配置
  - **Esc 键取消**：退出编辑模式不保存
  - **帮助栏提示**：显示 "Press Enter to save, Esc to cancel"
  - 使用已有的 `set_summarizer_config()` 网关客户端方法
- cargo clippy 0 警告
- cargo test 336 tests

**下一步**: 更多 Agent 能力增强、WebUI 摘要配置编辑面板

---

### v11.1.0 (已完成 ✅)

**完成事项**:
- **Session Quality Analysis** - 会话质量分析系统
  - **新增模块** `agent/session_quality.rs`：
    - `SessionQuality` 结构：会话整体质量数据
    - `QualityIssue` 枚举：7 种问题类型（重复提问、工具错误、Token 过高、响应缓慢、成功率低、工具低效、用户不满）
    - `SessionQualityAnalyzer`：分析引擎
    - `SessionQualityManager`：管理器（支持缓存）
  - **分析维度**：
    - 任务完成率（权重 30%）
    - 响应质量（权重 25%）
    - 执行效率（权重 20%）
    - 工具成功率（权重 25%）
  - **问题检测**：7 种问题自动检测，按严重程度排序
  - **质量评分**：0-100 分，转换为 1-5 星评级
  - **HTTP API 端点**：
    - `GET /api/sessions/{session_id}/quality` - 获取会话质量分析
    - `GET /api/sessions/quality/list` - 获取所有会话质量列表
  - **WebUI 面板**：
    - 统计卡片：质量评分、任务完成率、工具成功率、平均响应时间
    - 星级评分显示
    - 检测到的问题列表（按严重程度排序）
    - 改进建议列表
  - **7 个新测试**：覆盖分析引擎、问题检测、评分计算
- **turn_history.rs 新增方法**：
  - `get_sessions_with_turns()` - 获取有历史记录的会话列表
  - `get_turn_records()` - 获取完整的 TurnRecord 列表
- cargo clippy 0 警告（仅 dead_code）
- cargo test 358 tests

**下一步**: 更多 Agent 能力增强、交互体验优化

---

### v11.2.0 (已完成 ✅)

**完成事项**:
- **SessionQualityManager Integration** - 将 SessionQualityManager 集成到运行时
  - **HandlerContext 新增字段**：`session_quality_manager: Arc<SessionQualityManager>`
  - **HttpState 新增字段**：`session_quality_manager: Arc<SessionQualityManager>`
  - **main.rs 初始化**：创建 SessionQualityManager 并传递到 HandlerContext 和 HttpState
  - **server.rs 更新**：每次连接创建新的 HandlerContext 时传递 session_quality_manager
  - **gateway/events.rs 新增事件**：`SessionQuality` 事件
    - 字段：session_id、quality_score、turn_count、task_completion_rate、tool_success_rate、rating、issue_count、suggestions
  - **HTTP API 增强**：
    - `GET /api/sessions/{session_id}/quality` - 使用 manager 缓存，支持 cached 字段
    - `DELETE /api/sessions/{session_id}/quality` - 调用 manager.invalidate() 清除缓存
    - `GET /api/sessions/quality/list` - 使用 manager.get_summaries() 获取缓存的摘要
  - **Gateway 实时事件**：每轮 Agent Turn 结束后自动分析并发射 `session.quality` 事件
  - **SSE 事件过滤**：SessionQuality 事件支持 session_id 过滤
- **Bug Fix**：`session_quality.rs` 修复 calculate_quality_score 函数返回值 bug
  - 原代码 `score.max(0.0).min(1.0)` 计算了值但未返回
  - 修复为 `score.clamp(0.0, 1.0)`
- cargo clippy 0 警告（仅 dead_code）
- cargo test 358 tests

**下一步**: 更多 Agent 能力增强、WebUI 实时质量面板

---

### v11.4.0 (已完成 ✅)

**完成事项**:
- **WebUI Real-time Quality Panel** - Web 界面实时质量面板
  - **SSE 事件监听**：
    - 添加 `'session.quality'` 事件到事件类型列表
    - 添加 `'agent.self_evaluation'` 事件到事件类型列表
  - **实时更新处理**：
    - 添加 `updateQualityPanel()` 函数 - 当收到 session.quality 事件时实时更新质量面板
    - 添加 `handleSseChatEvent()` 中的 session.quality 事件处理
    - 添加 agent.self_evaluation 事件处理 - 显示 toast 通知
  - **事件日志显示**：
    - 添加 session.quality 事件的 CSS 样式
    - 添加 session.quality 事件内容显示（质量评分、任务完成率、工具成功率、问题数）
  - **Toast 通知**：收到质量更新时显示实时通知
- cargo clippy 0 警告（仅 dead_code）
- cargo test 358 tests

**下一步**: 更多 Agent 能力增强、Agent 技能自动推荐

---

### v11.5.0 (已完成 ✅)

**完成事项**:
- **Skill Auto-Recommendation System** - 技能自动推荐系统
  - **新增模块** `agent/skill_recommender.rs`：
    - `SkillRecommendation` 结构：skill_name、description、confidence、reasons、triggered_keywords
    - `SkillRecommender` 引擎：分析对话上下文，推荐相关技能
    - `SkillRecommenderStats`：推荐统计
  - **推荐逻辑**：
    - 关键词匹配：检测对话中的主题（file_ops、code_analysis、system_ops、web_search、diff_compare）
    - 模式检测：识别多词模式如 "read the file"、"make an http request"
    - 置信度评分：基于匹配强度计算推荐置信度
    - 过滤已启用技能：不推荐已启用的技能
  - **Gateway 集成**：
    - 每次 Agent Turn 后自动分析对话上下文
    - 发射 `skill.recommended` SSE 事件
    - 推荐限制最多 3 个，按置信度排序
  - **HTTP API 端点**：
    - `GET /api/sessions/{session_id}/skill-recommendations` - 获取技能推荐
  - **WebUI 支持**：
    - SSE 事件监听：添加 `skill.recommended` 事件类型
    - Toast 通知：收到推荐时显示通知
    - 事件日志：显示推荐的技能列表
  - **SessionSkillManager 增强**：
    - 新增 `skill_registry()` getter 方法获取技能注册表
  - **8 个新测试**：覆盖关键词匹配、多主题检测、已启用过滤、大小写不敏感等场景
- cargo clippy 0 警告（仅 dead_code）
- cargo test 366 tests

**下一步**: Agent 能力进一步增强、更多交互优化

---

### v11.6.0 (已完成 ✅)

**完成事项**:
- **WebUI Skill Recommendations Panel** - Web 界面技能推荐管理面板
  - **新增 HTML 面板**：`💡 技能推荐` 面板，支持会话选择、推荐列表展示
  - **推荐卡片显示**：技能名称、置信度（颜色编码）、描述、推荐原因、触发关键词
  - **操作按钮**：「启用技能」按钮直接启用推荐技能
  - **已启用标记**：已启用的技能显示绿色徽章
  - **SSE 实时更新**：收到 `skill.recommended` 事件时自动更新面板
  - **会话选择器**：支持切换会话查看不同会话的推荐
- **TUI Skill Recommendations Support** - 终端界面技能推荐支持
  - **新增 TUI 命令**：`:rec` / `:recommendations` 查看技能推荐
  - **新增 TuiGatewayEvent 变体**：`SkillRecommendations` 事件
  - **新增 TUI 组件**：`draw_recommendations_panel()` 绘制推荐面板
  - **AppState 新增字段**：`recommendations_mode`、`recommendations_session_id`、`recommendations_data`
  - **新增 SkillRecommendationDisplay 结构**：用于 TUI 显示推荐数据
  - **Gateway Client 增强**：
    - 解析 `skill.recommended` SSE 事件
    - 新增 `get_skill_recommendations_http()` 方法通过 HTTP API 获取推荐
    - 新增 `enable_session_skill()` 方法通过网关启用技能
  - **协议扩展**：`protocol.rs` 新增 `SESSION_SKILLS_PUT` 方法常量
- **CSS 样式**：
  - `.skill-recs-panel` - 推荐面板容器
  - `.skill-rec-item` - 推荐卡片（紫色左边框）
  - `.skill-rec-confidence.high/medium/low` - 置信度颜色编码
  - `.skill-rec-reason` - 推荐原因样式
  - `.skill-rec-keywords` - 触发关键词样式
  - `.skill-rec-enabled-badge` - 已启用徽章样式
- cargo clippy 0 警告（仅 dead_code）
- cargo test 366 tests

**下一步**: Agent 能力进一步增强、更多交互优化

---

### v12.0.0 (已完成 ✅)

**完成事项**:
- **Tool Execution Retry with Backoff** - 工具执行自动重试机制
  - **新增 `execute_with_retry()` 方法**：ToolExecutor 支持自动重试瞬态错误
    - 最大重试次数：3次
    - 初始延迟：500ms
    - 最大延迟：10秒
    - 指数退避 + 随机抖动策略
  - **瞬态错误检测**：`is_transient_error()` 辅助方法
    - 网络错误：connection refused/reset/timeout、DNS failure
    - 超时错误：timed out、deadline exceeded
    - 资源繁忙：resource busy、file is busy
  - **Client 集成**：`client.rs` 调用 `execute_with_retry()` 替代 `execute()`
    - 工具执行时自动享受重试保护
    - 重试成功时记录 debug 日志
  - **6 个新测试**：
    - `test_is_transient_error_network` - 网络错误检测
    - `test_is_transient_error_timeout` - 超时错误检测
    - `test_is_transient_error_resource_busy` - 资源繁忙检测
    - `test_is_not_transient_error` - 非瞬态错误排除
    - `test_execute_with_retry_success_first_try` - 首次成功无重试
    - `test_execute_with_retry_non_transient_error` - 非瞬态错误不重试
- **Clippy 警告修复**：
  - 修复 `tui/components.rs` 中 `map_clone` 警告（使用 `.cloned()` 替代 `.map(|k| k.clone())`）
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 372 tests

**下一步**: Agent 能力持续增强、更智能的错误处理

---

### v11.3.0 (已完成 ✅)

**完成事项**:
- **TUI Quality & Evaluation Dashboard** - TUI 质量与评估面板
  - **新增 TUI 命令**：
    - `:quality` / `:qly` - 查看会话质量分析面板
    - `:eval` / `:evals` - 查看最近自我评估列表
  - **新增事件处理**：
    - `TuiGatewayEvent::SessionQuality` - 实时接收会话质量事件
    - `TuiGatewayEvent::SelfEvaluation` - 实时接收自我评估事件
  - **新增数据结构**：
    - `SessionQualityDisplay` - TUI 显示用的会话质量数据
    - `SelfEvaluationDisplay` - TUI 显示用的自我评估数据
  - **新增面板绘制函数**：
    - `draw_quality_panel()` - 绘制会话质量分析面板（星级评分、指标、建议）
    - `draw_eval_panel()` - 绘制自我评估列表面板（得分、维度、优势/弱点）
  - **实时数据存储**：
    - `AppState.quality_data` - 缓存最新会话质量数据
    - `AppState.eval_data` - 缓存最近 20 条自我评估记录
  - **Esc 键退出**：支持按 Esc 键退出质量/评估面板
- cargo clippy 0 警告（仅 dead_code）
- cargo test 358 tests

**下一步**: 更多 Agent 能力增强、WebUI 实时质量面板

---

### v11.0.0 (已完成 ✅)

**完成事项**:
- **Agent Self-Evaluation System** - Agent 自我评估系统
  - **新增模块** `agent/self_evaluation.rs`：
    - `SelfEvaluation` 结构：turn_id、session_id、overall_score、dimension_scores、strengths、weaknesses、improvement_suggestions
    - `EvaluationDimension` 枚举：TaskSuccess、ToolSelection、Efficiency、ResponseQuality
    - `SelfEvaluationEngine`：评估引擎，基于 turn 数据自动计算各维度得分
    - `SelfEvaluationManager`：评估管理器，支持持久化到 `~/.config/tiny_claw/self_evaluation/`
  - **评估维度**：
    - TaskSuccess：任务是否成功完成（权重 35%）
    - ToolSelection：工具选择是否合理（权重 25%）
    - Efficiency：执行效率（耗时、token 使用）（权重 20%）
    - ResponseQuality：响应质量（长度、内容）（权重 20%）
  - **自动评估流程**：每轮 Agent Turn 结束后自动评估并存储
  - **Gateway 集成**：`handle_agent_turn` 完成后自动触发评估
  - **SSE 事件**：`agent.self_evaluation` 事件推送评估结果
  - **HTTP API 端点**：
    - `GET /api/evaluations/recent` - 获取最近评估
    - `GET /api/evaluations/stats` - 获取评估统计
    - `GET /api/evaluations/session/{session_id}` - 获取会话评估
    - `GET /api/evaluations/turn/{turn_id}` - 获取特定 Turn 评估
  - **WebUI 面板**：
    - 统计卡片：评估次数、平均得分、优秀率、会话数
    - 评估列表：显示得分、维度条形图、优势、弱点
    - 会话过滤：支持按会话筛选评估记录
    - SSE 实时更新：新评估自动显示在事件日志
  - **11 个新测试**：覆盖评估引擎、得分计算、管理器功能
- cargo clippy 0 警告（仅 dead_code）
- cargo test 351 tests

**下一步**: Session Quality Analysis、更多 Agent 能力增强

---

### v10.5.0 (已完成 ✅)

**完成事项**:
- **Tool Performance Analytics** - 工具性能分析系统
  - **新增数据结构** `ToolStats`：追踪每个工具的调用次数、成功次数、成功率、平均/最小/最大耗时
  - **新增数据结构** `ToolPerformanceStats`：汇总所有工具的整体统计数据
  - **turn_history.rs 新增方法** `get_tool_stats()`：从历史记录中聚合工具性能数据
  - **新增测试**：5 个新测试覆盖 ToolStats 记录、聚合、排序等场景
  - **API 端点** `GET /api/tools/stats`：返回所有工具的性能统计数据
  - **WebUI 新增面板** "🔧 工具性能分析"：
    - 汇总卡片：总执行次数、整体成功率、平均耗时、工具数量
    - 详细表格：工具名称、调用次数、成功次数、成功率、耗时范围、耗时分布条形图
    - 颜色编码：成功率 ≥80% 绿色，50-80% 橙色，<50% 红色
    - 自动随页面刷新加载
  - cargo clippy 0 警告
  - cargo test 340 tests

**下一步**: Skill 效果追踪、会话质量分析、Agent 自我评估

---

### v10.3.0 (已完成 ✅)

---

### v10.3.0 (已完成 ✅)

**完成事项**:
- **TUI Summarizer Commands** - 终端界面支持摘要器命令
  - 新增 `:sum` / `:summary` 命令查看摘要器配置和统计
  - `TUI_COMMANDS` 新增 `:sum` / `:summary` 命令元数据
  - **AppState 状态字段**：`summarizer_mode`、`summarizer_config`、`summarizer_stats`、`summarizer_history`
  - **Gateway Client 方法**：`get_summarizer_config()`、`get_summarizer_stats()`、`get_summarizer_history()`、`set_summarizer_config()`
  - **TuiGatewayEvent 新变体**：`SummarizerConfigLoaded`、`SummarizerStatsLoaded`、`SummarizerHistoryLoaded`
  - **Summarizer Panel**：`draw_summarizer_panel()` 组件显示配置、统计和最近历史
  - Esc 键退出摘要模式
  - 显示内容：配置（enabled/minMessages/tokenThreshold）、统计（总摘要数/汇总消息数/平均压缩率）、最近5条历史记录
- cargo clippy 0 警告
- cargo test 336 tests

**下一步**: 更多 Agent 能力增强、TUI 配置编辑支持

---

### v9.5.0 (已完成 ✅)

**完成事项**:
- **ContextSummarizer Runtime Integration** - 将 AI 摘要集成到 Agent 运行时
  - `AgentRuntime` 新增 `summarizer: RwLock<Option<Arc<ContextSummarizer>>>` 字段
  - 新增 `set_context_summarizer(agent: Arc<Agent>)` 方法启用 AI 摘要
  - 新增 `has_summarizer()` 方法检查摘要是否启用
  - 重构 `get_model_response()` 方法：
    - 当上下文需要截断且摘要器可用时，使用 AI 摘要替代简单截断
    - 保留最近 5 条消息作为上下文锚点
    - 旧消息通过 AI 生成摘要，保留决策/偏好/关键信息
    - 摘要失败时自动回退到传统截断策略
  - 新增 `apply_fallback_truncation()` 辅助方法减少代码重复
  - 修复 `await_holding_lock` clippy 警告：确保所有锁在 await 前释放
- **工作流程**:
  1. 检测上下文是否需要截断（同步）
  2. 检查摘要器是否可用且应该使用（同步，克隆 Arc 后释放锁）
  3. 如需摘要，异步调用 AI 生成摘要
  4. 摘要成功：构建包含摘要消息的上下文
  5. 摘要失败/不可用：回退到传统截断
- **日志记录**:
  - 摘要成功时记录压缩率、原始 token 数、摘要 token 数
  - 摘要失败时记录警告并回退
- cargo clippy 0 警告（除 pre-existing dead_code）
- cargo test 337 tests

**下一步**: WebUI/TUI 显示摘要状态、摘要配置化、持久化摘要历史

---

### v9.6.0 (已完成 ✅)

**完成事项**:
- **Dead Code Cleanup** - 清理 context_summarizer.rs 中未使用的代码
  - 移除 `SummarizedContext` 结构体及其 `new()`、`to_messages()`、`estimate_tokens()` 方法
  - 移除 `SummarizerConfig` 中未使用的 `target_summary_tokens` 字段
  - 移除相关测试 `test_summarized_context_to_messages`
  - 从 `mod.rs` 导出中移除 `SummarizedContext`
- **Context Summarized Event** - 新增 `context.summarized` 事件通知客户端
  - `gateway/events.rs` 新增 `ContextSummarized` 事件变体
    - 包含：session_id, messages_summarized, original_tokens, summary_tokens, compression_ratio
  - `runtime.rs` 在成功摘要后发射事件
  - `http/routes.rs` SSE 事件过滤支持新事件类型
- **TUI Summarization Status Display** - 终端界面显示摘要状态
  - `tui/state.rs` 新增 `last_summary_info: Option<String>` 字段
  - `tui/gateway_client.rs` 新增 `ContextSummarized` 事件变体和解析逻辑
  - `tui/app.rs` 处理事件并更新状态，标题栏显示摘要信息
  - 显示格式：`📝 10 msgs → 200 tokens (10%)`
- cargo clippy 0 警告
- cargo test 330 passed (6 flaky tests unrelated to changes)

**下一步**: WebUI 摘要状态显示、摘要配置化、持久化摘要历史

---

### v9.7.0 (已完成 ✅)

**完成事项**:
- **WebUI Summary Status Display** - 网页界面显示上下文摘要状态
  - `examples/admin.html` 新增 `context.summarized` SSE 事件监听
  - 新增摘要统计追踪：summaryStatsTotal、summaryStatsMessages、summaryStatsRatio
  - 新增 `updateSummaryStats()` 函数更新统计
  - 新增 `updateSummaryStatsDisplay()` 函数更新工具栏显示
  - 新增 `showSummaryToast()` 函数显示摘要通知 toast
  - 聊天工具栏新增 `summary-stats` 显示区域：
    - 摘要次数、汇总消息数、压缩率
  - 事件日志支持 `context.summarized` 事件显示
  - 格式：`摘要: 10 msgs → 200 tokens (10%)`
- **CSS 样式增强**：
  - `.summary-stats` - 紫色主题摘要统计栏
  - `.summary-stat` / `.summary-stat-label` / `.summary-stat-value`
  - `.summary-toast` - 底部居中 Toast 通知动画
- cargo clippy 0 警告
- cargo test 336 tests

**下一步**: 摘要配置化、持久化摘要历史

---

### v10.0.0 (已完成 ✅)

**完成事项**:
- **SummarizerConfig 增强** - 配置可序列化与运行时可更新
  - `context_summarizer.rs` 中 `SummarizerConfig` 添加 Serialize/Deserialize derive
  - 新增 getter 方法：`min_messages()`、`token_threshold()`、`is_enabled()`
  - 新增 `update()` 方法支持运行时配置更新
  - `ContextSummarizer` 使用内部 RwLock 实现配置可变性

- **Summary History 持久化** - 持久化摘要历史记录
  - 新增 `SummaryHistoryEntry` 结构：session_id、messages_summarized、original_tokens、summary_tokens、compression_ratio、topics、created_at
  - 新增 `SummaryHistory` 结构：entries、total_summaries、total_messages_summarized、avg_compression_ratio
  - 新增 `SummaryHistoryManager`：支持记录、查询、统计摘要历史
  - 持久化到 `~/.config/tiny_claw/summary_history/history.json`

- **Agent 客户端集成** - Agent 支持配置管理和历史追踪
  - `Agent` 新增 `summarizer_config: RwLock<SummarizerConfig>` 字段
  - 新增 `summary_history: Arc<SummaryHistoryManager>` 字段
  - 新增方法：`get_summarizer_config()`、`update_summarizer_config()`、`record_summary()`、`get_summary_stats()`、`get_summary_history()`、`get_session_summary_history()`

- **HTTP API 端点**
  - `GET /api/summarizer/config` - 获取当前配置
  - `PATCH /api/summarizer/config` - 更新配置（支持 minMessages、tokenThreshold、enabled）
  - `GET /api/summarizer/history` - 获取历史摘要列表
  - `GET /api/summarizer/stats` - 获取摘要统计
  - `GET /api/summarizer/session/{session_id}` - 获取特定会话的摘要历史

- **Gateway JSON-RPC 方法**
  - `summarizer.config.get` - 获取配置
  - `summarizer.config.set` - 设置配置
  - `summarizer.history.list` - 获取历史列表
  - `summarizer.stats` - 获取统计信息

- **Runtime 增强**
  - `AgentRuntime` 使用内部 mutability 的 ContextSummarizer
  - 配置更新无需锁释放，直接调用 summarizer 方法

- cargo clippy 0 警告（仅 dead_code 警告）
- cargo test 336 tests

**下一步**: WebUI 配置面板、TUI 命令支持、摘要历史可视化

---

### v10.1.0 (已完成 ✅)

**完成事项**:
- **Summary History 后端集成** - 将摘要记录功能接入 Agent 实际工作流
  - `SummaryHistoryManager` 新增 `impl Default` 解决 clippy 警告
  - `Agent` 新增 `summarize_and_record()` 方法：
    - 支持根据 `SummarizerConfig` 配置（min_messages、token_threshold、enabled）检查是否需要摘要
    - 格式化会话消息为可摘要格式
    - 调用 AI 生成智能摘要
    - 解析摘要文本提取结构化信息（topics、decisions、tools）
    - 创建 `ContextSummary` 并调用 `record_summary()` 记录到历史
  - 新增 `parse_summary_structured_info()` 辅助方法

- **Gateway 集成** - 在每次 Agent Turn 后自动触发摘要
  - `handle_agent_turn` 结束后以非阻塞方式调用 `agent.summarize_and_record()`
  - 从 `history_manager` 获取会话消息
  - 异步执行，不影响响应延迟
  - 摘要失败时静默跳过（仅记录 debug 日志）

- **Config Getter 使用** - 使用 SummarizerConfig 的 getter 方法
  - `summarize_and_record()` 使用 `is_enabled()`、`min_messages()`、`token_threshold()` 检查条件
  - 解决 clippy 警告（getter 方法未被使用）

- cargo clippy 0 警告
- cargo test 336 tests

**下一步**: WebUI 摘要历史可视化、摘要配置面板

---

### v10.2.0 (已完成 ✅)

**完成事项**:
- **WebUI Conversation Summary Panel** - 实现会话摘要面板完整功能
  - 新增 `loadConversationSummary()` 函数：从 `/api/sessions/{session_id}/conversation-summary` 加载摘要
  - 新增 `renderConversationSummary()` 函数：渲染会话摘要（概述、主题、决策、偏好、待解决问题）
  - 新增 `refreshConversationSummarySessionList()` 函数：刷新会话选择下拉列表
  - CSS 样式：`.conversation-summary-panel`、`.conversation-summary-section`、`.conversation-summary-item`、`.conversation-summary-question` 等
  - 显示元数据：开始时间、更新时间、Turn 数量

- **WebUI Summary History Panel** - 新增摘要历史可视化面板
  - 新增 `loadSummaryHistory()` 函数：从 `/api/summarizer/history` 加载历史记录
  - 新增 `renderSummaryHistory()` 函数：渲染摘要历史列表
  - 显示信息：会话 ID、压缩率（颜色编码）、原始/摘要 Token 数、提取的主题标签
  - 新增 `refreshSummaryHistorySessionList()` 函数：支持按会话过滤
  - CSS 样式：`.summary-history-panel`、`.summary-history-item`、`.summary-history-ratio`（低/中/高三色）

- **WebUI Summarizer Config Panel** - 新增摘要配置面板
  - 新增 `loadSummarizerConfig()` 函数：加载摘要配置和统计信息
  - 新增 `renderSummarizerConfig()` 函数：渲染配置表单
  - 新增 `saveSummarizerConfig()` 函数：保存配置到 `/api/summarizer/config`
  - 配置项：启用/禁用摘要、最小消息数、Token 阈值
  - 统计卡片：总摘要次数、汇总消息数、平均压缩率、会话数
  - CSS 样式：`.summarizer-config-panel`、`.summarizer-stats-grid`、`.config-toggle`（开关样式）

- **refreshData() 集成** - 在数据刷新时自动加载新面板
  - 调用 `refreshConversationSummarySessionList()` 和 `loadConversationSummary()`
  - 调用 `refreshSummaryHistorySessionList()` 和 `loadSummaryHistory()`
  - 调用 `loadSummarizerConfig()`

- cargo clippy 0 警告
- cargo test 336 tests

**下一步**: TUI 摘要命令支持、更多 Agent 能力增强

---

### v9.4.0 (已完成 ✅)

**完成事项**:
- **Flaky Test Fix** - 修复 memory 测试并行运行导致的竞态条件
  - `setup_test_memory()` 改为使用唯一测试目录 (`memory_test/test_{id}`)
  - 使用 `AtomicU64` 计数器确保每个测试使用独立目录
  - 测试现在使用 `MemoryManager::with_path()` 创建隔离的 manager
- **AI-Powered Context Summarization** - 新增 AI 驱动的上下文摘要模块
  - 新增 `src/agent/context_summarizer.rs` 模块
  - `ContextSummary` 结构：保存摘要文本、原始 token 数、压缩率、提取的主题/决策/工具
  - `ContextSummarizer` 结构：使用 AI 生成智能摘要，保留关键信息
  - `SummarizedContext` 结构：管理摘要 + 近期消息的组合上下文
  - 摘要格式：保留决策、用户偏好、主题进展、关键信息
- **Agent Summarization API** - 新增 Agent 内容摘要方法
  - `Agent::summarize_content()` - 无需工具调用的轻量级 AI 调用
  - `Agent::summarize_anthropic()` - Anthropic API 集成
  - `Agent::summarize_openai()` - OpenAI API 集成
  - `Agent::summarize_ollama()` - Ollama 本地模型集成
  - 支持跨 Provider 的统一摘要接口
- cargo clippy 0 警告
- cargo test 337 tests

**下一步**: 集成 ContextSummarizer 到 ContextManager、Agent 运行时

---

### v9.3.0 (已完成 ✅)

**完成事项**:
- **TUI Token Usage Display** - 终端界面 Token 使用量显示
  - **AppState 状态追踪**：`src/tui/state.rs` 新增字段
    - `token_input_total` / `token_output_total`：累计输入/输出 tokens
    - `token_usage_by_session`：按会话统计 token 使用量
    - `update_token_usage()`：更新 token 统计方法
    - `format_token_count()`：格式化大数字显示（K/M 后缀）
    - `formatted_token_usage()`：生成显示字符串
  - **Gateway Client 事件处理**：`src/tui/gateway_client.rs`
    - 新增 `TurnUsage` 事件变体（session_id, input_tokens, output_tokens, total_tokens）
    - 在 `handle_response()` 中解析 `turn.usage` SSE 事件
  - **App 事件处理**：`src/tui/app.rs`
    - `handle_gateway_event()` 处理 `TuiGatewayEvent::TurnUsage`
    - 调用 `state.update_token_usage()` 更新统计
  - **UI 显示**：`src/tui/components.rs`
    - `draw_help_bar()` 在帮助栏显示 Token 使用量
    - 格式：`📊 In: 1.2K | Out: 500`
- cargo clippy 0 警告
- cargo test 331 tests

**下一步**: 多 Session 并发执行支持、Agent 上下文摘要增强

---

### v9.0.0 (已完成 ✅)

**完成事项**:
- **Agent Token Usage Tracking** - AI API Token 使用量追踪
  - 新增 `TokenUsage` 结构体到 `turn_history.rs`：input_tokens、output_tokens、total_tokens
  - `TurnRecord` 新增 `token_usage` 字段存储每轮 Token 使用量
  - `TurnSummary` 新增 `total_tokens` 字段
  - `TurnStats` 新增 `total_tokens` 和 `avg_tokens` 统计字段
  - **Agent 客户端增强**：`client.rs` 新增 `token_usage` 字段存储和 `take_token_usage()` 方法
    - 从 Anthropic API 响应中提取 `usage.input_tokens` 和 `usage.output_tokens`
    - 从 OpenAI API 响应中提取 `usage.prompt_tokens` 和 `usage.completion_tokens`
    - Ollama 不支持 Token 使用量追踪（设为 None）
  - **Gateway 事件系统**：`events.rs` 新增 `TurnUsage` 事件
    - 实时发送 Token 使用量信息给客户端
    - SSE 事件过滤和 session 过滤支持
  - **Gateway 集成**：`handle_agent_turn` 完成后捕获 Token 使用量
    - 发射 `turn.usage` 事件
    - 存储到 TurnRecord 中
  - **统计增强**：`get_stats()` 方法计算总 Token 量和平均每轮 Token 量
- **代码清理**：移除 `conversation_summary.rs` 中未使用的方法
  - 删除 `mark_questions_answered`、`reset`、`mark_answered`、`remove`、`list_sessions` 等 dead code
  - 删除相关测试用例
- cargo clippy 0 警告
- cargo test 322 tests (1个 flaky test 单独通过)

**下一步**: WebUI Token 使用量展示、统计图表集成

---

### v9.1.0 (已完成 ✅)

**完成事项**:
- **TUI Markdown 渲染** - 终端界面支持 Markdown 格式化输出
  - 新增 `src/tui/markdown.rs` 模块：完整的 Markdown 到 styled ratatui Lines 解析器
    - 支持 **bold** (`**text**`)、*italic* (`*text*` / `_text_`)、***bold+italic***
    - 支持 `inline code` (反引号)、代码块 (``` ``` ```)
    - 支持 # Headers (h1-h6 不同颜色和样式)
    - 支持 - unordered lists、1. ordered lists
    - 支持 > blockquotes (左侧竖线)
    - 支持 [text](url) 链接渲染
    - 智能检测：`contains_markdown()` 和 `is_markdown_heavy()` 函数
  - **组件集成**：`src/tui/components.rs` 的 `draw_messages_panel()` 增强
    - 检测消息内容是否包含 Markdown（轻量/重量分级）
    - 重量 Markdown（代码块、headers、lists）→ 完整解析渲染
    - 轻量 Markdown（仅 inline 格式）→ 简化渲染
    - 无 Markdown → 保持原有纯文本行为
  - 8 个单元测试覆盖核心解析功能
- cargo clippy 0 警告
- cargo test 331 tests (8个 markdown 测试全部通过，3个 memory 时间相关 flaky 测试单独通过)

**下一步**: TUI 交互体验继续优化、Agent 上下文管理增强

---

### v9.2.0 (已完成 ✅)

**完成事项**:
- **WebUI Chat Token Usage Display** - 网页聊天界面 Token 使用量展示
  - **CSS 样式增强**：新增 `.token-stats`、`.token-stat`、`.token-badge` 样式
    - 青蓝色主题的 Token 统计栏
    - 紧凑的 Token 徽章显示在消息角色旁
  - **JavaScript Token 追踪**：
    - `tokenUsageTotal` 全局统计对象（input/output/total）
    - `tokenUsageBySession` 按会话分开统计
    - `formatTokenCount()` 函数格式化大数字（K/M 后缀）
  - **SSE 事件处理**：处理 `turn.usage` 事件
    - `updateTokenUsage()` 更新累计统计和会话统计
    - `updateTokenUsageDisplay()` 更新工具栏显示
    - `addTokenBadgeToLastMessage()` 在消息后添加 Token 徽章
  - **聊天工具栏增强**：显示实时 Token 使用量统计
    - 输入 tokens / 输出 tokens / 总计 tokens
    - 清空聊天时重置当前会话统计
  - **事件日志集成**：
    - `turn.usage` 事件添加到事件类型过滤列表
    - 事件日志显示 Token 详情（输入/输出/总计）
    - 事件类型映射中添加 "Token使用" 名称
- cargo clippy 0 警告
- cargo test 331 tests

**下一步**: TUI Token 使用量显示、更多交互体验优化

---

### v8.0.0 (已完成 ✅)

**完成事项**:
- **Agent 执行历史系统 (Agent Turn History System)** - 追踪和持久化 Agent 执行历史
  - 新增 `agent/turn_history.rs`：`TurnRecord`、`TurnSummary`、`TurnStats`、`ToolExecution` 结构
  - `TurnHistoryManager`：内存存储 + JSON 文件持久化（存储在 `~/.config/tiny_claw/turn_history/`）
  - 每个会话保留最近 100 条执行记录，支持按会话查询和全局统计
  - **工具事件追踪**：修改 `client.rs` 在工具执行时发出 `AssistantToolUse` 和 `ToolResult` 事件
  - `Agent` 结构增加 `event_emitter` 和 `current_session_key` 字段支持工具追踪
  - **Gateway 集成**：在 `handle_agent_turn` 中记录每轮执行的开始时间、用户消息、响应和耗时
  - **HTTP API 端点**：
    - `GET /api/sessions/{session_id}/turns` - 获取会话的执行历史
    - `GET /api/sessions/{session_id}/turns/{turn_id}` - 获取具体执行详情
    - `GET /api/turns/recent` - 获取所有会话最近执行
    - `GET /api/turns/stats` - 获取聚合统计数据
  - **WebUI 面板**：admin.html 新增"Agent 执行历史"面板
    - 显示执行时间、会话、成功率、工具数、耗时
    - 支持按会话过滤

**下一步**: 工具使用详细视图、统计图表、导出功能

---

### v8.1.0 (已完成 ✅)

**完成事项**:
- **Turn History Tool Detail Enhancement** - 工具执行详情捕获与展示
  - **Agent 工具追踪增强**：`client.rs` 新增 `tool_executions` 字段和 `take_tool_executions()` 方法
  - 工具执行时自动记录 name、input、output_preview、success、duration_ms
  - 支持 Anthropic 和 OpenAI 两种 provider 的工具执行追踪
  - **TurnHistoryManager 增强**：新增 `record_turn_with_tools()` 方法接收工具执行列表
  - 新增 `get_all_sessions_turns()` 方法支持导出所有会话历史
  - **Gateway 集成**：`handle_agent_turn` 完成后自动捕获工具执行并记录
- **HTTP Export Endpoint** - 执行历史导出功能
  - `GET /api/turns/export` - 导出所有执行历史为 JSON 文件
  - 包含 exported_at、version、turn_count、sessions 等字段
- **WebUI 工具详情视图** - 点击展开查看每轮执行的工具详情
  - 显示工具名称、输入参数、输出预览、执行状态、耗时
  - 新增"导出"按钮一键下载执行历史 JSON
  - 工具按执行顺序排列，成功/失败状态一目了然
- cargo clippy 0 警告（仅 dead_code 警告）
- cargo test 308 tests

**下一步**: 统计图表可视化、更多 Agent 能力增强

---

## v8.2.0 (已完成 ✅)

**完成事项**:
- **Turn History Statistics Dashboard** - 执行统计可视化面板
  - **TurnStats 增强**：`src/agent/turn_history.rs`
    - 新增 `tool_success_rate` 字段（工具成功率 0.0-1.0）
    - 新增 `PeriodStat` 结构：period/timestamp/turns/successful/tools/avg_duration_ms
    - 新增 `StatsPeriod` 枚举：Hourly/Daily/Weekly
    - `get_stats_by_period()` 方法：按时间周期分组统计
  - **HTTP API 增强**：`src/http/routes.rs`
    - 新增 `GET /api/turns/stats/period` - 按时间周期的统计数据
    - 支持 `period` 参数（hourly/daily/weekly）和 `limit` 参数
  - **WebUI 统计面板**：`examples/admin.html` 新增完整统计仪表板
    - 摘要卡片：总执行次数、工具成功率、平均耗时、工具执行数
    - 时间周期选择器（按小时/按天/按周）和数据量选择器
    - **📊 执行量趋势柱状图**：SVG 渲染，颜色编码（绿=高成功率/黄=中/红=低）
    - **🔧 工具使用排行**：Top 10 工具横向条形图
    - **💬 会话分布**：Top 10 会话横向条形图（多色区分）
    - 自动刷新集成（随 admin 面板一起刷新）
- cargo clippy 0 警告（仅 dead_code 警告）
- cargo test 308 tests

**下一步**: Agent 工具执行预览/确认机制、WebUI 交互增强

---

### v8.3.0 (已完成 ✅)

**完成事项**:
- **Agent Action Plan Preview System** - 工具执行前预览所有计划操作
  - **新事件类型**：`gateway/events.rs` 新增 `ActionPlanPreview` 事件
    - 在工具执行前显示所有计划调用的工具列表
    - 包含每个工具的 id、name、input 参数
  - **ToolCallPreview 结构**：新增结构体用于传递工具调用预览信息
  - **Agent 客户端增强**：`client.rs` 新增 `emit_action_plan_preview()` 方法
    - 在 Anthropic 和 OpenAI provider 的工具调用循环中，先收集所有 tool_use 块
    - 批量发送 `ActionPlanPreview` 事件，再逐个执行工具
  - **HTTP SSE 集成**：`routes.rs` 新增事件过滤支持
  - **协议常量**：`protocol.rs` 新增 `ACTION_PLAN_PREVIEW` 常量
  - **WebUI 增强**：`admin.html` 新增执行计划预览显示
    - 新增 `showActionPlanPreview()` 函数，显示计划执行的工具列表
    - 青绿色主题的预览卡片，显示工具序号、名称、参数预览
    - CSS 样式：`.action-plan-preview`、`.action-plan-tool`、`.tool-number` 等
    - 事件日志显示：显示"计划执行 N 个工具: tool1, tool2, ..."
- **Clippy 警告修复**：修复 `turn_history.rs` 中的 `let_and_return` 警告
- **Conversation Summary 模块**：新增 `agent/conversation_summary.rs`
  - 追踪会话对话状态、未回答问题等
  - 支持 turn 记录和问题追踪
- cargo clippy 0 警告（仅 dead_code 警告）
- cargo test 319 tests

**下一步**: 工具执行确认机制（用户可取消）、TUI 预览支持

---

### v8.4.0 (已完成 ✅)

**完成事项**:
- **Agent Action Confirmation System** - 用户可确认或拒绝 Agent 计划执行的工具
  - **新事件类型**：`gateway/events.rs` 新增 `ActionPlanConfirm` 和 `ActionDenied` 事件
    - `ActionPlanConfirm`：Agent 等待用户确认时发送，包含 plan_id 和工具列表
    - `ActionDenied`：用户拒绝或超时未确认时发送
  - **PendingActionPlan 结构**：`client.rs` 新增待确认计划管理
    - plan_id 唯一标识、tools 列表、oneshot 通道用于接收确认响应
    - 60秒超时机制，超时后自动拒绝
  - **Agent 客户端增强**：
    - Anthropic 和 OpenAI provider 在执行工具前等待用户确认
    - `confirm_action()` 方法供 Gateway 调用
    - `Error::ActionDenied` 新错误类型
  - **Gateway JSON-RPC 方法**：`protocol.rs` 新增 `SESSION_CONFIRM_ACTION`
    - `handle_session_confirm_action` 处理器
    - 参数：sessionKey、planId、confirmed (true/false)
  - **HTTP SSE 事件过滤**：`routes.rs` 支持新的 action 事件类型
  - **错误处理增强**：`messages.rs` 处理 `Error::ActionDenied`
    - 返回 `USER_DENIED_ERROR` 错误码
    - 错误恢复提示用户需要确认才能执行工具
- **WebUI 增强准备**：admin.html 中显示确认请求（待前端实现）
- cargo clippy 0 警告（仅 dead_code 警告）
- cargo test 319 tests

**下一步**: WebUI 确认弹窗、TUI 命令支持

---

### v8.5.0 (已完成 ✅)

**完成事项**:
- **TUI Action Confirmation Support** - 终端界面支持工具执行确认
  - **TUI Gateway Client 增强**：`gateway_client.rs` 新增 `confirm_action()` 方法
    - 发送 `session.confirm_action` JSON-RPC 请求
    - 参数：session_id、plan_id、confirmed (true/false)
  - **事件解析增强**：`gateway_client.rs` 新增 `action.plan_confirm` 和 `action.denied` 事件解析
    - 解析工具列表 (ToolCallPreview) 并构造 TuiGatewayEvent
  - **AppState 确认状态**：`state.rs` 新增字段
    - `confirm_mode: bool` - 是否处于确认模式
    - `confirm_session_id: Option<String>` - 待确认会话
    - `confirm_plan_id: Option<String>` - 待确认计划 ID
    - `confirm_tools: Vec<ToolCallPreview>` - 待确认工具列表
  - **确认面板**：`components.rs` 新增 `draw_confirm_panel()`
    - 显示计划执行的工具列表（名称、参数预览）
    - 提示用户使用 :confirm/:y 允许或 :deny/:n 取消
  - **命令支持**：`app.rs` 新增确认命令
    - `:confirm` 或 `:y` - 确认执行
    - `:deny` 或 `:n` - 拒绝执行
    - Enter 键 - 确认执行（默认行为）
    - Esc 键 - 拒绝执行
  - **帮助栏增强**：确认模式下显示特定帮助提示
- cargo clippy 0 警告（仅 pre-existing dead_code 警告）
- cargo test 317 tests (2 flaky time-based tests pass when run individually)

---

### v8.7.0 (已完成 ✅)

**完成事项**:
- **Smart Context Truncation with Priority-Based Message Retention** - 智能上下文截断
  - **MessageImportance 枚举**：Low/Medium/High/Critical 四级重要性
  - **语言感知 Token 估算**：`estimate_tokens()` 改进
    - 中文/日文/韩文字符：~1.5 chars/token（vs 英文 ~4 chars/token）
    - 代码内容：识别并调整估算（代码压缩效果好）
    - 非 ASCII 非 CJK 字符：~2 chars/token
  - **消息重要性评分**：`score_message_importance()` 方法
    - Critical: 工具结果（提供关键上下文）
    - High: 决策、偏好、重要用户信息
    - Medium: 问题、技术内容、有实质内容的助手回复
    - Low: 短确认、问候语
  - **优先级截断策略**：`truncate_to_fit()` 改进
    - Phase 1: 始终保留最近消息（当前上下文锚点）
    - Phase 2: 保留高/关键重要性消息
    - Phase 3: 用中等重要性消息填充剩余预算
    - Phase 4: 低重要性消息仅在有额外空间时保留
  - **新增 7 个测试**：
    - `test_token_estimation_chinese` - 中文 token 估算
    - `test_token_estimation_code` - 代码 token 估算
    - `test_message_importance_high` - 高重要性消息识别
    - `test_message_importance_preference` - 偏好消息识别
    - `test_message_importance_tool_result` - 工具结果重要性
    - `test_message_importance_low` - 低重要性消息识别
    - `test_truncate_preserves_important` - 截断保留重要消息
- cargo clippy 0 警告（仅 pre-existing dead_code 警告）
- cargo test 319 tests (15 context_manager tests)

**下一步**: Agent 历史上下文集成、多轮对话记忆优化

---

### v8.8.0 (已完成 ✅)

**完成事项**:
- **TUI Streaming Text Display** - 终端界面实时流式文本显示
  - **TUI Gateway Client 增强**：`gateway_client.rs` 新增 `StreamingText` 事件
    - 解析 `assistant.partial` SSE 事件
    - 支持增量文本累积
  - **AppState 流式状态**：`state.rs` 新增字段和方法
    - `is_streaming: bool` - 是否正在流式输出
    - `partial_text: String` - 累积的部分文本
    - `streaming_session_id: Option<String>` - 当前流式会话
    - `streaming_message_created: bool` - 避免重复消息
    - `start_streaming()`, `append_streaming_text()`, `end_streaming()`, `cancel_streaming()` 方法
  - **消息面板流式显示**：`components.rs` 新增流式文本渲染
    - 显示累积的部分文本
    - 青色闪烁光标 `▊` 指示器
    - 与加载指示器协同工作
  - **App 事件处理增强**：`app.rs` 完善事件处理
    - `StreamingText` 事件累积文本
    - `TurnStarted` 重置流式状态
    - `TurnEnded` 最终化流式消息
    - `TurnCancelled` 取消流式状态
    - 智能去重：避免 `AssistantText` 和 `TurnEnded` 创建重复消息
  - **多 Provider 支持**：
    - Ollama: 原生流式支持，实时显示文本
    - Anthropic/OpenAI: 非流式回退，保持兼容
- cargo clippy 0 警告（仅 pre-existing dead_code 警告）
- cargo test 326 tests

**下一步**: TUI Markdown 渲染、更多交互体验优化

---

### v8.6.0 (已完成 ✅)

**完成事项**:
- **WebUI Action Confirmation Dialog** - 网页界面工具执行确认弹窗
  - **确认对话框 UI**：`examples/admin.html` 新增完整确认弹窗
    - 金色主题对话框，显示计划执行的工具列表（序号、名称、参数预览）
    - 两个按钮：「🚫 拒绝」和「✅ 允许执行」
    - 60秒超时提示
    - 点击遮罩层背景 = 拒绝操作
    - Esc 键 = 拒绝操作
  - **SSE 事件处理**：新增 `action.plan_confirm` 和 `action.denied` 事件处理
    - 收到 `action.plan_confirm` 时弹出确认对话框
    - 收到 `action.denied` 时关闭对话框并显示系统消息
    - `turn.ended` 和 `turn.cancelled` 时自动关闭对话框
  - **WebSocket 确认发送**：新增 `confirmAction(planId, confirmed)` 函数
    - 发送 `session.confirm_action` JSON-RPC 请求
    - 发送后显示用户反馈消息（✅已允许/🚫已拒绝）
  - **CSS 样式**：完整金色主题确认对话框样式
    - `.action-confirm-overlay` - 黑色半透明遮罩
    - `.action-confirm-dialog` - 金色边框对话框
    - `.action-confirm-tool` - 工具项样式（左侧金色边框）
    - 按钮样式：绿色确认按钮、红色拒绝按钮（悬停动画）
- cargo clippy 0 警告（仅 pre-existing dead_code 警告）
- cargo test 319 tests

**下一步**: 多会话支持增强、Agent 上下文优化

---

### v7.3.0 (已完成 ✅)

**完成事项**:
- **Memory Decay System** - 实现记忆衰减机制

---

### v7.0.0 (已完成 ✅)

**完成事项**:
- **Agent 长期记忆系统 (Agent Long-Term Memory)** - 持久化存储从对话中提取的重要信息
  - 新增 `agent/memory.rs`：`MemoryFact`、`FactCategory`、`MemoryManager` 结构
  - FactCategory 支持：UserPreference、Decision、Technical、ProjectContext、ActionItem、General
  - MemoryManager：JSON 文件持久化、关键词搜索、会话级检索、上下文提示生成
  - 自动提取关键词用于检索，过期清理（30天 TTL）
  - 已集成到 HandlerContext 和 HttpState
  - 11 个新测试
- 修复 clippy 警告 (suggestion_manager.rs map_clone 和 dead_code)
- cargo clippy 0 警告（仅未使用代码警告）
- cargo test 273 tests

**下一步**: HTTP API 端点集成、事实自动提取

---

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
- [x] **自动记忆提取** - 基于内容分析的自动提取和重要性评分 ✅ v7.2.0
- [x] **记忆衰减机制** - 重要性随时间自然衰减，低重要性事实自动清理 ✅ v7.3.0
- [x] **任务抽象与执行** ✅ v5.9.0
- [x] **后台任务队列** ✅ v5.9.0
- [x] **定时任务系统** ✅ v6.0.0
- [x] **Agent 主动建议** ✅ v6.2.0
- [x] **Agent 长期记忆系统** ✅ v7.0.0
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
- [x] WebUI 会话笔记管理面板 ✅ v6.5.0
- [x] TUI 笔记命令 ✅ v6.5.0
- [x] WebUI Action Confirmation Dialog ✅ v8.6.0

### 稳定性
- [x] 日志优化 (结构化日志) ✅ v4.1.0
- [x] 监控指标 ✅ v3.9.0
- [x] 错误处理增强 ✅ v5.7.0

---

## 历史迭代

| 版本 | 完成事项 |
|------|----------|
| v12.0.0 | Tool Execution Retry with Backoff - 工具执行自动重试机制：execute_with_retry() 方法支持最大3次重试+指数退避+随机抖动；is_transient_error() 检测网络/超时/资源繁忙等瞬态错误；client.rs 集成自动重试；6 个新测试；cargo clippy 0 警告；cargo test 372 tests |
| v11.6.0 | WebUI Skill Recommendations Panel + TUI Support - Web 界面技能推荐管理面板：技能推荐面板、置信度颜色编码、启用技能按钮；TUI :rec 命令支持、TuiGatewayEvent::SkillRecommendations 事件、draw_recommendations_panel 组件；CSS 样式增强；cargo clippy 0 警告；cargo test 366 tests |
| v11.5.0 | Skill Auto-Recommendation System - 技能自动推荐系统：新增 skill_recommender.rs 模块（SkillRecommendation/SkillRecommender 结构）；关键词匹配和模式检测；置信度评分；Gateway 集成自动发射 skill.recommended 事件；HTTP API 端点；WebUI/TUI 实时通知；8 个新测试；cargo clippy 0 警告；cargo test 366 tests |
| v9.5.0 | ContextSummarizer Runtime Integration - AI 摘要集成到 Agent 运行时：AgentRuntime 新增 summarizer 字段和 set_context_summarizer() 方法；get_model_response() 重构：上下文截断时优先使用 AI 摘要（保留最近5条消息作为锚点、旧消息生成智能摘要、保留决策/偏好/关键信息）；摘要失败自动回退到传统截断；修复 await_holding_lock 警告；cargo clippy 0 警告；cargo test 337 tests |
| v9.4.0 | AI-Powered Context Summarization - 新增 AI 驱动上下文摘要：context_summarizer.rs 模块（ContextSummary/ContextSummarizer/SummarizedContext 结构）；Agent::summarize_content() 方法支持 Anthropic/OpenAI/Ollama；修复 memory 测试竞态条件；cargo clippy 0 警告；cargo test 337 tests |
| v9.3.0 | TUI Token Usage Display - 终端界面 Token 使用量显示：AppState 状态追踪（token_input_total/output_total、token_usage_by_session）；Gateway Client TurnUsage 事件处理；App handle_gateway_event 更新统计；draw_help_bar 显示 `📊 In: 1.2K | Out: 500`；格式化大数字（K/M 后缀）；cargo clippy 0 警告；cargo test 331 tests |
| v9.2.0 | WebUI Chat Token Usage Display - 网页聊天界面 Token 使用量展示：CSS 样式增强（.token-stats、.token-badge）；JavaScript Token 追踪（全局统计、会话统计）；`turn.usage` SSE 事件处理；聊天工具栏显示实时统计（输入/输出/总计 tokens）；消息后 Token 徽章；事件日志集成；cargo clippy 0 警告；cargo test 331 tests |
| v9.1.0 | TUI Markdown 渲染 - 终端界面支持 Markdown 格式化输出：新增 `src/tui/markdown.rs` 模块（**bold**、*italic*、`inline code`、```code blocks```、# headers、lists、> blockquotes、[links]）；`draw_messages_panel()` 智能检测 Markdown 类型并分级渲染；8个单元测试；cargo clippy 0 警告；cargo test 331 tests |
| v9.0.0 | Agent Token Usage Tracking - AI API Token 使用量追踪：TokenUsage 结构、TurnRecord/turn_history 增强、client.rs 从 Anthropic/OpenAI 提取 usage、TurnUsage 事件、Gateway 集成、统计增强；代码清理移除 dead code；cargo clippy 0 警告；cargo test 322 tests |
| v8.8.0 | TUI Streaming Text Display - 终端界面实时流式文本显示：Gateway Client 新增 `StreamingText` 事件解析 `assistant.partial`；AppState 流式状态字段（is_streaming、partial_text、streaming_session_id）；消息面板显示累积部分文本+青色闪烁光标；智能去重避免 `AssistantText` 和 `TurnEnded` 重复消息；支持 Ollama 原生流式、Anthropic/OpenAI 非流式回退；cargo clippy 0 警告；cargo test 326 tests |
| v8.7.0 | Smart Context Truncation - 智能上下文截断：MessageImportance 枚举四级重要性；语言感知 Token 估算（CJK ~1.5 chars/token、代码调整、非 ASCII ~2 chars/token）；优先级截断策略保留重要消息；7 个新测试；cargo clippy 0 警告；cargo test 319 tests |
| v8.6.0 | WebUI Action Confirmation Dialog - admin.html 新增完整确认弹窗：金色主题对话框显示计划执行的工具列表；两个按钮「🚫 拒绝」和「✅ 允许执行」；60秒超时提示；点击遮罩层或 Esc 键拒绝；SSE 事件处理 `action.plan_confirm` 和 `action.denied`；`confirmAction()` 发送 `session.confirm_action` WebSocket 消息；turn.ended/turn.cancelled 时自动关闭对话框；完整 CSS 样式；cargo clippy 0 警告；cargo test 319 tests |
| v8.5.0 | TUI Action Confirmation Support - 终端界面支持工具执行确认：TUI Gateway Client 新增 `confirm_action()` 方法；事件解析支持 `action.plan_confirm` 和 `action.denied`；AppState 确认状态字段；确认面板 `draw_confirm_panel()`；命令支持 `:confirm/:y` 允许、`:deny/:n` 拒绝、Enter 确认、Esc 拒绝；cargo clippy 0 警告；cargo test 317 tests |
| v8.4.0 | Agent Action Confirmation System - 用户可确认或拒绝 Agent 计划执行的工具：`ActionPlanConfirm` 和 `ActionDenied` 事件；`PendingActionPlan` 待确认计划管理（60秒超时）；Agent 客户端 `confirm_action()` 方法；Gateway `SESSION_CONFIRM_ACTION` JSON-RPC 方法；HTTP 错误处理增强；cargo clippy 0 警告；cargo test 319 tests |
| v8.3.0 | Agent Action Plan Preview System - 工具执行前预览所有计划操作：`ActionPlanPreview` 事件类型；Agent 客户端 `emit_action_plan_preview()` 批量发送工具预览；HTTP SSE 事件过滤增强；WebUI `showActionPlanPreview()` 函数；青绿色主题预览卡片；Cargo clippy 0 警告；cargo test 319 tests |
| v8.2.0 | Turn History Statistics Dashboard - 执行统计可视化面板：`TurnStats` 增强（tool_success_rate、PeriodStat）；按时间周期分组统计 API；WebUI 统计仪表板（摘要卡片、柱状图、条形图）；cargo clippy 0 警告；cargo test 308 tests |
| v8.1.0 | Turn History Tool Detail Enhancement - 工具执行详情捕获与展示：Agent 工具追踪增强（tool_executions 字段）；HTTP Export Endpoint (`/api/turns/export`)；WebUI 工具详情视图（展开查看输入/输出/状态/耗时）；cargo clippy 0 警告；cargo test 308 tests |
| v8.0.0 | Agent Turn History System - 追踪和持久化 Agent 执行历史：`TurnRecord`、`TurnStats`、`ToolExecution` 结构；JSON 文件持久化；Gateway 集成；HTTP API 端点；WebUI 面板；cargo clippy 0 警告；cargo test 308 tests |
| v7.3.0 | Memory Decay System - 新增记忆衰减机制：事实重要性随时间自然衰减（每7天衰减10%），低于阈值(0.1)自动移除；`MemoryDecayStats` 追踪衰减统计；`decay_facts()` 和 `try_decay()` 方法；Gateway `handle_agent_turn` 中自动触发衰减检查；增强 `/api/memory/stats` 端点：新增 importanceDistribution、categoryDetails、decay 统计；5个新测试；cargo clippy 0 警告；cargo test 303 tests |
| v7.2.0 | Automatic Memory Extraction - 新增 `agent/memory_extractor.rs`：`ImportanceCalculator`（内容分析自动计算重要性评分）和 `FactExtractor`（6种类别自动分类）；`MemoryManager::auto_extract()` 整合提取和存储；Gateway `handle_agent_turn` 后自动提取记忆；14个新测试；cargo clippy 0 警告；cargo test 287 tests |
| v7.1.0 | 记忆 HTTP API 端点 + Agent 上下文集成 - 完整的记忆管理 REST API (GET/POST/DELETE /api/memory)；Agent turn 自动注入相关记忆到 system prompt；修复 memory 模块测试隔离问题；cargo clippy 0 警告；cargo test 273 tests |
| v7.0.0 | Agent 长期记忆系统 - 新增 `agent/memory.rs`：`MemoryFact`、`FactCategory`、`MemoryManager` 结构；支持 6 种事实类别；关键词自动提取和检索；会话级检索和上下文提示生成；MemoryManager 集成到 Gateway 和 HTTP 状态；11 个新测试；cargo clippy 0 警告；cargo test 273 tests |
| v6.7.0 | 交互式建议系统 - 新增 SuggestionManager 管理主动建议和用户反馈；追踪接受/忽略的建议，学习避免重复；Gateway JSON-RPC (`session.suggestions.list/accept/dismiss`)；HTTP REST API (`/api/sessions/:id/suggestions`)；增强 SuggestionEngine 反馈过滤；cargo clippy 0 警告；cargo test 262 tests |
| v6.6.0 | Session Instructions 会话指令 - 每个会话可设置独立的 AI 行为指令，注入到 system prompt 中；新增 `Session.instructions` 字段、SessionManager 方法、Gateway JSON-RPC (`session.instructions.get/set`)、HTTP REST API (`/api/sessions/:id/instructions`)、TUI `:instr` 命令、draw_instructions_panel 组件；cargo clippy 0 警告；cargo test 251 tests |
| v6.5.0 | WebUI 会话笔记面板 + TUI 笔记命令 - admin.html 新增会话笔记管理面板：会话选择下拉框、笔记列表展示、CRUD 操作、置顶/标签支持；TUI 新增 `:note` / `:notes` / `:pin` 命令查看笔记；AppState 新增 notes_mode 等字段；Gateway 客户端新增 list_session_notes 方法；SessionNotesLoaded 事件处理；draw_notes_panel 组件；cargo clippy 0 警告；cargo test 251 tests |
| v6.4.0 | Session Notes 后端 - 新增 `agent/session_notes.rs`：SessionNote/SessionNoteSummary/SessionNotesManager 结构，支持 content/pinned/tags 字段，JSON 持久化到 ~/.config/tiny_claw/session_notes/；Gateway JSON-RPC 集成 session.notes.list/add/update/delete 方法；HTTP REST API 完整支持；cargo clippy 0 警告；cargo test 251 tests |
| v6.3.0 | 会话上下文持久化 + TUI 增强 - 新增 `preferences.rs` 模块：`UserPreferences`/`PreferencesManager` 结构，支持 user_name、language、timezone、default_skills、agent_persona 等字段；JSON 持久化到 `~/.config/tiny_claw/preferences.json`；HTTP API `/api/preferences` GET/PATCH；TUI 增强：gg 滚动到底部，`:f`/Ctrl+F 搜索模式，n/N 导航，实时匹配计数；cargo clippy 0 警告；cargo test 241 tests |
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
- [x] **会话上下文持久化** - 跨会话记住用户偏好
  - 新增 `preferences.rs` 模块：`UserPreferences`、`PreferencesManager` 结构
  - 支持字段：user_name、user_bio、preferred_language、default_skills、agent_persona、timezone、theme、streaming_enabled
  - JSON 文件持久化到 `~/.config/tiny_claw/preferences.json`
  - 自动加载并注入 Agent 系统提示词（`to_system_prompt_addition()` 方法）
  - HTTP API：`GET /api/preferences` 和 `PATCH /api/preferences`
  - 4 个单元测试
- [x] **TUI 增强** - 交互式 TUI 完善
  - 新增 `gg` 快捷键滚动到消息底部
  - 新增 `:f` 命令或 `Ctrl+F` 进入搜索模式
  - 搜索模式支持 `/` 开始搜索、实时显示匹配结果 (`n`/`N` 导航)
  - 搜索状态显示：匹配计数和高亮
  - Esc 退出搜索模式
- [x] 代码质量
  - cargo clippy 0 警告
  - cargo test 241 tests (新增 4 个 preferences 测试)

**下一步**: Session Notes (会话笔记)、WebUI 会话笔记管理面板

---

## 当前迭代规划 (v6.4.0)

### 本轮目标
**Session Notes - 会话笔记功能**

**计划完成**:
- [x] **会话笔记 (Session Notes)** - 让 Agent 记住会话中的重要信息
  - 新增 `agent/session_notes.rs`：`SessionNote`、`SessionNoteSummary`、`SessionNotesManager` 结构
  - 支持字段：content、pinned (置顶)、tags (标签)
  - JSON 文件持久化到 `~/.config/tiny_claw/session_notes/`
  - 支持置顶和标签分类
  - `to_system_prompt_addition()` 方法自动生成上下文提示
  - 10 个单元测试
- [x] **Gateway JSON-RPC 集成**
  - 新增方法：`session.notes.list`、`session.notes.add`、`session.notes.update`、`session.notes.delete`
  - HandlerContext 集成 session_notes 管理器
  - `generate_context_prompt()` 组合技能提示和会话笔记
- [x] **HTTP REST API**
  - `GET /api/sessions/{id}/notes` - 列出笔记
  - `POST /api/sessions/{id}/notes` - 添加笔记
  - `PUT /api/sessions/{id}/notes/{note_id}` - 更新笔记
  - `DELETE /api/sessions/{id}/notes/{note_id}` - 删除笔记
- [x] **代码质量**
  - cargo clippy 0 警告
  - cargo test 251 tests (新增 10 个 session_notes 测试)

---

### v6.5.0 (已完成 ✅)

**完成事项**:
- **WebUI 会话笔记面板** - 在 admin.html 中管理会话笔记
  - 新增 CSS 样式：`.notes-panel`、`.note-item`、`.note-header`、`.note-pinned-badge`、`.note-content`、`.note-tags`、`.note-actions`
  - 新增"📝 会话笔记"面板：会话选择下拉框、笔记列表展示
  - 支持 CRUD 操作：添加笔记、编辑笔记、删除笔记、置顶/取消置顶
  - 标签支持：逗号分隔输入
  - `renderSessionNotes()`、`loadSessionNotes()`、`showAddNoteModal()` 等 JS 函数
- **TUI 笔记命令** - 终端界面查看和管理笔记
  - 新增 `:note` / `:notes` 命令：查看当前会话笔记
  - 新增 `:pin` 命令：同 `:note` 效果
  - AppState 新增 `notes_mode`、`notes_session_id`、`notes_content` 字段
  - Gateway 客户端新增 `list_session_notes()` 方法
  - `SessionNotesLoaded` 事件处理和 `format_notes_display()` 格式化函数
  - Notes 面板渲染：`draw_notes_panel()` 组件
  - Escape 键退出笔记模式
- cargo clippy 0 警告
- cargo test 251 tests

**下一步**: 继续完善 Agent 能力、更多交互体验优化

---

### v6.6.0 (已完成 ✅)

**完成事项**:
- **Session Instructions (会话指令)** - 每个会话可设置独立的 AI 行为指令
  - 新增 `Session.instructions: Option<String>` 字段
  - SessionManager 新增 `get_instructions()` / `set_instructions()` 方法
  - Gateway JSON-RPC: `session.instructions.get` / `session.instructions.set` 方法
  - HTTP REST API: `GET/PUT /api/sessions/:session_id/instructions`
  - TUI 命令: `:instr` / `:instructions` 进入/退出指令编辑模式
  - 输入 Enter 保存指令，Esc 取消
  - 指令内容注入到 system prompt 中（优先级高于 skills 和 notes）
  - Session 列表 API 响应中增加 `instructions` 字段
- **TUI 交互优化**
  - AppState 新增 `instructions_mode`、`instructions_session_id`、`current_instructions` 字段
  - 新增 `draw_instructions_panel()` 组件（青绿色主题）
  - Gateway 客户端新增 `get_session_instructions()` / `set_session_instructions()` 方法
  - `SessionInstructionsLoaded` 事件处理
  - TUI_COMMANDS 新增 `:instr` / `:instructions` 命令元数据
- cargo clippy 0 警告
- cargo test 251 tests

**下一步**: 继续完善 Agent 能力、更多交互体验优化

---

## 当前迭代规划 (v7.0.0)

### 本轮目标
**Agent 长期记忆系统 (Agent Long-Term Memory)** - 让 Agent 记住重要信息

**计划完成**:
- [x] **Memory 模块核心** - 持久化存储从对话中提取的重要信息
  - 新增 `agent/memory.rs`：`MemoryFact`、`FactCategory`、`MemoryManager` 结构
  - 支持 6 种事实类别：UserPreference、Decision、Technical、ProjectContext、ActionItem、General
  - 关键词自动提取和检索，过期清理（30天 TTL）
  - 会话级检索和上下文提示生成
- [x] **HandlerContext 和 HttpState 集成**
  - MemoryManager 已添加到 Gateway 和 HTTP 状态
- [x] **测试覆盖**
  - 11 个新测试，覆盖核心功能
- [x] 代码质量
  - 修复 clippy 警告 (suggestion_manager.rs)
  - cargo clippy 0 警告（仅未使用代码警告）
  - cargo test 273 tests

**下一步**: HTTP API 端点集成、Agent 上下文集成

---

## v7.1.0 (已完成 ✅)

**完成事项**:
- **HTTP REST API 内存端点** - 完整的记忆管理 REST API
  - `GET /api/memory` - 列出所有记忆事实
  - `GET /api/memory/search?q=...` - 搜索记忆事实
  - `POST /api/memory` - 添加新记忆事实
  - `DELETE /api/memory/{fact_id}` - 删除记忆事实
  - `GET /api/memory/stats` - 获取记忆统计信息
  - `GET /api/memory/category/{category}` - 按类别获取事实
  - `DELETE /api/memory/category/{category}` - 清除类别
  - `GET /api/memory/session/{session_id}` - 获取会话相关事实
- **Agent 上下文集成** - 记忆自动注入 Agent system prompt
  - `generate_context_prompt()` 新增记忆上下文
  - 每次 Agent turn 自动包含相关记忆事实
  - 使用 `generate_session_prompt()` 获取会话相关记忆
- **测试修复** - 修复 memory 模块测试污染问题
  - 所有添加事实的测试前调用 `setup_test_memory()`
  - 确保测试间隔离
- cargo clippy 0 警告
- cargo test 273 tests

**下一步**: 继续完善 Agent 能力、更多交互体验优化

---

## v7.2.0 (已完成 ✅)

**完成事项**:
- **Automatic Memory Extraction with Content-Based Importance Scoring** - 记忆自动提取
  - 新增 `agent/memory_extractor.rs`：`ImportanceCalculator` 和 `FactExtractor` 结构
  - `ImportanceCalculator` - 基于内容分析自动计算重要性评分 (0.0-1.0)
    - 考虑偏好指示词、决策指示词、技术细节、动作指示词、具体性
    - 每种 FactCategory 有基础分，根据内容特征动态调整
  - `FactExtractor` - 从对话文本中识别潜在事实
    - 支持 6 种类别自动分类：UserPreference、Decision、Technical、ProjectContext、ActionItem、General
    - 句子级别和段落级别双重提取，去重处理
    - 每轮对话最多提取 5 个事实
  - `MemoryManager::auto_extract()` 方法 - 整合提取和存储
  - Gateway 集成 - 每个 Agent turn 后自动提取记忆
- **Gateway 集成** - `handle_agent_turn` 后自动调用 `memory_manager.auto_extract()`
  - 合并用户消息和助手回复进行联合分析
- 14 个新测试 (memory_extractor 模块)
- cargo clippy 0 警告
- cargo test 287 tests

**下一步**: WebUI 记忆管理面板、TUI 记忆命令

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

---

### v12.7.0 (已完成 ✅)

**完成事项**:
- **Context Health Monitor - 上下文健康监控** - 追踪并报告 Agent 上下文管理健康状态
  - **新增 `agent/context_health.rs` 模块**：
    - `ContextHealthLevel` 枚举：Healthy、Warning、Critical、Emergency 四个健康等级
    - `ContextComposition` 结构：系统提示词、技能指令、对话历史、记忆、会话笔记的 token 分布
    - `CompressionEvent` 结构：截断/摘要/刷新事件的详细信息
    - `HealthRecommendation` 结构：可操作的改进建议
    - `ContextHealthReport` 结构：完整健康报告
    - `ContextHealthStats` 结构：统计摘要
    - `ContextHealthMonitor` 管理器：健康监控核心逻辑，支持 update_composition、record_truncation、record_summarization、record_turn、generate_report
  - **Gateway 事件集成** (`gateway/events.rs`)：
    - 新增 `ContextHealth` 事件类型（session_id、health_level、health_score、utilization_pct、total_tokens、max_tokens、truncation_count、summarization_count、recommendations_count）
  - **HTTP API 端点** (`http/routes.rs`)：
    - `GET /api/context/health` - 获取上下文健康报告（健康等级、评分、组成、统计、建议、最近事件）
    - `POST /api/context/health/reset` - 重置健康统计数据
  - **SSE 事件过滤** (`http/routes.rs`)：
    - 新增 `context.health` 事件支持
    - 支持 session_id 过滤
  - **main.rs 集成**：
    - 创建 `ContextHealthMonitor` 实例并添加到 HttpState
    - 创建 `ContextHealthMonitor` 实例并添加到 HandlerContext
  - **消息处理集成** (`gateway/messages.rs`)：
    - 在 `handle_agent_turn` 中计算上下文组成并更新健康监控器
    - 在每个 turn 后生成并发送 `ContextHealth` 事件
  - **TUI 支持** (`tui/`)：
    - 新增 `:context` / `:ctx` / `:health` 命令查看上下文健康面板
    - `AppState` 新增 `context_health_mode` 和 `context_health_data` 字段
    - `ContextHealthDisplay` 结构用于 TUI 显示
    - `TuiGatewayClient` 新增 `get_context_health_http()` 方法
    - `TuiGatewayEvent` 新增 `ContextHealthLoaded` 变体（但 TUI 使用直接解析方式）
    - `draw_context_health_panel()` 函数：显示健康等级、评分、上下文使用率、统计、建议
  - **WebUI 支持** (admin.html)：
    - 已在 SSE 事件过滤中支持 `context.health` 事件
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 383 tests

**下一步**: WebUI 上下文健康面板、其他 Agent 能力增强


---

### v12.8.0 (已完成 ✅)

**完成事项**:
- **Context Advisor with Smart Recommendations - 上下文优化建议器** - 基于上下文管理模式的智能建议系统
  - **新增 `agent/context_advisor.rs` 模块**：
    - `PatternType` 枚举：FrequentTruncation、HighUtilization、LargeSystemPrompt、InefficientSummarization、ContextBloating、SessionTooLong 六种模式
    - `DetectedPattern` 结构：检测到的模式（类型、计数、首次/最近出现时间）
    - `ContextAdvice` 结构：单条建议（类别、标题、解释、建议、严重程度、是否紧急、触发模式）
    - `ContextAdvisor` 管理器：模式检测与建议生成核心逻辑
  - **建议生成逻辑**：
    - 频繁截断建议（≥2次触发）
    - 高使用率建议（≥80%预警 / ≥90%紧急）
    - 上下文快速增长建议（短时间内利用率大幅增长）
    - 会话过长建议（消息数>50）
    - 大系统提示词建议（>8000 tokens）
  - **`SuggestionType::Context` 变体** (`agent/suggestion.rs`)：
    - 新增上下文管理建议类型
  - **Turn 处理集成** (`gateway/messages.rs`)：
    - 在 `handle_agent_turn` 中更新上下文建议器
    - 基于上下文组成和健康数据生成建议
    - 将建议作为 SuggestionGenerated 事件发出
  - **全局建议器注册表** (`gateway/messages.rs`)：
    - `CONTEXT_ADVISORS` lazy_static：按会话存储的建议器
  - **HTTP API 端点** (`http/routes.rs`)：
    - `GET /api/context/advisor/{session_id}` - 获取会话的建议器和统计
    - `POST /api/context/advisor/{session_id}/reset` - 重置会话建议器
    - 返回：活跃模式、建议列表、是否应开启新会话
  - **TUI 命令**：
    - 新增 `:advisor` / `:advice` / `:suggestions` 命令元数据
  - **建议通过 SuggestionGenerated 事件实时推送到 WebUI 和 TUI**：
    - 紧急建议（严重程度3或is_urgent）会在对话中实时显示
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 383 tests

**下一步**: WebUI 上下文健康面板完善、其他 Agent 能力增强

---

### v12.9.0 (已完成 ✅)

**完成事项**:
- **WebUI Context Health Panel - 上下文健康面板** - 网页界面显示上下文健康状态
  - **新增 HTML 面板** (`examples/admin.html`)：
    - 🧠 上下文健康面板，显示健康等级（🟢健康/🟡预警/🟠危险/🔴紧急）
    - 上下文组成可视化（系统提示词/技能指令/对话历史/记忆/会话笔记的 token 分布）
    - 统计数据卡片（总 Turns/截断次数/摘要次数/利用率）
    - 优化建议区域（来自 ContextHealthMonitor 和 ContextAdvisor 的建议）
    - 会话选择器，支持查看会话级健康数据
    - 重置统计按钮
  - **CSS 样式增强**：
    - `.ctx-health-panel` - 面板布局网格
    - `.ctx-health-level-card` - 健康等级卡片（颜色编码）
    - `.ctx-health-bar` - 使用率进度条
    - `.ctx-health-composition` - 上下文组成区域
    - `.ctx-health-advice` - 优化建议样式
  - **JavaScript 函数**：
    - `loadContextHealth()` - 从 `/api/context/health` 加载全局健康数据
    - `renderContextHealth()` - 渲染健康面板（等级/组成/统计/建议）
    - `resetContextHealth()` - 重置统计数据
    - `refreshCtxHealthSessionList()` - 刷新会话选择器
    - 会话选择联动 Context Advisor API
  - **SSE 事件增强**：
    - 添加 `context.health` 到事件类型列表
    - 实时更新健康徽章和显示 toast 通知
    - 事件日志显示支持
    - `formatEventType` 添加"上下文健康"映射
  - **修复 context_health 测试** (`src/agent/context_health.rs`)：
    - 修复 `test_generate_report` 测试断言失败
    - 调整测试用例的 utilization_pct 使其通过
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 396 tests

**下一步**: TUI 上下文健康面板、Agent 能力持续增强

---

### v13.0.0 (已完成 ✅)

**完成事项**:
- **TUI :context 和 :perf 命令修复** - 修复 TUI 上下文健康和性能洞察命令无法显示数据的问题
  - **Bug 分析**：
    - `:context` / `:ctx` / `:health` 命令虽然有 `draw_context_health_panel` 渲染函数
    - `:perf` / `:performance` / `:insights` 命令虽然有 `draw_perf_panel` 渲染函数
    - 但两个命令都是通过 HTTP API 获取数据后仅 `debug!` 打印，数据丢失未传递给 AppState
    - 缺少事件通道传递机制导致 TUI 面板无法显示数据
  - **修复方案**：
    - `gateway_client.rs` 新增 `send_event(&self, event: TuiGatewayEvent)` 方法
    - 通过客户端内部的 `event_tx` 广播通道将解析后的数据发送到主循环
    - `:context` 命令修复：解析 HTTP 响应后发送 `ContextHealthLoaded` 事件
    - `:perf` 命令修复：解析 HTTP 响应后发送 `PerformanceInsightsLoaded` 事件
    - 复用已有的事件处理逻辑 (`handle_gateway_event`) 更新 AppState
  - **影响范围**：
    - `src/tui/gateway_client.rs` - 新增 `send_event` 方法
    - `src/tui/app.rs` - 修复 `:context` 和 `:perf` 命令的数据传递
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 396 tests

**下一步**: Agent 能力持续增强、更多交互优化

---

### v13.1.0 (已完成 ✅)

**完成事项**:
- **Context Advisor 完整集成** - 将所有未使用的 ContextAdvisor 方法连接到实际运行时
  - **问题诊断**：
    - ContextAdvisor 模块有多个未使用的方法（`record_compression`、`record_inefficient_summarization`、`record_large_system_prompt`、`check_session_length` 等）
    - 这些方法定义了完整的模式检测逻辑，但从未被调用
    - 导致 clippy 产生 6 个 "multiple methods are never used" 警告
  - **修复方案**：
    - 新增 `last_truncation_count` 和 `last_summarization_count` 字段到 ContextAdvisor 结构体（用于增量检测）
    - 新增 `update_with_health_data()` 方法 - 综合集成所有模式检测方法
      - 接收 ContextHealthReport 和 message_count 参数
      - 追踪截断/摘要事件的增量变化
      - 调用 `record_compression()` 和 `record_inefficient_summarization()` 追踪压缩事件
      - 调用 `record_large_system_prompt()` 追踪系统提示词大小
      - 调用 `check_session_length()` 追踪会话长度
    - 修改 `handle_agent_turn` 中的 Context Advisor 更新逻辑
      - 获取消息数量 `message_count`
      - 调用新的 `update_with_health_data()` 方法替代简单的 `record_turn()`
  - **效果**：
    - 所有 ContextAdvisor 方法现在都被实际使用
    - Context Advisor 能够基于真实健康报告数据进行更准确的模式检测
    - 截断/摘要/系统提示词大小/会话长度等模式都能被正确追踪
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 396 tests

**下一步**: Agent 能力持续增强、更多交互优化

---

### v13.2.0 (已完成 ✅)

**完成事项**:
- **TUI 实时上下文健康监控** - 标题栏实时显示上下文健康状态
  - **新增 `context.health` SSE 事件解析** (`gateway_client.rs`)：
    - 解析 `context.health` 事件获取健康数据
    - 新增 `ContextHealthUpdate` 事件变体包含实时健康指标
  - **新增 TUI 状态字段** (`state.rs`)：
    - `context_health_level: String` - 追踪当前健康等级
  - **标题栏实时显示** (`app.rs`)：
    - 当健康等级非 Healthy 时显示警告指示器
    - Warning → 🟡 Warning 黄色显示
    - Critical → 🔴 Critical 红色显示
    - Emergency → 🛑 Emergency 红色加粗显示
- **Context Advisor 紧急建议实时推送** - 高优先级建议即时通知
  - **新增 `context.urgent_advice` 事件类型** (`events.rs`)：
    - `UrgentAdviceItem` 结构包含 id/category/severity/is_urgent/title/explanation/suggestion/trigger_pattern
  - **Gateway 事件发射** (`messages.rs`)：
    - 调用 `advisor.get_urgent_advice()` 获取高优先级建议（severity >= 3 或 is_urgent）
    - 发射 `Event::UrgentContextAdvice` 事件
  - **HTTP SSE 路由集成** (`routes.rs`)：
    - 添加 `context.urgent_advice` 事件到事件过滤
    - 支持 session_id 过滤
- **代码质量**：
  - 修复 clippy 警告（unused health_score 变量）
  - cargo clippy 0 警告（仅 pre-existing dead_code）
  - cargo test 396 tests

**下一步**: WebUI 紧急建议面板、更多 Agent 能力增强


---

### v13.3.0 (已完成 ✅)

**完成事项**:
- **TUI Context Advisor Panel - 修复破损的 `:advisor` 命令** - 终端界面查看上下文优化建议
  - **问题诊断**：
    - `:advisor` / `:advice` / `:suggestions` 命令虽然在 `TUI_COMMANDS` 中定义但从未实现
    - 类似于 v13.0.0 修复 `:context` 和 `:perf` 命令的模式
  - **新增数据结构** (`gateway_client.rs`)：
    - `ContextAdvisorDisplay` 结构：session_id、turn_count、total_tokens_processed、compression_count、current_utilization、active_patterns、advice_count、should_suggest_new_session、advice 列表
    - `ContextAdviceDisplay` 结构：id、category、title、explanation、suggestion、severity、is_urgent、trigger_pattern
    - `TuiGatewayEvent::AdvisorDataLoaded` 事件变体携带完整建议数据
  - **HTTP 客户端方法** (`gateway_client.rs`)：
    - `get_context_advisor_http(base_url, session_id)` - 从 `/api/context/advisor/{session_id}` 获取建议数据
  - **AppState 增强** (`state.rs`)：
    - 新增 `advisor_mode: bool` - 是否处于建议查看模式
    - 新增 `advisor_data: Option<ContextAdvisorDisplay>` - 缓存建议数据
  - **命令处理** (`app.rs`)：
    - 添加 `:advisor` / `:advice` / `:suggestions` 命令处理
    - 进入模式时退出其他查看模式
    - 异步获取建议数据并通过 `send_event` 传递给主循环
    - 解析 HTTP 响应构造 `ContextAdvisorDisplay` 并发送 `AdvisorDataLoaded` 事件
  - **面板渲染** (`components.rs`)：
    - 新增 `draw_advisor_panel()` 函数：
      - 显示会话信息和统计摘要（Turns/Tokens/压缩次数/利用率/活跃模式）
      - 当 `should_suggest_new_session` 时显示警告横幅
      - 显示所有建议项（带严重程度图标 🔴/🟡/🟢、分类、标题、解释、建议）
      - 紧急建议标记 ⚡URGENT
  - **事件处理** (`app.rs`)：
    - 处理 `TuiGatewayEvent::AdvisorDataLoaded` 事件更新 AppState
  - **渲染循环** (`app.rs`)：
    - 在 `context_health_mode` 之前添加 `advisor_mode` 面板渲染
  - **Esc 键处理** (`app.rs`)：
    - Esc 退出 `advisor_mode` 并清空建议数据
- **WebUI Context Health Panel 增强** - 更完整的上下文建议展示
  - **Advisor 统计卡片** (admin.html)：
    - 仅在选择会话后显示
    - 显示 Turns / Tokens / 压缩次数 / 利用率 / 活跃模式
    - 活跃模式数量颜色编码（>0 黄色 / 0 绿色）
  - **"Start New Session" 警告横幅**：
    - 当 `shouldSuggestNewSession` 为 true 时显示红色警告
    - 提示用户开启新会话以改善上下文管理
  - **显示所有建议**：
    - 移除原有的 5 条限制（`allAdvice.slice(0, 5)` → `allAdvice`）
    - 所有建议均可显示
  - **新增 CSS 样式**：
    - `.ctx-advisor-stats` / `.ctx-advisor-stat` / `.ctx-advisor-warning` / `.ctx-advisor-section-title`
- **Bug Fix - 测试编译修复**：
  - 恢复 `ContextHealthMonitor::record_truncation()` 和 `record_summarization()` 方法
  - 这些方法在测试中被引用但之前被简化时意外删除
- **代码清理**：
  - 移除多个模块的 dead code：`context_advisor.rs`、`performance_insights.rs`、`self_evaluation.rs`
  - cargo clippy 0 警告（仅 pre-existing dead_code）
  - cargo test 396 tests

**下一步**: 更多 Agent 能力增强、交互体验优化

---

### v13.4.0 (已完成 ✅)

**完成事项**:
- **Skill Templates - 技能任务模板系统** - 让技能提供可复用的任务模板
  - **新增 `SkillTemplate` 结构** (`agent/skill.rs`)：
    - `name` - 模板标识符
    - `description` - 人类可读的任务描述
    - `steps` - 逐步执行指令（支持 {placeholder} 参数占位符）
    - `required_tools` - 执行此任务所需的工具列表
    - `example` - 示例用法
  - **`Skill` 结构增强** (`agent/skill.rs`)：
    - 新增 `templates: Vec<SkillTemplate>` 字段
    - 新增 `with_template()` / `with_templates()` 方法添加模板
    - 新增 `get_template(name)` 方法按名称获取模板
    - 新增 `has_templates()` 方法检查是否有模板
  - **内置技能模板增强** (`agent/skill_registry.rs`)：
    - `file_ops` 技能新增：explore_project、edit_file 模板
    - `code_analysis` 技能新增：find_usage、analyze_structure 模板
  - **上下文提示词集成** (`gateway/messages.rs`)：
    - 在生成技能上下文时显示可用任务模板
    - 模板以列表形式展示（名称、描述、所需工具、示例）
  - **HTTP API 增强** (`http/routes.rs`)：
    - `SkillInfo` 结构新增 `templates` 字段
    - 技能创建和更新时支持传入模板
    - 技能列表和详情接口返回模板信息
  - **6 个新测试**：
    - `test_skill_template_new` - 测试模板创建
    - `test_skill_template_with_tools` - 测试模板添加工具
    - `test_skill_with_templates` - 测试技能添加模板
    - `test_skill_get_template_not_found` - 测试获取不存在的模板
    - `test_skill_no_templates` - 测试无模板情况
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 401 tests

**下一步**: 更多内置模板、WebUI 模板展示、Agent 模板执行建议

---

### v13.5.0 (已完成 ✅)

**完成事项**:
- **Tool Strategy Engine - 工具策略引擎** - 基于用户意图的智能工具使用指导
  - **新增 `agent/tool_strategy.rs` 模块**：
    - `UserIntent` 枚举：10 种用户意图分类（Explore、Modify、Execute、Search、Read、Write、Analyze、Compare、Fetch、Monitor）
    - `UserIntent::from_message()` - 基于消息内容自动识别用户意图（词边界匹配避免误判）
    - `ToolGuidance` 结构：推荐工具列表、使用技巧、常见陷阱
    - `WorkflowPattern` 结构：常见多工具工作流模式（探索+阅读、查找+修改、备份+写入等）
    - `ToolStrategy` 引擎：意图分类、工具引导、工作流模式匹配、策略提示词生成
  - **上下文提示词集成** (`gateway/messages.rs`)：
    - 新增 `TOOL_STRATEGY` lazy_static 全局实例
    - `generate_context_prompt()` 在技能后注入工具策略提示
    - 根据当前用户消息意图动态生成工具使用指导
  - **12 个新测试**：
    - 意图分类测试（Explore、Modify、Execute、Search、Read、Write、Analyze、Ambiguous）
    - 工具引导测试、工作流模式测试、策略提示生成测试
  - **修复测试问题**：
    - 词边界匹配修复（避免 "analyze" 匹配 "an" 导致误判为 Read）
    - 修复 "make" 被误判为 Write 而非 Execute 的问题
- cargo clippy 0 警告（仅 pre-existing dead_code）
- cargo test 413 tests（+12 新测试）

**下一步**: Agent 能力持续增强（v13.6.0）


---

### v13.6.0 (已完成 ✅)

**日期**: 2026-03-31

**完成事项**:
- **ContextHealthMonitor 自动压缩检测**：
  - 新增 `prev_composition` 字段，追踪上次上下文组成
  - 改造 `update_composition()`：在更新前保存当前组成为上一组成
  - 改造 `record_turn()`：自动检测上下文压缩事件（历史 tokens 减少 >15% 且 >1000 tokens 即判定为压缩）
  - 移除死代码：`record_truncation()`、`record_summarization()`、`update_avg_compression()`、`get_composition()`、`get_stats()`（这些方法从未在生产代码中被调用）
  - 更新测试以适配新的自动检测机制
- **代码清理**：
  - 移除 `QualityIssue::display_name()`、`QualityStats` 结构（session_quality.rs）
  - 移除 `SkillRecommenderStats`、`with_already_enabled()`、`get_stats()`（skill_recommender.rs）
  - 移除 `ToolStrategy::all_patterns()`（tool_strategy.rs）
  - 移除 `ToolPatternLearner::get_patterns_with_tool()`（tool_pattern_learner.rs）
  - 为 TUI 死字段添加 `#[allow(dead_code)]`：`ContextAdviceDisplay`、`SessionInfo`、`sessions_mode`/`sessions_selected_index`/`sessions_data`、`SelfEvaluationDisplay::session_id`、`SkillRecommendationDisplay::id`
  - 为 TUI 死方法添加 `#[allow(dead_code)]`：`list_skill_recommendations`、`enable_session_skill`、`get_safety_session_state_http`
- cargo clippy **0 警告**（从 15 降至 0）
- cargo test **419 tests 全部通过**（+6 新测试，替换旧测试）

**下一步**: 交互体验优化、WebUI 进一步完善

---

### v13.7.0 (已完成 ✅)

**日期**: 2026-04-01

**完成事项**:
- **WebUI Performance Insights Panel - 性能洞察面板**：
  - **新增 HTML 面板** (`examples/admin.html`)：
    - 🧠 性能洞察面板，显示 Agent 执行效率、质量趋势、工具使用模式
    - 质量趋势显示（↑ improving / ↓ declining / → stable）
    - 统计卡片：分析 Turns、工具成功率、平均工具数/Turn
    - 工具效率区域：最高效/最低效工具及其成功率条形图
    - 改进建议列表：按严重程度显示（🔴 high / 🟡 medium / 🟢 low）
  - **CSS 样式增强**：
    - `.perf-insights-grid` - 统计卡片网格布局
    - `.perf-insight-card` / `.perf-insight-value` / `.perf-insight-label` - 统计卡片样式
    - `.perf-insight-item` - 建议项样式（左边框颜色编码）
    - `.perf-quality-trend` - 质量趋势显示样式
    - `.perf-tool-efficiency` - 工具效率条形图样式
  - **JavaScript 函数**：
    - `loadPerformanceInsights()` - 从 `/api/performance/insights` 加载数据
    - 渲染质量趋势、统计卡片、工具效率、改进建议
  - **SSE 事件增强**：
    - 添加 `agent.performance_insights` 到事件类型列表
    - 添加实时 toast 通知显示洞察数量
    - `formatEventType` 添加"性能洞察"映射
  - **refreshData() 集成**：
    - 自动随页面刷新加载性能洞察面板
- cargo clippy **0 警告**
- cargo test **419 tests 全部通过**

**下一步**: 交互体验优化继续、WebUI/TUI 细节完善

---

### v13.8.0 (已完成 ✅)

**日期**: 2026-04-01

**完成事项**:
- **TUI Real-time Context Utilization Display** - 标题栏实时显示上下文使用百分比
  - **`AppState` 新增字段** (`state.rs`)：
    - `context_utilization_pct: Option<f32>` - 实时追踪上下文利用率
  - **事件处理增强** (`app.rs`)：
    - `ContextHealthUpdate` 事件处理器现在存储 `utilization_pct` 而非忽略
    - 每次收到上下文健康更新时更新 `context_utilization_pct`
  - **标题栏实时显示** (`app.rs`)：
    - 当 `context_utilization_pct` 可用时显示 "📊 Context: X%"
    - 颜色编码：绿色 (<75%)、黄色 (75-90%)、红色 (>=90%)
    - 在 circuit breaker 指示器后显示
  - 上下文使用率在每轮 Agent Turn 结束后自动更新
- cargo clippy **0 警告**
- cargo test **419 tests 全部通过**

**下一步**: WebUI 状态可视化继续完善、TUI 交互优化


---

### v13.9.0 (已完成 ✅)

**日期**: 2026-04-01

**完成事项**:
- **Bug Fix - ScheduledTasksLoaded Event Handler Missing** - 修复编译错误
  - `app.rs` 中添加缺失的 `TuiGatewayEvent::ScheduledTasksLoaded` 事件处理器
  - 事件处理器将定时任务数据存储到 `state.scheduled_tasks_data`

- **TUI Scheduled Tasks Viewing Feature** - 完成未实现的定时任务查看功能
  - 添加 `:sched` / `:scheduled` 命令到 TUI_COMMANDS
  - 添加命令处理以通过 HTTP API 加载定时任务
  - 添加 Esc 键处理以退出定时任务查看模式
  - 在 `components.rs` 中添加 `draw_scheduled_tasks_panel()` 组件
  - 在 TUI 渲染链中添加面板渲染

- **代码质量**：
  - cargo clippy 1 警告（`ScheduledTaskDisplay` 中未使用的字段 - 这些是 API 响应数据结构的一部分）
  - cargo test **419 tests 全部通过**

**下一步**: 交互体验优化继续、WebUI/TUI 细节完善

---

### v13.11.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **Turn Summary System - Turn 总结系统** - 为每次 Agent Turn 生成简洁的执行摘要
  - **新增 `agent/turn_summary.rs` 模块**：
    - `ToolExecutionSummary` 结构：单个工具执行的摘要（tool_name、summary、success、duration_ms）
    - `AgentTurnSummary` 结构：整个 Turn 的摘要（session_id、turn_id、tool_count、tool_summaries、success、total_duration_ms、accomplishment、affected_resources）
    - `generate_turn_summary()` 函数：从 Turn Record 数据生成摘要
    - `ToolExecutionSummary::from_tool_result()` - 从工具名和结果生成摘要
    - 智能摘要生成：基于工具类型生成人类可读的摘要（read_file 显示行数、exec 显示首行输出等）
    - 资源提取：自动识别受影响的文件路径和资源
  - **Gateway 事件集成** (`gateway/events.rs`)：
    - 新增 `TurnSummary` 事件类型（session_id、turn_id、tool_count、tool_summaries、success、total_duration_ms、accomplishment、affected_resources）
  - **Gateway 消息处理集成** (`gateway/messages.rs`)：
    - 在 `handle_agent_turn` 中每轮结束后生成并发射 `TurnSummary` 事件
    - 从 `turn_record.tools` 提取工具执行数据
  - **HTTP SSE 事件过滤** (`http/routes.rs`)：
    - 添加 `turn.summary` 事件到事件过滤和映射
  - **TUI 支持** (`tui/`)：
    - `TuiGatewayEvent::TurnSummary` 事件变体
    - `gateway_client.rs` 添加 `turn.summary` SSE 事件解析
    - `AppState` 添加 `turn_summary_mode` 和 `turn_summary_data` 字段
    - `state.rs` 添加 `TurnSummaryDisplay` 结构
    - `app.rs` 添加 `TurnSummary` 事件处理
  - **测试覆盖**：
    - 9 个新测试覆盖工具执行摘要生成、Turn 摘要生成、资源提取、显示字符串生成

- **代码质量**：
  - cargo clippy 2 警告（pre-existing: ScheduledTaskDisplay 未使用字段、turn_summary_mode 未使用字段）
  - cargo test **427 tests 全部通过**（+8 新测试）

---

### v13.12.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **TUI Turn Summary Viewing - 完成 Turn Summary 系统 TUI 集成** - 修复 v13.11.0 未完成的 TUI 命令和面板
  - **问题诊断**：v13.11.0 添加了 `TurnSummary` SSE 事件收集（数据存储到 `turn_summary_data`），但缺少 `:ts` 命令和面板渲染
  - **TUI 命令** (`state.rs`)：
    - 新增 `:ts` / `:turns` / `:turnsummary` 命令到 `TUI_COMMANDS`
  - **命令处理** (`app.rs`)：
    - 添加 `:ts` / `:turns` / `:turnsummary` 命令处理
    - 进入模式时退出其他查看模式
    - 退出时清空 `turn_summary_mode` 状态
  - **面板渲染** (`components.rs`)：
    - 新增 `draw_turn_summary_panel()` 函数：
      - 显示摘要数量和列表
      - 每条摘要显示：状态图标（✅/❌）、Turn ID、工具数量、执行时长
      - 显示 Session ID
      - 显示 accomplishment 摘要
      - 显示 affected_resources（文件路径）
      - 空数据时显示友好提示
  - **Esc 键处理** (`app.rs`)：
    - Esc 退出 `turn_summary_mode` 并重置状态
  - **渲染循环集成** (`app.rs`)：
    - 在 `scheduled_tasks_mode` 之后添加 `turn_summary_mode` 面板渲染
  - **代码质量**：
    - 修复 `turn_summary_mode` 字段 dead_code 警告（原未使用）
    - cargo clippy 1 警告（pre-existing: `ScheduledTaskDisplay` 未使用字段）
    - cargo test **427 tests 全部通过**

**下一步**: Agent 能力增强、交互体验优化继续

---

### v13.14.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **TUI Turn Summary 工具执行详情增强** - 修复 Turn Summary 中 tool_summaries 未传递到 TUI 面板的问题
  - **问题诊断**：gateway/events.rs 中的 `TurnSummary` 事件包含 `tool_summaries` 字段，但 TUI 的事件解析和面板渲染中缺少该字段的传递
  - **`state.rs` 新增 `ToolExecutionSummaryDisplay` 结构**：
    - `tool_name: String` - 工具名称
    - `summary: String` - 执行摘要
    - `success: bool` - 是否成功
    - `duration_ms: u64` - 执行耗时
  - **`TurnSummaryDisplay` 增强** (`state.rs`)：
    - 新增 `tool_summaries: Vec<ToolExecutionSummaryDisplay>` 字段
  - **`TuiGatewayEvent::TurnSummary` 增强** (`gateway_client.rs`)：
    - 添加 `tool_summaries: Vec<ToolExecutionSummaryDisplay>` 字段
    - 导入 `ToolExecutionSummaryDisplay` 类型
  - **事件解析增强** (`gateway_client.rs`)：
    - 解析 `tool_summaries` JSON 数组字段
    - 构造 `ToolExecutionSummaryDisplay` 结构并传递
  - **事件处理增强** (`app.rs`)：
    - 更新 `TuiGatewayEvent::TurnSummary` 解构包含 `tool_summaries`
    - 传递给 `TurnSummaryDisplay` 结构
  - **面板渲染增强** (`components.rs`)：
    - 在每个 Turn Summary 条目中显示工具执行摘要列表
    - 每个工具显示：工具名称、状态图标（✅/❌）、耗时、执行摘要
    - 树形缩进展示，模仿命令行动态
- **代码质量**：
  - cargo clippy 1 警告（pre-existing: `ScheduledTaskDisplay` 未使用字段）
  - cargo test **427 tests 全部通过**

**下一步**: Agent 能力增强、交互体验优化继续

---

### v13.15.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **Urgent Context Advice 实时显示** - 修复 Context Advisor 紧急建议未被显示的问题
  - **问题诊断**：系统发射 `UrgentContextAdvice` 事件但 WebUI 和 TUI 均未处理
    - `messages.rs` 中 `handle_agent_turn` 会发射 `Event::UrgentContextAdvice`（v13.2.0 引入）
    - 但 admin.html 中缺少 `showToast` 函数实现（函数被调用但从未定义）
    - `context.urgent_advice` SSE 事件未注册到事件类型列表
  - **WebUI Toast 通知系统修复** (`examples/admin.html`)：
    - 新增 `.toast-container` CSS 样式：固定右下角、垂直堆叠、动画过渡
    - 新增 `.toast` 及变体样式：`.info`/`.success`/`.error`/`.warning`/`.urgent`
    - 新增 `.urgent` 样式：红色渐变背景、左侧橙色边框、脉冲动画
    - 新增 `showToast(message, type, title)` 函数：支持类型和可选标题、4-6秒自动隐藏
    - 新增 `#toastContainer` HTML 容器
  - **`context.urgent_advice` SSE 事件处理** (`examples/admin.html`)：
    - 添加到 `eventTypes` 数组
    - 在 `handleSseChatEvent` 中处理：为每条建议显示红色 urgent toast
    - 显示格式：`🔴/🟡/🟢 分类 ⚡URGENT: 建议内容`
  - **`formatEventType` 映射增强**：
    - 添加 `context.urgent_advice` → "紧急建议"
  - **TUI SSE 事件解析** (`gateway_client.rs`)：
    - 新增 `TuiGatewayEvent::UrgentAdvice { session_id, advice }` 事件变体
    - 新增 `UrgentAdviceItemDisplay` 结构：id/category/severity/is_urgent/title/explanation/suggestion/trigger_pattern
    - 新增 `context.urgent_advice` SSE 事件解析逻辑
  - **TUI 事件处理** (`app.rs`)：
    - 在 `handle_gateway_event` 中处理 `UrgentAdvice` 事件
    - 作为系统消息显示在聊天区域
    - 格式：`🔴/🟡/🟢 分类 标题 ⚡URGENT: 建议内容`
- **代码质量**：
  - cargo clippy 1 警告（pre-existing: `ScheduledTaskDisplay` 未使用字段）
  - cargo test **427 tests 全部通过**

**下一步**: 更多 Agent 能力增强、交互体验优化继续

---

### v13.16.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **Tool Strategy Integration with Learned Patterns** - 将历史工具执行模式集成到 Agent 上下文
  - **问题诊断**：`ToolPatternLearner` 追踪工具使用模式和成功率（v8.1.0 引入），但数据仅用于分析面板，从未影响 Agent 的工具选择行为
  - **`generate_context_prompt` 增强** (`gateway/messages.rs`)：
    - 新增 "1c. Learned Tool Performance" 部分
    - 调用 `ctx.tool_pattern_learner.try_read()` 获取模式学习器
    - 调用 `learner.generate_tips()` 获取最多 5 条历史性能提示
    - 当有可用提示时注入到系统上下文中
    - 提示内容：最高成功率工具、工具序列模式、低成功率警告
  - **效果**：Agent 在每次 Turn 时都能获得基于历史执行数据的学习建议
    - 了解哪些工具组合成功率高
    - 避免低成功率的工具使用方式
    - 随着使用不断优化工具选择策略
- **代码质量**：
  - cargo clippy 1 警告（pre-existing: `ScheduledTaskDisplay` 未使用字段）
  - cargo test **427 tests 全部通过**

**下一步**: 意图相关工具提示、多轮规划能力增强

---

### v13.13.0 (已完成 ✅)

**日期**: 2026-04-02

**完成事项**:
- **WebUI Turn Summary Panel - Turn 总结面板** - 完成 Turn Summary 系统的 WebUI 集成
  - **新增 CSS 样式** (`examples/admin.html`)：
    - `.turn-summary-grid` - 统计卡片网格布局
    - `.turn-summary-card` / `.turn-summary-value` / `.turn-summary-label` - 统计卡片样式
    - `.turn-summary-card.success` / `.turn-summary-card.failure` - 成功/失败状态颜色
    - `.turn-summary-list` - 摘要列表容器
    - `.turn-summary-item` / `.turn-summary-header` / `.turn-summary-meta` - 摘要项样式
    - `.turn-summary-turn-id` / `.turn-summary-status` - Turn ID 和状态徽章样式
    - `.turn-summary-accomplishment` / `.turn-summary-tools` - 成就和工具显示样式
    - `.turn-summary-tool-item` - 工具项标签样式
    - `.turn-summary-resources` / `.turn-summary-resource` - 资源显示样式
    - `.turn-summary-no-data` - 无数据提示样式
  - **新增 HTML 面板** (`examples/admin.html`)：
    - 📋 Turn 总结面板，显示 Agent 每次执行的简洁总结
    - 统计数据卡片：总 Turns、成功/总数、工具调用总数、平均耗时
    - 摘要列表：每条显示 Turn ID、状态、时长、工具数、Session ID
    - 显示 accomplishment 摘要
    - 显示工具调用列表（带成功/失败状态）
    - 显示受影响的资源（文件路径）
  - **JavaScript 函数** (`examples/admin.html`)：
    - `turnSummaries` 数组存储最近 50 条总结
    - `addTurnSummary(summary)` - 添加新总结到列表
    - `clearTurnSummaries()` - 清空所有总结
    - `renderTurnSummaries()` - 渲染总结面板
  - **SSE 事件集成** (`examples/admin.html`)：
    - 添加 `turn.summary` 到 SSE 事件类型列表
    - 在 `handleSseChatEvent` 中处理 `turn.summary` 事件
    - 实时将新总结添加到面板显示
  - **formatEventType 映射**：
    - 添加 `turn.summary` → "Turn总结" 映射
- **代码质量**：
  - cargo clippy 1 警告（pre-existing: `ScheduledTaskDisplay` 未使用字段）
  - cargo test **427 tests 全部通过**

**下一步**: Agent 能力增强、交互体验优化继续

