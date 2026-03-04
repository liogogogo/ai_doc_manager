# DocGuardian

> AI 自动化流程下的文档健康管理工具 · macOS 桌面应用 · v0.5

DocGuardian 为任意项目**从零生成 AI 治理框架**（AGENTS.md），并**持续监控文档健康度**——自动执行记忆垃圾回收、知识冲突检测、隐式规则提取和冗余清理，确保 AI 编码助手始终获得高纯度的上下文输入。

## 核心功能

| 功能 | 状态 | 描述 |
|------|------|------|
| **F0: Project Initializer** | ✅ | 扫描项目技术栈 → 确定性规则引擎生成基础 AGENTS.md + progress.md + .docguardian.toml |
| **F5: LLM 智能生成** | ✅ | 12 章节体系 + 流式输出 + 多轮会话优化 + 选中定向优化 |
| **F6: 偏差检测** | ✅ | 三态徽标（未初始化/需更新/已治理）+ 偏差详情 + AI 增量更新建议 |
| **增强扫描** | ✅ | 依赖树深度解析（npm + Cargo）+ CI 配置解析（GitHub Actions / GitLab） |
| F1: Memory GC | 🔜 | 自动归档已完结的进度条目，维持工作记忆在容量上限内 |
| F2: Rule Extractor | 🔜 | 从失败日志中提取反复出现的错误模式，建议新增编码规则 |
| F3: Conflict Detector | 🔜 | 语义比对代码变更与文档，标记过时/冲突的描述 |
| F4: Redundancy Pruner | 🔜 | 识别可被 linter/脚本替代的自然语言文档，建议删除或重写 |

## 技术栈

- **桌面框架**: Tauri 2.0 (Rust + WebView)
- **前端**: React 18 + TypeScript + TailwindCSS + Zustand
- **后端**: Rust (SQLite, git2, notify, pulldown-cmark)
- **LLM**: 可插拔适配器 — OpenAI / Claude / 智谱GLM / DeepSeek / Ollama / 自定义 API（支持流式+完整模式）

## 前置条件

- macOS 10.15+
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [Tauri CLI](https://tauri.app/): `cargo install tauri-cli`

## 快速开始

```bash
# 安装前端依赖
npm install

# 开发模式（同时启动前端 + Rust 后端）
cargo tauri dev

# 构建发布版本
cargo tauri build
```

## 项目结构

```
ai_doc_manager/
├── src/                  # React 前端
│   ├── pages/            # 页面组件
│   ├── components/       # UI 组件
│   ├── stores/           # Zustand 状态管理
│   └── hooks/            # 自定义 Hooks
├── src-tauri/            # Rust 后端
│   └── src/
│       ├── core/         # 七大核心引擎
│       ├── commands/     # Tauri IPC 命令
│       ├── services/     # 共享服务层
│       └── models/       # 数据模型
├── docs/design/          # 产品与技术设计文档
└── AGENTS.md             # AI 编码宪法
```

## 配置

首次接入项目后，DocGuardian 会在项目根目录生成 `.docguardian.toml`，详见 [产品设计文档](docs/design/product-spec.md#五项目配置文件格式)。

## License

MIT
