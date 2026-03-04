# AGENTS.md - AI 编码助手治理文件

## 1. 头部元信息

- **版本**: 1.0
- **更新日期**: 2024-10-30
- **项目描述**: ai_doc_manager - 基于 Tauri (Rust + TypeScript/React) 的 AI 文档治理桌面应用，管理代码与文档的一致性、冲突检测和内存 GC。
- **适用 AI 助手**: GitHub Copilot, Cursor, Cline, Windsurf, 及兼容 OpenAI API 的编码助手。

## 2. Scope（权限边界）

采用三区模型，基于项目扫描结果定义。

### ✏️ 白名单（AI 可读写）
- `src/` - 前端 React 源码目录
- `src-tauri/src/` - 后端 Rust 源码目录
- `docs/` - 项目文档目录
- `.ai/` - AI 治理相关文件（如 progress.md）
- `.docguardian.toml` - 项目配置文件

### 👁️ 只读区（AI 仅可读取，不可修改）
- `package.json` - npm 依赖定义
- `tsconfig.json` - TypeScript 配置
- `tailwind.config.js` - Tailwind CSS 配置
- `vite.config.ts` - 构建配置
- `src-tauri/Cargo.toml` - Rust 依赖定义
- `src-tauri/Cargo.lock` - Rust 依赖锁文件
- `src-tauri/tauri.conf.json` - Tauri 应用配置

### 🚫 黑名单（AI 禁止访问）
- `dist/` - 构建产物目录
- `node_modules/` - npm 依赖目录
- `src-tauri/target/` - Rust 构建产物目录
- `local_data/` - 应用运行时本地数据（如 SQLite 数据库）
- `.env`, `*.key`, `*.pem` - 敏感配置文件
- `AGENTS.md` - 本治理文件自身

## 3. Don't（安全红线）

违反以下任何一条，AI 必须立即停止操作并报告用户。

1. **🚫 禁止硬编码密钥或凭据**：不得在任何源码文件中写入 API Key、密码、JWT Secret 等敏感信息。必须使用环境变量或配置文件。
2. **🚫 禁止反向依赖**：不得修改 `package.json` 或 `Cargo.toml` 中已定义的依赖版本，除非是 `Ask First` 章节明确授权的任务。
3. **🚫 禁止凭记忆编写第三方 API/SDK 调用**：所有外部库（如 `@tauri-apps/api`, `rusqlite`, `git2`）的调用必须基于其官方文档或项目内现有用法，不得猜测 API。
4. **🚫 禁止在指令模糊时盲猜**：当用户指令存在歧义（如“优化一下”）或涉及关键逻辑时，必须主动询问澄清，不得自行假设。
5. **🚫 禁止删除或弱化测试**：不得删除现有测试文件，或将 `#[test]` 改为 `#[ignore]`，或将 `it('...')` 改为 `it.skip('...')`。
6. **🚫 禁止静默覆盖或删除治理文件**：不得修改或删除 `AGENTS.md`、`.ai/progress.md`、`.docguardian.toml`，除非是执行 `Governance Loop` 章节定义的更新流程。

## 4. Style Guide（编码风格）

基于项目代码采样提炼的编码模式。

### Rust 后端风格
- **命令定义**：使用 `#[tauri::command(rename_all = "snake_case")]` 宏，异步函数返回 `Result<T, String>`。错误统一用 `.map_err(|e| format!("...: {}", e))` 或 `.map_err(|e| e.to_string())` 转换为 `String`。
  ```rust
  // ✅ Good (来自 src-tauri/src/commands/init.rs)
  #[tauri::command(rename_all = "snake_case")]
  pub async fn check_governance(root_path: String) -> Result<GovernanceStatus, String> {
      let root = Path::new(&root_path);
      if !root.exists() {
          return Err(format!("路径不存在: {}", root_path));
      }
      // ...
  }
  ```
- **数据模型**：使用 `#[derive(Debug, Clone, Serialize, Deserialize)]` 定义结构体，字段使用 `snake_case`。
  ```rust
  // ✅ Good (来自 src-tauri/src/models/conflict.rs)
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Conflict {
      pub id: String,
      pub project_id: String,
      pub document_id: String,
      // ...
  }
  ```
- **模块组织**：按功能拆分命令模块（如 `commands::project`, `commands::gc`），在 `lib.rs` 的 `invoke_handler` 中统一注册。

### TypeScript/React 前端风格
- **状态管理**：使用 Zustand，Store 定义遵循 `interface State` + `create<State>` 模式，更新函数使用 `(set) => ({ ... })`。
  ```typescript
  // ✅ Good (来自 src/stores/projectStore.ts)
  interface ProjectState {
    projects: Project[];
    currentProject: Project | null;
    setCurrentProject: (project: Project) => void;
  }
  export const useProjectStore = create<ProjectState>((set) => ({
    projects: [],
    currentProject: null,
    setCurrentProject: (project) => set({ currentProject: project }),
  }));
  ```
