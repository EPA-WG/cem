use crate::diagnostics::Diagnostic;
use crate::engine::FailLevel;
use crate::scheduler::trace::{SchedulerEvent, SchedulerEventKind, SchedulerTrace};
use serde::{Deserialize, Serialize};

pub const DETERMINISTIC_TIMESTAMP: &str = "1970-01-01T00:00:00.000Z";
pub const CLI_REPORT_JSON_SCHEMA_URI: &str = "https://cem.dev/schema/cli/report.schema.json";

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportAst {
    #[serde(rename = "schedulerTrace", default)]
    pub scheduler_trace: SchedulerTraceReport,
    #[serde(rename = "convert", skip_serializing_if = "Option::is_none")]
    pub convert: Option<ConvertReport>,
    #[serde(rename = "transform", skip_serializing_if = "Option::is_none")]
    pub transform: Option<TransformReport>,
    #[serde(rename = "transformGraph", skip_serializing_if = "Option::is_none")]
    pub transform_graph: Option<TransformGraphReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerTraceReport {
    #[serde(rename = "eventCount")]
    pub event_count: u64,
    pub events: Vec<SchedulerTraceReportEvent>,
}

impl SchedulerTraceReport {
    pub fn from_trace(trace: &SchedulerTrace) -> Self {
        Self::from_events(trace.snapshot())
    }

    pub fn from_events(events: Vec<SchedulerEvent>) -> Self {
        let events: Vec<SchedulerTraceReportEvent> = events
            .into_iter()
            .map(SchedulerTraceReportEvent::from)
            .collect();
        Self {
            event_count: events.len() as u64,
            events,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchedulerTraceReportEvent {
    pub sequence: u64,
    #[serde(rename = "scopeId")]
    pub scope_id: u32,
    pub kind: SchedulerEventKind,
    pub task: String,
}

impl From<SchedulerEvent> for SchedulerTraceReportEvent {
    fn from(event: SchedulerEvent) -> Self {
        Self {
            sequence: event.sequence,
            scope_id: event.scope,
            kind: event.kind,
            task: event.task,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformReport {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub output_kind: String,
    #[serde(rename = "hasSourceMap")]
    pub has_source_map: bool,
    #[serde(rename = "outputSpanCount")]
    pub output_span_count: u64,
    #[serde(rename = "sourceMapRef", skip_serializing_if = "Option::is_none")]
    pub source_map_ref: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConvertReport {
    pub output_count: u64,
    pub outputs: Vec<ConvertOutputReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOutputReport {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub output_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversion: Option<crate::engine::ConvertExecutionMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphReport {
    pub export_count: u64,
    pub exports: Vec<TransformGraphExportReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphExportReport {
    pub export_id: String,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub output_kind: String,
    #[serde(rename = "hasSourceMap")]
    pub has_source_map: bool,
    #[serde(rename = "outputSpanCount")]
    pub output_span_count: u64,
    #[serde(rename = "sourceMapRef", skip_serializing_if = "Option::is_none")]
    pub source_map_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collection_items: Vec<TransformGraphCollectionItemReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphCollectionItemReport {
    pub input: String,
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(rename = "hasSourceMap")]
    pub has_source_map: bool,
    #[serde(rename = "outputSpanCount")]
    pub output_span_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub inputs: Vec<String>,
    pub summary: ReportSummary,
    pub options: ReportOptionsSnapshot,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "reportAst", default)]
    pub report_ast: ReportAst,
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
            report_ast: ReportAst::default(),
        }
    }

    pub fn deterministic(
        inputs: Vec<String>,
        diagnostics: Vec<Diagnostic>,
        options: ReportOptionsSnapshot,
    ) -> Self {
        Self::new(
            inputs,
            diagnostics,
            options,
            DETERMINISTIC_TIMESTAMP.to_owned(),
        )
    }

    pub fn with_scheduler_trace(mut self, trace: &SchedulerTrace) -> Self {
        self.set_scheduler_trace(trace);
        self
    }

    pub fn with_scheduler_trace_report(mut self, trace: SchedulerTraceReport) -> Self {
        self.report_ast.scheduler_trace = trace;
        self
    }

    pub fn set_scheduler_trace(&mut self, trace: &SchedulerTrace) {
        self.report_ast.scheduler_trace = SchedulerTraceReport::from_trace(trace);
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
