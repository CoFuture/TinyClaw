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

### v2.7.0 (已完成 ✅)

**完成事项**:
- TUI 模块 - 交互式终端界面
- 会话列表面板 - 显示所有会话
- 消息查看面板 - 显示消息历史
- 输入面板 - 发送消息
- 帮助栏与覆盖层
- 键盘导航支持
- `--tui` / `-t` 命令行标志

**下一步计划**: 见下方

---

### v2.6.0 (已完成 ✅)
**完成事项**:
- TUI 模块 - 交互式终端界面
- 会话列表面板 - 显示所有会话
- 消息查看面板 - 显示消息历史
- 输入面板 - 发送消息
- 帮助栏与覆盖层
- 键盘导航支持
- `--tui` / `-t` 命令行标志

**下一步计划**: 见下方

---

### v2.5.0 (已完成 ✅)
**完成事项**:
- ContextManager 模块 - 上下文管理核心
- Token 估算 - 基于字符数 (~4字符=1 token)
- 上下文截断策略 - 保留系统消息+最近消息
- Provider 格式转换 - Anthropic/OpenAI/Ollama
- Runtime 集成 - 自动截断过长上下文

**下一步计划**: 见下方

---

### v2.4.0 (已完成 ✅)
**完成事项**:
- 会话导出/导入 API
- 活动连接 API
- 工具输入 schema 验证

**下一步计划**: 见下方

---

### v2.0.2 (已完成 ✅)

---

## 当前迭代规划 (v2.8.0)

### 本轮目标
结合项目长期愿景和当前现状，本轮迭代重点：

#### 1. 错误处理增强 (P1) ✅
**目标**: 提供更友好、更详细的错误信息

**具体内容**:
- ✅ 添加结构化错误代码体系（13种错误类型）
- ✅ 错误恢复建议（ErrorRecovery 结构）
- ✅ 错误代码映射（Error → JSON-RPC code）

**新增功能**:
- `ResponseError::with_recovery()` - 带恢复建议的错误响应
- `error_codes` 模块 - 标准化错误代码（INTERNAL_ERROR, NETWORK_ERROR, AGENT_ERROR 等）
- `map_error_to_response()` - 错误类型智能映射，包含：
  - SessionNotFound → SESSION_NOT_FOUND
  - Agent (按原因细分) → AGENT_ERROR/AUTH_ERROR/RATE_LIMIT_ERROR
  - Network (按原因细分) → NETWORK_ERROR
  - Tool → TOOL_ERROR
  - Protocol → PROTOCOL_ERROR
  - Config → CONFIG_ERROR
  - 通用错误 → INTERNAL_ERROR

#### 2. 待办: WebUI 增强 (P1)
**目标**: 完善 Web 管理界面

**具体内容**:
- Dashboard 页面优化
- 会话管理界面
- 实时状态显示

#### 3. 基础修复 (持续)
- `cargo clippy` 无警告 ✅
- `cargo test` 全部通过（153/154，1个预存失败）
- Bug 修复

---

## 待办事项池

### Agent 能力增强
- [ ] 上下文管理机制 (Context struct + 压缩策略)
- [ ] Skill 机制 (轻量级工具集)
- [ ] 工具扩展 (更多内置工具)
- [ ] Agent 配置文件支持

### 交互体验
- [ ] WebUI 完善 (Dashboard + 会话管理)
- [ ] Terminal UI (TUI)
- [ ] 实时反馈 (执行进度、流式输出)
- [ ] 命令行客户端

### 稳定性
- [ ] 错误处理增强
- [ ] 日志优化 (结构化日志)
- [ ] 监控指标

---

## 历史迭代

| 版本 | 完成事项 |
|------|----------|
| v2.0.2 | Request ID 追踪、SQLite 会话恢复、Clippy 清理 |
| v2.0.1 | ... |
| v2.0.0 | ... |

---

*更新时间: 2026-03-21*
