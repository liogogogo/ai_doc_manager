use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub config: ProjectConfig,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub layers: LayerConfig,
    pub gc: GcConfig,
    pub conflict_detection: ConflictDetectionConfig,
    pub rule_extraction: RuleExtractionConfig,
    pub pruner: PrunerConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub rule_paths: Vec<String>,
    pub state_paths: Vec<String>,
    pub state_capacity: u32,
    pub archive_dir: String,
    pub contract_paths: Vec<String>,
    pub decision_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcConfig {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub auto_commit: bool,
    pub commit_message_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetectionConfig {
    pub enabled: bool,
    pub watch_branches: Vec<String>,
    pub exclude_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExtractionConfig {
    pub enabled: bool,
    pub failure_log: String,
    pub min_frequency: u32,
    pub target_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunerConfig {
    pub enabled: bool,
    pub interval_days: u32,
    pub scan_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub max_tokens_per_request: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealth {
    pub project_id: String,
    pub health_score: u32,
    pub doc_count: u32,
    pub conflict_count: u32,
    pub stale_count: u32,
    pub rule_suggestion_count: u32,
    pub memory_line_count: u32,
    pub memory_capacity: u32,
    pub last_gc_at: Option<i64>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            layers: LayerConfig {
                rule_paths: vec!["AGENTS.md".into(), ".cursorrules".into()],
                state_paths: vec![".ai/progress.md".into()],
                state_capacity: 100,
                archive_dir: ".ai/archive".into(),
                contract_paths: vec!["docs/design/*.md".into()],
                decision_paths: vec!["docs/adr/*.md".into()],
            },
            gc: GcConfig {
                enabled: true,
                interval_minutes: 30,
                auto_commit: false,
                commit_message_template: "docs(gc): archive completed items from {source}".into(),
            },
            conflict_detection: ConflictDetectionConfig {
                enabled: true,
                watch_branches: vec!["main".into(), "develop".into()],
                exclude_paths: vec!["docs/adr/*".into()],
            },
            rule_extraction: RuleExtractionConfig {
                enabled: true,
                failure_log: ".ai/failures.jsonl".into(),
                min_frequency: 3,
                target_files: vec!["AGENTS.md".into()],
            },
            pruner: PrunerConfig {
                enabled: true,
                interval_days: 7,
                scan_extensions: vec!["md".into(), "txt".into(), "rst".into()],
            },
            llm: LlmConfig {
                provider: "ollama".into(),
                model: "llama3.1:8b".into(),
                base_url: "http://localhost:11434".into(),
                api_key: None,
                max_tokens_per_request: 16384,
            },
        }
    }
}
