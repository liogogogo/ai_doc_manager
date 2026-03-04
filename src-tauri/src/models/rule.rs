use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleStatus {
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSuggestion {
    pub id: String,
    pub project_id: String,
    pub cluster_id: Option<String>,
    pub pattern: String,
    pub frequency: u32,
    pub suggestion: String,
    pub golden_example: Option<String>,
    pub target_file: String,
    pub status: RuleStatus,
    pub created_at: i64,
}
