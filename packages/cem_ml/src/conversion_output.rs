//! Shared helpers for conversion output pipelines.
//!
//! Format-specific modules still own their AST rendering rules. This module
//! keeps reusable writer/presentation mechanics in one place so CSV, YAML, and
//! future JSON output layers do not copy the same host-side glue.

use crate::conversion::ConversionOutputPipelineExecution;
use crate::diagnostics::{Diagnostic, Severity};
#[cfg(test)]
use crate::interpreter::OutputSpan;
#[cfg(test)]
use crate::source::ByteRange;
#[cfg(test)]
use crate::source_map::SourceMapStack;
use crate::transform_template::{
    transform_template_ensure_text_ends_with_newline, TransformTemplateEncodedArtifact,
    TransformTemplateEncodedArtifactPayload, TransformTemplateModuleVisibility,
    TransformTemplateOutputFunctionDescriptor, TransformTemplateOutputFunctionImplementation,
    TransformTemplateOutputFunctionKind, TransformTemplateOutputProducedKind,
    DEFAULT_FORMATTER_TAB_SIZE,
};
#[cfg(test)]
use serde_json::Value;

pub const CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE: &str =
    "cem.converter.output_pipeline_execution";

pub(crate) fn default_formatter_tab_size() -> usize {
    DEFAULT_FORMATTER_TAB_SIZE as usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormatterLineEndingMode {
    Lf,
    Crlf,
    Preserve,
}

pub(crate) fn parse_formatter_line_ending_option(
    key: &str,
    value: &str,
) -> Result<FormatterLineEndingMode, String> {
    match value {
        "lf" => Ok(FormatterLineEndingMode::Lf),
        "crlf" => Ok(FormatterLineEndingMode::Crlf),
        "preserve" => Ok(FormatterLineEndingMode::Preserve),
        _ => Err(format!(
            "Formatter option `{key}` must be `lf`, `crlf`, or `preserve`"
        )),
    }
}

pub(crate) fn parse_positive_formatter_usize_option(
    key: &str,
    value: &str,
) -> Result<usize, String> {
    let size = value
        .parse::<usize>()
        .map_err(|_| format!("Formatter option `{key}` must be a positive integer"))?;
    if size == 0 {
        return Err(format!(
            "Formatter option `{key}` must be greater than zero"
        ));
    }
    Ok(size)
}

pub(crate) fn resolve_formatter_line_ending(
    source_line_ending: Option<&str>,
    mode: Option<FormatterLineEndingMode>,
) -> Option<String> {
    match mode {
        Some(FormatterLineEndingMode::Lf) => Some("lf".to_owned()),
        Some(FormatterLineEndingMode::Crlf) => Some("crlf".to_owned()),
        Some(FormatterLineEndingMode::Preserve) => {
            let source_line_ending = source_line_ending.unwrap_or("lf");
            Some(
                match source_line_ending {
                    "crlf" => "crlf",
                    _ => "lf",
                }
                .to_owned(),
            )
        }
        None => None,
    }
}

pub(crate) fn html_pre_container_prefix(output_class: &str, tab_size: usize) -> String {
    format!(
        r#"<pre class="cem-output {output_class}" style="white-space: pre; tab-size: {tab_size}">"#
    )
}

pub(crate) fn wrap_html_pre_container_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    output_class: &str,
    tab_size: usize,
) {
    let Some(text) = artifact.value.as_str() else {
        return;
    };
    let prefix = html_pre_container_prefix(output_class, tab_size);
    let prefix_len = prefix.len() as u64;
    for span in &mut artifact.output_spans {
        span.output_range.start = span.output_range.start.saturating_add(prefix_len);
    }
    let mut wrapped = format!("{prefix}{text}</pre>");
    transform_template_ensure_text_ends_with_newline(&mut wrapped);
    artifact.value = TransformTemplateEncodedArtifactPayload::Text(wrapped);
}

#[cfg(test)]
pub(crate) fn output_span_value_for_source_map(
    text: &str,
    source_map: Option<&Value>,
) -> Option<Value> {
    let source_map = source_map?;
    let origin = serde_json::from_value::<SourceMapStack>(source_map.clone()).ok()?;
    serde_json::to_value(OutputSpan {
        output_range: ByteRange::new(0, u32::try_from(text.len()).unwrap_or(u32::MAX)),
        origin,
    })
    .ok()
}

