use crate::core::compliance_checker::ComplianceChecker;
use crate::models::violation::{
    ComplianceReport, Violation, ViolationCategory, ViolationSeverity, ViolationStatus,
};
use crate::services::db::Database;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub async fn run_compliance_check(
    db: State<'_, Arc<Database>>,
    project_id: String,
    root_path: String,
) -> Result<ComplianceReport, String> {
    tracing::info!("Running compliance check for project {} at {}", project_id, root_path);

    let root = Path::new(&root_path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root_path));
    }

    let checker = ComplianceChecker::new(root, &project_id);
    let report = checker.run_all_checks();

    let conn = db.conn();

    conn.execute(
        "DELETE FROM violations WHERE project_id = ?1 AND status = 'open'",
        rusqlite::params![project_id],
    )
    .map_err(|e| e.to_string())?;

    for v in &report.violations {
        conn.execute(
            "INSERT INTO violations (id, project_id, category, severity, file_path, line_number, description, rule_ref, status, detected_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                v.id,
                v.project_id,
                v.category.as_str(),
                v.severity.as_str(),
                v.file_path,
                v.line_number,
                v.description,
                v.rule_ref,
                v.status.as_str(),
                v.detected_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    tracing::info!(
        "Compliance check complete: {} violations (H:{} M:{} L:{})",
        report.total, report.high, report.medium, report.low
    );

    Ok(report)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_violations(
    db: State<'_, Arc<Database>>,
    project_id: String,
    status_filter: Option<String>,
) -> Result<Vec<Violation>, String> {
    let conn = db.conn();

    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match &status_filter {
        Some(status) => (
            "SELECT id, project_id, category, severity, file_path, line_number, description, rule_ref, status, detected_at
             FROM violations WHERE project_id = ?1 AND status = ?2 ORDER BY detected_at DESC"
                .to_string(),
            vec![
                Box::new(project_id) as Box<dyn rusqlite::types::ToSql>,
                Box::new(status.clone()),
            ],
        ),
        None => (
            "SELECT id, project_id, category, severity, file_path, line_number, description, rule_ref, status, detected_at
             FROM violations WHERE project_id = ?1 ORDER BY detected_at DESC"
                .to_string(),
            vec![Box::new(project_id) as Box<dyn rusqlite::types::ToSql>],
        ),
    };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(Violation {
                id: row.get(0)?,
                project_id: row.get(1)?,
                category: ViolationCategory::from_str(&row.get::<_, String>(2)?),
                severity: ViolationSeverity::from_str(&row.get::<_, String>(3)?),
                file_path: row.get(4)?,
                line_number: row.get(5)?,
                description: row.get(6)?,
                rule_ref: row.get(7)?,
                status: ViolationStatus::from_str(&row.get::<_, String>(8)?),
                detected_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut violations = Vec::new();
    for row in rows {
        violations.push(row.map_err(|e| e.to_string())?);
    }

    Ok(violations)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_violation_status(
    db: State<'_, Arc<Database>>,
    violation_id: String,
    new_status: String,
) -> Result<(), String> {
    let status = match new_status.as_str() {
        "resolved" | "dismissed" | "open" => new_status.as_str(),
        _ => return Err(format!("Invalid status: {}", new_status)),
    };

    let conn = db.conn();
    let updated = conn
        .execute(
            "UPDATE violations SET status = ?1 WHERE id = ?2",
            rusqlite::params![status, violation_id],
        )
        .map_err(|e| e.to_string())?;

    if updated == 0 {
        return Err(format!("Violation not found: {}", violation_id));
    }

    tracing::info!("Violation {} status updated to {}", violation_id, status);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn setup_git_hooks(root_path: String) -> Result<String, String> {
    let root = Path::new(&root_path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root_path));
    }

    let git_dir = root.join(".git");
    if !git_dir.exists() {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .map_err(|e| format!("Failed to init git: {}", e))?;
        tracing::info!("Initialized git repository at {}", root_path);
    }

    let hooks_dir = root.join(".githooks");
    std::fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;

    let pre_commit_script = r#"#!/bin/bash
# Auto-generated by DocGuardian - Pre-commit compliance check
# Based on AGENTS.md Commands section

set -e

echo "🔍 Running pre-commit compliance checks..."

# 1. Secret/credential scan (assignment pattern)
echo "  Checking for hardcoded secrets..."
if grep -r -n -E '(API[_-]?KEY|SECRET|TOKEN|PASSWORD)\s*[=:]\s*["'"'"'][^"'"'"']{8,}["'"'"']' \
    --include="*.rs" --include="*.ts" --include="*.tsx" --include="*.js" \
    --include="*.toml" --include="*.json" \
    --exclude-dir=node_modules --exclude-dir=target --exclude-dir=dist \
    --exclude-dir=.git --exclude-dir=local_data . 2>/dev/null | \
    grep -v -E '(test|mock|dummy|example|placeholder)'; then
    echo "❌ Possible hardcoded secrets detected. Please review."
    exit 1
else
    echo "  ✅ No hardcoded secrets found"
fi

# 2. Scope check - verify no forbidden paths are modified
echo "  Checking scope boundaries..."
FORBIDDEN_CHANGES=$(git diff --cached --name-only 2>/dev/null | grep -E '^(dist/|node_modules/|target/)' || true)
if [ -n "$FORBIDDEN_CHANGES" ]; then
    echo "❌ Modifications to forbidden paths detected:"
    echo "$FORBIDDEN_CHANGES"
    exit 1
else
    echo "  ✅ All changes within allowed scope"
fi

# 3. Test weakening check
echo "  Checking for test weakening..."
if git diff --cached -U0 2>/dev/null | grep -E '^\+.*(\#\[ignore\]|it\.skip|xit\(|xdescribe\(|describe\.skip)'; then
    echo "❌ Test weakening patterns detected in staged changes."
    exit 1
else
    echo "  ✅ No test weakening found"
fi

# 4. TypeScript type check (if available)
if [ -f "node_modules/.bin/tsc" ]; then
    echo "  Running TypeScript type check..."
    npx tsc --noEmit || { echo "❌ TypeScript type errors"; exit 1; }
    echo "  ✅ TypeScript types OK"
else
    echo "  ⏭ Skipping TypeScript check (tsc not available)"
fi

# 5. Rust check (if available)
if command -v cargo &> /dev/null && [ -d "src-tauri" ]; then
    echo "  Running cargo check..."
    cd src-tauri && cargo check --quiet && cd ..
    echo "  ✅ Rust compilation OK"
else
    echo "  ⏭ Skipping Rust check (cargo not available)"
fi

echo "✅ All pre-commit checks passed!"
"#;

    let hook_path = hooks_dir.join("pre-commit");
    std::fs::write(&hook_path, pre_commit_script).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms).map_err(|e| e.to_string())?;
    }

    std::process::Command::new("git")
        .args(["config", "core.hooksPath", ".githooks"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to set hooksPath: {}", e))?;

    tracing::info!("Git hooks set up at {}", hooks_dir.display());
    Ok(format!("Git hooks installed at {}", hooks_dir.display()))
}
