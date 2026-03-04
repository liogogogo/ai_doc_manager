use std::collections::HashMap;
use std::path::Path;

/// Sampled code patterns extracted from a real project, used to ground
/// LLM-generated governance documents in actual implementation details.
#[derive(Debug, Clone, Default)]
pub struct CodePatterns {
    /// Representative code snippets keyed by category (e.g. "tauri_command", "store", "component")
    pub snippets: Vec<CodeSnippet>,
    /// Entry-point / wiring patterns (e.g. how commands are registered, routes defined)
    pub wiring: Vec<CodeSnippet>,
    /// Error handling patterns observed
    pub error_patterns: Vec<String>,
    /// Import / module structure patterns
    pub module_patterns: Vec<String>,
    /// Large files (>300 lines) with line counts, for Context Budget navigation index
    pub large_files: Vec<LargeFileInfo>,
}

#[derive(Debug, Clone)]
pub struct LargeFileInfo {
    pub rel_path: String,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct CodeSnippet {
    pub category: String,
    pub rel_path: String,
    pub content: String,
    pub description: String,
}

const MAX_SNIPPET_LINES: usize = 30;
const MAX_TOTAL_CHARS: usize = 12000;

/// Sample code patterns from a project for use in LLM prompt context.
/// Keeps total output under ~8k chars to avoid prompt bloat.
pub fn sample_project_patterns(root: &Path) -> CodePatterns {
    let mut patterns = CodePatterns::default();
    let mut total_chars = 0usize;

    sample_rust_patterns(root, &mut patterns, &mut total_chars);
    sample_ts_patterns(root, &mut patterns, &mut total_chars);
    extract_error_patterns(root, &mut patterns);
    extract_module_patterns(root, &mut patterns);
    scan_large_files(root, &mut patterns);

    patterns
}

/// Render all patterns into a prompt-friendly string block.
pub fn render_patterns_for_prompt(patterns: &CodePatterns) -> String {
    if patterns.snippets.is_empty() && patterns.wiring.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n## 项目代码模式采样（真实代码片段，用于生成贴合项目的治理规则）\n\n");

    // Group snippets by category
    let mut by_category: HashMap<&str, Vec<&CodeSnippet>> = HashMap::new();
    for s in &patterns.snippets {
        by_category.entry(&s.category).or_default().push(s);
    }
    for s in &patterns.wiring {
        by_category.entry("wiring").or_default().push(s);
    }

    for (cat, snippets) in &by_category {
        out.push_str(&format!("### {} 模式\n", category_label(cat)));
        for s in snippets {
            out.push_str(&format!(
                "\n**{}** (`{}`)\n```\n{}\n```\n",
                s.description, s.rel_path, s.content
            ));
        }
    }

    if !patterns.error_patterns.is_empty() {
        out.push_str("\n### 错误处理惯例\n");
        for p in &patterns.error_patterns {
            out.push_str(&format!("- {}\n", p));
        }
    }

    if !patterns.module_patterns.is_empty() {
        out.push_str("\n### 模块组织惯例\n");
        for p in &patterns.module_patterns {
            out.push_str(&format!("- {}\n", p));
        }
    }

    if !patterns.large_files.is_empty() {
        out.push_str("\n### 大文件清单（>300 行，需在 Context Budget 中提供导航索引）\n");
        for f in &patterns.large_files {
            out.push_str(&format!("- `{}` — {} 行\n", f.rel_path, f.line_count));
        }
    }

    out
}

fn category_label(cat: &str) -> &str {
    match cat {
        "tauri_command" => "Tauri 命令",
        "store" => "状态管理 (Store)",
        "component" => "React 组件",
        "hook" => "自定义 Hook",
        "wiring" => "入口注册/连接",
        "rust_model" => "Rust 数据模型",
        "rust_service" => "Rust 服务层",
        "cross_layer" => "跨端 IPC 调用",
        _ => cat,
    }
}

/// Scan frontend source files for `invoke(` calls to show IPC usage patterns.
fn extract_invoke_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    let src_dir = root.join("src");
    if !src_dir.is_dir() {
        return patterns;
    }
    let mut seen = std::collections::HashSet::new();
    walk_ts_files(&src_dir, &mut |path| {
        if patterns.len() >= 6 {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.contains("invoke(") || trimmed.contains("invoke<") {
                    let normalized = trimmed.to_string();
                    if seen.insert(normalized.clone()) && patterns.len() < 6 {
                        patterns.push(normalized);
                    }
                }
            }
        }
    });
    patterns
}

