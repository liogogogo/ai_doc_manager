use crate::services::git::GitService;
use crate::services::llm::LlmAdapter;
use crate::services::vector_index::{IndexedChunk, VectorIndex};
use std::path::Path;

pub struct ConflictDetector {
    vector_index: VectorIndex,
}

#[derive(Debug)]
pub struct DetectedConflict {
    pub document_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub description: String,
    pub suggestion: String,
    pub severity: String,
}

impl ConflictDetector {
    pub fn new() -> Self {
        Self {
            vector_index: VectorIndex::new(),
        }
    }

    /// Build the vector index from all documents in the project
    pub fn index_documents(&mut self, doc_chunks: Vec<IndexedChunk>) {
        self.vector_index.clear();
        for chunk in doc_chunks {
            self.vector_index.add_chunk(chunk);
        }
    }

    /// Detect conflicts between recent code changes and existing documentation
    pub async fn detect(
        &self,
        project_path: &Path,
        llm: &dyn LlmAdapter,
    ) -> Result<Vec<DetectedConflict>, ConflictError> {
        // 1. Get recent git diff
        let git = GitService::open(project_path)
            .map_err(|e| ConflictError::GitError(e.to_string()))?;

        let diff = git.get_recent_diff(5)
            .map_err(|e| ConflictError::GitError(e.to_string()))?;

        if diff.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Summarize the code changes
        let summary_prompt = format!(
            "Summarize the following code diff in 2-3 sentences, focusing on:\n\
             - What APIs/interfaces changed\n\
             - What data structures changed\n\
             - What business logic changed\n\n\
             Diff:\n{}\n\nSummary:",
            &diff[..diff.len().min(8000)]
        );

        let summary = llm.complete(&summary_prompt, 512).await
            .map_err(|e| ConflictError::LlmError(e.to_string()))?;

        // 3. Search for related doc chunks
        let related_chunks = self.vector_index.search(&summary, 10);

        if related_chunks.is_empty() {
            return Ok(Vec::new());
        }

        // 4. Check each chunk for conflicts
        let mut conflicts = Vec::new();
        for chunk in related_chunks {
            let check_prompt = format!(
                "Given this code change summary:\n{}\n\n\
                 And this documentation excerpt:\n{}\n\n\
                 Is the documentation now INCORRECT or OUTDATED because of the code change? \
                 If yes, respond with:\n\
                 CONFLICT: [description of the conflict]\n\
                 SUGGESTION: [how to fix the documentation]\n\
                 SEVERITY: [high/medium/low]\n\n\
                 If no conflict, respond with: NO_CONFLICT",
                summary, chunk.text
            );

            let response = llm.complete(&check_prompt, 512).await
                .map_err(|e| ConflictError::LlmError(e.to_string()))?;

            if response.contains("CONFLICT:") {
                let description = response
                    .lines()
                    .find(|l| l.starts_with("CONFLICT:"))
                    .map(|l| l.trim_start_matches("CONFLICT:").trim().to_string())
                    .unwrap_or_default();

                let suggestion = response
                    .lines()
                    .find(|l| l.starts_with("SUGGESTION:"))
                    .map(|l| l.trim_start_matches("SUGGESTION:").trim().to_string())
                    .unwrap_or_default();

                let severity = response
                    .lines()
                    .find(|l| l.starts_with("SEVERITY:"))
                    .map(|l| l.trim_start_matches("SEVERITY:").trim().to_lowercase())
                    .unwrap_or_else(|| "medium".to_string());

                conflicts.push(DetectedConflict {
                    document_path: chunk.document_id.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    description,
                    suggestion,
                    severity,
                });
            }
        }

        Ok(conflicts)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConflictError {
    #[error("Git error: {0}")]
    GitError(String),
    #[error("LLM error: {0}")]
    LlmError(String),
}
