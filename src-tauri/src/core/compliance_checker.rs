use crate::models::violation::{
    ComplianceReport, Violation, ViolationCategory, ViolationSeverity, ViolationStatus,
};
use std::path::{Path, PathBuf};

const SECRET_PATTERNS: &[&str] = &[
    "api_key", "api-key", "apikey",
    "secret_key", "secret-key", "secretkey",
    "jwt_secret", "jwt-secret",
    "private_key", "private-key",
    "database_url", "database-url",
    "openai_api_key", "anthropic_api_key",
];

const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", "dist", ".git", ".ai", "local_data",
];

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "json", "toml",
];

// Only JS/TS patterns — Rust uses #[ignore] but raw strings make static matching unreliable.
const TEST_WEAKEN_PATTERNS: &[&str] = &[
    "it.skip(",
    "xit(",
    "xdescribe(",
    "describe.skip(",
    "test.skip(",
];

const JS_TS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

pub struct ComplianceChecker {
    root_path: PathBuf,
    project_id: String,
}

impl ComplianceChecker {
    pub fn new(root_path: &Path, project_id: &str) -> Self {
        Self {
            root_path: root_path.to_path_buf(),
            project_id: project_id.to_string(),
        }
    }

    pub fn run_all_checks(&self) -> ComplianceReport {
        let mut violations = Vec::new();

        violations.extend(self.check_secrets());
        violations.extend(self.check_scope());
        violations.extend(self.check_test_weakening());

        let high = violations.iter().filter(|v| v.severity == ViolationSeverity::High).count();
        let medium = violations.iter().filter(|v| v.severity == ViolationSeverity::Medium).count();
        let low = violations.iter().filter(|v| v.severity == ViolationSeverity::Low).count();

        ComplianceReport {
            project_id: self.project_id.clone(),
            total: violations.len(),
            high,
            medium,
            low,
            violations,
            checked_at: chrono::Utc::now().timestamp(),
        }
    }