/// Extract the import ordering pattern from the largest page component.
fn extract_import_pattern(root: &Path) -> Option<String> {
    let pages_dir = root.join("src/pages");
    if !pages_dir.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(&pages_dir).ok()?;

    let mut best_file: Option<(std::path::PathBuf, usize)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "tsx" || e == "ts").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let import_count = content.lines().filter(|l| l.trim().starts_with("import ")).count();
                if import_count > best_file.as_ref().map(|b| b.1).unwrap_or(0) {
                    best_file = Some((path, import_count));
                }
            }
        }
    }

    let (file_path, _) = best_file?;
    let content = std::fs::read_to_string(&file_path).ok()?;
    let imports: Vec<&str> = content.lines()
        .take_while(|l| {
            let t = l.trim();
            t.is_empty() || t.starts_with("import ") || t.starts_with("//") || t.starts_with("from ")
        })
        .collect();

    if imports.len() < 3 {
        return None;
    }

    let filename = file_path.file_name()?.to_string_lossy();
    Some(format!(
        "前端导入顺序（来自 {}）：{}",
        filename,
        imports.iter()
            .filter(|l| l.trim().starts_with("import "))
            .map(|l| {
                if l.contains("react") { "React 核心" }
                else if l.contains("lucide") || l.contains("@radix") { "第三方 UI 库" }
                else if l.contains("@tauri") { "Tauri API" }
                else if l.contains("@/") || l.contains("../") || l.contains("./") { "本地模块" }
                else { "第三方库" }
            })
            .collect::<Vec<_>>()
            .join(" → ")
    ))
}

/// Recursively walk .ts/.tsx files, calling `f` on each path.
fn walk_ts_files(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name != "node_modules" && name != "dist" && name != ".next" {
                walk_ts_files(&path, f);
            }
        } else if path.extension().map(|e| e == "ts" || e == "tsx").unwrap_or(false) {
            f(&path);
        }
    }
}

// ── Rust pattern sampling ──

fn sample_rust_patterns(root: &Path, patterns: &mut CodePatterns, total: &mut usize) {
    // 1. Sample up to 2 Tauri commands (from different files for diversity)
    let cmd_dir = root.join("src-tauri/src/commands");
    if cmd_dir.is_dir() {
        let commands = find_tauri_commands(&cmd_dir, 2);
        for snippet in commands {
            if *total < MAX_TOTAL_CHARS {
                *total += snippet.content.len();
                patterns.snippets.push(snippet);
            }
        }
    }

    // 2. Sample entry-point wiring (invoke_handler in lib.rs)
    let lib_rs = root.join("src-tauri/src/lib.rs");
    if lib_rs.exists() && *total < MAX_TOTAL_CHARS {
        if let Some(snippet) = extract_invoke_handler(&lib_rs) {
            *total += snippet.content.len();
            patterns.wiring.push(snippet);
        }
    }

    // 3. Sample a Rust model
    let models_dir = root.join("src-tauri/src/models");
    if models_dir.is_dir() && *total < MAX_TOTAL_CHARS {
        if let Some(snippet) = sample_first_struct(&models_dir, "rust_model") {
            *total += snippet.content.len();
            patterns.snippets.push(snippet);
        }
    }

    // 4. Sample a service layer pattern (core/ or services/)
    for service_dir_name in &["core", "services"] {
        let service_dir = root.join(format!("src-tauri/src/{}", service_dir_name));
        if service_dir.is_dir() && *total < MAX_TOTAL_CHARS {
            if let Some(snippet) = find_first_impl_block(&service_dir, "rust_service") {
                *total += snippet.content.len();
                patterns.snippets.push(snippet);
                break;
            }
        }
    }

    // 5. Extract command signatures for cross-layer context
    if cmd_dir.is_dir() && *total < MAX_TOTAL_CHARS {
        let sigs = extract_command_signatures(&cmd_dir);
        if !sigs.is_empty() {
            let sig_text = sigs.join("\n");
            *total += sig_text.len();
            patterns.snippets.push(CodeSnippet {
                category: "cross_layer".into(),
                rel_path: "src-tauri/src/commands/".into(),
                content: sig_text,
                description: "所有已注册 Tauri 命令签名（前端可通过 invoke 调用）".into(),
            });
        }
    }
}

