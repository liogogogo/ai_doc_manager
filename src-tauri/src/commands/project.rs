use crate::models::project::{Project, ProjectConfig, ProjectHealth};
use crate::services::db::Database;
use std::sync::Arc;
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub async fn add_project(
    db: State<'_, Arc<Database>>,
    name: String,
    root_path: String,
) -> Result<Project, String> {
    // Canonicalize path to get absolute path
    let path = std::path::Path::new(&root_path);
    let canonical = if path.is_absolute() {
        if !path.exists() {
            return Err(format!("路径不存在: {}", root_path));
        }
        path.to_path_buf()
    } else {
        path.canonicalize()
            .map_err(|e| format!("无法解析路径 '{}': {}", root_path, e))?
    };
    if !canonical.is_dir() {
        return Err(format!("路径不是目录: {}", canonical.display()));
    }
    let abs_path = canonical.to_string_lossy().to_string();
    let project_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| name.clone());

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let config = ProjectConfig::default();
    let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;

    let conn = db.conn();
    conn.execute(
        "INSERT INTO projects (id, name, root_path, config, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, project_name, abs_path, config_json, now, now],
    ).map_err(|e| e.to_string())?;

    Ok(Project {
        id,
        name: project_name,
        root_path: abs_path,
        config,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_project(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<(), String> {
    let conn = db.conn();
    conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![project_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_projects(
    db: State<'_, Arc<Database>>,
) -> Result<Vec<Project>, String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare("SELECT id, name, root_path, config, created_at, updated_at FROM projects ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            let config_str: String = row.get(3)?;
            let config: ProjectConfig = serde_json::from_str(&config_str)
                .unwrap_or_default();
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                config,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(projects)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_project_health(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<ProjectHealth, String> {
    let conn = db.conn();

    let doc_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE project_id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let conflict_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM conflicts WHERE project_id = ?1 AND status = 'open'",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let stale_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE project_id = ?1 AND health = 'stale'",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let rule_suggestion_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM rule_suggestions WHERE project_id = ?1 AND status = 'proposed'",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Calculate health score (simple formula)
    let penalty = (conflict_count * 10 + stale_count * 5 + rule_suggestion_count * 3) as u32;
    let health_score = 100u32.saturating_sub(penalty);

    Ok(ProjectHealth {
        project_id,
        health_score,
        doc_count,
        conflict_count,
        stale_count,
        rule_suggestion_count,
        memory_line_count: 0,
        memory_capacity: 100,
        last_gc_at: None,
    })
}