    fn check_secrets(&self) -> Vec<Violation> {
        let mut violations = Vec::new();
        let files = self.collect_source_files(&self.root_path);

        for file_path in files {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                for (line_idx, line) in content.lines().enumerate() {
                    let line_lower = line.to_lowercase();
                    let trimmed = line.trim();

                    if trimmed.starts_with("//")
                        || trimmed.starts_with("/*")
                        || trimmed.starts_with('*')
                        || (trimmed.starts_with('#') && !trimmed.starts_with("#["))
                    {
                        continue;
                    }

                    for pattern in SECRET_PATTERNS {
                        if !line_lower.contains(pattern) {
                            continue;
                        }
                        if self.looks_like_secret_assignment(&line_lower, pattern) {
                            let rel = self.rel_path(&file_path);
                            violations.push(Violation {
                                id: uuid::Uuid::new_v4().to_string(),
                                project_id: self.project_id.clone(),
                                category: ViolationCategory::Secret,
                                severity: ViolationSeverity::High,
                                file_path: rel.clone(),
                                line_number: Some((line_idx + 1) as u32),
                                description: format!(
                                    "Possible hardcoded secret '{}' in {}:{}",
                                    pattern, rel, line_idx + 1
                                ),
                                rule_ref: "Don't #1".to_string(),
                                status: ViolationStatus::Open,
                                detected_at: chrono::Utc::now().timestamp(),
                            });
                            break;
                        }
                    }
                }
            }
        }
        violations
    }

    fn looks_like_secret_assignment(&self, line: &str, pattern: &str) -> bool {
        if let Some(pos) = line.find(pattern) {
            let after = &line[pos + pattern.len()..];
            let after_trimmed = after.trim_start();

            if !after_trimmed.starts_with('=')
                && !after_trimmed.starts_with(':')
                && !after_trimmed.starts_with("\":") {
                return false;
            }

            let value_part = after_trimmed.trim_start_matches(|c: char| c == '=' || c == ':' || c == ' ');

            if value_part.starts_with('"') || value_part.starts_with('\'') {
                let quote = value_part.chars().next().unwrap();
                if let Some(end) = value_part[1..].find(quote) {
                    let value = &value_part[1..1 + end];
                    if value.len() >= 8
                        && !value.contains("env")
                        && !value.contains("ENV")
                        && !value.starts_with("${")
                        && !value.starts_with("process.env")
                        && value != "your_api_key_here"
                        && value != "placeholder"
                        && value != "changeme"
                        && !value.contains("dummy")
                        && !value.contains("example")
                        && !value.contains("test")
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check whether any git-tracked changes touch forbidden paths defined in AGENTS.md.
    /// Only runs when a `.git` directory is present; silently skips otherwise.
    fn check_scope(&self) -> Vec<Violation> {
        let mut violations = Vec::new();

        let git_dir = self.root_path.join(".git");
        if !git_dir.exists() {
            return violations;
        }

        let agents_md_path = self.root_path.join("AGENTS.md");
        let forbidden_dirs = if agents_md_path.exists() {
            self.parse_forbidden_paths(&agents_md_path)
        } else {
            vec![
                "dist".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "local_data".to_string(),
            ]
        };

        for cmd_args in [
            vec!["diff", "--name-only", "HEAD"],
            vec!["diff", "--name-only", "--cached"],
        ] {
            if let Ok(output) = std::process::Command::new("git")
                .args(&cmd_args)
                .current_dir(&self.root_path)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for file in stdout.lines() {
                        let file_trimmed = file.trim();
                        if file_trimmed.is_empty() {
                            continue;
                        }
                        for forbidden in &forbidden_dirs {
                            if file_trimmed.starts_with(&format!("{}/", forbidden))
                                || file_trimmed == forbidden.as_str()
                            {
                                violations.push(Violation {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    project_id: self.project_id.clone(),
                                    category: ViolationCategory::ScopeBreak,
                                    severity: ViolationSeverity::High,
                                    file_path: file_trimmed.to_string(),
                                    line_number: None,
                                    description: format!(
                                        "File '{}' modified in forbidden path '{}'",
                                        file_trimmed, forbidden
                                    ),
                                    rule_ref: format!("Scope: {}/", forbidden),
                                    status: ViolationStatus::Open,
                                    detected_at: chrono::Utc::now().timestamp(),
                                });
                            }
                        }
                    }
                }
            }
        }

        violations.dedup_by(|a, b| a.file_path == b.file_path);
        violations
    }

    fn parse_forbidden_paths(&self, agents_md_path: &Path) -> Vec<String> {
        let mut forbidden = Vec::new();
        if let Ok(content) = std::fs::read_to_string(agents_md_path) {
            let mut in_scope_table = false;
            for line in content.lines() {
                if line.contains("| 路径") && line.contains("| 权限") {
                    in_scope_table = true;
                    continue;
                }
                if in_scope_table {
                    if line.trim().starts_with('|') {
                        if line.contains("🚫") {
                            if let Some(path) = self.extract_table_path(line) {
                                forbidden.push(path);
                            }
                        }
                    } else if !line.trim().is_empty() && !line.contains("---") {
                        in_scope_table = false;
                    }
                }
            }
        }
        if forbidden.is_empty() {
            forbidden = vec![
                "dist".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "local_data".to_string(),
            ];
        }
        forbidden
    }

    fn extract_table_path(&self, line: &str) -> Option<String> {
        let cells: Vec<&str> = line.split('|').collect();
        if cells.len() >= 2 {
            let path_cell = cells[1].trim();
            let path = path_cell
                .trim_matches('`')
                .trim_end_matches('/')
                .to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
        None
    }

    /// Scan JS/TS files only — avoids Rust raw-string false positives.
    fn check_test_weakening(&self) -> Vec<Violation> {
        let mut violations = Vec::new();
        let files = self.collect_source_files(&self.root_path);

        for file_path in files {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !JS_TS_EXTENSIONS.contains(&ext) {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&file_path) {
                for (line_idx, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                        continue;
                    }

                    for pattern in TEST_WEAKEN_PATTERNS {
                        if trimmed.contains(pattern) {
                            let rel = self.rel_path(&file_path);
                            violations.push(Violation {
                                id: uuid::Uuid::new_v4().to_string(),
                                project_id: self.project_id.clone(),
                                category: ViolationCategory::TestWeaken,
                                severity: ViolationSeverity::Medium,
                                file_path: rel.clone(),
                                line_number: Some((line_idx + 1) as u32),
                                description: format!(
                                    "Test weakening pattern '{}' found in {}:{}",
                                    pattern, rel, line_idx + 1
                                ),
                                rule_ref: "Don't #5".to_string(),
                                status: ViolationStatus::Open,
                                detected_at: chrono::Utc::now().timestamp(),
                            });
                            break;
                        }
                    }
                }
            }
        }
        violations
    }

    fn collect_source_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.walk_dir(dir, &mut files);
        files
    }

    fn walk_dir(&self, dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if SKIP_DIRS.contains(&dir_name) || dir_name.starts_with('.') {
                    continue;
                }
                self.walk_dir(&path, out);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if SOURCE_EXTENSIONS.contains(&ext) {
                        out.push(path);
                    }
                }
            }
        }
    }

    fn rel_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}
