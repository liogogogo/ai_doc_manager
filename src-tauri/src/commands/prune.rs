use crate::services::db::Database;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct PruneItem {
    pub file_path: String,
    pub line_range: String,
    pub snippet: String,
    pub category: String, // "linter" | "script" | "stale"
    pub replacement: String,
}

#[derive(Debug, Serialize)]
pub struct PruneScanResult {
    pub items_found: u32,
    pub items: Vec<PruneItem>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn scan_redundancy(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<PruneScanResult, String> {
    tracing::info!("Scanning redundancy for project {}", project_id);

    // TODO: Implement actual redundancy scanning:
    // 1. Glob all .md/.txt/.rst files in the project
    // 2. Parse each file into paragraphs
    // 3. For each paragraph, use LLM to classify:
    //    a. Can be replaced by linter rule
    //    b. Can be replaced by script/Makefile
    //    c. Stale / outdated content
    //    d. Informational (keep)
    // 4. For a/b/c, generate replacement suggestions
    // 5. Return items

    Ok(PruneScanResult {
        items_found: 0,
        items: Vec::new(),
    })
}