- **组件结构**：页面组件放在 `src/pages/`，使用 `interface` 定义 Props/State，从 `lucide-react` 导入图标，使用 `cn()` 工具函数合并 Tailwind 类名。
  ```typescript
  // ✅ Good (来自 src/pages/DashboardPage.tsx)
  import { FileText, AlertTriangle } from "lucide-react";
  import { cn } from "@/lib/utils";
  interface MetricCardProps { label: string; value: string | number; icon: React.ElementType; }
  function MetricCard({ label, value, icon: Icon }: MetricCardProps) {
    return <div className="card flex items-center gap-4 p-5">...</div>;
  }
  ```
- **类型安全**：启用 `typescript-strict`，避免使用 `any`。

### 跨层约定
- **IPC 命名映射**：前端调用 Tauri 命令时，命令名与后端定义的 `snake_case` 函数名一致（通过 `rename_all` 属性确保）。
- **新增功能流程**：
  1. 在 `src-tauri/src/commands/` 下创建新模块或函数。
  2. 在 `src-tauri/src/lib.rs` 的 `invoke_handler!` 宏中注册。
  3. 在前端 `src/stores/` 或 `src/hooks/` 中添加对应的调用逻辑。
  4. 在 `src/pages/` 中添加或更新相关 UI 组件。

## 5. Ask First（需确认）

执行以下操作前，AI 必须明确向用户请求确认：
1. **新增 npm 或 Cargo 依赖**。
2. **修改数据库 Schema**（如 `src-tauri/src/db/migrations/` 下的文件）。
3. **删除或修改公共 API/接口**（如已注册的 Tauri 命令、React 组件 Props）。
4. **修改持续集成配置**（如 `.github/workflows/` 下的文件）。
5. **拆分现有文件**（当文件超过 300 行，且逻辑可独立时）。

## 6. Commands（验证命令）

所有命令使用防御性写法，确保在条件不满足时优雅跳过。

### ① 秒级检查（每次编辑后建议运行）
```bash
# 1. 硬编码密钥扫描
if command -v grep &> /dev/null; then
  grep -r --include="*.rs" --include="*.ts" --include="*.tsx" --include="*.js" --include="*.json" -E "(api[_-]?key|secret|password|token|jwt)[\s]*=[\s]*['\"][^'\"]{8,}['\"]" src/ src-tauri/src/ 2>/dev/null || echo "✅ 未发现硬编码密钥"
else
  echo "⏭ 跳过：grep 命令不可用"
fi

# 2. TypeScript 类型检查
if [ -f "node_modules/.bin/tsc" ]; then
  npx tsc --noEmit --project .
else
  echo "⏭ 跳过：TypeScript 编译器未安装"
fi

# 3. Rust 语法检查
if command -v cargo &> /dev/null && [ -f "src-tauri/Cargo.toml" ]; then
  cd src-tauri && cargo check --quiet && cd ..
else
  echo "⏭ 跳过：Cargo 不可用或不在项目根目录"
fi
```

### ② 分钟级检查（功能完成后运行）
```bash
# 1. 运行单元测试
if command -v cargo &> /dev/null && [ -f "src-tauri/Cargo.toml" ]; then
  cd src-tauri && cargo test --quiet && cd ..
else
  echo "⏭ 跳过：Rust 测试环境不可用"
fi

# 2. 前端构建验证
if [ -f "node_modules/.bin/vite" ]; then
  npx vite build --mode development 2>&1 | tail -20
else
  echo "⏭ 跳过：Vite 未安装"
fi

# 3. Tauri 构建验证
if command -v cargo &> /dev/null && [ -f "src-tauri/Cargo.toml" ]; then
  cd src-tauri && cargo build --release --quiet && cd ..
else
  echo "⏭ 跳过：Tauri 构建环境不可用"
fi
```