/// Find up to `limit` Tauri commands from different files for diversity.
fn find_tauri_commands(cmd_dir: &Path, limit: usize) -> Vec<CodeSnippet> {
    let mut results = Vec::new();
    let entries = match std::fs::read_dir(cmd_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };
    for entry in entries.flatten() {
        if results.len() >= limit {
            break;
        }
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false)
            && path.file_name().map(|n| n != "mod.rs").unwrap_or(false)
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(snippet) = extract_first_function_block(&content, "#[tauri::command") {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let rel = format!("src-tauri/src/commands/{}", filename);
                    results.push(CodeSnippet {
                        category: "tauri_command".into(),
                        rel_path: rel,
                        content: snippet,
                        description: format!("Tauri 命令模式（来自 {}）", filename),
                    });
                }
            }
        }
    }
    results
}

/// Find the first `impl` block in a directory (for service/core layer sampling).
fn find_first_impl_block(dir: &Path, category: &str) -> Option<CodeSnippet> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false)
            && path.file_name().map(|n| n != "mod.rs").unwrap_or(false)
        {
            let content = std::fs::read_to_string(&path).ok()?;
            if let Some(snippet) = extract_first_function_block(&content, "impl ") {
                let filename = path.file_name()?.to_string_lossy().to_string();
                let parent = dir.file_name()?.to_string_lossy().to_string();
                return Some(CodeSnippet {
                    category: category.into(),
                    rel_path: format!("src-tauri/src/{}/{}", parent, filename),
                    content: snippet,
                    description: format!("服务层实现模式（来自 {}）", filename),
                });
            }
        }
    }
    None
}

/// Extract all `#[tauri::command]` function signatures (name + args + return type)
/// for cross-layer IPC context.
fn extract_command_signatures(cmd_dir: &Path) -> Vec<String> {
    let mut sigs = Vec::new();
    let entries = match std::fs::read_dir(cmd_dir) {
        Ok(e) => e,
        Err(_) => return sigs,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false)
            && path.file_name().map(|n| n != "mod.rs").unwrap_or(false)
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lines: Vec<&str> = content.lines().collect();
                let mut i = 0;
                while i < lines.len() {
                    if lines[i].contains("#[tauri::command") {
                        // Collect fn signature (may span multiple lines until `{` or `)`)
                        let mut sig = String::new();
                        let start = if i + 1 < lines.len() && lines[i + 1].contains("pub async fn") {
                            i + 1
                        } else if i + 1 < lines.len() && lines[i + 1].contains("pub fn") {
                            i + 1
                        } else {
                            i += 1;
                            continue;
                        };
                        for j in start..lines.len().min(start + 5) {
                            sig.push_str(lines[j].trim());
                            sig.push(' ');
                            if lines[j].contains('{') {
                                break;
                            }
                        }
                        let sig = sig.trim_end_matches(|c: char| c == '{' || c == ' ').trim().to_string();
                        if !sig.is_empty() {
                            sigs.push(sig);
                        }
                    }
                    i += 1;
                }
            }
        }
    }
    sigs
}

