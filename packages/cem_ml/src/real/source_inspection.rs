//! Source-language adapters share the same presentation boundary as documents.
use super::*;

pub(super) fn load(input: &EngineInput, context: &EngineContext) -> Option<SourceInspection> {
    let uri = input_uri(input, context);
    let resolved;
    let input = if uri == input.uri {
        input
    } else {
        resolved = EngineInput {
            uri,
            ..input.clone()
        };
        &resolved
    };
    let mut source = context
        .source_inspection_stages
        .iter()
        .find_map(|stage| stage.inspect(input))?;
    source.diagnostics.extend(root_scope_execution_diagnostics(
        &input.uri,
        &input.root_scope,
        "input",
    ));
    project_diagnostics_for_source(&mut source.diagnostics, &input.bytes);
    project_diagnostic_uris(&mut source.diagnostics, input, context);
    Some(source)
}

fn stream(
    source: &SourceInspection,
    show: InspectView,
    uri: &str,
) -> (projection::CemTreeAstStream, &'static str) {
    let schema = CEM_AST_PROJECTION_SCHEMA_URI;
    let records = |view, fields, records| {
        inspect_records_presentation_stream_with_identity(
            view,
            uri,
            fields,
            records,
            &source.identity,
        )
    };
    match show {
        InspectView::Ast | InspectView::Tree => (
            source
                .ast
                .clone()
                .unwrap_or_else(|| projection::CemTreeAstStream::new(Vec::new())),
            schema,
        ),
        InspectView::Events => (
            source
                .events
                .clone()
                .unwrap_or_else(|| projection::CemTreeAstStream::new(Vec::new())),
            CEM_EVENTS_PROJECTION_SCHEMA_URI,
        ),
        InspectView::Summary => (
            records(
                "summary",
                vec![
                    ("nodes", source.offsets.len().to_string()),
                    ("diagnostics", source.diagnostics.len().to_string()),
                ],
                Vec::new(),
            ),
            schema,
        ),
        InspectView::Diagnostics => (
            records(
                "diagnostics",
                Vec::new(),
                source
                    .diagnostics
                    .iter()
                    .map(|d| {
                        let mut fields = vec![
                            ("code", d.code.clone()),
                            ("severity", inspect_severity_label(d.severity).into()),
                            ("message", d.message.clone()),
                        ];
                        if let Some(offset) = d.byte_offset {
                            fields.push(("byte-offset", offset.to_string()));
                        }
                        if let Some(line) = d.line {
                            fields.push(("line", line.to_string()));
                        }
                        if let Some(column) = d.column {
                            fields.push(("column", column.to_string()));
                        }
                        fields
                    })
                    .collect(),
            ),
            schema,
        ),
        InspectView::SourceOffsets => (
            records(
                "source-offsets",
                Vec::new(),
                source
                    .offsets
                    .iter()
                    .map(|r| {
                        vec![
                            ("byte-offset", r.start.to_string()),
                            ("byte-length", r.len.to_string()),
                        ]
                    })
                    .collect(),
            ),
            schema,
        ),
    }
}

pub(super) fn parse(
    request: &ParseRequest,
    source: SourceInspection,
) -> EngineResult<ParseResponse> {
    if matches!(
        request.projection,
        ParseProjection::DomJson | ParseProjection::Json
    ) {
        let mut diagnostics = source.diagnostics;
        diagnostics.push(Diagnostic {
            uri: Some(input_uri(&request.input, &request.context)),
            code: "cem.source.dom_projection_unsupported".into(),
            severity: Severity::Error,
            message:
                "Source languages have no document DOM; use ast, events, ast-json or events-json"
                    .into(),
            ..Diagnostic::default()
        });
        return Ok(ParseResponse {
            primary: Value::Null,
            primary_bytes: None,
            diagnostics,
        });
    }
    let show = if request.projection == ParseProjection::Events {
        InspectView::Events
    } else {
        InspectView::Ast
    };
    let uri = input_uri(&request.input, &request.context);
    let (stream, schema) = stream(&source, show, &uri);
    // JSON projections are an explicit output boundary, never a parser handoff.
    let primary = stream.clone().into_cemt_subject();
    let primary_bytes = request
        .presentation_scope
        .as_ref()
        .filter(|_| {
            matches!(
                request.projection,
                ParseProjection::Ast | ParseProjection::Events
            )
        })
        .map(|scope| {
            cem_presentation_primary_bytes(&request.context, stream, schema, scope, &uri, "parse")
        })
        .transpose()?;
    request.context.ensure_active()?;
    Ok(ParseResponse {
        primary,
        primary_bytes,
        diagnostics: source.diagnostics,
    })
}

pub(super) fn inspect(
    request: &InspectRequest,
    mut source: SourceInspection,
    started: Instant,
) -> EngineResult<InspectResponse> {
    if matches!(
        request.show,
        InspectView::Ast | InspectView::Tree | InspectView::Events
    ) && source.ast.is_none()
    {
        return Err(EngineError::Internal(
            source
                .diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    source.diagnostics.extend(time_budget_diagnostics(
        &request.input.root_scope,
        &["inspectms", "inspecttimebudgetms"],
        started.elapsed().as_nanos(),
    ));
    project_diagnostic_uris(&mut source.diagnostics, &request.input, &request.context);
    let uri = input_uri(&request.input, &request.context);
    let (stream, schema) = stream(&source, request.show, &uri);
    let body = match request.show {
        InspectView::Summary => {
            json!({"kind": "summary", "input": uri, "nodes": source.offsets.len(), "diagnosticCount": source.diagnostics.len()})
        }
        InspectView::Diagnostics => {
            json!({"kind": "diagnostics", "input": uri, "diagnostics": source.diagnostics})
        }
        InspectView::SourceOffsets => {
            json!({"kind": "source-offsets", "input": uri, "offsets": source.offsets.iter().map(|r| json!({"byteStart": r.start, "byteLen": r.len})).collect::<Vec<_>>()})
        }
        _ => stream.clone().into_cemt_subject(),
    };
    let primary_bytes = request
        .presentation_scope
        .as_ref()
        .map(|scope| {
            cem_presentation_primary_bytes(&request.context, stream, schema, scope, &uri, "inspect")
        })
        .transpose()?;
    request.context.ensure_active()?;
    Ok(InspectResponse {
        view: request.show,
        body,
        primary_bytes,
    })
}