pub(crate) struct CemtOutputFunctionDescriptorSpec<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub category: &'a str,
    pub subject: &'a str,
    pub kind: TransformTemplateOutputFunctionKind,
    pub produces: TransformTemplateOutputProducedKind,
    pub content_type: &'a str,
    pub schema: &'a str,
    pub canonical: bool,
    pub profile: Option<String>,
}

pub(crate) fn cemt_output_function_descriptor(
    spec: CemtOutputFunctionDescriptorSpec<'_>,
) -> TransformTemplateOutputFunctionDescriptor {
    TransformTemplateOutputFunctionDescriptor {
        kind: spec.kind,
        owner: Some(spec.owner.to_owned()),
        name: spec.name.to_owned(),
        category: spec.category.to_owned(),
        subject: spec.subject.to_owned(),
        produces: spec.produces,
        content_type: spec.content_type.to_owned(),
        schema: spec.schema.to_owned(),
        canonical: spec.canonical,
        streamable: true,
        visibility: TransformTemplateModuleVisibility::Public,
        implementation: TransformTemplateOutputFunctionImplementation::Cemt,
        profile: spec.profile,
        extends: None,
        capability: None,
        deterministic: true,
        trusted: false,
        lossy: false,
        fallback: None,
        params: Vec::new(),
        body_declared: false,
        body_expression: None,
    }
}

pub(crate) fn output_pipeline_diagnostic(
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
    message: String,
) -> Diagnostic {
    Diagnostic {
        uri: diagnostic_uri.map(str::to_owned),
        code: CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE.to_owned(),
        severity: Severity::Error,
        message: format!(
            "converter `{converter_id}` could not execute CEMT output pipeline: {message}"
        ),
        node: diagnostic_node.map(str::to_owned),
        details: None,
        ..Diagnostic::default()
    }
}

pub(crate) fn failed_pipeline_execution(
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    ConversionOutputPipelineExecution {
        output: None,
        diagnostics: vec![output_pipeline_diagnostic(
            converter_id,
            diagnostic_node,
            diagnostic_uri,
            message,
        )],
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
        ..ConversionOutputPipelineExecution::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::{FrameSpan, SourceMapFrame, TransformKind};
    use crate::transform_template::{
        TransformTemplateEncodedArtifactIdentity, TransformTemplateEncodingTarget,
    };

    #[test]
    fn html_pre_container_wraps_text_and_shifts_output_spans() {
        let mut artifact = TransformTemplateEncodedArtifact::new(
            TransformTemplateEncodedArtifactIdentity::new(
                TransformTemplateOutputProducedKind::Text,
                TransformTemplateEncodingTarget::new(
                    "text/html",
                    "https://cem.dev/ns/data/html/1",
                    "html-document",
                ),
            ),
            Value::String("id,name\n".to_owned()),
        );
        artifact.output_spans.push(OutputSpan {
            output_range: ByteRange::new(0, 2),
            origin: SourceMapStack::default(),
        });

        wrap_html_pre_container_artifact(&mut artifact, "cem-output-json", 4);

        let output = artifact.value.as_str().unwrap();
        assert!(output.starts_with(
            r#"<pre class="cem-output cem-output-json" style="white-space: pre; tab-size: 4">"#
        ));
        assert!(output.ends_with("</pre>\n"));
        assert_eq!(
            artifact.output_spans[0].output_range.start,
            html_pre_container_prefix("cem-output-json", 4).len() as u64
        );
    }

    #[test]
    fn source_map_output_span_value_preserves_origin() {
        let source_map = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: crate::source::SourceId(1),
                span: FrameSpan::Single(ByteRange::new(8, 3)),
                transform: TransformKind::ContentTypeTransform {
                    content_type: "application/json".to_owned(),
                },
            }],
        };
        let value = output_span_value_for_source_map(
            "abc",
            Some(&serde_json::to_value(source_map).unwrap()),
        )
        .unwrap();

        assert_eq!(value["outputRange"]["start"], 0);
        assert_eq!(value["outputRange"]["len"], 3);
        assert_eq!(value["origin"]["frames"][0]["source_id"], 1);
    }
}
