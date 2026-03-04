# DocGuardian — AI 自动化流程文档健康管理工具

> **版本**: 0.5 · **日期**: 2026-02-28
> **平台**: macOS 10.15+ (Intel & Apple Silicon)
> **定位**: 面向 AI 辅助编码工作流的文档健康度管理桌面工具
> **核心理念**: 高纯度上下文 → 低幻觉率 → 更可靠的 AI 输出

---

## 一、产品定位与核心价值

### 1.1 一句话定义
DocGuardian 是一款 macOS 原生桌面应用，它能为任意项目**从零生成 AI 治理框架**，并**持续监控文档健康度**——自动执行记忆垃圾回收、隐式规则提取、知识冲突检测和冗余清理，确保 AI 编码助手始终获得**高纯度的上下文输入**，从根本上降低 AI 的幻觉率和重复性错误。

### 1.2 背景：为什么文档质量直接影响 AI 输出

AI 编码助手（Windsurf / Cursor / Claude Code）的输出质量高度依赖注入上下文的质量。当项目文档出现以下问题时，AI 会系统性地产生错误：

```
文档问题类型            → AI 症状
──────────────────────────────────────────────────────────
完全没有 AI 治理框架    → AI 无约束地生成代码，风格混乱、无规范可循
状态文件无限膨胀        → AI 混淆历史进度与当前任务，重复造轮子
设计文档与代码不一致    → AI 按过时接口生成代码，引入难以发现的 Bug
错误经验未沉淀为规则    → 同类低级错误反复出现，无法被 AI 自我修正
冗余的自然语言规范      → 稀释有效上下文，降低 AI 对关键约束的注意力
```

### 1.3 解决的核心问题（按优先级排序）

| 优先级 | 痛点 | 频率 | DocGuardian 的解法 |
|--------|------|------|-------------------|
| **P0** | 项目完全没有 AI 治理框架（无 AGENTS.md、无 progress.md），AI 编码助手无约束运行 | **100%** · 90% 的现有项目处于此状态 | **Project Initializer** — 扫描项目自动生成 AGENTS.md + progress.md + .docguardian.toml |
| **P0** | `progress.md` / `memory.md` 无限膨胀，已完结任务占据宝贵上下文窗口 | **极高频** · 每个长期项目必然遇到 | **Memory GC** — 自动识别并归档已完结条目，维持工作记忆 ≤ 容量上限 |
| P1 | 同类错误（Redis 未设超时、API 未统一格式）反复出现，经验从未沉淀 | **高频** · 被严重低估的隐性成本 | **Implicit Rule Extractor** — 从失败日志/fix commit 中聚类提取规则，写入 AGENTS.md |
| P2 | 代码接口/数据结构已变更，但三个月前的设计文档无人更新 | **中高频** · 感知滞后但危害深远 | **Knowledge Conflict Detector** — 定期语义比对代码变更与文档，输出冲突报告 |
| P3 | 文档中堆积无法被机器验证的口头规范，稀释上下文信噪比 | **中频** · 长期积累问题 | **Redundancy Pruner** — 识别可被 linter/脚本替代的文档段落，建议删除或重写 |

### 1.4 目标用户

| 用户画像 | 当前状态 | DocGuardian 入口 | 占比（估计） |
|----------|---------|-----------------|-------------|
| **AI 新手开发者** | 刚开始使用 AI 编码助手，项目没有任何治理框架 | Project Initializer（从零搭建） | ~30% |
| **AI 进阶开发者** | 有零散 AGENTS.md / progress.md，但未系统化管理 | Project Initializer（增强现有）+ Memory GC | ~50% |
| **AI 深度用户/团队** | 已有完整治理框架，但文档维护跟不上代码变更速度 | Memory GC + Rule Extractor + Conflict Detector | ~20% |

- **核心用户**：使用 Windsurf / Cursor / Claude Code 等 AI 编码助手的个人开发者和小型团队（2-10 人）
- **项目特征**：中大型代码仓库，文档与代码共存于同一 Git 仓库
- **非目标用户**：纯文档团队（无 AI 编码工具）、大型企业（需要私有化部署方案）

---

## 二、功能设计

### 2.1 功能全景

