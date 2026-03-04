use crate::services::llm::LlmAdapter;
use crate::services::markdown_parser;
use glob::glob;
use std::path::Path;

pub struct RedundancyPruner {
    scan_extensions: Vec<String>,
}

#[derive(Debug)]
pub enum PruneCategory {
    ReplacableByLinter,
    ReplacableByScript,
    Stale,
    Informational,
}

#[derive(Debug)]
pub struct PruneCandidate {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
    pub category: PruneCategory,
    pub replacement: String,
}

impl RedundancyPruner {
    pub fn new(scan_extensions: Vec<String>) -> Self {
        Self { scan_extensions }
    }

    /// Scan all documents in the project for redundant content
    pub async fn scan(
        &self,
        project_path: &Path,
        llm: &dyn LlmAdapter,
    ) -> Result<Vec<PruneCandidate>, PruneError> {
        let mut candidates = Vec::new();

        // Glob all matching files
        for ext in &self.scan_extensions {
            let pattern = format!("{}/**/*.{}", project_path.display(), ext);
            let paths: Vec<_> = glob(&pattern)
                .map_err(|e| PruneError::GlobError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            for path in paths {
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let sections = markdown_parser::parse_sections(&content);

                for section in &sections {
                    if section.content.len() < 20 {
                        continue;
                    }

                    let prompt = format!(
                        "Classify this documentation paragraph into ONE of these categories:\n\
                         A) LINTER - describes a rule that could be enforced by a linter (ESLint, Prettier, etc.)\n\
                         B) SCRIPT - describes a process that could be a shell script or Makefile target\n\
                         C) STALE - appears outdated or no longer accurate\n\
                         D) KEEP - valuable informational content that should be retained\n\n\
                         Paragraph:\n\"{}\"\n\n\
                         Respond with ONLY the letter (A/B/C/D) and a brief replacement suggestion if A/B/C.",
                        &section.content[..section.content.len().min(1000)]
                    );

                    let response = llm.complete(&prompt, 256).await
                        .map_err(|e| PruneError::LlmError(e.to_string()))?;

                    let trimmed = response.trim().to_uppercase();
                    let category = if trimmed.starts_with('A') {
                        PruneCategory::ReplacableByLinter
                    } else if trimmed.starts_with('B') {
                        PruneCategory::ReplacableByScript
                    } else if trimmed.starts_with('C') {
                        PruneCategory::Stale
                    } else {
                        continue; // Category D = keep
                    };

                    let replacement = response
                        .lines()
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();

                    candidates.push(PruneCandidate {
                        file_path: path.display().to_string(),
                        start_line: section.start_line,
                        end_line: section.end_line,
                        snippet: section.content[..section.content.len().min(100)].to_string(),
                        category,
                        replacement,
                    });
                }
            }
        }

        Ok(candidates)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PruneError {
    #[error("Glob error: {0}")]
    GlobError(String),
    #[error("LLM error: {0}")]
    LlmError(String),
}
