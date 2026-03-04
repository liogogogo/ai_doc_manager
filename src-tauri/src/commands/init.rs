use crate::core::project_initializer::{self, GeneratedRule, InitPlan};
use crate::services::tech_detector;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct GovernanceStatus {
    pub has_agents_md: bool,
    pub has_progress_md: bool,
    pub has_config_toml: bool,
    pub agents_md_path: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_governance(root_path: String) -> Result<GovernanceStatus, String> {
    let root = Path::new(&root_path);
    if !root.exists() {
        return Err(format!("路径不存在: {}", root_path));
    }

    let agents = root.join("AGENTS.md");
    let progress = root.join(".ai/progress.md");
    let config = root.join(".docguardian.toml");

    Ok(GovernanceStatus {
        has_agents_md: agents.exists(),
        has_progress_md: progress.exists(),
        has_config_toml: config.exists(),
        agents_md_path: if agents.exists() {
            Some(agents.to_string_lossy().to_string())
        } else {
            None
        },
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn scan_project(root_path: String) -> Result<InitPlan, String> {
    tracing::info!("scan_project called with root_path: {}", root_path);
    let path = Path::new(&root_path);
    // Canonicalize to handle relative paths
    let root = if path.is_absolute() {
        path.to_path_buf()
    } else {
        path.canonicalize().map_err(|e| format!("无法解析路径 '{}': {}", root_path, e))?
    };
    if !root.exists() {
        return Err(format!("路径不存在: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("路径不是目录: {}", root.display()));
    }
    let scan = tech_detector::scan_project(&root).map_err(|e| {
        tracing::error!("scan error: {}", e);
        e.to_string()
    })?;
    tracing::info!("scan complete: {} langs, {} frameworks", scan.languages.len(), scan.frameworks.len());
    let plan = project_initializer::generate_init_plan(&scan);
    tracing::info!("plan generated: {} rules, {} files", plan.rules.len(), plan.files.len());
    Ok(plan)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_init_rules(
    root_path: String,
    rules: Vec<GeneratedRule>,
) -> Result<InitPlan, String> {
    let root = Path::new(&root_path);
    let scan = tech_detector::scan_project(root).map_err(|e| e.to_string())?;
    let mut plan = project_initializer::generate_init_plan(&scan);
    // Replace auto-generated rules with user-edited rules
    plan.rules = rules;
    // Regenerate files with updated rules
    let files = crate::core::project_initializer::generate_files_from_plan(&plan);
    plan.files = files;
    Ok(plan)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn confirm_init(
    root_path: String,
    plan: InitPlan,
) -> Result<Vec<String>, String> {
    let root = Path::new(&root_path);
    let written = project_initializer::execute_init_plan(root, &plan)
        .map_err(|e| e.to_string())?;
    Ok(written)
}

#[derive(Debug, Serialize)]
pub struct GovernanceFileContent {
    pub name: String,
    pub path: String,
    pub content: String,
    pub exists: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn read_governance_file(
    root_path: String,
    file_type: String,
) -> Result<GovernanceFileContent, String> {
    let root = Path::new(&root_path);
    if !root.exists() {
        return Err(format!("路径不存在: {}", root_path));
    }

    let (file_path, file_name) = match file_type.as_str() {
        "agents_md" => (root.join("AGENTS.md"), "AGENTS.md"),
        "progress_md" => (root.join(".ai/progress.md"), "progress.md"),
        "config_toml" => (root.join(".docguardian.toml"), ".docguardian.toml"),
        _ => return Err(format!("未知的文件类型: {}", file_type)),
    };

    if !file_path.exists() {
        return Ok(GovernanceFileContent {
            name: file_name.to_string(),
            path: file_path.to_string_lossy().to_string(),
            content: String::new(),
            exists: false,
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    Ok(GovernanceFileContent {
        name: file_name.to_string(),
        path: file_path.to_string_lossy().to_string(),
        content,
        exists: true,
    })
}
