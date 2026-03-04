use crate::models::rule::RuleSuggestion;
use crate::services::db::Database;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct RuleExtractionResult {
    pub suggestions_found: u32,
    pub suggestions: Vec<RuleSuggestion>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn extract_rules(
    db: State<'_, Arc<Database>>,
    project_id: String,
) -> Result<RuleExtractionResult, String> {
    tracing::info!("Extracting rules for project {}", project_id);

    // TODO: Implement actual rule extraction:
    // 1. Read .ai/failures.jsonl
    // 2. Scan git log for fix/revert commits
    // 3. Embed and cluster failure signals
    // 4. For clusters with frequency >= threshold, generate rule suggestions via LLM
    // 5. Return suggestions

    Ok(RuleExtractionResult {
        suggestions_found: 0,
        suggestions: Vec::new(),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn accept_rule(
    db: State<'_, Arc<Database>>,
    rule_id: String,
    target_file: String,
    content: String,
) -> Result<(), String> {
    // Write the rule content to the target file
    let existing = std::fs::read_to_string(&target_file).unwrap_or_default();
    let updated = format!("{}\n\n{}", existing.trim_end(), content);
    std::fs::write(&target_file, updated)
        .map_err(|e| format!("Failed to write to {}: {}", target_file, e))?;

    // Update status in DB
    let conn = db.conn();
    conn.execute(
        "UPDATE rule_suggestions SET status = 'accepted' WHERE id = ?1",
        rusqlite::params![rule_id],
    )
    .map_err(|e| e.to_string())?;

    tracing::info!("Rule {} accepted and written to {}", rule_id, target_file);
    Ok(())
}
