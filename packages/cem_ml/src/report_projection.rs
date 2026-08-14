//! Host-neutral byte projection for command-service reports.
//!
//! Browser, Node, and native adapters receive these bytes and only decide how
//! to publish them. Projection semantics and media types stay in common Rust.

use std::fmt;

use crate::diagnostics::Severity;
use crate::report::Report;
use crate::run_config::NormalizedReportProjection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedReportV1 {
    pub projection: NormalizedReportProjection,
    pub content_type: &'static str,
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ReportProjectionErrorV1 {
    Serialization(serde_json::Error),
}

impl fmt::Display for ReportProjectionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(error) => {
                write!(formatter, "serializing command report failed: {error}")
            }
        }
    }
}

impl std::error::Error for ReportProjectionErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for ReportProjectionErrorV1 {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

pub fn project_report_v1(
    report: &Report,
    projection: NormalizedReportProjection,
) -> Result<ProjectedReportV1, ReportProjectionErrorV1> {
    let (content_type, extension, bytes) = match projection {
        NormalizedReportProjection::Text => (
            "text/plain; charset=utf-8",
            "txt",
            render_text(report)?.into_bytes(),
        ),
        NormalizedReportProjection::Json => (
            "application/json",
            "json",
            serde_json::to_vec_pretty(report)?,
        ),
        NormalizedReportProjection::Xml => {
            ("application/xml", "xml", render_xml(report)?.into_bytes())
        }
        NormalizedReportProjection::Cem => (
            "application/cem+xml",
            "cem",
            render_cem(report)?.into_bytes(),
        ),
        NormalizedReportProjection::Html => (
            "text/html; charset=utf-8",
            "html",
            render_html(report)?.into_bytes(),
        ),
        NormalizedReportProjection::Markdown => (
            "text/markdown; charset=utf-8",
            "md",
            render_markdown(report)?.into_bytes(),
        ),
    };
    Ok(ProjectedReportV1 {
        projection,
        content_type,
        extension,
        bytes,
    })
}

fn render_text(report: &Report) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    out.push_str("cem-ml report\n");
    out.push_str(&format!("generated: {}\n", report.generated_at));
    append_summary_text(&mut out, report);
    out.push_str("inputs:\n");
    for input in &report.inputs {
        out.push_str("- ");
        out.push_str(input);
        out.push('\n');
    }
    out.push_str("diagnostics:\n");
    for diagnostic in &report.diagnostics {
        out.push_str("- ");
        out.push_str(severity_label(diagnostic.severity));
        out.push(' ');
        out.push_str(&diagnostic.code);
        if let Some(uri) = diagnostic.uri.as_deref() {
            out.push(' ');
            out.push_str(uri);
            if let Some(line) = diagnostic.line {
                out.push(':');
                out.push_str(&line.to_string());
                if let Some(column) = diagnostic.column {
                    out.push(':');
                    out.push_str(&column.to_string());
                }
            }
        }
        out.push_str(": ");
        out.push_str(&diagnostic.message);
        out.push('\n');
    }
    out.push_str("report-ast: ");
    out.push_str(&serde_json::to_string(&report.report_ast)?);
    out.push('\n');
    Ok(out)
}

fn render_xml(report: &Report) -> Result<String, serde_json::Error> {
    let mut out =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cem-ml-report generated-at=\"");
    push_xml_escaped(&mut out, &report.generated_at);
    out.push_str("\">\n  <summary");
    append_summary_xml_attributes(&mut out, report);
    out.push_str("/>\n  <inputs>\n");
    for input in &report.inputs {
        out.push_str("    <input uri=\"");
        push_xml_escaped(&mut out, input);
        out.push_str("\"/>\n");
    }
    out.push_str("  </inputs>\n  <diagnostics>\n");
    for diagnostic in &report.diagnostics {
        out.push_str("    <diagnostic severity=\"");
        out.push_str(severity_label(diagnostic.severity));
        out.push_str("\" code=\"");
        push_xml_escaped(&mut out, &diagnostic.code);
        out.push_str("\"");
        if let Some(uri) = diagnostic.uri.as_deref() {
            out.push_str(" uri=\"");
            push_xml_escaped(&mut out, uri);
            out.push('"');
        }
        out.push('>');
        push_xml_escaped(&mut out, &diagnostic.message);
        out.push_str("</diagnostic>\n");
    }
    out.push_str("  </diagnostics>\n  <report-ast-json>");
    push_xml_escaped(&mut out, &serde_json::to_string(&report.report_ast)?);
    out.push_str("</report-ast-json>\n</cem-ml-report>\n");
    Ok(out)
}

