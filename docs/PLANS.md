# TinyClaw 迭代计划 (PLANS)

> 本文件记录每轮迭代的完成事项与下一步规划
> 每次迭代前阅读以了解长期愿景与当前优先级

---

## 项目愿景

**TinyClaw** - OpenClaw 的 Rust 实现子集，一个**小而精**的生产级 AI Agent Gateway。

### 长期目标
1. **Agent 能力提升** - 对标 OpenClaw，实现上下文管理、Skill 机制
2. **交互体验打磨** - WebUI 完善、TUI 支持、实时反馈
3. **稳定可靠** - 保持代码精简、高质量、可维护

---

## 优先级定义

| 优先级 | 领域 | 说明 |
|--------|------|------|
| P0 | **Agent 能力** | 上下文管理、Skill 机制、工具扩展 |
| P0 | **交互体验** | WebUI 完善、TUI、实时反馈 |
| P1 | **多 Session** | 并发会话管理、Session 隔离 |
| P1 | **稳定性** | 错误处理、重试机制、断路器 |

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

## 当前迭代规划 (v3.1.0)

### 本轮目标
结合 PLANS.md 优先级指引和项目现状，本轮迭代重点：**TUI 完善 (P0)**

#### 1. TUI Gateway 集成 (P0)
**目标**: TUI 连接 Gateway 进行实时对话

**已完成** ✅:
- **TUI Gateway Client** - 新增 WebSocket 客户端模块 (`src/tui/gateway_client.rs`)
  - 支持连接到本地 Gateway (ws://127.0.0.1:18790)
  - 完整的 JSON-RPC 协议支持
  - 事件订阅机制实时接收网关事件
  - 异步消息发送与接收
- **TUI 应用集成** - 更新 `src/tui/app.rs` 和 `src/tui/state.rs`
  - 连接状态追踪与显示
  - 用户消息通过 WebSocket 发送给 Agent
  - 实时显示 Agent 回复、工具调用等事件
  - 错误提示与重连支持
- **UI 增强**
  - 连接状态指示器 (●/○)
  - 加载状态显示
  - 错误消息覆盖层
  - 新增 `:r` 命令重连网关

**使用方式**:
1. 先启动 TinyClaw 主程序 (`cargo run`)
2. 再启动 TUI (`cargo run -- --tui` 或 `cargo run -t`)
3. TUI 会自动连接到 Gateway WebSocket

**下一步**: 
- 多 Session 支持 (在 TUI 中创建/切换会话)
- TUI 持久化消息历史

#### 2. 基础修复 (持续)
- `cargo clippy` 无警告
- `cargo test` 全部通过 (168 tests)

#### 2. 基础修复 (持续)
- `cargo clippy` 无警告
- `cargo test` 全部通过
- Bug 修复

---

## 待办事项池

### Agent 能力增强
- [ ] 上下文管理机制 (Context struct + 压缩策略)
- [x] Skill 机制 (轻量级工具集) ✅ v3.0.0
- [ ] 工具扩展 (更多内置工具)
- [ ] Agent 配置文件支持

### 交互体验
- [x] WebUI 技能管理面板 ✅ v3.0.0
- [x] Terminal UI (TUI) ✅ v2.6.0
- [x] 实时反馈 (SSE 事件流) ✅ v3.1.0
- [x] TUI Gateway 集成 ✅ v3.2.0
- [ ] 命令行客户端

### 稳定性
- [ ] 错误处理增强
- [ ] 日志优化 (结构化日志)
- [ ] 监控指标

---

## 历史迭代

| 版本 | 完成事项 |
|------|----------|
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

*更新时间: 2026-03-21 23:35*