```
┌──────────────────────────────────────────────────────────────┐
│                      DocGuardian App v0.5                      │
│                                                              │
│  ┌───────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │  Project       │  │  Dashboard   │  │  Settings         │  │
│  │  Manager       │  │  (Health     │  │  & LLM Config     │  │
│  │  + LLM Editor  │  │   Score)     │  │                   │  │
│  │  + Drift Panel │  │              │  │                   │  │
│  └──────┬────────┘  └──────┬───────┘  └───────┬───────────┘  │
│         │                  │                   │              │
│  ┌──────▼──────────────────▼───────────────────▼───────────┐  │
│  │                  Core Engine (Rust)                      │  │
│  │                                                         │  │
│  │  ┌──────────────────┐  ┌─────────────────────────────┐  │  │
│  │  │ F0: Project      │  │ F5: LLM Generation Engine   │  │  │
│  │  │ Initializer      │  │ (12章节 + 流式 + 多轮优化)   │  │  │
│  │  │ (P0 — 冷启动)    │  │ (P0 — 智能生成)             │  │  │
│  │  └──────────────────┘  └─────────────────────────────┘  │  │
│  │  ┌──────────────────┐  ┌─────────────────────────────┐  │  │
│  │  │ F6: Governance   │  │ F1: Memory GC               │  │  │
│  │  │ Drift Detection  │  │ (P0 — 上下文瘦身)            │  │  │
│  │  │ (P0 — 偏差检测)  │  │                             │  │  │
│  │  └──────────────────┘  └─────────────────────────────┘  │  │
│  │  ┌──────────────────┐  ┌─────────────────────────────┐  │  │
│  │  │ F2: Rule         │  │ F3: Conflict Detector       │  │  │
│  │  │ Extractor        │  │ (P2 — 文档债检测)            │  │  │
│  │  │ (P1 — 规则沉淀)  │  │                             │  │  │
│  │  └──────────────────┘  └─────────────────────────────┘  │  │
│  │  ┌──────────────────┐  ┌─────────────────────────────┐  │  │
│  │  │ F4: Redundancy   │  │ Shared: File Watcher +      │  │  │
│  │  │ Pruner           │  │ Tech Detector + Git Service  │  │  │
│  │  │ (P3 — 噪音清理)  │  │ + Orchestrator              │  │  │
│  │  └──────────────────┘  └─────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │          LLM Adapter (Pluggable, Stream/Complete)       │  │
│  │  OpenAI │ Claude │ 智谱GLM │ DeepSeek │ Ollama │ Custom │  │
│  └─────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 七大核心功能详细设计

#### F0: 项目初始化向导 (Project Initializer) — P0

**核心价值**：90% 的项目没有 AI 治理框架，用户添加项目后面对空白的 AGENTS.md 无从下手。Project Initializer 扫描项目内容，5 分钟内生成一套量身定制的文档治理框架，是 DocGuardian 所有功能的**前置条件**，也是产品的**第一个 Aha Moment**。

**触发方式**：
- 在「项目管理」页面添加新项目后，自动扫描并检测治理框架
- 未检测到 AGENTS.md 时弹出确认对话框 → 点击「立即生成」进入全屏编辑器
- 已有 AGENTS.md 时静默添加，不打断用户
- 全屏编辑器内可多轮 LLM 优化、选中文本定向优化

**扫描维度**（v0.5 增强扫描）：
```
项目扫描
├── 技术栈识别
│   ├── 语言：package.json / Cargo.toml / requirements.txt / go.mod
│   ├── 框架：依赖列表推断 → React / Vue / Django / Axum / Express 等
│   └── 工具链：.eslintrc / .prettierrc / clippy.toml / .editorconfig
├── 依赖树深度解析 [v0.5 新增]
│   ├── npm dependencies + devDependencies（名称+版本）
│   ├── Cargo.toml [dependencies] + [dev-dependencies]（名称+版本）
│   └── 注入 LLM prompt，生成针对具体依赖的治理规则
├── CI/CD 配置解析 [v0.5 新增]
│   ├── GitHub Actions workflows（触发器+步骤摘要）
│   ├── GitLab CI（检测 .gitlab-ci.yml）
│   └── 注入 LLM prompt，生成 CI 相关治理规则
├── 现有文档发现
│   ├── README.md → 提取项目描述、约定、技术栈声明
│   ├── docs/ 目录 → 识别已有架构/设计文档
│   ├── CONTRIBUTING.md → 提取协作规范
│   └── CHANGELOG.md → 提取版本发布惯例
├── 代码库特征
│   ├── 目录结构 → 推断模块划分和命名约定
│   ├── Git log 近 100 条 → 分析 fix/revert 频率，推断常见痛点
│   └── 文件统计 → 代码行数、文档行数、测试覆盖率
└── AI 工具检测
    ├── .cursorrules / .windsurfrules → 已有 AI 约束规则
    ├── .ai/ 目录 → 已有上下文文件（progress.md, memory.md）
    └── AGENTS.md → 是否已存在（存在则进入"增强"模式）
```

**生成产物**：

| 文件 | 内容 | 生成策略 |
|------|------|---------|
| `AGENTS.md` | AI 编码宪法 | 技术栈专属约束 + 目录规范 + 禁止事项 + 从 linter 配置反向翻译 |
| `.ai/progress.md` | 当前任务追踪 | 空模板，预填项目模块结构框架 |
| `.ai/context-priority.md` | 上下文注入优先级 | 按重要性排序的文件推荐列表 |
| `.docguardian.toml` | DocGuardian 配置 | 路径自动映射到实际文件位置 |
| `docs/adr/0001-initial-architecture.md` | 架构现状快照（可选） | 当前项目结构 + 技术栈 + 依赖关系 |

**AGENTS.md 生成策略**（v0.5 双引擎架构）：

```
┌───────────────────────────────────────────────────────┐
│                    生成流程                              │
│                                                       │
│  项目扫描 ──→ 确定性层（规则引擎）──→ 基础 AGENTS.md    │
│                     │                                 │
│                     ▼                                 │
│              LLM 智能层 ──→ 12 章节完整 AGENTS.md      │
│                     │                                 │
│                     ▼                                 │
│              多轮优化 ←─→ 用户反馈                     │
│              选中定向优化                               │
└───────────────────────────────────────────────────────┘
```

**确定性层**（无需 LLM，<5 秒）：
1. 从 linter 配置反向生成规则
2. 从技术栈推断最佳实践
3. 从目录结构推断命名约定
4. 从 Git log 推断痛点约束
5. Fallback：即使无 LLM 配置，仍可生成基础 AGENTS.md

**LLM 智能层**（v0.5，流式生成）：
1. 将扫描结果（技术栈 + 依赖树 + CI 配置 + Git 统计）注入 prompt
2. LLM 按 **12 章节体系**生成完整 AGENTS.md：
   - 头部元信息 / Scope（权限边界）/ Don't（安全红线）/ Ask First
   - Commands（三级验证命令）/ Project Structure / Verification（验证清单）
   - Memory / Examples / When Stuck / Governance Loop（治理闭环）/ Changelog
3. 流式输出 + 实时行号 + 智能自动滚动
4. 多轮会话优化：保持上下文，用户可连续输入优化指令
5. 选中定向优化：选中文本 → 浮动按钮 → 自动填充优化输入框 → 定向修改

**用户交互流程**（v0.5）：
```
1. 用户进入「项目管理」页面（应用默认首页）
   ┌──────────────────────────────────────────────────────┐
   │  项目管理                              [+ 添加项目]  │
   │                                                      │
   │  ● my-web-app    /Users/cc/projects/...               │
   │    当前 · 🟢 已治理 v1.0 · 健康评分 87                │
   │                                                      │
   │  ○ api-service   /Users/cc/work/api/...               │
   │    🟠 需更新 (3) · 健康评分 72                         │
   │    ┌─── 偏差详情 ──────────────────────────────┐      │
   │    │ 🔴 框架 Fastify 未在 AGENTS.md 中提及     │      │
   │    │ 🟠 工具 eslint 未在 AGENTS.md 中提及      │      │
   │    │ ⚪ 缺少 Verification 章节                 │      │
   │    │         [✨ AI 生成更新建议]               │      │
   │    └──────────────────────────────────────────┘      │
   │                                                      │
   │  ○ new-project   /Users/cc/work/new/...               │
   │    🟡 未初始化                                        │
   └──────────────────────────────────────────────────────┘