fn render_cem(report: &Report) -> Result<String, serde_json::Error> {
    let mut out = String::from("@doc cem-ml 1\n{report");
    push_cem_attribute(&mut out, "generated-at", &report.generated_at);
    out.push_str(" |\n  {summary");
    push_cem_attribute(
        &mut out,
        "input-count",
        &report.summary.input_count.to_string(),
    );
    push_cem_attribute(
        &mut out,
        "info-count",
        &report.summary.info_count.to_string(),
    );
    push_cem_attribute(
        &mut out,
        "warning-count",
        &report.summary.warning_count.to_string(),
    );
    push_cem_attribute(
        &mut out,
        "error-count",
        &report.summary.error_count.to_string(),
    );
    push_cem_attribute(
        &mut out,
        "fatal-count",
        &report.summary.fatal_count.to_string(),
    );
    push_cem_attribute(
        &mut out,
        "hard-violation-count",
        &report.summary.hard_violation_count.to_string(),
    );
    out.push_str("}\n  {inputs |\n");
    for input in &report.inputs {
        out.push_str("    {input");
        push_cem_attribute(&mut out, "uri", input);
        out.push_str("}\n");
    }
    out.push_str("  }\n  {diagnostics |\n");
    for diagnostic in &report.diagnostics {
        out.push_str("    {diagnostic");
        push_cem_attribute(&mut out, "severity", severity_label(diagnostic.severity));
        push_cem_attribute(&mut out, "code", &diagnostic.code);
        push_cem_attribute(&mut out, "message", &diagnostic.message);
        if let Some(uri) = diagnostic.uri.as_deref() {
            push_cem_attribute(&mut out, "uri", uri);
        }
        out.push_str("}\n");
    }
    out.push_str("  }\n  {report-ast");
    push_cem_attribute(
        &mut out,
        "json",
        &serde_json::to_string(&report.report_ast)?,
    );
    out.push_str("}\n}\n");
    Ok(out)
}

fn render_html(report: &Report) -> Result<String, serde_json::Error> {
    let mut out = String::from(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>cem-ml report</title></head><body><main>\n<h1>cem-ml report</h1>\n<dl class=\"summary\">\n",
    );
    for (label, value) in summary_pairs(report) {
        out.push_str("<dt>");
        out.push_str(label);
        out.push_str("</dt><dd>");
        out.push_str(&value.to_string());
        out.push_str("</dd>\n");
    }
    out.push_str("</dl>\n<h2>Inputs</h2><ul>\n");
    for input in &report.inputs {
        out.push_str("<li><code>");
        push_xml_escaped(&mut out, input);
        out.push_str("</code></li>\n");
    }
    out.push_str("</ul>\n<h2>Diagnostics</h2><ol>\n");
    for diagnostic in &report.diagnostics {
        out.push_str("<li data-severity=\"");
        out.push_str(severity_label(diagnostic.severity));
        out.push_str("\"><code>");
        push_xml_escaped(&mut out, &diagnostic.code);
        out.push_str("</code> ");
        push_xml_escaped(&mut out, &diagnostic.message);
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n<script type=\"application/json\" id=\"cem-report-ast\">");
    push_xml_escaped(&mut out, &serde_json::to_string(&report.report_ast)?);
    out.push_str("</script>\n</main></body></html>\n");
    Ok(out)
}

