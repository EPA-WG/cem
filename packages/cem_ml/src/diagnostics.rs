use crate::source::line_index::{HostCoordinate, LineIndex};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapStack};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub fn is_hard_violation(self) -> bool {
        matches!(self, Severity::Error | Severity::Fatal)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub uri: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u64>,
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Origin-first source-map stack, projected on demand into `line`/
    /// `column`. Per `cem-ml-cli-contract.md` §Output Shapes the JSON key
    /// is `sourceMap`.
    #[serde(rename = "sourceMap", skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
}

impl Default for Diagnostic {
    fn default() -> Self {
        Self {
            uri: None,
            line: None,
            column: None,
            byte_offset: None,
            code: String::new(),
            severity: Severity::Info,
            message: String::new(),
            node: None,
            details: None,
            source_map: None,
        }
    }
}

pub fn project_diagnostics_for_source(diagnostics: &mut [Diagnostic], source_bytes: &[u8]) {
    let line_index = LineIndex::from_bytes_lossy(source_bytes);
    project_diagnostics_with_line_index(diagnostics, &line_index, SourceId(1));
}

pub fn project_diagnostics_with_line_index(
    diagnostics: &mut [Diagnostic],
    line_index: &LineIndex,
    source_id: SourceId,
) {
    for diagnostic in diagnostics {
        if diagnostic.uri.is_some() {
            continue;
        }
        if let Some(byte_offset) = diagnostic.byte_offset {
            let coordinate = line_index.project_host(byte_offset);
            diagnostic.line.get_or_insert(coordinate.line);
            diagnostic.column.get_or_insert(coordinate.column);
            if let Some(details) = diagnostic_details_object_mut(diagnostic) {
                details
                    .entry("coordinates")
                    .or_insert_with(|| coordinate_details(byte_offset, coordinate));
            }
        }
        if let Some(source_map) = diagnostic.source_map.as_ref() {
            if let Some(coordinates) =
                source_map_coordinate_details(source_map, line_index, source_id)
            {
                if let Some(details) = diagnostic_details_object_mut(diagnostic) {
                    details.entry("sourceMapCoordinates").or_insert(coordinates);
                }
            }
        }
    }
}

pub fn coordinate_details(byte_offset: u64, coordinate: HostCoordinate) -> Value {
    json!({
        "byteOffset": byte_offset,
        "line": coordinate.line,
        "column": coordinate.column,
        "utf16Offset": coordinate.utf16_offset,
        "utf16Column": coordinate.utf16_column,
        "columnEncoding": "utf-16",
    })
}

pub fn source_map_coordinate_details(
    source_map: &SourceMapStack,
    line_index: &LineIndex,
    source_id: SourceId,
) -> Option<Value> {
    let frames = source_map
        .frames
        .iter()
        .filter(|frame| frame.source_id == source_id)
        .filter_map(|frame| {
            let ranges = source_map_span_coordinate_ranges(&frame.span, line_index);
            (!ranges.is_empty()).then(|| {
                json!({
                    "sourceId": frame.source_id.0,
                    "ranges": ranges,
                })
            })
        })
        .collect::<Vec<_>>();

    (!frames.is_empty()).then(|| {
        json!({
            "columnEncoding": "utf-16",
            "frames": frames,
        })
    })
}

fn source_map_span_coordinate_ranges(span: &FrameSpan, line_index: &LineIndex) -> Vec<Value> {
    match span {
        FrameSpan::Single(range) => vec![range_coordinate_details(*range, line_index)],
        FrameSpan::Multi(ranges) => ranges
            .iter()
            .map(|range| range_coordinate_details(*range, line_index))
            .collect(),
    }
}

fn range_coordinate_details(range: ByteRange, line_index: &LineIndex) -> Value {
    json!({
        "byteStart": range.start,
        "byteLen": range.len,
        "start": coordinate_details(range.start, line_index.project_host(range.start)),
        "end": coordinate_details(range.end(), line_index.project_host(range.end())),
    })
}

fn diagnostic_details_object_mut(diagnostic: &mut Diagnostic) -> Option<&mut Map<String, Value>> {
    if diagnostic.details.is_none() {
        diagnostic.details = Some(json!({}));
    }
    diagnostic.details.as_mut().and_then(Value::as_object_mut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::{SourceMapFrame, TransformKind};

    #[test]
    fn diagnostic_projection_fills_web_host_coordinates_without_moving_byte_offset() {
        let source = "{p | first\r\né😀 {bad}}\n";
        let mut diagnostics = vec![Diagnostic {
            byte_offset: Some(source.find("{bad").expect("fixture has nested brace") as u64),
            code: "cem.test.coordinate".to_owned(),
            severity: Severity::Error,
            message: "coordinate".to_owned(),
            source_map: Some(SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(19, 5)),
                    transform: TransformKind::CemTokenizer,
                }],
            }),
            ..Diagnostic::default()
        }];

        project_diagnostics_for_source(&mut diagnostics, source.as_bytes());

        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.byte_offset, Some(19));
        assert_eq!(diagnostic.line, Some(2));
        assert_eq!(diagnostic.column, Some(5));
        assert_eq!(
            diagnostic.details.as_ref().and_then(|details| details
                .pointer("/coordinates/utf16Offset")
                .and_then(Value::as_u64)),
            Some(16)
        );
        assert_eq!(
            diagnostic.details.as_ref().and_then(|details| details
                .pointer("/sourceMapCoordinates/frames/0/ranges/0/start/utf16Column")
                .and_then(Value::as_u64)),
            Some(5)
        );
    }

    #[test]
    fn diagnostic_projection_skips_explicit_foreign_uri() {
        let mut diagnostics = vec![Diagnostic {
            uri: Some("external.cem".to_owned()),
            byte_offset: Some(0),
            code: "cem.test.foreign".to_owned(),
            severity: Severity::Warning,
            message: "foreign".to_owned(),
            ..Diagnostic::default()
        }];

        project_diagnostics_for_source(&mut diagnostics, b"{p}");

        assert_eq!(diagnostics[0].line, None);
        assert_eq!(diagnostics[0].column, None);
        assert!(diagnostics[0].details.is_none());
    }
}