### ③ 提交前检查（准备提交代码时运行）
```bash
#!/bin/bash
# 全量检查脚本
echo "🚀 开始提交前全量检查..."

# 1. 代码风格与 lint (如果配置了)
# (项目暂无 eslint/rustfmt 配置，此处预留)

# 2. 类型与语法检查
if [ -f "node_modules/.bin/tsc" ]; then
  echo "📘 检查 TypeScript 类型..."
  npx tsc --noEmit --project . || { echo "❌ TypeScript 类型错误"; exit 1; }
fi

if command -v cargo &> /dev/null && [ -f "src-tauri/Cargo.toml" ]; then
  echo "🦀 检查 Rust 语法..."
  cd src-tauri && cargo check --quiet && cd .. || { echo "❌ Rust 语法错误"; exit 1; }
fi

# 3. 运行测试套件
if command -v cargo &> /dev/null && [ -f "src-tauri/Cargo.toml" ]; then
  echo "🧪 运行 Rust 测试..."
  cd src-tauri && cargo test --quiet && cd .. || { echo "❌ 测试失败"; exit 1; }
fi

# 4. 安全扫描 (检测 Don't 规则)
echo "🔒 执行安全规则扫描..."
if command -v grep &> /dev/null; then
  # 检测是否误改了治理文件
  if git status --porcelain | grep -E "AGENTS.md|\.ai/progress.md|\.docguardian.toml" | grep -v "^??"; then
    echo "⚠️  检测到治理文件被修改，请确认是否通过 Governance Loop 流程。"
    # 此处不退出，仅警告，因为可能是合法更新。
  fi
fi

echo "✅ 所有检查通过！可以提交。"
```

## 7. Project Structure

```
ai_doc_manager/
├── ✏️ src/                           # 前端 React 源码
│   ├── ✏️ pages/                    # 页面组件 (ConflictsPage, DashboardPage, GCPage)
│   ├── ✏️ stores/                   # Zustand 状态管理 (projectStore, uiStore)
│   ├── ✏️ lib/                      # 工具函数
│   └── ✏️ App.tsx
├── ✏️ src-tauri/                    # 后端 Rust 源码
│   ├── ✏️ src/
│   │   ├── ✏️ commands/             # Tauri 命令模块 (project, init, gc, conflict, rule, prune, llm)
│   │   ├── ✏️ models/               # 数据模型 (conflict, document, project)
│   │   ├── ✏️ db/                   # 数据库相关
│   │   └── ✏️ lib.rs                # 入口和命令注册
│   ├── 👁️ Cargo.toml
│   └── 👁️ tauri.conf.json
├── ✏️ docs/                         # 项目文档
├── ✏️ .ai/                          # AI 会话记忆
│   └── ✏️ progress.md
├── ✏️ .docguardian.toml             # 项目配置
├── 👁️ package.json
├── 👁️ tsconfig.json
├── 👁️ tailwind.config.js
├── 👁️ vite.config.ts
├── 🚫 dist/
├── 🚫 node_modules/
├── 🚫 src-tauri/target/
├── 🚫 local_data/
└── 🚫 AGENTS.md                     # 本文件
```

## 8. Verification（验证清单）

AI 在完成任何编码任务后，必须按顺序执行以下验证步骤：

1. **✅ 范围自检**：确认所有修改的文件均在 `Scope` 白名单内，未触及黑名单。
2. **✅ 编译检查**：运行 `Commands` 章节中的秒级检查 2 和 3（TypeScript 和 Rust 语法检查）。
3. **✅ 规则扫描**：运行 `Commands` 章节中的秒级检查 1（硬编码密钥扫描），确保未违反 `Don't` 第1条。
4. **✅ 功能测试**：如果修改涉及后端逻辑，运行 `cargo test`；如果涉及前端，手动检查相关页面能否正常渲染。
5. **✅ 更新记忆**：根据 `Memory` 章节的规则，将本次任务摘要写入 `.ai/progress.md`。

## 9. Memory（会话记忆）

- **读写路径**：`.ai/progress.md`
- **容量上限**：200 行。当文件超过 200 行时，AI 应主动在文件顶部添加 `## GC: <日期>` 注释，并归档或删除最旧的内容，保留最近 5 次任务的记录。
- **会话协议**：
  - **开始**：新会话开始时，AI 首先读取 `AGENTS.md` 和 `.ai/progress.md` 以获取上下文。
  - **结束**：任务完成后，必须在 `.ai/progress.md` 末尾追加记录，格式为 `- [YYYY-MM-DD HH:MM] <任务摘要>`。
- **上下文注入优先级**：
  1. `AGENTS.md` (本治理文件)
  2. `.ai/progress.md` (进度记忆)
  3. 与当前任务直接相关的源码文件
  4. 用户的当前指令

## 10. Examples（项目专属模式）

### 示例 1: 定义新的 Tauri 命令
```rust
// ✅ Good (遵循项目模式，来自采样)
#[tauri::command(rename_all = "snake_case")]
pub async fn get_document_stats(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<DocumentStats, String> {
    tracing::info!("Fetching stats for project {}", project_id);
    // 实际查询逻辑...
    Ok(DocumentStats { total: 10, updated_today: 2 })
}
```
```rust
// ❌ Bad (违反 Style Guide)
// 错误1: 未使用 rename_all 宏，导致前后端命名不一致
#[tauri::command]
pub async fn GetDocumentStats(db: State<'_, Arc<Database>>) -> Result<String, String> {
    // 错误2: 直接 panic，未进行错误处理
    let conn = db.get_conn().unwrap();
    // ...
}
```