fn extract_invoke_handler(lib_path: &Path) -> Option<CodeSnippet> {
    let content = std::fs::read_to_string(lib_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let start = lines.iter().position(|l| l.contains("invoke_handler"))?;
    let mut depth = 0i32;
    let mut end = start;
    for (i, line) in lines[start..].iter().enumerate() {
        depth += line.matches('[').count() as i32;
        depth -= line.matches(']').count() as i32;
        end = start + i;
        if depth <= 0 && i > 0 {
            break;
        }
    }
    let block: Vec<&str> = lines[start..=end.min(lines.len() - 1)].to_vec();
    if block.is_empty() {
        return None;
    }

    Some(CodeSnippet {
        category: "wiring".into(),
        rel_path: "src-tauri/src/lib.rs".into(),
        content: block.join("\n"),
        description: "命令注册入口 (invoke_handler)".into(),
    })
}

fn sample_first_struct(dir: &Path, category: &str) -> Option<CodeSnippet> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false)
            && path.file_name().map(|n| n != "mod.rs").unwrap_or(false)
        {
            let content = std::fs::read_to_string(&path).ok()?;
            if let Some(snippet) = extract_first_struct_block(&content) {
                let filename = path.file_name()?.to_string_lossy().to_string();
                let parent = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                return Some(CodeSnippet {
                    category: category.into(),
                    rel_path: format!("src-tauri/src/{}/{}", parent, filename),
                    content: snippet,
                    description: "数据模型定义模式".into(),
                });
            }
        }
    }
    None
}

// ── TypeScript pattern sampling ──

fn sample_ts_patterns(root: &Path, patterns: &mut CodePatterns, total: &mut usize) {
    // 1. Sample a Zustand store
    let stores_dir = root.join("src/stores");
    if stores_dir.is_dir() && *total < MAX_TOTAL_CHARS {
        if let Some(snippet) = sample_ts_file(&stores_dir, "store", "Zustand Store 定义模式") {
            *total += snippet.content.len();
            patterns.snippets.push(snippet);
        }
    }

    // 2. Sample a page component
    let pages_dir = root.join("src/pages");
    if pages_dir.is_dir() && *total < MAX_TOTAL_CHARS {
        if let Some(snippet) =
            sample_ts_component_head(&pages_dir, "component", "页面组件结构模式")
        {
            *total += snippet.content.len();
            patterns.snippets.push(snippet);
        }
    }

    // 3. Sample a custom hook
    let hooks_dir = root.join("src/hooks");
    if hooks_dir.is_dir() && *total < MAX_TOTAL_CHARS {
        if let Some(snippet) = sample_ts_file(&hooks_dir, "hook", "自定义 Hook 模式") {
            *total += snippet.content.len();
            patterns.snippets.push(snippet);
        }
    }

    // 4. Extract invoke() call patterns from frontend (how frontend calls backend)
    if *total < MAX_TOTAL_CHARS {
        let invoke_patterns = extract_invoke_patterns(root);
        if !invoke_patterns.is_empty() {
            let invoke_text = invoke_patterns.join("\n");
            *total += invoke_text.len();
            patterns.snippets.push(CodeSnippet {
                category: "cross_layer".into(),
                rel_path: "src/".into(),
                content: invoke_text,
                description: "前端 invoke() 调用模式（展示前后端 IPC 约定）".into(),
            });
        }
    }

    // 5. Extract import ordering pattern from a representative file
    if *total < MAX_TOTAL_CHARS {
        let import_pattern = extract_import_pattern(root);
        if let Some(pattern) = import_pattern {
            patterns.module_patterns.push(pattern);
        }
    }
}

fn sample_ts_file(dir: &Path, category: &str, description: &str) -> Option<CodeSnippet> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension()?.to_string_lossy().to_string();
        if ext == "ts" || ext == "tsx" {
            let content = std::fs::read_to_string(&path).ok()?;
            let lines: Vec<&str> = content.lines().take(MAX_SNIPPET_LINES).collect();
            if lines.len() < 3 {
                continue;
            }
            let parent_name = dir.file_name()?.to_string_lossy().to_string();
            let filename = path.file_name()?.to_string_lossy().to_string();
            return Some(CodeSnippet {
                category: category.into(),
                rel_path: format!("src/{}/{}", parent_name, filename),
                content: lines.join("\n"),
                description: description.into(),
            });
        }
    }
    None
}