fn render_markdown(report: &Report) -> Result<String, serde_json::Error> {
    let mut out = String::from("# cem-ml report\n\n");
    out.push_str(&format!("Generated: `{}`\n\n", report.generated_at));
    for (label, value) in summary_pairs(report) {
        out.push_str(&format!("- {label}: {value}\n"));
    }
    out.push_str("\n## Inputs\n\n");
    for input in &report.inputs {
        out.push_str("- `");
        out.push_str(&input.replace('`', "\\`"));
        out.push_str("`\n");
    }
    out.push_str("\n## Diagnostics\n\n");
    for diagnostic in &report.diagnostics {
        out.push_str(&format!(
            "- **{}** `{}` — {}\n",
            severity_label(diagnostic.severity),
            diagnostic.code.replace('`', "\\`"),
            diagnostic.message
        ));
    }
    out.push_str("\n## Report AST\n\n```json\n");
    out.push_str(&serde_json::to_string_pretty(&report.report_ast)?);
    out.push_str("\n```\n");
    Ok(out)
}

fn append_summary_text(out: &mut String, report: &Report) {
    for (label, value) in summary_pairs(report) {
        out.push_str(label);
        out.push_str(": ");
        out.push_str(&value.to_string());
        out.push('\n');
    }
}

fn append_summary_xml_attributes(out: &mut String, report: &Report) {
    for (label, value) in summary_pairs(report) {
        out.push(' ');
        out.push_str(label);
        out.push_str("=\"");
        out.push_str(&value.to_string());
        out.push('"');
    }
}

fn summary_pairs(report: &Report) -> [(&'static str, u32); 6] {
    [
        ("inputs", report.summary.input_count),
        ("info", report.summary.info_count),
        ("warnings", report.summary.warning_count),
        ("errors", report.summary.error_count),
        ("fatal", report.summary.fatal_count),
        ("hard-violations", report.summary.hard_violation_count),
    ]
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

fn push_xml_escaped(out: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
}

fn push_cem_attribute(out: &mut String, name: &str, value: &str) {
    out.push_str(" @");
    out.push_str(name);
    out.push_str("=\"");
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(character),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::engine::FailLevel;
    use crate::report::{ReportOptionsSnapshot, DETERMINISTIC_TIMESTAMP};

    fn report() -> Report {
        Report::deterministic(
            vec!["studio://catalog/data.cem".to_owned()],
            vec![Diagnostic {
                uri: Some("studio://catalog/data.cem".to_owned()),
                line: Some(2),
                column: Some(3),
                code: "fixture.warning".to_owned(),
                severity: Severity::Warning,
                message: "escape <this> & that".to_owned(),
                ..Diagnostic::default()
            }],
            ReportOptionsSnapshot {
                fail_level: FailLevel::Validate,
                schema: None,
                content_type: Some("application/cem+xml".to_owned()),
                base_uri: Some("studio://catalog/".to_owned()),
            },
        )
    }

    #[test]
    fn every_normalized_projection_is_byte_stable_and_host_neutral() {
        let report = report();
        for (projection, content_type, extension, marker) in [
            (
                NormalizedReportProjection::Text,
                "text/plain; charset=utf-8",
                "txt",
                "cem-ml report",
            ),
            (
                NormalizedReportProjection::Json,
                "application/json",
                "json",
                DETERMINISTIC_TIMESTAMP,
            ),
            (
                NormalizedReportProjection::Xml,
                "application/xml",
                "xml",
                "<cem-ml-report",
            ),
            (
                NormalizedReportProjection::Cem,
                "application/cem+xml",
                "cem",
                "@doc cem-ml 1",
            ),
            (
                NormalizedReportProjection::Html,
                "text/html; charset=utf-8",
                "html",
                "<!doctype html>",
            ),
            (
                NormalizedReportProjection::Markdown,
                "text/markdown; charset=utf-8",
                "md",
                "# cem-ml report",
            ),
        ] {
            let first = project_report_v1(&report, projection).unwrap();
            let second = project_report_v1(&report, projection).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.content_type, content_type);
            assert_eq!(first.extension, extension);
            assert!(String::from_utf8(first.bytes).unwrap().contains(marker));
        }
    }
}
