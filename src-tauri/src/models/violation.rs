use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationCategory {
    Secret,
    ScopeBreak,
    TestWeaken,
    DepDowngrade,
    ConfigLeak,
}

impl ViolationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Secret => "secret",
            Self::ScopeBreak => "scope_break",
            Self::TestWeaken => "test_weaken",
            Self::DepDowngrade => "dep_downgrade",
            Self::ConfigLeak => "config_leak",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "secret" => Self::Secret,
            "scope_break" => Self::ScopeBreak,
            "test_weaken" => Self::TestWeaken,
            "dep_downgrade" => Self::DepDowngrade,
            "config_leak" => Self::ConfigLeak,
            _ => Self::Secret,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    High,
    Medium,
    Low,
}

impl ViolationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "high" => Self::High,
            "medium" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationStatus {
    Open,
    Resolved,
    Dismissed,
}

impl ViolationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "open" => Self::Open,
            "resolved" => Self::Resolved,
            "dismissed" => Self::Dismissed,
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: String,
    pub project_id: String,
    pub category: ViolationCategory,
    pub severity: ViolationSeverity,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub description: String,
    pub rule_ref: String,
    pub status: ViolationStatus,
    pub detected_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub project_id: String,
    pub total: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub violations: Vec<Violation>,
    pub checked_at: i64,
}
