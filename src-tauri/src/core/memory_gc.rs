use crate::services::llm::LlmAdapter;
use crate::services::markdown_parser;
use std::path::Path;

pub struct MemoryGcEngine {
    capacity: u32,
    archive_dir: String,
}

impl MemoryGcEngine {
    pub fn new(capacity: u32, archive_dir: &str) -> Self {
        Self {
            capacity,
            archive_dir: archive_dir.to_string(),
        }
    }

    /// Analyze a state file and identify completed items
    pub async fn analyze(
        &self,
        file_path: &Path,
        llm: &dyn LlmAdapter,
    ) -> Result<GcPlan, GcError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| GcError::IoError(e.to_string()))?;

        let line_count = markdown_parser::count_lines(&content);

        if (line_count as u32) <= self.capacity {
            return Ok(GcPlan {
                items_to_archive: Vec::new(),
                lines_before: line_count as u32,
                estimated_lines_after: line_count as u32,
            });
        }

        let sections = markdown_parser::parse_sections(&content);

        let prompt = format!(
            "Analyze the following progress/status document sections and identify which items are COMPLETED (done, finished, resolved, shipped). \
             Return ONLY the section headings that are completed, one per line.\n\n{}",
            sections
                .iter()
                .map(|s| format!("## {}\n{}", s.heading, s.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        let response = llm.complete(&prompt, 1024).await
            .map_err(|e| GcError::LlmError(e.to_string()))?;

        let completed_headings: Vec<String> = response
            .lines()
            .map(|l| l.trim().trim_start_matches("- ").trim_start_matches("* ").to_string())
            .filter(|l| !l.is_empty())
            .collect();

        let items_to_archive: Vec<GcArchiveItem> = sections
            .iter()
            .filter(|s| completed_headings.iter().any(|h| s.heading.contains(h.as_str())))
            .map(|s| GcArchiveItem {
                heading: s.heading.clone(),
                content: s.content.clone(),
                start_line: s.start_line,
                end_line: s.end_line,
            })
            .collect();

        let archived_lines: u32 = items_to_archive
            .iter()
            .map(|i| (i.end_line - i.start_line) as u32)
            .sum();

        Ok(GcPlan {
            items_to_archive,
            lines_before: line_count as u32,
            estimated_lines_after: (line_count as u32).saturating_sub(archived_lines),
        })
    }

    /// Execute the GC plan: archive items and trim the source file
    pub fn execute(&self, file_path: &Path, plan: &GcPlan) -> Result<String, GcError> {
        if plan.items_to_archive.is_empty() {
            return Ok(String::new());
        }

        // Create archive directory
        let archive_path = Path::new(&self.archive_dir);
        std::fs::create_dir_all(archive_path)
            .map_err(|e| GcError::IoError(e.to_string()))?;

        // Generate archive filename
        let date = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let source_name = file_path.file_stem().unwrap_or_default().to_string_lossy();
        let archive_file = archive_path.join(format!("{}-{}.md", date, source_name));

        // Write archive
        let archive_content = plan
            .items_to_archive
            .iter()
            .map(|item| format!("## {}\n{}\n", item.heading, item.content))
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        std::fs::write(&archive_file, &archive_content)
            .map_err(|e| GcError::IoError(e.to_string()))?;

        // Rewrite source file without archived sections
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| GcError::IoError(e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut keep_lines: Vec<&str> = Vec::new();

        let archived_ranges: Vec<(usize, usize)> = plan
            .items_to_archive
            .iter()
            .map(|i| (i.start_line, i.end_line))
            .collect();

        for (i, line) in lines.iter().enumerate() {
            if !archived_ranges.iter().any(|(start, end)| i >= *start && i < *end) {
                keep_lines.push(line);
            }
        }

        let new_content = keep_lines.join("\n");
        std::fs::write(file_path, new_content)
            .map_err(|e| GcError::IoError(e.to_string()))?;

        Ok(archive_file.to_string_lossy().to_string())
    }
}

#[derive(Debug)]
pub struct GcPlan {
    pub items_to_archive: Vec<GcArchiveItem>,
    pub lines_before: u32,
    pub estimated_lines_after: u32,
}

#[derive(Debug)]
pub struct GcArchiveItem {
    pub heading: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum GcError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("LLM error: {0}")]
    LlmError(String),
}
