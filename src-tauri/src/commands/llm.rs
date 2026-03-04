use crate::services::code_sampler;
use crate::services::llm;
use crate::services::llm::ChatMessage;
use crate::services::tech_detector::{self, TechScanResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Managed state: holds multi-turn conversation history for refinement sessions.
/// Cleared on each new generation, accumulated during refinement.
pub struct ConversationState(pub Mutex<Vec<ChatMessage>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens_per_request: u32,
}

fn default_max_tokens() -> u32 {
    16384
}

/// Persist LLM config to app data dir
#[tauri::command(rename_all = "snake_case")]
pub async fn save_llm_config(
    app: tauri::AppHandle,
    config: LlmConfig,
) -> Result<(), String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&app_data).ok();
    let path = app_data.join("llm_config.json");
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    tracing::info!("LLM config saved: provider={}, model={}", config.provider, config.model);
    Ok(())
}

/// Load LLM config from app data dir
#[tauri::command(rename_all = "snake_case")]
pub async fn get_llm_config(app: tauri::AppHandle) -> Result<Option<LlmConfig>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let path = app_data.join("llm_config.json");
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: LlmConfig = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(config))
}

/// Test LLM connectivity
#[tauri::command(rename_all = "snake_case")]
pub async fn test_llm_connection(config: LlmConfig) -> Result<bool, String> {
    let adapter = llm::create_adapter(
        &config.provider,
        &config.base_url,
        &config.model,
        config.api_key.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    tracing::info!("Testing LLM connection: provider={}, model={}", config.provider, config.model);
    match adapter.health_check().await {
        Ok(v) => {
            tracing::info!("LLM connection test passed");
            Ok(v)
        }
        Err(e) => {
            tracing::warn!("LLM connection test failed: {}", e);
            Err(e.to_string())
        }
    }
}

/// Generate AGENTS.md content using LLM, with streaming via Tauri events.
/// Clears conversation history and starts a fresh session.
/// Enhanced pipeline: scan → code sampling → LLM generation → quality self-check.
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_agents_md_llm(
    app: tauri::AppHandle,
    root_path: String,
) -> Result<String, String> {
    let config = load_llm_config(&app)?;

    let root = Path::new(&root_path);

    // Phase 1: Scan project tech stack
    let scan = tech_detector::scan_project(root).map_err(|e| e.to_string())?;

    // Phase 2: Sample real code patterns for grounding
    let patterns = code_sampler::sample_project_patterns(root);
    let patterns_text = code_sampler::render_patterns_for_prompt(&patterns);
    tracing::info!(
        "Code sampling complete: {} snippets, {} wiring, {} error patterns, prompt addition={}chars",
        patterns.snippets.len(),
        patterns.wiring.len(),
        patterns.error_patterns.len(),
        patterns_text.len()
    );

    // Phase 3: Build prompt with enriched context
    let system_msg = ChatMessage {
        role: "system".into(),
        content: build_system_prompt(),
    };
    let user_msg = ChatMessage {
        role: "user".into(),
        content: build_generation_prompt(&scan, &patterns_text),
    };
    let messages = vec![system_msg.clone(), user_msg.clone()];

    tracing::info!(
        "Generating AGENTS.md via LLM (provider={}, model={}), prompt len={}",
        config.provider,
        config.model,
        messages.iter().map(|m| m.content.len()).sum::<usize>()
    );

    // Phase 4: Stream LLM generation
    let result = stream_or_complete_messages(&app, &config, &messages).await?;

    tracing::info!("LLM generation complete, response len={}", result.len());
    let content = extract_markdown_content(&result);

    // Save conversation history
    {
        let conv_state = app.state::<ConversationState>();
        let mut history = conv_state.0.lock().unwrap();
        history.clear();
        history.push(system_msg);
        history.push(user_msg);
        history.push(ChatMessage {
            role: "assistant".into(),
            content: content.clone(),
        });
        tracing::info!("Conversation history initialized: {} messages", history.len());
    }

    // Phase 5: Quality self-check — emit score via event so UI can display it
    let score = compute_quality_score(&content, &scan);
    tracing::info!("Quality score: {}/100 — {:?}", score.total, score.details);
    let _ = app.emit("agents-quality-score", &score);

    Ok(content)
}

/// Refine existing AGENTS.md content based on user feedback via LLM (streaming).
/// Appends to conversation history for multi-turn refinement.
#[tauri::command(rename_all = "snake_case")]
pub async fn refine_agents_md(
    app: tauri::AppHandle,
    current_content: String,
    user_feedback: String,
    root_path: String,
) -> Result<String, String> {
    let config = load_llm_config(&app)?;

    // Build messages snapshot in a sync block, then drop the lock before any .await
    let messages_snapshot = {
        let conv_state = app.state::<ConversationState>();
        let mut history = conv_state.0.lock().unwrap();

        // If no history (e.g. user edited manually then refines), bootstrap with current content
        if history.is_empty() {
            let root = Path::new(&root_path);
            let scan = tech_detector::scan_project(root).map_err(|e| e.to_string())?;
            let patterns = code_sampler::sample_project_patterns(root);
            let patterns_text = code_sampler::render_patterns_for_prompt(&patterns);
            history.push(ChatMessage {
                role: "system".into(),
                content: build_system_prompt(),
            });
            history.push(ChatMessage {
                role: "user".into(),
                content: build_generation_prompt(&scan, &patterns_text),
            });
            history.push(ChatMessage {
                role: "assistant".into(),
                content: current_content.clone(),
            });
        }

        // Build targeted refinement instruction
        let refine_instruction = build_refine_instruction(&user_feedback);
        history.push(ChatMessage {
            role: "user".into(),
            content: refine_instruction,
        });

        history.clone()
        // MutexGuard dropped here
    };

    tracing::info!(
        "Refining AGENTS.md via LLM, feedback='{}', {} messages in history",
        user_feedback.chars().take(50).collect::<String>(),
        messages_snapshot.len()
    );

    let result = stream_or_complete_messages(&app, &config, &messages_snapshot).await?;

    tracing::info!("LLM refinement complete, response len={}", result.len());
    let content = extract_markdown_content(&result);

    // Save assistant reply to history (new lock scope)
    {
        let conv_state = app.state::<ConversationState>();
        let mut history = conv_state.0.lock().unwrap();
        history.push(ChatMessage {
            role: "assistant".into(),
            content: content.clone(),
        });
        tracing::info!("Conversation history updated: {} messages", history.len());
    }

    // Quality self-check after refinement
    let root = Path::new(&root_path);
    if let Ok(scan) = tech_detector::scan_project(root) {
        let score = compute_quality_score(&content, &scan);
        tracing::info!("Post-refine quality score: {}/100", score.total);
        let _ = app.emit("agents-quality-score", &score);
    }

    Ok(content)
}

/// Result of comparing current AGENTS.md against latest project scan.
#[derive(Debug, Clone, Serialize)]
pub struct GovernanceDrift {
    pub is_stale: bool,
    pub drifts: Vec<DriftItem>,
    pub current_version: String,
    pub agents_md_len: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftItem {
    pub category: String,
    pub description: String,
    pub severity: String, // "high" | "medium" | "low"
}

/// Check if the existing AGENTS.md is still in sync with the project.
/// Compares the content of AGENTS.md against a fresh project scan.
#[tauri::command(rename_all = "snake_case")]
pub async fn check_governance_freshness(
    root_path: String,
) -> Result<GovernanceDrift, String> {
    let root = Path::new(&root_path);
    let agents_path = root.join("AGENTS.md");

    if !agents_path.exists() {
        return Ok(GovernanceDrift {
            is_stale: true,
            drifts: vec![DriftItem {
                category: "missing".into(),
                description: "AGENTS.md 文件不存在".into(),
                severity: "high".into(),
            }],
            current_version: "0.0".into(),
            agents_md_len: 0,
        });
    }

    let agents_content = std::fs::read_to_string(&agents_path).map_err(|e| e.to_string())?;
    let scan = tech_detector::scan_project(root).map_err(|e| e.to_string())?;

    let mut drifts = Vec::new();

    // Extract version from AGENTS.md header
    let current_version = agents_content
        .lines()
        .find(|l| l.contains("版本"))
        .and_then(|l| {
            l.split_whitespace()
                .find(|w| w.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
        })
        .unwrap_or("1.0")
        .to_string();

    let content_lower = agents_content.to_lowercase();

    // Check: languages mentioned? (fuzzy: split by -/_ and check all parts)
    for lang in &scan.languages {
        let lang_str = format!("{:?}", lang);
        if !fuzzy_match_in_content(&content_lower, &lang_str) {
            drifts.push(DriftItem {
                category: "language".into(),
                description: format!("检测到语言 {} 未在 AGENTS.md 中提及", lang_str),
                severity: "medium".into(),
            });
        }
    }

    // Check: frameworks mentioned? (fuzzy)
    for fw in &scan.frameworks {
        let fw_str = format!("{:?}", fw);
        if !fuzzy_match_in_content(&content_lower, &fw_str) {
            drifts.push(DriftItem {
                category: "framework".into(),
                description: format!("检测到框架 {} 未在 AGENTS.md 中提及", fw_str),
                severity: "high".into(),
            });
        }
    }

    // Check: top-level dirs mentioned in Project Structure?
    for dir in &scan.dir_structure.top_level_dirs {
        if !content_lower.contains(&dir.to_lowercase()) {
            drifts.push(DriftItem {
                category: "directory".into(),
                description: format!("目录 {}/ 未在 Project Structure 中列出", dir),
                severity: "low".into(),
            });
        }
    }

    // Check: tools/linters mentioned? (fuzzy)
    for tool in &scan.tools {
        if !fuzzy_match_in_content(&content_lower, &tool.name) {
            drifts.push(DriftItem {
                category: "tool".into(),
                description: format!("工具 {} ({}) 未在 AGENTS.md 中提及", tool.name, tool.config_file),
                severity: "medium".into(),
            });
        }
    }

    // Check: key sections exist?
    let required_sections = [
        ("Scope", "权限边界"),
        ("Don't", "安全红线"),
        ("Ask First", "需确认"),
        ("Commands", "验证命令"),
        ("Verification", "验证清单"),
        ("Governance Loop", "治理闭环"),
        ("Changelog", "更新记录"),
    ];
    for (section, label) in &required_sections {
        if !agents_content.contains(section) {
            drifts.push(DriftItem {
                category: "section".into(),
                description: format!("缺少 {} ({}) 章节", section, label),
                severity: "high".into(),
            });
        }
    }

    // Check: CI config exists but not reflected?
    if scan.dir_structure.has_ci && !agents_content.contains("CI") && !agents_content.contains("ci") {
        drifts.push(DriftItem {
            category: "ci".into(),
            description: "项目有 CI 配置，但 AGENTS.md 未提及 CI 相关规则".into(),
            severity: "medium".into(),
        });
    }

    // Check: test dir exists but Verification section missing detail?
    if scan.dir_structure.has_tests && !agents_content.contains("test") && !agents_content.contains("Test") {
        drifts.push(DriftItem {
            category: "testing".into(),
            description: "项目有测试目录，但 AGENTS.md 未包含测试相关命令".into(),
            severity: "medium".into(),
        });
    }

    let is_stale = !drifts.is_empty();

    Ok(GovernanceDrift {
        is_stale,
        drifts,
        current_version,
        agents_md_len: agents_content.len(),
    })
}

/// Use LLM to generate specific update suggestions based on detected drift.
/// Streams the suggestions via Tauri events.
#[tauri::command(rename_all = "snake_case")]
pub async fn suggest_governance_updates(
    app: tauri::AppHandle,
    root_path: String,
) -> Result<String, String> {
    let config = load_llm_config(&app)?;

    let root = Path::new(&root_path);
    let agents_path = root.join("AGENTS.md");
    let agents_content = std::fs::read_to_string(&agents_path).map_err(|e| e.to_string())?;
    let scan = tech_detector::scan_project(root).map_err(|e| e.to_string())?;

    // Build drift summary
    let drift_result = check_governance_freshness(root_path.clone()).await?;
    if !drift_result.is_stale {
        return Ok("治理框架与项目现状一致，无需更新。".into());
    }

    let drift_summary: String = drift_result.drifts.iter()
        .map(|d| format!("- [{}] {}", d.severity, d.description))
        .collect::<Vec<_>>()
        .join("\n");

    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: build_system_prompt(),
        },
        ChatMessage {
            role: "user".into(),
            content: format!(
r#"当前项目的 AGENTS.md（版本 {}）与项目实际状态存在以下偏差：

{}

请基于以上偏差，输出具体的 AGENTS.md 更新建议。格式要求：
1. 每条建议标明涉及的章节（如 Scope / Don't / Commands 等）
2. 给出具体的修改内容（可直接复制粘贴的 Markdown 片段）
3. 按优先级排序（高→低）
4. 最后给出建议的新版本号

不要输出完整的 AGENTS.md，只输出增量更新建议。"#,
                drift_result.current_version,
                drift_summary
            ),
        },
    ];

    tracing::info!(
        "Generating governance update suggestions, {} drifts detected",
        drift_result.drifts.len()
    );

    let result = stream_or_complete_messages(&app, &config, &messages).await?;
    let content = extract_markdown_content(&result);

    Ok(content)
}

/// Apply LLM-generated update suggestions to AGENTS.md.
/// Takes the suggestions text and current AGENTS.md, asks LLM to merge them
/// into a complete updated document, backs up the original, then writes the new version.
#[tauri::command(rename_all = "snake_case")]
pub async fn apply_governance_updates(
    app: tauri::AppHandle,
    root_path: String,
    suggestions: String,
) -> Result<String, String> {
    let config = load_llm_config(&app)?;

    let root = Path::new(&root_path);
    let agents_path = root.join("AGENTS.md");
    let agents_content = std::fs::read_to_string(&agents_path).map_err(|e| e.to_string())?;

    // Backup current AGENTS.md before overwriting
    let backup_path = root.join("AGENTS.md.bak");
    std::fs::copy(&agents_path, &backup_path).map_err(|e| e.to_string())?;
    tracing::info!("Backed up AGENTS.md to AGENTS.md.bak");

    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: build_system_prompt(),
        },
        ChatMessage {
            role: "user".into(),
            content: format!(
r#"以下是当前的 AGENTS.md 完整内容：

{}

---

以下是基于偏差分析生成的更新建议：

{}

---

请将以上更新建议合并到当前 AGENTS.md 中，输出完整的更新后文档。要求：
1. 保留原有所有章节结构和内容
2. 将建议中的修改准确插入到对应章节
3. 如果建议中提到新版本号，更新头部的版本号和更新日期（今天是 {}）
4. 直接输出完整 Markdown（不要用 ```markdown 包裹）
5. 不要遗漏任何原有内容"#,
                agents_content,
                suggestions,
                chrono::Local::now().format("%Y-%m-%d"),
            ),
        },
    ];

    tracing::info!("Applying governance updates via LLM, suggestions len={}", suggestions.len());

    let result = stream_or_complete_messages(&app, &config, &messages).await?;
    let content = extract_markdown_content(&result);

    // Write updated AGENTS.md
    std::fs::write(&agents_path, &content).map_err(|e| e.to_string())?;
    tracing::info!("AGENTS.md updated, new len={}", content.len());

    Ok(content)
}

// ── Internal helpers ──

/// Fuzzy match: split name by `-`, `_`, or camelCase boundaries, then check
/// if ALL parts appear (case-insensitive) in the content.
/// e.g. "typescript-strict" matches if content contains both "typescript" and "strict".
fn fuzzy_match_in_content(content_lower: &str, name: &str) -> bool {
    // First try exact match (case-insensitive)
    if content_lower.contains(&name.to_lowercase()) {
        return true;
    }
    // Split by common delimiters and camelCase
    let parts: Vec<String> = name
        .split(|c: char| c == '-' || c == '_' || c == ' ')
        .flat_map(|seg| {
            // Split camelCase: "JetpackCompose" → ["Jetpack", "Compose"]
            let mut words = Vec::new();
            let mut current = String::new();
            for ch in seg.chars() {
                if ch.is_uppercase() && !current.is_empty() {
                    words.push(current.clone());
                    current.clear();
                }
                current.push(ch);
            }
            if !current.is_empty() {
                words.push(current);
            }
            words
        })
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_lowercase())
        .collect();

    if parts.is_empty() {
        return false;
    }
    parts.iter().all(|part| content_lower.contains(part.as_str()))
}

/// Use streaming for OpenAI-compatible providers with full message history.
async fn stream_or_complete_messages(
    app: &tauri::AppHandle,
    config: &LlmConfig,
    messages: &[ChatMessage],
) -> Result<String, String> {
    // For OpenAI-compatible providers, use streaming
    let is_streaming_capable = config.provider != "ollama";

    if is_streaming_capable {
        let key = config.api_key.as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "API Key 未配置".to_string())?;
        let adapter = crate::services::llm::OpenAiCompatibleAdapter::new(
            &config.base_url,
            &config.model,
            key,
        );
        let app_clone = app.clone();
        let result = adapter
            .stream_complete_messages(messages, config.max_tokens_per_request, move |chunk| {
                let _ = app_clone.emit("llm-chunk", chunk);
            })
            .await
            .map_err(|e| {
                tracing::error!("LLM streaming failed: {}", e);
                e.to_string()
            })?;
        Ok(result)
    } else {
        // Fallback for Ollama: merge system + user messages into a single prompt
        // so the system prompt is not silently discarded
        let merged_prompt = messages.iter()
            .filter(|m| m.role == "system" || m.role == "user")
            .map(|m| {
                if m.role == "system" {
                    format!("[System Instructions]\n{}\n[End System Instructions]\n", m.content)
                } else {
                    m.content.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let adapter = llm::create_adapter(
            &config.provider,
            &config.base_url,
            &config.model,
            config.api_key.as_deref(),
        )
        .map_err(|e| e.to_string())?;

        let result = adapter
            .complete(&merged_prompt, config.max_tokens_per_request)
            .await
            .map_err(|e| {
                tracing::error!("LLM generation failed: {}", e);
                e.to_string()
            })?;
        Ok(result)
    }
}

fn load_llm_config(app: &tauri::AppHandle) -> Result<LlmConfig, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let path = app_data.join("llm_config.json");
    if !path.exists() {
        return Err("LLM 未配置，请先在设置中配置大模型 API".into());
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

fn build_system_prompt() -> String {
    r#"你是 AGENTS.md 治理文档生成器。目标：输出能被 AI 编码助手（Cursor/Copilot/Claude Code）直接机器解析和执行的治理规则。

## 写作原则（按优先级排序）

1. **表格 > 散文**：Scope 用表格（路径 | 权限 | 约束），Don't 用编号单行规则，不写解释性段落
2. **约束 > 解释**：只写"禁止 X"，不写"因为 Y 所以禁止 X"。AI 不需要 why，只需要 what
3. **一次定义**：每条规则只在一个章节出现。Scope 定义权限 → Don't 定义红线 → Ask First 定义确认项，三者不重复
4. **具体 > 抽象**：写 `#[ignore]` 而非"弱化测试"；写 `src-tauri/src/commands/` 而非"后端命令目录"
5. **采样驱动**：Style Guide 和 Examples 必须从代码采样中提炼，不能虚构

## 输出格式

- 直接输出 Markdown，不要用 ```markdown 包裹
- 中文为主，技术术语保留英文
- 章节用 `## N.` 编号（1-15），层级清晰
- Scope 用表格；Don't 用 `N. 动词 + 对象` 单行格式；Commands 用 bash 代码块"#
        .to_string()
}

fn build_generation_prompt(scan: &TechScanResult, code_patterns: &str) -> String {
    let langs: Vec<String> = scan.languages.iter().map(|l| format!("{:?}", l)).collect();
    let fws: Vec<String> = scan.frameworks.iter().map(|f| format!("{:?}", f)).collect();
    let tools: Vec<String> = scan.tools.iter().map(|t| t.name.clone()).collect();
    let top_dirs = scan.dir_structure.top_level_dirs.join(", ");

    let mut ctx = String::new();
    ctx.push_str(&format!("项目: {} | 语言: {} | 框架: {}\n",
        scan.project_name,
        if langs.is_empty() { "未知".into() } else { langs.join(", ") },
        if fws.is_empty() { "无".into() } else { fws.join(", ") },
    ));
    if !tools.is_empty() {
        ctx.push_str(&format!("工具: {}\n", tools.join(", ")));
    }
    ctx.push_str(&format!("目录: {}\n", top_dirs));
    ctx.push_str(&format!("规模: {} 文件, ~{} 行代码, ~{} 行文档\n",
        scan.dir_structure.total_files,
        scan.dir_structure.total_lines_code,
        scan.dir_structure.total_lines_docs,
    ));
    if scan.git_stats.is_git_repo {
        ctx.push_str(&format!("Git: {} commits, {} fix, {} revert",
            scan.git_stats.total_commits, scan.git_stats.fix_commits,
            scan.git_stats.revert_commits));
        if !scan.git_stats.recent_fix_patterns.is_empty() {
            ctx.push_str(&format!(", 常见修复: {}", scan.git_stats.recent_fix_patterns.join(", ")));
        }
        ctx.push('\n');
    }
    if scan.dir_structure.has_tests { ctx.push_str("有测试目录\n"); }
    if scan.dir_structure.has_ci { ctx.push_str("有 CI 配置\n"); }

    // Dependencies (all language ecosystems)
    append_deps(&mut ctx, "npm 依赖", &scan.dependencies.npm_deps, 20);
    append_deps(&mut ctx, "npm devDeps", &scan.dependencies.npm_dev_deps, 15);
    append_deps(&mut ctx, "Cargo 依赖", &scan.dependencies.cargo_deps, 20);
    append_deps(&mut ctx, "Go 依赖", &scan.dependencies.go_deps, 20);
    append_deps_custom(&mut ctx, "Python 依赖", &scan.dependencies.python_deps, 20, false);
    append_deps(&mut ctx, "Java 依赖", &scan.dependencies.java_deps, 20);
    append_deps(&mut ctx, "Swift 依赖", &scan.dependencies.swift_deps, 20);
    append_deps(&mut ctx, "Ruby 依赖", &scan.dependencies.ruby_deps, 20);
    append_deps(&mut ctx, "PHP 依赖", &scan.dependencies.php_deps, 20);

    // CI config
    if let Some(ci) = &scan.ci_config {
        ctx.push_str(&format!("CI: {} ", ci.provider));
        for wf in &ci.workflows {
            ctx.push_str(&format!("[{}: triggers={}, steps={}] ",
                wf.name,
                wf.triggers.join("/"),
                wf.steps_summary.join(" → ")
            ));
        }
        ctx.push('\n');
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    format!(
r#"根据以下扫描结果和代码采样，生成 AGENTS.md。

## 扫描结果
{ctx}
{code_patterns}
## 输出结构（严格 15 章，编号 `## 1.` 到 `## 15.`）

**1. 头部元信息** — 版本 1.0 | 日期 {today} | 一句话项目描述（含核心技术栈）| 适用 AI 助手列表

**2. Scope** — 用表格，不要散文列表。格式：

| 路径 | 权限 | 约束 |
|------|------|------|
| `src/` | ✏️ 读写 | — |
| `Cargo.toml` | ✏️ 读写 | 需 Ask First |
| `Cargo.lock` | 👁️ 只读 | — |
| `dist/` | 🚫 禁止 | — |

白名单应包含 AI 常改的配置文件（配合 Ask First）。`docs/` 是文档目录，应为 ✏️ 读写而非禁止。末尾附临时提权协议（4 步：说明→确认→执行→记录审计日志到 progress.md）。

**3. Don't** — 8-10 条单行规则，格式：`N. 禁止 [动作]（具体反模式）`。不要写解释性段落。
必须覆盖：硬编码密钥 | 依赖降级 | 凭记忆写三方 API（列出库名）| 指令模糊盲猜 | 弱化测试（`#[ignore]`, `it.skip`）| 覆盖治理文件 | 未确认写入 LLM 输出 | 操作黑名单路径 | 配置泄露
规则中提到的具体文件名必须出现在 Scope 表中，不要引用 Scope 未列出的文件。

**4. Style Guide** — 核心章节，必须从代码采样提炼。按语言分节，每节用简洁的规则列表：
- Rust：错误处理分层（thiserror 领域错误 / map_err IPC 边界 / 库调用就地处理）、Tauri 命令规范（rename_all, State 注入, async Result<T,String>）、序列化约定（Serialize/Deserialize derive）
- 前端：Zustand store 模式、导入顺序（从采样中提炼分组规则）、组件结构
- 跨层：IPC 命名映射 + 新增命令 4 步流程（commands/ → mod.rs → lib.rs → 前端 invoke）+ 完整的错误传播链

**5. Ask First** — 单行列表：新增依赖 | 改 API/Schema | 删公共接口 | 改 CI | 拆文件(>300行) | 改 Scope | 改构建配置

**6. Commands** — 分 3 个 bash 代码块（秒级快检 / 分钟级测试 / 提交前全量），每条命令必须有 `if ... then ... else echo "⏭ 跳过: 原因"; fi` 守卫。注意：检测目录用 `[ -d "dir" ]`（不是 `-f`）。密钥扫描用赋值模式正则（匹配 `KEY = "value"` 而非仅关键词）。提交前全量不要同时放 `cargo build --release` 和 `cargo tauri build`（后者已包含 Rust 编译）。

**7. Project Structure** — 基于扫描的目录树，**不标权限 emoji**（权限已在 Scope 表定义，不要重复）。只展示目录层级关系，用简洁注释说明用途（如 `commands/ — Tauri 命令模块`）。

**8. Verification** — 7 步编号列表，每步一行命令或描述：
① 范围自检（确认修改在白名单内）② lint ③ 类型检查 ④ 单测 ⑤ 构建 ⑥ `git diff --stat` ⑦ 更新 `.ai/progress.md`

**9. Memory** — 路径 `.ai/progress.md` | 归档 `.ai/archive/YYYY-MM.md` | 上限 200 行 | GC >30 天 | 会话结束更新 4 字段 | 优先级：AGENTS.md > progress.md > 任务文件 > 用户指令

**10. Examples** — 基于采样的跨端完整链路（Good 4 步 + Bad 标注安全风险），至少 2 组

**11. Context Budget** — ≤8 文件/次 | >500 行只加载关键函数 | 基于上方大文件清单生成导航索引（格式：`路径 (行数)` — `[L范围]` 功能描述）| 溢出时优先保留治理文件

**12. Multi-Agent** — 身份标签 `[Cursor]`/`[Claude]` | 写前先读 | 中断前记录阻塞项

**13. When Stuck** — 搜索 → 文档 → 问用户 → 2 次失败暂停

**14. Governance Loop** — 触发条件 + 更新流程 + 健康度指标表格（新鲜度≤30天 | 高危偏差=0 | 验证通过率 | 记忆≤200行）+ 回滚策略（Git, `docs(agents): v版本 — 摘要`）

**15. Changelog** — 表格：版本 | 日期 | 变更摘要 | 触发原因

## 自检清单（生成后逐条验证）
1. Scope 是否用表格而非散文？Don't 是否单行规则无解释段落？
2. Style Guide 是否从采样提炼？是否包含错误传播链和跨端 4 步流程？
3. Commands 每条是否有 if 守卫？密钥扫描是否用赋值模式正则？
4. Examples 是否展示完整跨端链路？Bad 是否标注安全风险？
5. Context Budget 是否为大文件清单中的每个文件生成了导航索引？
6. 同一约束是否只在一个章节定义（Scope/Don't/Ask First 不重复）？
"#,
    )
}

fn append_deps(ctx: &mut String, label: &str, deps: &[crate::services::tech_detector::DepEntry], limit: usize) {
    append_deps_custom(ctx, label, deps, limit, true);
}

fn append_deps_custom(ctx: &mut String, label: &str, deps: &[crate::services::tech_detector::DepEntry], limit: usize, use_at: bool) {
    if deps.is_empty() {
        return;
    }
    let deps_str: Vec<String> = deps.iter()
        .take(limit)
        .map(|d| if use_at {
            format!("{}@{}", d.name, d.version)
        } else {
            format!("{}{}", d.name, d.version)
        })
        .collect();
    ctx.push_str(&format!("{}: {}\n", label, deps_str.join(", ")));
}

/// Extract raw markdown from LLM response (strip ```markdown fences if present)
fn extract_markdown_content(raw: &str) -> String {
    let trimmed = raw.trim();

    // Try to strip ```markdown ... ``` wrapper
    if let Some(rest) = trimmed.strip_prefix("```markdown") {
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```md") {
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }

    trimmed.to_string()
}

// ── Quality scoring ──

/// Deterministic quality score computed locally (no LLM call) to give
/// immediate feedback on the generated AGENTS.md quality.
#[derive(Debug, Clone, Serialize)]
pub struct QualityScore {
    pub total: u32,
    pub details: Vec<ScoreItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreItem {
    pub dimension: String,
    pub score: u32,
    pub max: u32,
    pub note: String,
}

fn compute_quality_score(content: &str, scan: &TechScanResult) -> QualityScore {
    let mut items = Vec::new();
    let content_lower = content.to_lowercase();

    // 1. Section completeness (15 pts)
    let required_sections = [
        "Scope", "Don't", "Style Guide", "Ask First", "Commands",
        "Project Structure", "Verification", "Memory", "Examples",
        "Context Budget", "Multi-Agent", "When Stuck", "Governance", "Changelog",
    ];
    let found = required_sections.iter()
        .filter(|s| content.contains(*s))
        .count();
    let section_score = ((found as f32 / required_sections.len() as f32) * 15.0) as u32;
    items.push(ScoreItem {
        dimension: "章节完备性".into(),
        score: section_score,
        max: 15,
        note: format!("{}/{} 必要章节", found, required_sections.len()),
    });

    // 2. Tech stack coverage (15 pts)
    let mut tech_total = 0u32;
    let mut tech_found = 0u32;
    for lang in &scan.languages {
        tech_total += 1;
        if fuzzy_match_in_content(&content_lower, &format!("{:?}", lang)) {
            tech_found += 1;
        }
    }
    for fw in &scan.frameworks {
        tech_total += 1;
        if fuzzy_match_in_content(&content_lower, &format!("{:?}", fw)) {
            tech_found += 1;
        }
    }
    let tech_score = if tech_total > 0 {
        ((tech_found as f32 / tech_total as f32) * 15.0) as u32
    } else {
        15
    };
    items.push(ScoreItem {
        dimension: "技术栈覆盖".into(),
        score: tech_score,
        max: 15,
        note: format!("{}/{} 语言/框架被提及", tech_found, tech_total),
    });

    // 3. Code specificity — real code blocks + real project paths (15 pts)
    let code_blocks = content.matches("```").count() / 2;
    let has_real_paths = scan.dir_structure.top_level_dirs.iter()
        .filter(|d| content_lower.contains(&d.to_lowercase()))
        .count();
    let specificity_score = {
        let block_pts = (code_blocks.min(8) as f32 / 8.0 * 8.0) as u32;
        let path_pts = (has_real_paths.min(5) as f32 / 5.0 * 7.0) as u32;
        block_pts + path_pts
    };
    items.push(ScoreItem {
        dimension: "内容特异性".into(),
        score: specificity_score,
        max: 15,
        note: format!("{} 代码块, {} 个真实路径", code_blocks, has_real_paths),
    });

    // 4. Command executability — defensive patterns (12 pts)
    let has_defensive = content.contains("if ") && content.contains("then");
    let has_three_levels = content.contains("秒级") || content.contains("文件级");
    let has_bash_blocks = content.matches("```bash").count();
    let cmd_score = {
        let mut pts = 0u32;
        if has_defensive { pts += 4; }
        if has_three_levels { pts += 4; }
        pts += (has_bash_blocks.min(4) as u32) * 1;
        pts.min(12)
    };
    items.push(ScoreItem {
        dimension: "命令可执行性".into(),
        score: cmd_score,
        max: 12,
        note: format!("防御性={}, 分级={}, {} bash块", has_defensive, has_three_levels, has_bash_blocks),
    });

    // 5. Architecture-aware Style Guide (15 pts)
    let mut style_pts = 0u32;
    let has_error_propagation = content_lower.contains("错误传播")
        || content_lower.contains("error propagation")
        || (content_lower.contains("map_err") && content_lower.contains("catch"));
    let has_cross_layer = content_lower.contains("ipc")
        || content_lower.contains("invoke_handler")
        || content_lower.contains("跨层");
    let has_import_order = content_lower.contains("导入顺序") || content_lower.contains("import order");
    let has_naming_convention = content_lower.contains("snake_case") || content_lower.contains("pascal");
    let has_serialization = content_lower.contains("serialize") || content_lower.contains("序列化");

    if has_error_propagation { style_pts += 4; }
    if has_cross_layer { style_pts += 4; }
    if has_import_order { style_pts += 3; }
    if has_naming_convention { style_pts += 2; }
    if has_serialization { style_pts += 2; }
    items.push(ScoreItem {
        dimension: "架构感知 Style Guide".into(),
        score: style_pts.min(15),
        max: 15,
        note: format!(
            "错误链={}, 跨层={}, 导入序={}, 命名={}, 序列化={}",
            has_error_propagation, has_cross_layer, has_import_order,
            has_naming_convention, has_serialization
        ),
    });

    // 6. Verification & operational completeness (10 pts)
    let mut verify_pts = 0u32;
    let has_scope_check = content_lower.contains("范围自检") || content_lower.contains("白名单");
    let has_memory_update = content_lower.contains("更新记忆") || content_lower.contains("progress.md");
    let has_lint_step = content_lower.contains("lint") || content_lower.contains("clippy");
    let has_build_step = content_lower.contains("cargo build") || content_lower.contains("npm run build");
    let has_git_diff = content_lower.contains("git diff");
    if has_scope_check { verify_pts += 2; }
    if has_memory_update { verify_pts += 2; }
    if has_lint_step { verify_pts += 2; }
    if has_build_step { verify_pts += 2; }
    if has_git_diff { verify_pts += 2; }
    items.push(ScoreItem {
        dimension: "验证清单完整性".into(),
        score: verify_pts.min(10),
        max: 10,
        note: format!(
            "范围自检={}, 记忆更新={}, lint={}, 构建={}, diff={}",
            has_scope_check, has_memory_update, has_lint_step, has_build_step, has_git_diff
        ),
    });

    // 7. Advanced governance features (18 pts)
    let mut adv_pts = 0u32;
    if content.contains("临时提权") || content.contains("提权协议") { adv_pts += 3; }
    if content.contains("Context Budget") || content.contains("上下文预算") { adv_pts += 2; }
    if content.contains("Multi-Agent") || content.contains("多助手") { adv_pts += 2; }
    if content.contains("Memory") && (content.contains("200") || content.contains("容量")) { adv_pts += 2; }
    if content.contains("archive") || content.contains("归档") { adv_pts += 2; }
    if content.contains("健康度") || content.contains("量化") { adv_pts += 3; }
    if content_lower.contains("git") && content_lower.contains("回滚") { adv_pts += 2; }
    let dont_rules = content.lines()
        .filter(|l| {
            let t = l.trim();
            (t.starts_with("- **") || t.starts_with("1.") || t.starts_with("2.") ||
             t.starts_with("3.") || t.starts_with("4.") || t.starts_with("5.") ||
             t.starts_with("6.") || t.starts_with("7.") || t.starts_with("8.") ||
             t.starts_with("9.") || t.starts_with("10."))
            && t.contains("禁止")
        })
        .count();
    if dont_rules >= 8 { adv_pts += 2; }
    items.push(ScoreItem {
        dimension: "前沿治理特性".into(),
        score: adv_pts.min(18),
        max: 18,
        note: format!("{}pts — 提权/预算/协作/健康度/归档/回滚/红线深度({}条)", adv_pts, dont_rules),
    });

    let total: u32 = items.iter().map(|i| i.score).sum();

    QualityScore { total, details: items }
}

// ── Refine instruction builder ──

fn build_refine_instruction(user_feedback: &str) -> String {
    let feedback_lower = user_feedback.to_lowercase();

    // Detect if user is asking for a specific section to be improved
    let section_hints: Vec<(&str, &str)> = vec![
        ("scope", "Scope（权限边界）"),
        ("权限", "Scope（权限边界）"),
        ("don't", "Don't（安全红线）"),
        ("红线", "Don't（安全红线）"),
        ("style", "Style Guide（编码风格）"),
        ("风格", "Style Guide（编码风格）"),
        ("command", "Commands（验证命令）"),
        ("验证", "Commands（验证命令）"),
        ("example", "Examples（示例）"),
        ("示例", "Examples（示例）"),
        ("memory", "Memory（会话记忆）"),
        ("记忆", "Memory（会话记忆）"),
        ("context", "Context Budget（上下文预算）"),
        ("上下文", "Context Budget（上下文预算）"),
    ];

    let matched_sections: Vec<&str> = section_hints.iter()
        .filter(|(key, _)| feedback_lower.contains(key))
        .map(|(_, label)| *label)
        .collect();

    let section_directive = if matched_sections.is_empty() {
        String::new()
    } else {
        format!(
            "\n重点优化以下章节：{}\n其他章节保持不变，不要删减内容。\n",
            matched_sections.join("、")
        )
    };

    format!(
        "请根据以下优化意见修改 AGENTS.md，直接输出完整的优化后 Markdown（不要用 ```markdown 包裹）。\
        {section_directive}\
        \n优化意见：\n{user_feedback}\n\n\
        要求：\n\
        1. 保留所有原有章节结构和编号\n\
        2. 优化内容必须具体到项目（引用真实路径、命令、代码模式），不要泛化\n\
        3. 如果是增强某个章节，在该章节中标注改进的部分",
    )
}
