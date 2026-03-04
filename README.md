# AI Doc Manager

基于 Tauri 的 AI 文档治理桌面应用，用于管理代码与文档的一致性、冲突检测和内存 GC。

## 🎯 项目说明

本项目旨在通过 AI 辅助的方式，帮助开发团队维护代码与文档的一致性，检测潜在冲突，并提供智能的内存管理建议。

## 📁 项目结构

```
ai_doc_manager/
├── src/              # 前端 React + TypeScript 源码
├── src-tauri/        # 后端 Rust 源码
├── docs/             # 项目文档
├── .githooks/        # Git hooks（pre-commit 等）
├── .github/          # GitHub 配置（PR/Issue 模板）
├── .ai/              # AI 会话记忆
├── AGENTS.md         # AI 编码助手治理文件
└── README.md         # 本文件
```

## 🚀 快速开始

### 前置要求

- Node.js >= 18.0.0
- Rust >= 1.75.0
- npm 或 yarn

### 安装依赖

```bash
# 安装前端依赖
npm install

# 配置 Git Hooks
git config core.hooksPath .githooks
```

### 开发运行

```bash
# 启动开发服务器（前端 + 后端）
npm run tauri dev
```

### 构建

```bash
# 构建生产版本
npm run tauri build
```

## 🔧 开发指南

### Git Hooks

项目配置了以下 git hooks：

- **pre-commit**: 提交前自动检查
  - 硬编码密钥扫描
  - 范围边界检查
  - 测试弱化检查
  - TypeScript 类型检查
  - Rust 编译检查

- **commit-msg**: 提交信息格式验证
  - 遵循 Conventional Commits 规范
  - 格式: `<type>(<scope>): <subject>`

### Commit Message 规范

```
feat(frontend): 添加用户登录功能
fix(backend): 修复数据库连接泄漏
docs: 更新 API 文档
style: 代码格式化
refactor(core): 重构冲突检测逻辑
perf: 优化启动速度
test: 添加单元测试
chore: 更新依赖版本
```

详见 `.gitmessage` 文件。

### AI 编码规范

本项目使用 `AGENTS.md` 文件来治理 AI 编码助手的行为，包括：

- ✏️ 白名单：AI 可读写的文件
- 👁️ 只读区：AI 仅可读取的配置文件
- 🚫 黑名单：AI 禁止访问的目录
- 安全红线、编码风格、验证命令等

使用 AI 编码助手（如 Cursor, GitHub Copilot）前，请先阅读 `AGENTS.md`。

## 📖 技术栈

- **前端**: React 18 + TypeScript + Vite + Tailwind CSS
- **后端**: Rust + Tauri 2.x
- **状态管理**: Zustand
- **数据库**: SQLite (通过 rusqlite)
- **图标**: lucide-react

## 🤝 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

请确保：
- ✅ 所有 pre-commit checks 通过
- ✅ 遵循 Commit Message 规范
- ✅ 更新相关文档
- ✅ 添加必要的测试

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

感谢所有贡献者对本项目的支持！

## 📞 联系方式

- GitHub Issues: [https://github.com/liogogogo/ai_doc_manager/issues](https://github.com/liogogogo/ai_doc_manager/issues)
- Email: 705110706@qq.com