fn sample_ts_component_head(
    dir: &Path,
    category: &str,
    description: &str,
) -> Option<CodeSnippet> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .map(|e| e == "tsx" || e == "ts")
            .unwrap_or(false)
        {
            let content = std::fs::read_to_string(&path).ok()?;
            let lines: Vec<&str> = content.lines().collect();
            // Take imports + first component declaration (up to first return or MAX_SNIPPET_LINES)
            let limit = lines
                .iter()
                .position(|l| l.trim().starts_with("return"))
                .map(|i| (i + 3).min(MAX_SNIPPET_LINES))
                .unwrap_or(MAX_SNIPPET_LINES);
            let head: Vec<&str> = lines.iter().take(limit).copied().collect();
            if head.len() < 3 {
                continue;
            }
            let parent_name = dir.file_name()?.to_string_lossy().to_string();
            let filename = path.file_name()?.to_string_lossy().to_string();
            return Some(CodeSnippet {
                category: category.into(),
                rel_path: format!("src/{}/{}", parent_name, filename),
                content: head.join("\n"),
                description: description.into(),
            });
        }
    }
    None
}

// ── Pattern extraction helpers ──

fn extract_error_patterns(root: &Path, patterns: &mut CodePatterns) {
    let cmd_dir = root.join("src-tauri/src/commands");
    if !cmd_dir.is_dir() {
        return;
    }
    let mut seen = std::collections::HashSet::new();

    if let Ok(entries) = std::fs::read_dir(&cmd_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                // Detect .map_err patterns
                if content.contains(".map_err(|e| e.to_string())") && seen.insert("map_err_tostring")
                {
                    patterns
                        .error_patterns
                        .push("Tauri 命令统一用 `.map_err(|e| e.to_string())` 将错误转为 String".into());
                }
                if content.contains(".unwrap_or(") && seen.insert("unwrap_or") {
                    patterns
                        .error_patterns
                        .push("数据库查询使用 `.unwrap_or(0)` 提供默认值而非 panic".into());
                }
                // Detect thiserror usage
                if content.contains("thiserror") && seen.insert("thiserror") {
                    patterns
                        .error_patterns
                        .push("领域错误使用 `thiserror` 定义枚举".into());
                }
            }
        }
    }

    // Check core directory for thiserror
    let core_dir = root.join("src-tauri/src/core");
    if core_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&core_dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if content.contains("#[derive(Error")
                        && !patterns.error_patterns.iter().any(|p| p.contains("thiserror"))
                    {
                        patterns
                            .error_patterns
                            .push("领域错误使用 `thiserror` 定义枚举".into());
                    }
                }
            }
        }
    }
}

fn extract_module_patterns(root: &Path, patterns: &mut CodePatterns) {
    // Check if commands are organized as separate modules
    let cmd_mod = root.join("src-tauri/src/commands/mod.rs");
    if cmd_mod.exists() {
        if let Ok(content) = std::fs::read_to_string(&cmd_mod) {
            let mods: Vec<&str> = content
                .lines()
                .filter(|l| l.starts_with("pub mod "))
                .collect();
            if !mods.is_empty() {
                patterns.module_patterns.push(format!(
                    "Rust 命令按功能拆分模块：{}",
                    mods.join(", ")
                ));
            }
        }
    }

    // Check frontend structure
    let pages_dir = root.join("src/pages");
    let stores_dir = root.join("src/stores");
    let hooks_dir = root.join("src/hooks");
    let mut fe_parts = Vec::new();
    if pages_dir.is_dir() {
        fe_parts.push("pages/（页面组件）");
    }
    if stores_dir.is_dir() {
        fe_parts.push("stores/（Zustand 状态）");
    }
    if hooks_dir.is_dir() {
        fe_parts.push("hooks/（自定义 Hooks）");
    }
    if !fe_parts.is_empty() {
        patterns.module_patterns.push(format!(
            "前端按职责分层：{}",
            fe_parts.join(" + ")
        ));
    }
}