### 示例 2: 创建新的 React 状态与组件
```typescript
// ✅ Good (遵循项目模式，来自采样)
// 在 src/stores/新建 documentStore.ts
interface DocumentState {
  documents: Document[];
  selectedDocId: string | null;
  setSelectedDoc: (id: string) => void;
}
export const useDocumentStore = create<DocumentState>((set) => ({
  documents: [],
  selectedDocId: null,
  setSelectedDoc: (id) => set({ selectedDocId: id }),
}));

// 在 src/pages/新建 DocumentsPage.tsx
import { useDocumentStore } from "@/stores/documentStore";
export function DocumentsPage() {
  const { documents, selectedDocId } = useDocumentStore();
  return (<div className="p-6">...</div>);
}
```
```typescript
// ❌ Bad (违反 Style Guide)
// 错误1: 使用非 Zustand 的状态管理
import { useState } from "react";
const [docs, setDocs] = useState([]); // 应使用全局 Store

// 错误2: 未使用项目约定的工具函数和图标库
<div class="card">...</div> // 应为 className，且建议使用 `cn()` 和设计好的 card 样式
```

## 11. Context Budget（上下文预算）

- **单次加载上限**：AI 助手单次上下文窗口限制为 **128K tokens**。
- **溢出策略**：
  1. 优先加载 `AGENTS.md` 和 `.ai/progress.md`。
  2. 然后加载与当前任务最相关的单个文件（如正在编辑的 `.rs` 或 `.tsx` 文件）。
  3. 如果需要更多上下文，使用 **导航索引** 而非加载整个文件：仅加载函数/结构体定义，跳过实现细节。
- **长文件导航索引**：对于超过 300 行的文件，在对话中提供文件的结构摘要，例如：
  > `src-tauri/src/commands/llm.rs` 包含：`save_llm_config`, `get_llm_config`, `test_llm_connection`, `generate_agents_md_llm` 等函数。

## 12. Multi-Agent Protocol（多助手协作）

- **身份标识**：在 `.ai/progress.md` 中记录每个任务的执行助手（如 `[Copilot]`）。
- **冲突预防**：
  1. 开始工作前，检查 `.ai/progress.md` 中是否有其他助手正在处理相关模块的记录。
  2. 如果存在，等待其任务标记为完成或直接询问用户。
- **交接协议**：
  1. 完成任务后，在 `.ai/progress.md` 中清晰记录修改点、待办项和潜在风险。
  2. 下一个助手必须阅读前序记录后才能开始工作。

## 13. When Stuck（遇到阻塞时）

按顺序执行以下步骤，最多尝试 2 轮：
1.  **搜索确认**：在项目源码中搜索类似模式或函数调用。例如，不确定如何调用 `git2`，就在项目中搜索 `git2`。
2.  **文档优先**：如果项目内无参考，查找官方文档（如 `docs.rs/git2`）。**禁止凭记忆编写**。
3.  **询问用户**：如果文档无法解决问题，向用户清晰描述阻塞点、已尝试的方案和你的建议。
4.  **熔断机制**：如果以上步骤在单次任务中失败 2 次，主动暂停并告知用户：“我已尝试 X 和 Y，但仍无法解决 Z。建议您手动处理或提供更详细的指引。”

## 14. Governance Loop（治理闭环）

- **触发条件**：
  1.  项目技术栈变更（如新增依赖）。
  2.  代码模式发生显著演变（如新的状态管理方式）。
  3.  每完成 10 个主要功能点后。
  4.  AI 助手在执行任务时频繁遇到规则未覆盖的边界情况。
- **更新流程**：
  1.  **提议**：AI 或用户提出 `AGENTS.md` 的更新建议。
  2.  **生成**：运行命令 `cargo run -- generate-agents-md` （需实现此命令）或由 AI 基于当前代码采样草拟更新。
  3.  **评审**：用户评审变更，AI 辅助解释修改原因。
  4.  **应用**：用户确认后，更新 `AGENTS.md` 和 `Changelog`。
- **健康度指标**：
  - **规则覆盖率**：`(被 Commands 检测的 Don't 规则数 / Don't 规则总数) * 100%`，目标 >90%。
  - **构建成功率**：提交前检查（第6章③）的通过率。
  - **用户干预率**：因 `Ask First` 或 `When Stuck` 而暂停任务的频率，应逐渐降低。
- **回滚策略**：`AGENTS.md` 本身应被 Git 版本控制。任何导致构建失败或规则冲突的更新，应立即回滚到上一个可用版本。

## 15. Changelog

| 版本 | 日期       | 变更摘要                                     | 触发原因                   |
| :--- | :--------- | :------------------------------------------- | :------------------------- |
| 1.0  | 2024-10-30 | 初始版本创建。                               | 项目初始化，首次生成治理文件。 |