2. 点击「添加项目」→ 系统文件夹选择器 → 确认
3. 自动扫描技术栈 + 依赖树 + CI 配置 + 检测治理框架
4. 分支逻辑：
   ├── 已有 AGENTS.md → 直接添加 + 自动执行偏差检测（显示治理状态徽标）
   └── 未检测到治理框架 → 弹出确认对话框：

   ┌──────────────────────────────────────────────┐
   │    ⚠ 未检测到 AI 治理框架                     │
   │    是否立即使用 AI 智能生成？                  │
   │                                              │
   │    [暂时跳过]         [✨ 立即生成]            │
   └──────────────────────────────────────────────┘

5. 点击「立即生成」→ 进入全屏编辑器：

   ┌──────────────────────────────────────────────┐
   │ ← my-app  📄Rust,TS 🔧Tauri,React  [重新生成]│
   ├──────────────────────────────────────────────┤
   │ 1  # AGENTS.md                               │
   │ 2                                             │
   │ 3  **版本**: 1.0                              │
   │ 4  **更新日期**: 2026-02-28                    │
   │ 5  ...                                        │
   │ 6  ## 1. Scope（权限边界）                    │
   │ 7  ...                                        │
   │    （12 章节流式实时渲染）                      │
   │                                               │
   │              [优化选中内容]  ← 选中文本时浮现   │
   ├──────────────────────────────────────────────┤
   │ 📝 [输入优化意见…                    ] [发送] │
   │                          [🛡 写入治理框架]     │
   └──────────────────────────────────────────────┘

6. 用户可在编辑器中：
   a. 直接编辑文本
   b. 输入优化意见 → LLM 多轮会话优化（保持上下文）
   c. 选中某段文本 → 点击浮动按钮 → 自动填充优化输入框 → 定向优化
7. 满意后点击「写入治理框架」→ AGENTS.md + progress.md + .docguardian.toml 写入磁盘
```

**成功指标**：
- 从「添加项目」到「看到首个健康评分」≤ 5 分钟
- 生成的 AGENTS.md 中 ≥ 70% 的规则被用户保留（未删除）
- 用户在 7 天内未手动重写生成内容 = 质量达标

---

#### F1: 记忆垃圾回收器 (Memory GC) — P0

**核心价值**：AI 上下文窗口是有限资源。`progress.md` 里保留 3 个月前已完成的 Epic，等同于让 AI 在垃圾堆里找当前任务。Memory GC 自动清理这些"内存泄漏"。

**触发方式**：
- 文件保存时自动检测（File Watcher，实时）
- 定时扫描（可配置，默认每 30 分钟）
- 手动触发（立即执行）

**工作流程**：
```
1. 读取目标文件（如 progress.md, .ai/progress.md）
2. 解析结构：识别 Epic / Milestone / Task / Done 块
3. 判定状态：关键词匹配（✅ DONE completed shipped）+ LLM 语义判断
4. 生成归档摘要：已完结条目压缩为 1-2 行精简描述（保留可追溯性）
5. 执行操作：
   a. 将摘要追加到 .ai/archive/{date}-archive.md
   b. 从原文件中移除已完结条目
   c. 可选：生成 Git commit（需 auto_commit = true）
6. 验证：确保原文件行数 ≤ 容量上限，否则重新触发
```

**可配置项**：
- 监控文件路径列表（支持 glob）
- 容量上限（默认 100 行）
- 归档目录路径
- 是否自动 Git commit（默认 false，遵循 AGENTS.md 约束）

**成功指标**：状态文件长期维持在容量上限以内，AI 对"当前任务"的识别准确率提升

---

#### F2: 隐式规则提取 (Implicit Rule Extractor) — P1

**核心价值**：`AGENTS.md` 里的规则通常是被动写入的——某个 bug 修了、某个 PR 被打回，才会有人想起来补一条规则。但大多数团队从未系统化地做这件事，导致 AI 反复踩同一个坑。Rule Extractor 将这个过程自动化。

**触发方式**：
- 监控 `.ai/failures.jsonl` 文件变更（实时）
- 定时扫描 Git log 中的 fix/revert/bug commit（可配置）
- 手动触发

**工作流程**：
```
1. 采集失败信号：
   a. .ai/failures.jsonl 中的结构化错误记录
   b. Git log 中含 "fix", "revert", "bug", "mistake" 的 commit message
   c. PR/MR 评论中的 review 意见（需 GitHub/GitLab API，可选）
2. 向量 Embedding + 语义聚类：将相似失败信号归为同一簇
3. 频率过滤：同一聚类出现 ≥ N 次（默认 3 次）触发规则生成
4. LLM 生成规则建议：
   - 自然语言规则描述（适合写入 AGENTS.md 的格式）
   - Golden Example：正确用法代码片段
   - 反例：触发此错误的典型错误代码
   - 建议插入的目标文件（AGENTS.md / best_practices/）
5. 用户审批确认 → 批准后自动追加写入目标文件
```

**规则命中率追踪**（Phase 3 新增）：
- 记录每条规则的"写入时间"
- 定期扫描近期 commit，检测此类错误是否仍在发生
- 在 Rule 页面显示：`规则有效` / `规则未生效（仍有违反）`

**可配置项**：
- 失败日志路径
- 最小聚类频率（默认 3）
- 目标规则文件列表

---

#### F3: 知识冲突检测 (Knowledge Conflict Detector) — P2

**核心价值**：AI 编码助手本身不会主动回头检查 3 个月前写的架构文档是否还准确。DocGuardian 做这个异步、定期的检查，防止"文档债"悄悄积累。

**触发方式**：
- 定时扫描（默认每天一次，对比最近 N 个 commit）
- Git hook（`post-commit` / `pre-push`）
- 手动触发（选择 commit range）
- CI/CD 集成（输出 JSON 报告）

**工作流程**：
```
1. 获取代码变更 diff（指定 commit range）
2. LLM 提取变更语义摘要：改了哪些接口/数据结构/业务流程
3. 在文档向量索引中检索语义相关段落（Top-K）
4. 对每个候选段落，LLM 判断：是否与新代码逻辑矛盾？
5. 输出冲突报告：
   - 冲突文件路径 + 行号范围
   - 冲突描述（代码变更 vs 文档描述）
   - 建议修正内容
   - 严重等级（high / medium / low）
