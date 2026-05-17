use crate::diagnostics::Diagnostic;
use crate::engine::FailLevel;
use serde::{Deserialize, Serialize};

pub const DETERMINISTIC_TIMESTAMP: &str = "1970-01-01T00:00:00.000Z";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportSummary {
    #[serde(rename = "inputCount")]
    pub input_count: u32,
    #[serde(rename = "infoCount")]
    pub info_count: u32,
    #[serde(rename = "warningCount")]
    pub warning_count: u32,
    #[serde(rename = "errorCount")]
    pub error_count: u32,
    #[serde(rename = "fatalCount")]
    pub fatal_count: u32,
    #[serde(rename = "hardViolationCount")]
    pub hard_violation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOptionsSnapshot {
    #[serde(rename = "failLevel")]
    pub fail_level: FailLevel,
    pub schema: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    #[serde(rename = "baseUri")]
    pub base_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub inputs: Vec<String>,
    pub summary: ReportSummary,
    pub options: ReportOptionsSnapshot,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn new(
        inputs: Vec<String>,
        diagnostics: Vec<Diagnostic>,
        options: ReportOptionsSnapshot,
        generated_at: String,
    ) -> Self {
        let summary = compute_summary(&inputs, &diagnostics);
        Self {
            generated_at,
            inputs,
            summary,
            options,
            diagnostics,
        }
    }

    pub fn deterministic(
        inputs: Vec<String>,
        diagnostics: Vec<Diagnostic>,
        options: ReportOptionsSnapshot,
    ) -> Self {
        Self::new(inputs, diagnostics, options, DETERMINISTIC_TIMESTAMP.to_owned())
    }
}

fn compute_summary(inputs: &[String], diagnostics: &[Diagnostic]) -> ReportSummary {
    let mut summary = ReportSummary {
        input_count: inputs.len() as u32,
        ..Default::default()
    };
    for d in diagnostics {
        match d.severity {
            crate::diagnostics::Severity::Info => summary.info_count += 1,
            crate::diagnostics::Severity::Warning => summary.warning_count += 1,
            crate::diagnostics::Severity::Error => summary.error_count += 1,
            crate::diagnostics::Severity::Fatal => summary.fatal_count += 1,
        }
        if d.severity.is_hard_violation() {
            summary.hard_violation_count += 1;
        }
    }
    summary
}
