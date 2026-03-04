use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocLayer {
    Rule,
    State,
    Contract,
    Decision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocHealth {
    Healthy,
    Warning,
    Conflict,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub project_id: String,
    pub rel_path: String,
    pub layer: DocLayer,
    pub hash: String,
    pub line_count: u32,
    pub last_scanned: i64,
    pub health: DocHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocChunk {
    pub id: i64,
    pub document_id: String,
    pub chunk_text: String,
    pub start_line: u32,
    pub end_line: u32,
}
