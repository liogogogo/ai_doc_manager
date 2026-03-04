use crate::models::conflict::Conflict;
use crate::services::db::Database;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ConflictScanResult {
    pub conflicts_found: u32,
    pub conflicts: Vec<Conflict>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn scan_conflicts(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<ConflictScanResult, String> {
    tracing::info!("Scanning conflicts for project {}", project_id);

    // TODO: Implement actual conflict detection:
    // 1. Get recent git diff
    // 2. Summarize changes with LLM
    // 3. Search vector index for related doc chunks
    // 4. Compare each chunk with code changes via LLM
    // 5. Return detected conflicts

    Ok(ConflictScanResult {
        conflicts_found: 0,
        conflicts: Vec::new(),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_conflict(
    db: State<'_, Arc<Database>>,
    conflict_id: String,
    action: String, // "resolve" | "dismiss"
) -> Result<(), String> {
    let status = match action.as_str() {
        "resolve" => "resolved",
        "dismiss" => "dismissed",
        _ => return Err("Invalid action".into()),
    };

    let conn = db.conn();
    conn.execute(
        "UPDATE conflicts SET status = ?1 WHERE id = ?2",
        rusqlite::params![status, conflict_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