// ── Large file scanning ──

const LARGE_FILE_THRESHOLD: usize = 300;

/// Scan source directories for files exceeding LARGE_FILE_THRESHOLD lines.
/// Results are sorted by line count descending, capped at 10 entries.
fn scan_large_files(root: &Path, patterns: &mut CodePatterns) {
    let source_dirs = [
        ("src-tauri/src", &["rs"] as &[&str]),
        ("src", &["ts", "tsx"]),
    ];
    let mut large: Vec<LargeFileInfo> = Vec::new();

    for (dir_rel, exts) in &source_dirs {
        let dir = root.join(dir_rel);
        if dir.is_dir() {
            walk_source_files(&dir, exts, &mut |path| {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let line_count = content.lines().count();
                    if line_count > LARGE_FILE_THRESHOLD {
                        if let Ok(rel) = path.strip_prefix(root) {
                            large.push(LargeFileInfo {
                                rel_path: rel.to_string_lossy().to_string(),
                                line_count,
                            });
                        }
                    }
                }
            });
        }
    }

    large.sort_by(|a, b| b.line_count.cmp(&a.line_count));
    large.truncate(10);
    patterns.large_files = large;
}

/// Walk files with given extensions, skipping build/vendor dirs.
fn walk_source_files(dir: &Path, exts: &[&str], f: &mut dyn FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if !matches!(name.as_str(), "node_modules" | "dist" | "target" | ".next" | ".git") {
                walk_source_files(&path, exts, f);
            }
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if exts.iter().any(|e| *e == ext_str.as_ref()) {
                f(&path);
            }
        }
    }
}

// ── Code block extractors ──

fn extract_first_function_block(source: &str, marker: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let marker_line = lines.iter().position(|l| l.contains(marker))?;

    // Include attribute lines before the function
    let mut start = marker_line;
    while start > 0 && lines[start - 1].trim().starts_with('#') {
        start -= 1;
    }

    let mut depth = 0i32;
    let mut end = marker_line;
    let mut found_body = false;

    for (i, line) in lines[marker_line..].iter().enumerate() {
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth > 0 {
            found_body = true;
        }
        end = marker_line + i;
        if found_body && depth == 0 {
            break;
        }
        if i > MAX_SNIPPET_LINES {
            // Truncate long functions
            end = marker_line + MAX_SNIPPET_LINES;
            break;
        }
    }

    let block: Vec<&str> = lines[start..=end.min(lines.len() - 1)].to_vec();
    if block.is_empty() {
        return None;
    }

    let result = block.join("\n");
    if result.lines().count() > MAX_SNIPPET_LINES {
        let truncated: Vec<&str> = result.lines().take(MAX_SNIPPET_LINES).collect();
        Some(format!("{}\n    // ... (truncated)", truncated.join("\n")))
    } else {
        Some(result)
    }
}

fn extract_first_struct_block(source: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    // Find a pub struct with derive
    let struct_line = lines.iter().position(|l| l.contains("pub struct "))?;

    let mut start = struct_line;
    while start > 0
        && (lines[start - 1].trim().starts_with("#[") || lines[start - 1].trim().is_empty())
    {
        if lines[start - 1].trim().starts_with("#[") {
            start -= 1;
        } else {
            break;
        }
    }

    let mut depth = 0i32;
    let mut end = struct_line;
    let mut found_body = false;
    for (i, line) in lines[struct_line..].iter().enumerate() {
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth > 0 {
            found_body = true;
        }
        end = struct_line + i;
        if found_body && depth == 0 {
            break;
        }
        if i > MAX_SNIPPET_LINES {
            break;
        }
    }

    let block: Vec<&str> = lines[start..=end.min(lines.len() - 1)].to_vec();
    if block.len() < 2 {
        return None;
    }
    Some(block.join("\n"))
}