6. 可选：自动提交修正 PR
```

**向量索引策略**：
- 首次接入项目时全量索引 `docs/` 目录
- 文件变更时增量更新（基于文件 hash 检测）
- 本地向量数据库：SQLite + `sqlite-vss`（Phase 1 降级为关键词检索）
- Embedding 模型：本地 `all-MiniLM-L6-v2`（~80MB），可配置为 API

---

#### F4: 冗余文档清理 (Redundancy Pruner) — P3

**核心价值**：每一段"所有代码必须使用 2 空格缩进"的文字说明，只要 `.prettierrc` 里配了就是噪音。这类无法被机器验证的口头规范会稀释 AI 上下文的信噪比。Pruner 识别并清除它们。

**触发方式**：
- 项目首次接入时全量扫描
- 定时扫描（默认每周一次）
- 手动触发

**工作流程**：
```
1. 扫描所有 .md / .txt / .rst 文档
2. 逐段落分析，LLM 分类为：
   a. LINTER  — 可被 linter 规则替代（ESLint / Prettier / clippy）
   b. SCRIPT  — 可被 shell 脚本 / Makefile / CI pipeline 替代
   c. STALE   — 过时的、与当前代码不匹配的描述
   d. KEEP    — 纯信息性/决策性内容（保留）
3. 对 a/b/c 类别生成处理建议：
   - LINTER：建议删除 + 给出具体的 linter rule 名称
   - SCRIPT：建议删除 + 给出 Makefile target 或 CI step 模板
   - STALE：建议加 [DEPRECATED] 标记或重写
4. 列表展示，用户逐条审批（接受 / 忽略 / 标记 DEPRECATED）
```

---

#### F5: LLM 智能生成引擎 (LLM Generation Engine) — P0 [v0.5 新增]

**核心价值**：将 AGENTS.md 的生成从「确定性规则拼接」升级为「LLM 智能生成 + 人机协同优化」，输出质量从"可用"跃升为"高质量、前沿治理体系"。

**核心能力**：
```
1. 流式生成 — SSE 实时输出，前端逐字渲染 + 行号同步
2. 12 章节体系 — 覆盖 Scope/Don't/Ask First/Commands/Structure/Verification/
                  Memory/Examples/When Stuck/Governance Loop/Changelog
