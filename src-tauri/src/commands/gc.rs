use crate::services::db::Database;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct GcResult {
    pub source_file: String,
    pub archive_file: String,
    pub items_archived: u32,
    pub lines_before: u32,
    pub lines_after: u32,
}

#[derive(Debug, Serialize)]
pub struct GcStatus {
    pub file_path: String,
    pub current_lines: u32,
    pub capacity: u32,
    pub completed_items: u32,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_memory_gc(
    db: State<'_, Arc<Database>>,
    project_id: String,
    file_path: String,
) -> Result<GcResult, String> {
    // Read the target file
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;

    let lines_before = content.lines().count() as u32;

    // TODO: Implement actual GC logic with LLM analysis
    // For now, return a placeholder
    tracing::info!("Memory GC triggered for {} ({})", file_path, project_id);

    Ok(GcResult {
        source_file: file_path,
        archive_file: String::new(),
        items_archived: 0,
        lines_before,
        lines_after: lines_before,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_gc_status(
    _db: State<'_, Arc<Database>>,
    project_id: String,
    file_paths: Vec<String>,
) -> Result<Vec<GcStatus>, String> {
    let mut statuses = Vec::new();

    for path in file_paths {
        let lines = match std::fs::read_to_string(&path) {
            Ok(content) => content.lines().count() as u32,
            Err(_) => 0,
        };

        statuses.push(GcStatus {
            file_path: path,
            current_lines: lines,
            capacity: 100,
            completed_items: 0,
        });
    }

    Ok(statuses)
}
