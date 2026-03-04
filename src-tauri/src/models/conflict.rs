use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStatus {
    Open,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: Option<i64>,
    pub commit_hash: Option<String>,
    pub description: String,
    pub suggestion: Option<String>,
    pub severity: Severity,
    pub status: ConflictStatus,
    pub created_at: i64,
}