3. 多轮会话 — 后端 ConversationState (Mutex<Vec<ChatMessage>>)，保持上下文
4. 选中定向优化 — 选中文本 → 浮动按钮 → 自动填充优化输入 → 精准修改
5. 增强扫描注入 — 依赖树 + CI 配置 → prompt，生成针对具体依赖的规则
6. System Prompt — 五大治理理念（权限最小化/可验证性/可回滚性/闭环治理/零信任外部）
```

**技术实现**：
- 后端：`src-tauri/src/commands/llm.rs`
  - `generate_agents_md_llm` — 扫描 → prompt 构建 → 流式/完整调用 → 会话初始化
  - `refine_agents_md` — 追加用户消息 → LLM 优化 → 会话历史更新
  - `ConversationState` — Tauri managed state，线程安全的会话管理
- 前端：`src/pages/ProjectsPage.tsx`（InitWizardModal 部分）
  - 流式渲染 + 智能自动滚动（用户滚动时暂停，回到底部时恢复）
  - `selectedText` + `refineInputRef` 实现选中定向优化

**LLM 适配**：
- 支持 OpenAI / Claude / 智谱GLM / Ollama / DeepSeek / 自定义 API
- 统一适配层：`stream_or_complete_messages()` 函数处理流式/非流式
- 配置持久化：`~/.docguardian/llm_config.json`

---

#### F6: 治理框架偏差检测 (Governance Drift Detection) — P0 [v0.5 新增]

**核心价值**：项目在持续演进，但 AGENTS.md 往往停留在初次生成的状态。F6 自动检测 AGENTS.md 与项目现状之间的偏差（Drift），提醒用户更新治理框架，并可通过 LLM 生成增量更新建议。

**检测维度**：
```
偏差检测
├── 语言偏差 — 扫描检测到的语言 vs AGENTS.md 中提及的语言
├── 框架偏差 — 扫描检测到的框架 vs AGENTS.md 中提及的框架
├── 目录偏差 — 顶层目录 vs Project Structure 章节中的目录
├── 工具偏差 — 检测到的 linter/formatter vs AGENTS.md 中提及的工具
├── 章节完整性 — 7 个关键章节是否存在
│   (Scope / Don't / Ask First / Commands / Verification / Governance Loop / Changelog)
├── CI 偏差 — 有 CI 配置但 AGENTS.md 未提及 CI 规则
└── 测试偏差 — 有测试目录但 AGENTS.md 未包含测试命令
```

**前端展示**：
- 项目列表三态徽标：
  - 🟡 **未初始化** — 无 AGENTS.md
  - 🟠 **需更新 (N)** — 检测到 N 项偏差，可点击展开偏差详情
  - 🟢 **已治理 vX.X** — 无偏差，显示当前版本号
- 偏差详情面板：按严重级别（高/中/低）列出每项偏差
- **「AI 生成更新建议」按钮** — 将偏差摘要发送给 LLM，生成增量更新建议（非全量重写）

**技术实现**：
- 后端：`src-tauri/src/commands/llm.rs`
  - `check_governance_freshness` — 确定性扫描对比，返回 `GovernanceDrift`
  - `suggest_governance_updates` — 基于偏差调用 LLM 生成增量建议
- 前端：`src/pages/ProjectsPage.tsx`
  - `driftMap` 状态 — 缓存每个项目的偏差结果
  - 可展开偏差详情面板 + LLM 建议渲染

---

### 2.3 辅助功能

| 功能 | 优先级 | 描述 |
|------|--------|------|
| **文档健康度仪表盘** | P0 | 实时显示：健康评分、记忆占用率、冲突数、规则建议数、上次 GC/初始化时间 |
| **上下文注入优先级排序** | P1 | 对项目文档按"重要性 × 时效性"打分，生成 `.ai/context-priority.md`，告知用户哪些文档应优先注入 AI 上下文 |
| **macOS Menu Bar 常驻** | P1 | 托盘图标显示健康评分，后台静默运行，发现问题时通知 |
| **多项目管理** | P2 | 支持同时监控多个 Git 仓库，侧边栏快速切换 |
| **macOS 原生通知** | P2 | 报告冲突/规则建议/GC 完成/初始化完成，点击直达对应功能面板 |
| **CLI 模式** | P3 | `docguardian init` / `docguardian gc` / `docguardian scan` / `docguardian report`，供 CI/CD 集成 |
| **LLM 成本追踪** | P3 | 显示每次操作消耗的 Token 数和预估费用（API 模式下） |

> **注**：原"项目配置向导"已升级为核心功能 F0（Project Initializer），不再作为辅助功能。

---

## 三、技术架构

### 3.1 技术栈选型

| 层级 | 技术选型 | 理由 |
|------|---------|------|
| **桌面框架** | **Tauri 2.0** (Rust + WebView) | 原生 macOS 体验、极小体积（~5MB）、安全沙箱、Rust 性能 |
| **前端 UI** | **React + TypeScript + TailwindCSS + shadcn/ui** | 现代 UI、组件生态成熟、开发效率高 |
| **核心引擎** | **Rust** | 文件监控（notify crate）、Git 操作（git2 crate）、高性能文本处理 |
| **向量数据库** | **SQLite + sqlite-vss** | 零依赖、嵌入式、macOS 原生支持 |
| **本地 Embedding** | **ONNX Runtime + all-MiniLM-L6-v2** | 离线可用、隐私友好、~80MB 模型 |
| **LLM 适配层** | **Pluggable Adapter (Stream/Complete)** | 支持 OpenAI / Claude / 智谱GLM / DeepSeek / Ollama(本地) / 自定义 API |
| **持久化** | **SQLite** (应用状态 + 向量索引 + 配置) | 单文件数据库、可靠、零运维 |
| **CLI** | **clap** (Rust) | 原生 CLI 框架，编译为独立二进制 |

### 3.2 系统架构图

```
┌───────────────────────────────────────────────────────────────┐
│                      Tauri Shell (macOS)                       │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │               React Frontend (WebView)                  │  │
│  │                                                         │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────┐ ┌──────────────┐ │  │
│  │  │Projects  │ │Dashboard │ │ GC /   │ │  Rules /     │ │  │
│  │  │ Manager  │ │ (Health) │ │Conflict│ │  Pruner /    │ │  │
│  │  │+LLMEditor│ │          │ │ Views  │ │  Settings    │ │  │
│  │  │+DriftPane│ │          │ │        │ │  +LLMConfig  │ │  │
│  │  └──────────┘ └──────────┘ └────────┘ └──────────────┘ │  │
│  └──────────────────────┬──────────────────────────────────┘  │
│                         │ Tauri IPC (useTauriCommand hook)     │
│  ┌──────────────────────▼──────────────────────────────────┐  │
│  │               Rust Backend (Core Engine)                │  │
│  │                                                         │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │              Orchestrator                        │   │  │
│  │  │  (调度七大核心功能 + 管理生命周期 + 调度定时任务)   │   │  │
│  │  └──┬───────┬──────┬──────┬──────────┬──────────┬───┘   │  │
│  │     │       │      │      │          │          │       │  │
│  │  ┌──▼────┐┌─▼──┐┌──▼──┐┌──▼───┐ ┌───▼────┐ ┌───▼────┐  │  │
│  │  │F0:Init││F5: ││F6:  ││F1:GC │ │F2:Rule │ │F3:Conf.│  │  │
│  │  │+Scan  ││LLM ││Drift││      │ │Extract │ │Detect  │  │  │
│  │  └──┬────┘│Gen ││Det. │└──┬───┘ └───┬────┘ └───┬────┘  │  │
│  │     │     └─┬──┘└──┬──┘   │         │          │       │  │
│  │     │       │      │      │    ┌────▼────┐     │       │  │
│  │     │       │      │      │    │F4:Pruner│     │       │  │
│  │     │       │      │      │    └────┬────┘     │       │  │
│  │  ┌──▼───────▼──────▼──────▼─────────▼──────────▼────┐  │  │
│  │  │              Shared Services                     │  │  │
│  │  │                                                  │  │  │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │  │  │
│  │  │  │  File    │ │  Git     │ │  LLM Adapter     │ │  │  │
│  │  │  │  Watcher │ │  Service │ │  (Stream/Complete │ │  │  │
│  │  │  │  (notify)│ │  (git2)  │ │   + Conversation)│ │  │  │
│  │  │  └──────────┘ └──────────┘ └──────────────────┘ │  │  │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │  │  │
│  │  │  │  Vector  │ │  SQLite  │ │  Tech Detector   │ │  │  │
│  │  │  │  Index   │ │  Store   │ │  (确定性层 +     │ │  │  │
│  │  │  │(sqlite-  │ │          │ │   依赖树 + CI)   │ │  │  │
│  │  │  │  vss)    │ │          │ │                  │ │  │  │
│  │  │  └──────────┘ └──────────┘ └──────────────────┘ │  │  │
│  │  │  ┌──────────┐                                   │  │  │
│  │  │  │ Markdown │                                   │  │  │
│  │  │  │ Parser   │                                   │  │  │
│  │  │  │(pulldown)│                                   │  │  │
│  │  │  └──────────┘                                   │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │               CLI Binary (clap)                         │  │
│  │  docguardian init | scan | gc | detect | extract | prune│  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘

External:
  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
  │ OpenAI   │ │ Claude   │ │ 智谱GLM  │ │ DeepSeek │ │ Ollama   │
  │ API      │ │ API      │ │ API      │ │ API      │ │ (Local)  │
  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘
```

### 3.3 数据模型

```sql
-- 项目表
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    root_path   TEXT NOT NULL UNIQUE,
    config      TEXT NOT NULL,  -- JSON: 四层模型路径配置、GC 阈值等
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- 文档索引表
CREATE TABLE documents (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    rel_path    TEXT NOT NULL,
    layer       TEXT NOT NULL,  -- 'rule' | 'state' | 'contract' | 'decision'
    hash        TEXT NOT NULL,  -- 文件内容 SHA256，用于变更检测
    line_count  INTEGER NOT NULL,
    last_scanned INTEGER NOT NULL,
    health      TEXT NOT NULL,  -- 'healthy' | 'warning' | 'conflict' | 'stale'
    UNIQUE(project_id, rel_path)
);

-- 向量索引表 (sqlite-vss 管理)
CREATE VIRTUAL TABLE doc_chunks_vss USING vss0(
    embedding(384)  -- MiniLM 维度
);

-- 文档分块表
CREATE TABLE doc_chunks (
    id          INTEGER PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    chunk_text  TEXT NOT NULL,
    start_line  INTEGER NOT NULL,
    end_line    INTEGER NOT NULL,
    metadata    TEXT  -- JSON
);

-- 冲突记录表
CREATE TABLE conflicts (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    document_id TEXT NOT NULL REFERENCES documents(id),
    chunk_id    INTEGER REFERENCES doc_chunks(id),
    commit_hash TEXT,
    description TEXT NOT NULL,
    suggestion  TEXT,          -- 建议修正内容
    status      TEXT NOT NULL, -- 'open' | 'resolved' | 'dismissed'
    created_at  INTEGER NOT NULL
);

-- 规则建议表
CREATE TABLE rule_suggestions (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    cluster_id  TEXT,
    pattern     TEXT NOT NULL,  -- 聚类后的错误模式描述
    frequency   INTEGER NOT NULL,
    suggestion  TEXT NOT NULL,  -- 建议的规则内容
    target_file TEXT NOT NULL,  -- 建议写入的文件
    status      TEXT NOT NULL,  -- 'proposed' | 'accepted' | 'rejected'
    created_at  INTEGER NOT NULL
);

-- GC 历史表
CREATE TABLE gc_history (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    source_file TEXT NOT NULL,
    archive_file TEXT NOT NULL,
    items_archived INTEGER NOT NULL,
    lines_before INTEGER NOT NULL,
    lines_after INTEGER NOT NULL,
    executed_at INTEGER NOT NULL
);

-- 初始化历史表（Project Initializer）
CREATE TABLE init_history (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES projects(id),
    mode          TEXT NOT NULL,  -- 'fresh' | 'enhance'
    tech_stack    TEXT NOT NULL,  -- JSON: 检测到的技术栈信息
    files_generated TEXT NOT NULL, -- JSON: 生成的文件列表及路径
    rules_total   INTEGER NOT NULL, -- 生成的规则总数
    rules_accepted INTEGER,        -- 用户确认保留的规则数
    scan_duration_ms INTEGER,      -- 扫描耗时（毫秒）
    executed_at   INTEGER NOT NULL
);
```

### 3.4 目录结构

```
ai_doc_manager/
├── docs/
│   ├── design/
│   │   └── product-spec.md          # 本文件
│   └── adr/                         # 架构决策记录
├── src-tauri/                       # Rust 后端
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                  # Tauri 入口
│   │   ├── commands/                # Tauri IPC 命令
│   │   │   ├── mod.rs
│   │   │   ├── project.rs           # 项目 CRUD（add/remove/list/health）
│   │   │   ├── init.rs              # 初始化命令（scan_project/confirm_init）
│   │   │   ├── llm.rs               # LLM 生成/优化/偏差检测/更新建议 (F5+F6)
│   │   │   ├── gc.rs                # Memory GC 命令
│   │   │   ├── conflict.rs          # 冲突检测命令
│   │   │   ├── rule.rs              # 规则提取命令
│   │   │   └── prune.rs             # 冗余清理命令
│   │   ├── core/                    # 核心业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── orchestrator.rs      # 调度器
│   │   │   ├── project_initializer.rs  # 项目初始化向导（F0 确定性层）
│   │   │   ├── memory_gc.rs         # 记忆 GC
│   │   │   ├── conflict_detector.rs # 知识冲突检测
│   │   │   ├── rule_extractor.rs    # 隐式规则提取
│   │   │   └── redundancy_pruner.rs # 冗余清理
│   │   ├── services/                # 共享服务
│   │   │   ├── mod.rs
│   │   │   ├── file_watcher.rs      # 文件监控
│   │   │   ├── git.rs               # Git 操作
│   │   │   ├── tech_detector.rs     # 技术栈检测 + 依赖树 + CI 配置解析
│   │   │   ├── llm/                 # LLM 适配层
│   │   │   │   ├── mod.rs
│   │   │   │   ├── adapter.rs       # Trait 定义 (stream_or_complete_messages)
│   │   │   │   ├── openai.rs        # 兼容 OpenAI/智谱GLM/DeepSeek/自定义
│   │   │   │   ├── claude.rs
│   │   │   │   └── ollama.rs
│   │   │   ├── vector_index.rs      # 向量索引
│   │   │   ├── markdown_parser.rs   # Markdown 解析
│   │   │   └── db.rs                # SQLite 数据层
│   │   └── models/                  # 数据结构
│   │       ├── mod.rs
│   │       ├── project.rs
│   │       ├── document.rs
│   │       ├── conflict.rs
│   │       └── rule.rs
│   └── tauri.conf.json
├── src/                             # React 前端
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   └── Header.tsx
│   │   ├── projects/                    # 项目管理（F0）
│   │   │   ├── ProjectList.tsx         # 项目列表（切换/删除）
│   │   │   ├── AddProjectInput.tsx     # 添加项目输入栏
│   │   │   └── InitWizardModal.tsx     # 初始化向导弹窗（检测 + 预览 + 规则勾选）
│   │   ├── dashboard/
│   │   │   ├── HealthScore.tsx        # 文档健康度评分
│   │   │   ├── MetricsCards.tsx       # 关键指标卡片
│   │   │   └── RecentActivity.tsx     # 最近活动
│   │   ├── gc/
│   │   │   ├── GCPanel.tsx            # GC 操作面板
│   │   │   └── ArchiveHistory.tsx     # 归档历史
│   │   ├── conflicts/
│   │   │   ├── ConflictList.tsx       # 冲突列表
│   │   │   └── ConflictDiff.tsx       # 冲突 Diff 视图
│   │   ├── rules/
│   │   │   ├── RuleSuggestions.tsx    # 规则建议列表
│   │   │   └── GoldenExample.tsx     # Golden Example 展示
│   │   ├── pruner/
│   │   │   └── PruneReport.tsx       # 冗余报告
│   │   └── settings/
│   │       ├── ProjectConfig.tsx      # 项目配置
│   │       └── LLMConfig.tsx         # LLM 配置
│   ├── pages/                       # 页面组件
│   │   ├── ProjectsPage.tsx         # 项目管理（默认首页 + InitWizardModal）
│   │   ├── DashboardPage.tsx        # 仪表盘
│   │   ├── GCPage.tsx               # 记忆回收
│   │   ├── ConflictsPage.tsx        # 冲突检测
│   │   ├── RulesPage.tsx            # 规则提取
│   │   ├── PrunerPage.tsx           # 冗余清理
│   │   └── SettingsPage.tsx         # 设置
│   ├── hooks/
│   │   └── useTauriCommand.ts      # Tauri IPC Hook
│   ├── stores/                     # Zustand 状态管理
│   │   ├── projectStore.ts
│   │   └── uiStore.ts
│   └── lib/
│       └── utils.ts
├── package.json
├── tsconfig.json
├── tailwind.config.ts
├── vite.config.ts
├── AGENTS.md                        # AI 编码宪法
└── README.md
```

---

## 四、用户交互设计

### 4.1 核心界面布局

```
┌─────────────────────────────────────────────────────────┐
│  🛡 DocGuardian      [my-saas-app ▼ → 项目管理]  v0.1.0 │
├───────────┬─────────────────────────────────────────────┤
│           │                                             │
│  � 项目管理 │  （应用默认首页，项目列表 + 添加 + 切换）   │
│           │                                             │
│  📊 仪表盘   │  ┌────────────────────────────────┐       │
│           │  │  健康评分 87 / 100  ██████████░░░ │       │
│  🧹 记忆回收 │  └────────────────────────────────┘       │
│           │                                             │
│  ⚡ 冲突检测 │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐    │
│           │  │  12  │ │   3  │ │   1  │ │   5  │    │
│  📏 规则提取 │  │ 文档 │ │ 冲突 │ │ 规则 │ │ 过期 │    │
│           │  └──────┘ └──────┘ └──────┘ └──────┘    │
│  ✂️ 冗余清理 │                                          │
│           │  最近活动                                   │
│  ─────────│  ─────────────────────────────────          │
│  ⚙ 设置    │  🔴 冲突: docs/auth.md 与代码实现不一致     │
│           │  🟢 GC: progress.md 归档 8 条 → 62 行       │
│           │  🟡 规则: 建议新增 Redis 锁使用规范           │
│           │                                             │
└───────────┴─────────────────────────────────────────────┘
```

### 4.2 交互流程

**首次接入项目**：
1. 用户在「项目管理」页面点击「添加项目」
2. 输入项目根目录路径，点击确认
3. DocGuardian 自动扫描技术栈 + 检测 AI 治理框架
4. 如果未检测到 AGENTS.md → 弹出初始化向导 Modal
5. 用户在 Modal 中预览生成文件、逐条勾选规则
6. 点击「生成治理框架」写入文件 / 或「暂时跳过」
7. 项目出现在列表中，可切换为当前活跃项目

**日常使用**：
1. 应用常驻 macOS Menu Bar（托盘图标）
2. 文件变更时自动后台检测
3. 发现问题时通过 macOS 通知提醒用户
4. 用户点击通知进入对应功能面板处理

---

## 五、项目配置文件格式

每个被管理的项目根目录下会生成 `.docguardian.toml`：

```toml
[project]
name = "my-saas-app"

[layers]
# 宪法与边界层
rule_paths = ["AGENTS.md", ".cursorrules", "AI_GOVERNANCE_CONSTITUTION.md"]

# 状态与记忆层
state_paths = [".ai/progress.md", ".ai/memory.md"]
state_capacity = 100  # 行数上限
archive_dir = ".ai/archive"

# 契约与蓝图层
contract_paths = ["docs/design/*.md", "proto/**/*.proto"]

# 决策快照层
decision_paths = ["docs/adr/*.md"]

[gc]
enabled = true
interval_minutes = 30
auto_commit = false
commit_message_template = "docs(gc): archive completed items from {source}"

[conflict_detection]
enabled = true
watch_branches = ["main", "develop"]
exclude_paths = ["docs/adr/*"]  # ADR 是只读的，不检测冲突

[rule_extraction]
enabled = true
failure_log = ".ai/failures.jsonl"
min_frequency = 3
target_files = ["AGENTS.md", "best_practices/*.md"]

[pruner]
enabled = true
interval_days = 7
scan_extensions = ["md", "txt", "rst"]

[llm]
provider = "ollama"         # "openai" | "claude" | "ollama" | "custom"
model = "llama3.1:8b"
base_url = "http://localhost:11434"
# api_key = "sk-..."       # 仅 openai/claude 需要，建议用环境变量
max_tokens_per_request = 4096
```

---

## 六、开发路线图

> 按痛点优先级驱动，先交付高频痛点功能，再扩展智能分析能力。

### Phase 1 — MVP (4 周)：解决 P0 痛点（冷启动 + 上下文瘦身）
> 目标：用户添加任意项目后，5 分钟内获得完整的 AI 治理框架 + 自动化记忆管理

- [x] 项目脚手架搭建（Tauri + React + Rust）
- [x] **F0: Project Initializer**：确定性扫描 → 生成 AGENTS.md + progress.md + .docguardian.toml
- [x] 项目管理：添加/移除/切换，初始化向导集成（ProjectsPage + InitWizardModal）
- [x] **F5: LLM 智能生成引擎**：12 章节体系 + 流式生成 + 多轮会话 + 选中定向优化
- [x] **F6: 治理框架偏差检测**：三态徽标 + 偏差详情面板 + AI 更新建议
- [x] **增强扫描**：依赖树深度解析（npm + Cargo）+ CI 配置解析（GitHub Actions / GitLab）
- [x] LLM 适配层：OpenAI / Claude / 智谱GLM / DeepSeek / Ollama / 自定义 API（Stream + Complete）
- [x] 设置面板：LLM 配置（多 Provider）+ `.docguardian.toml` 管理
- [ ] 文档健康度仪表盘（记忆占用率 + 基础指标 + 初始评分）
- [ ] **Memory GC 核心功能**：关键词检测 + LLM 语义判断 + 归档写入
- [ ] File Watcher：监控状态文件变更，自动触发 GC

### Phase 2 — 规则引擎 (4 周)：解决 P1 痛点
> 目标：让 AI 不再反复犯同一类错误

- [ ] **Implicit Rule Extractor**：失败日志聚类 + 规则生成 + 用户审批写入
- [ ] 规则命中率追踪：记录规则有效性，标记"仍有违反"的规则
- [ ] macOS Menu Bar 常驻 + 原生通知集成
- [ ] 上下文注入优先级排序面板

### Phase 3 — 冲突检测 (4 周)：解决 P2 痛点
> 目标：防止"文档债"悄悄积累

- [ ] 向量索引构建（Phase 1 降级为关键词，此阶段升级为 sqlite-vss）
- [ ] **Knowledge Conflict Detector**：Git diff → LLM 语义比对 → 冲突报告
- [ ] Git hook 自动安装（`post-commit` 触发扫描）
- [ ] CI/CD 集成：`docguardian scan --output json`
- [ ] LLM 成本追踪面板

### Phase 4 — 智能清理与生态 (4 周)：解决 P3 + 生态扩展
> 目标：完善功能闭环，接入开发者工具生态

- [ ] **Redundancy Pruner**：段落分类 + 替代方案建议
- [ ] CLI 完整模式：`docguardian gc` / `docguardian report` / `docguardian prune`
- [ ] GitHub / GitLab PR 评论集成（Rule Extractor 信号源）
- [ ] VS Code / Cursor 插件（快捷唤起 DocGuardian 面板）
- [ ] 多项目批量健康报告
- [ ] 自动更新（Tauri Updater）

---

## 七、关键设计决策

### D1: 为什么选 Tauri 而不是 Electron？
- **体积**：Tauri 产物 ~5MB vs Electron ~150MB
- **内存**：Tauri 使用系统 WebView，内存占用低 50-70%
- **安全**：Rust 后端无 Node.js 供应链风险
- **性能**：文件监控、Git 操作、文本处理均在 Rust 层，零序列化开销
- **macOS 亲和性**：Tauri 2.0 对 macOS 原生功能（通知、Menu Bar、沙箱）支持完善

### D2: 为什么优先支持本地 LLM（Ollama）？
- 文档内容可能包含敏感商业信息，本地推理**零隐私风险**
- 日常的 GC、分类等任务对模型能力要求不高，8B 模型足够
- 降低使用门槛，无需 API Key 即可开箱即用
- 复杂任务（冲突检测、规则提取）可选配云端大模型

### D3: 向量索引为什么用 SQLite-VSS？
- 与应用数据库共用 SQLite，**零额外依赖**
- 文档规模通常 <10K chunks，SQLite-VSS 性能完全够用
- 备份/迁移只需拷贝一个 `.db` 文件
- Phase 1 降级为关键词检索，降低初期复杂度，避免过早引入 ONNX 依赖

### D4: 为什么 Phase 1 用 Ollama 而不是 GPT-4o 验证 LLM 功能？
- **风险**：本地 8B 模型（llama3.1）对复杂语义任务（冲突检测、规则聚类）能力有限，可能导致误报率高
- **决策**：Phase 1 仅实现 Memory GC（任务简单，8B 模型足够），以此验证 LLM 适配层架构
- **Phase 2 起**：冲突检测和规则提取建议优先用 GPT-4o / Claude 3.5 做产品验证，确认准确率 >80% 后再考虑本地化
- **结论**：LLM 选型按功能复杂度分层，不强求所有功能都跑本地模型

### D5: 为什么功能优先级是 Initializer > GC > Rule Extractor > Conflict Detector > Pruner？
- **Initializer**：**所有功能的前置条件**——没有 AGENTS.md 和 progress.md，其余四大功能均无法运行。同时它是产品的 Aha Moment，决定了用户是否会继续使用
- **GC**：每个长期项目必然遇到的高频问题，且对 LLM 能力要求最低，最容易做到高可靠
- **Rule Extractor**：隐性成本最高的痛点（反复错误无法被 AI 自修正），且用户感知强烈
- **Conflict Detector**：危害深远但感知滞后；向量索引依赖增加了实现复杂度，放 Phase 3
- **Pruner**：用户不会因此直接损失，属于"锦上添花"型功能，最后交付
- **原则**：先解决"从 0 到 1"（冷启动），再解决"从 1 到 N"（持续维护）

### D6: Project Initializer 为什么不直接用 LLM 生成全部内容？
- **可靠性**：linter 配置反向翻译、目录结构推断是确定性逻辑，用规则引擎做更准确
- **速度**：确定性扫描 <5 秒，LLM 生成需要 30-60 秒，混合方案兼顾速度和智能
- **分层策略**：
  - **确定性层**（规则引擎）：技术栈识别、linter 配置读取、目录结构分析
  - **推断层**（LLM）：从 README 提取隐含规范、从 Git log 推断痛点、生成自然语言规则描述
- **Fallback**：即使 LLM 不可用（无 Ollama / 无 API Key），确定性层仍能生成一个可用的基础 AGENTS.md

### D7: 为什么把 Project Initializer 集成到项目管理页而非独立页面？
- **用户心智**：用户的第一个动作是「添加项目」，而非「初始化框架」。初始化应该是添加项目的**自然延伸**，而非需要用户主动寻找的独立功能
- **按需触发**：已有 AGENTS.md 的项目直接添加，不弹窗、不打断；只有缺失治理框架时才出现向导 Modal
- **可跳过**：用户可以选择「暂时跳过」，不强制初始化，避免首次使用时的流程压迫感
- **项目切换**：项目管理页同时承担项目列表和切换功能，减少导航层级
- **默认首页**：应用打开直接进入项目管理页（`/projects`），因为选择活跃项目是所有后续操作的前提
- **v0.3 → v0.4 的演进**：原设计为独立的 InitializerPage（4 步向导），实现后发现与项目管理流程脱节，重构为 ProjectsPage + InitWizardModal
