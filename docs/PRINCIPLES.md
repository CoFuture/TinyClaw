# TinyClaw 开发原则 (PRINCIPLES)

## 迭代开发原则

### 每次迭代前
1. 阅读本文档
2. 检查当前代码状态：`cargo clippy`
3. 规划本轮迭代要实现的功能

### 每次迭代中
1. 实现功能模块
2. 确保代码编译通过
3. 运行测试确保功能正常

### 每次提交前 (必须检查)
1. ✅ 运行 `cargo clippy` 并修复所有警告
2. ✅ 确保编译通过：`cargo build`
3. ✅ 运行测试：`cargo test`
4. ✅ 检查是否遵循本文档的代码规范

## 代码质量规范

### 必须遵循
1. **Clippy 检查**: 每次提交前必须运行 `cargo clippy` 并修复所有警告
2. **编译通过**: 不得有任何编译错误
3. **测试通过**: 所有测试必须通过
4. **禁止硬编码**: 配置应通过配置文件或环境变量

### Clippy 警告处理
```bash
# 自动修复
cargo clippy --fix --allow-dirty

# 检查剩余警告
cargo clippy 2>&1 | grep "^warning"
```

### 代码风格
1. 使用 `rustfmt` 格式化代码
2. 使用 4 空格缩进
3. 行长度不超过 100 字符
4. 公共 API 必须有文档注释

### 命名规范
- 模块：`snake_case` (如 `session_manager`)
- 结构体：`PascalCase` (如 `SessionManager`)
- 变量/函数：`snake_case` (如 `handle_request`)
- 常量：`SCREAMING_SNAKE_CASE` (如 `MAX_CONNECTIONS`)

## 模块架构

### OpenClaw 核心模块 (已实现)
1. **Gateway** - WebSocket 服务器 ✅ 已实现
2. **Agent Runtime** - Agent 运行时引擎 ✅ 已实现 (工具调用循环)
3. **Channels** - 消息通道 (Telegram, Discord 等) 🔄 待实现
4. **Sessions** - 会话管理 ✅ 已实现
5. **Tools** - 工具系统 ✅ 已实现
6. **Providers** - AI 模型提供商 ✅ 已实现 (Anthropic/OpenAI/Ollama)

### Agent Runtime 核心功能 (已实现)
1. **Agent Runtime** - Agent 核心运行时
   - ✅ 工具调用循环 (v1.5.0)
   - ✅ 消息处理
   - ✅ 状态管理

2. **AI 集成层** - AI 集成
   - ✅ 多模型支持 (Anthropic, OpenAI, Ollama)
   - ✅ 模型切换
   - ❌ Token 管理 (待实现)

3. **Tools 工具系统**
   - ✅ exec - 执行 Shell 命令
   - ✅ read_file - 读取文件
   - ✅ write_file - 写入文件
   - ✅ list_dir - 列出目录
   - ✅ http_request - HTTP 请求

4. **TUI 终端界面**
   - ✅ 基础 TUI 实现
   - ❌ 实时输出 (待增强)

5. **HTTP API** - HTTP 服务器
   - ✅ Web 管理界面 (admin.html)
   - ✅ REST API 端点
   - ✅ 指标收集 (metrics)
   - ✅ 速率限制 (ratelimit)

## 版本规范

### 版本号格式
- `主版本.次版本.修订号`
- 主版本：不兼容的 API 变更
- 次版本：向后兼容的功能添加
- 修订号：向后兼容的问题修复

### 提交信息格式
```
<type>: <short description>

<long description>

<footer>
```

Type:
- `feat`: 新功能
- `fix`: 错误修复
- `docs`: 文档更新
- `refactor`: 代码重构
- `perf`: 性能优化
- `test`: 测试更新
- `chore`: 构建/工具更新

## 文件结构
```
tiny_claw/
├── src/
│   ├── main.rs           # 入口
│   ├── lib.rs           # 库入口
│   ├── agent/           # Agent 运行时
│   │   ├── mod.rs
│   │   ├── runtime.rs   # Agent 运行时
│   │   ├── client.rs    # AI 客户端
│   │   └── tools.rs     # 工具
│   ├── common/          # 通用
│   ├── config/           # 配置
│   ├── gateway/          # WebSocket 网关
│   └── http/             # HTTP 服务器
├── docs/
│   ├── DESIGN.md        # 设计文档
│   ├── PROJECT.md       # 项目说明
│   ├── ITERATIONS.md    # 迭代记录
│   └── PRINCIPLES.md    # 本文档
└── examples/            # 示例配置
```

## 测试规范

### 必须包含的测试
1. 单元测试：每个公共函数
2. 集成测试：模块间交互

### 覆盖率要求
- **目标**: 代码行覆盖率 ≥ 60%
- **核心模块** (agent/client.rs, gateway/messages.rs, http/routes.rs): ≥ 70%
- **辅助模块** (tools.rs, history.rs, session.rs): ≥ 80%

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test <test_name>

# 查看测试覆盖率
cargo tarpaulin --out Json
```

### 覆盖率检查
- 每次 PR 必须通过覆盖率检查
- 覆盖率下降超过 5% 必须修复

## 文档规范

### 必须包含的文档
1. README.md - 项目介绍
2. docs/PROJECT.md - 详细项目说明
3. docs/ITERATIONS.md - 版本迭代记录
4. 公共 API 文档注释

## 安全规范

1. 不得在代码中硬编码密钥
2. 敏感信息使用环境变量
3. 用户输入必须验证
4. 执行命令时注意注入攻击
