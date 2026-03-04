# 项目进度追踪 (AI Session Progress)

> **用途**: 跨会话记忆。每次 AI 会话结束前必须更新本文件。
> **格式**: 保持精简，仅记录关键状态。

## 当前状态

- **最后会话**: 2026-03-04
- **最后完成**: [Cursor] LLM 集成全面优化
- **当前进行中**: 无

## 已完成里程碑

| 日期 | 里程碑 | 关键变更 |
|:--|:--|:--|
| 2026-02-28 | 治理框架初始化 | DocGuardian 生成 AGENTS.md，技术栈: Rust, TypeScript |
| 2026-02-28 | 重新生成治理框架功能 | [Cursor] ProjectsPage 新增「重新生成」入口（卡片悬浮 + 偏差面板），含确认对话框；从偏差面板触发时自动将偏差项预填到优化栏，实现偏差感知式重新生成 |
| 2026-02-28 | 生成管线质量优化 | [Cursor] 基于模型A/B对比分析优化管线：① system prompt 增加架构感知 Style Guide、弹性权限、量化治理等前沿指导 ② generation prompt 每章节细化到具体产出标准+10项自检清单 ③ code_sampler 多命令采样+服务层+命令签名+invoke调用链+导入顺序 ④ 质量评分新增"架构感知 Style Guide"维度 ⑤ Ollama 合并 system prompt 修复 |
| 2026-02-28 | 生成管线二次改进 | [Cursor] ① 头部元信息要求独立 `## 1.` 编号 ② Verification 7 步 ③ 大文件扫描 ④ Context Budget 导航索引 ⑤ 评分 7 维度 |
| 2026-02-28 | prompt 精简重构 | [Cursor] system prompt 50→12 行；generation prompt 120→55 行；引导表格化输出 |
| 2026-02-28 | Runtime Compliance Loop | [Cursor] 新增运行时合规闭环：violation 模型+DB 表、compliance_checker.rs（密钥/Scope/测试弱化/配置泄露 4 类检查）、3 个 Tauri 命令+setup_git_hooks、ProjectsPage 合规面板、DashboardPage 违规卡、Git pre-commit hook 生成 |
| 2026-02-28 | Compliance Checker 质量修复 | [Cursor] ① 密钥扫描移除高误报 pattern（password/token），新增 openai/anthropic 专项 ② 注释跳过修复优先级 bug + 补 `/*` `*` ③ Scope 新增 Strategy 2 磁盘扫描（无 git 也可用）④ config_leak 白名单 + 扩展 .env/credentials.json 检测 ⑤ walk_dir 拆分 apply_skip |
| 2026-03-01 | 合规检查序列化 Bug 修复 | [Cursor] ViolationStatus/Severity/Category 枚举缺少 `#[serde(rename_all)]`，导致前端 `===` 严格比较不匹配，violations 面板始终显示"全部通过" |
| 2026-03-01 | 合规检查器误报修复 | [Cursor] 25 项扫描结果全为误报：① Strategy 2 跳过 dist/node_modules/target 等构建产物目录 ② test_weakening 增加 raw string 块跟踪 + `pattern_in_string_literal` 检测 ③ config_leak 扩展 `ALLOW_CONFIG_CHECK` 排除列表 + raw string 块跟踪 |
| 2026-03-01 | 合规检查器减法重构 | [Cursor] 删除 Strategy 2 磁盘扫描 + check_config_leak；test_weakening 限定为 JS/TS 文件并移除复杂 raw string 追踪；walk_dir 去掉 apply_skip 参数；整体从 4 检查项缩减为 3 项（密钥/Scope git diff/测试弱化） |
| 2026-03-04 | Git 配置完善与 GitHub 推送 | [Cursor] ① 修复 TypeScript 类型错误（未使用变量、null vs undefined）② 清理 git 仓库（移除 7.2GB 的 target/ 构建产物）③ 完善 .gitignore（添加敏感文件、临时文件等）④ 添加 .gitattributes（行尾规范、diff 配置）⑤ 配置 commit-msg hook 验证提交格式 ⑥ 添加 .gitmessage 模板 ⑦ 添加 GitHub PR/Issue 模板 ⑧ 完善 README.md ⑨ 添加 MIT 许可证 ⑩ 成功推送到 https://github.com/liogogogo/ai_doc_manager.git |
| 2026-03-04 | LLM 集成全面优化 | [Cursor] ① adapter.rs 重构：新增 StreamEvent/ChatMessage/Cancelled/Timeout，trait 统一 stream_complete_messages ② ollama.rs 重写：/api/generate→/api/chat 多轮流式(NDJSON)，chunk 间 60s 超时 ③ openai_compatible.rs 增强：reasoning_content 支持、重试(2 次退避 1s/3s)、chunk 间超时 ④ commands/llm.rs 统一：去掉 Ollama 特殊分支，所有 provider 走 trait 流式 ⑤ 取消机制：CancellationFlag(AtomicBool) + cancel_llm_generation 命令 ⑥ 前端：停止生成按钮、llm-reasoning 事件监听、可折叠推理面板 |

> 更早期的里程碑已归档至 `.ai/archive/`，以保持本文件精简。

## 待办事项

- [x] 项目治理框架初始化（AGENTS.md + progress.md）
- [x] [Cursor] 支持「重新生成治理框架」功能（ProjectsPage.tsx）
- [x] [Cursor] 治理框架生成管线质量优化（llm.rs prompt + code_sampler.rs 采样 + 评分维度）

## 阻塞项

| 任务 | 阻塞原因 | 等待谁 |
|:--|:--|:--|
| <!-- 无 --> | — | — |

## 关键决策

| 日期 | 决策 | 原因 |
|:--|:--|:--|
| 2026-02-28 | 引入 AI 治理框架 | 规范 AI 编码助手行为，降低幻觉率 |

## 已知技术债务

- <!-- 在此记录已知的技术债务 -->
