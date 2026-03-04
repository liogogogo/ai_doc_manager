use crate::services::llm::LlmAdapter;
use std::path::Path;

pub struct RuleExtractor {
    min_frequency: u32,
}

#[derive(Debug)]
pub struct ExtractedRule {
    pub pattern: String,
    pub frequency: u32,
    pub suggestion: String,
    pub golden_example: Option<String>,
    pub target_file: String,
}

impl RuleExtractor {
    pub fn new(min_frequency: u32) -> Self {
        Self { min_frequency }
    }

    /// Extract implicit rules from failure signals
    pub async fn extract(
        &self,
        failure_log_path: &Path,
        project_path: &Path,
        llm: &dyn LlmAdapter,
    ) -> Result<Vec<ExtractedRule>, RuleError> {
        let mut signals = Vec::new();

        // 1. Read failure log if exists
        if failure_log_path.exists() {
            let content = std::fs::read_to_string(failure_log_path)
                .map_err(|e| RuleError::IoError(e.to_string()))?;
            for line in content.lines() {
                if !line.trim().is_empty() {
                    signals.push(line.to_string());
                }
            }
        }

        // 2. Scan git log for fix/revert commits
        if let Ok(git) = crate::services::git::GitService::open(project_path) {
            // TODO: Parse git log for fix/revert patterns
            // This requires extending GitService with log enumeration
        }

        if signals.len() < self.min_frequency as usize {
            return Ok(Vec::new());
        }

        // 3. Cluster signals and extract rules via LLM
        let signals_text = signals.join("\n");
        let prompt = format!(
            "Analyze the following error/failure signals and identify recurring patterns. \
             For each pattern that appears {} or more times, suggest a rule.\n\n\
             Signals:\n{}\n\n\
             For each pattern, respond in this format:\n\
             PATTERN: [description]\n\
             FREQUENCY: [count]\n\
             RULE: [the rule to add]\n\
             EXAMPLE: [code example showing correct usage]\n\
             TARGET: [which file to add the rule to, e.g. AGENTS.md]\n\
             ---",
            self.min_frequency, signals_text
        );

        let response = llm.complete(&prompt, 2048).await
            .map_err(|e| RuleError::LlmError(e.to_string()))?;

        // Parse the LLM response into structured rules
        let rules = parse_rule_response(&response);

        Ok(rules)
    }
}

fn parse_rule_response(response: &str) -> Vec<ExtractedRule> {
    let mut rules = Vec::new();
    let blocks: Vec<&str> = response.split("---").collect();

    for block in blocks {
        let lines: Vec<&str> = block.lines().collect();

        let pattern = lines.iter()
            .find(|l| l.starts_with("PATTERN:"))
            .map(|l| l.trim_start_matches("PATTERN:").trim().to_string());

        let frequency = lines.iter()
            .find(|l| l.starts_with("FREQUENCY:"))
            .and_then(|l| l.trim_start_matches("FREQUENCY:").trim().parse::<u32>().ok());

        let suggestion = lines.iter()
            .find(|l| l.starts_with("RULE:"))
            .map(|l| l.trim_start_matches("RULE:").trim().to_string());

        let example = lines.iter()
            .find(|l| l.starts_with("EXAMPLE:"))
            .map(|l| l.trim_start_matches("EXAMPLE:").trim().to_string());

        let target = lines.iter()
            .find(|l| l.starts_with("TARGET:"))
            .map(|l| l.trim_start_matches("TARGET:").trim().to_string());

        if let (Some(pattern), Some(freq), Some(suggestion), Some(target)) =
            (pattern, frequency, suggestion, target)
        {
            rules.push(ExtractedRule {
                pattern,
                frequency: freq,
                suggestion,
                golden_example: example,
                target_file: target,
            });
        }
    }

    rules
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("LLM error: {0}")]
    LlmError(String),
}
