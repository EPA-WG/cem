//! Source-language adapters share the same presentation boundary as documents.
use super::*;
mod native_projection;
mod xpath;
#[cfg(test)]
mod xpath_tests;

/// Retain source-language lifecycle results instead of validating them as CEM.
pub(super) fn native_lifecycle_parse(
    input: &EngineInput,
    context: &EngineContext,
    loaded: &mut LoadedInput,
    projection: ParseProjection,
) -> Option<SourceInspection> {
    let identity = effective_input_identity(input, context);
    let schema = identity.schema.as_deref().map(str::trim);
    let content_type = identity.content_type.as_deref().map(content_type_essence);
    let has_hard_diagnostics = loaded
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation());
    // A missing or refused owner must preserve its native diagnosis even when
    // a generic MIME or whitespace-padded alias accompanied the selector.
    // This recognition never admits or projects a successful unmatched input.
    let native_schema = matches!(
        schema,
        Some(
            CSS_SCHEMA_URI
                | crate::schema::registry::SCSS_SCHEMA_URI
                | XSLT_SCHEMA_URI
                | crate::schema::registry::XSLT_NAMESPACE_URI
                | RELAX_NG_SCHEMA_URI
                | MARKDOWN_SCHEMA_URI
        )
    );
    let native_mime = matches!(
        content_type.as_deref(),
        Some(
            CSS_CONTENT_TYPE
                | crate::schema::registry::SCSS_CONTENT_TYPE
                | "text/x-scss"
                | XSLT_CONTENT_TYPE
                | crate::validation::xslt::XSLT_TEXT_CONTENT_TYPE
                | crate::schema::registry::RELAX_NG_XML_CONTENT_TYPE
                | crate::schema::registry::RELAX_NG_COMPACT_CONTENT_TYPE
                | MARKDOWN_CONTENT_TYPE
        )
    );
    let xslt_namespace = content_type
        .as_deref()
        .filter(|mime| !mime.is_empty())
        .is_none()
        && schema.filter(|schema| !schema.is_empty()).is_none()
        && (identity.default_namespace.as_deref().map(str::trim)
            == Some(crate::schema::registry::XSLT_NAMESPACE_URI)
            || identity
                .namespaces
                .values()
                .any(|uri| uri.trim() == crate::schema::registry::XSLT_NAMESPACE_URI));
    let refused_source = has_hard_diagnostics
        && (native_schema
            || native_mime
            || xslt_namespace
            || loaded.adapter_id == Some("markdown"));
    if !matches!(
        loaded.ast_stream.as_ref(),
        Some(
            LoadedInputAstStream::CssDocument(_)
                | LoadedInputAstStream::XsltStylesheet(_)
                | LoadedInputAstStream::RelaxNgDocument(_)
                | LoadedInputAstStream::MarkdownDocument(_)
        )
    ) && !matches!(
        loaded.adapter_id,
        Some("css" | "scss" | "xslt" | "relax-ng" | "markdown")
    ) && !refused_source
    {
        return None;
    }
    let mut diagnostics = loaded.diagnostics.clone();
    diagnostics.extend(root_scope_execution_diagnostics(
        &input.uri,
        &input.root_scope,
        "input",
    ));
    project_diagnostics_for_source(&mut diagnostics, &input.bytes);
    project_diagnostic_uris(&mut diagnostics, input, context);
    let ranges: Vec<_> = match loaded.ast_stream.as_ref() {
        Some(LoadedInputAstStream::CssDocument(doc)) => doc
            .events
            .iter()
            .map(|event| {
                (
                    event.source_range.start.byte_offset,
                    event.source_range.byte_length,
                )
            })
            .collect(),
        Some(LoadedInputAstStream::XsltStylesheet(doc)) => doc
            .xml_document
            .events
            .iter()
            .map(|event| {
                (
                    event.source_range.start.byte_offset,
                    event.source_range.byte_length,
                )
            })
            .collect(),
        Some(LoadedInputAstStream::RelaxNgDocument(doc)) => {
            if let Some(xml) = doc.xml_document.as_ref() {
                xml.events
                    .iter()
                    .map(|event| {
                        (
                            event.source_range.start.byte_offset,
                            event.source_range.byte_length,
                        )
                    })
                    .collect()
            } else {
                doc.compact_tokens
                    .iter()
                    .map(|token| {
                        (
                            token.source_range.start.byte_offset,
                            token.source_range.byte_length,
                        )
                    })
                    .collect()
            }
        }
        Some(LoadedInputAstStream::MarkdownDocument(doc)) => doc.events.iter().map(|event|
            (event.source_range.start.byte_offset, event.source_range.byte_length)).collect(),
        _ => Vec::new(),
    };
    let offsets = ranges
        .into_iter()
        .map(|(offset, length)| ByteRange::new(offset, u32::try_from(length).unwrap_or(u32::MAX)))
        .collect();
    let projected = if has_hard_diagnostics {
        None
    } else {
        match loaded
            .ast_stream
            .take()
            .map(|native| native_projection::project(native, projection))
            .transpose()
        {
            Ok(stream) => stream.flatten(),
            Err(message) => {
                diagnostics.push(Diagnostic {
                    uri: Some(input_uri(input, context)),
                    code: "cem.source.projection_failed".into(),
                    severity: Severity::Error,
                    message,
                    ..Default::default()
                });
                None
            }
        }
    };
    let (ast, events) = if projection == ParseProjection::Ast {
        (projected, None)
    } else {
        (None, projected)
    };
    Some(SourceInspection {
        identity,
        ast,
        events,
        offsets,
        diagnostics,
    })
}

#[cfg(all(test, feature = "external-schema-assets"))]
#[test]
fn missing_native_source_selectors_keep_admission_errors_without_cem_fallback() {
    use crate::schema::registry::{SCSS_CONTENT_TYPE, XSLT_NAMESPACE_URI};
    let identity = |schema: Option<&str>, content_type: Option<&str>| FormatIdentity {
        schema: schema.map(str::to_owned),
        content_type: content_type.map(str::to_owned),
        ..Default::default()
    };
    let mut default_namespace = identity(None, None);
    default_namespace.default_namespace = Some(format!(" {XSLT_NAMESPACE_URI} "));
    let mut prefixed_namespace = identity(Some(" "), None);
    prefixed_namespace
        .namespaces
        .insert("xsl".into(), format!(" {XSLT_NAMESPACE_URI} "));
    let mut empty_mime_namespace = default_namespace.clone();
    empty_mime_namespace.content_type = Some(" ".into());
    for declared in [
        identity(Some(XSLT_SCHEMA_URI), Some(XML_CONTENT_TYPE)),
        identity(Some(RELAX_NG_SCHEMA_URI), Some(XML_CONTENT_TYPE)),
        identity(Some(XSLT_NAMESPACE_URI), None),
        default_namespace,
        prefixed_namespace,
        empty_mime_namespace,
        identity(None, Some(SCSS_CONTENT_TYPE)),
        identity(Some(MARKDOWN_SCHEMA_URI), Some(MARKDOWN_CONTENT_TYPE)),
    ] {
        let response = RealCemMlEngine
            .parse(ParseRequest {
                input: EngineInput {
                    uri: "memory://unadmitted/source".into(),
                    bytes: b"<source/>".to_vec(),
                    identity: Some(declared.clone()),
                    from_format: None,
                    root_scope: Default::default(),
                },
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: None,
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.schema_asset.missing" && d.severity.is_hard_violation()),
            "{declared:?}: {:?}",
            response.diagnostics
        );
        assert!(
            !response
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("cem.schema.")
                    || d.code.starts_with("cem.tokenizer.")
                    || d.code.starts_with("cem.lint.")),
            "{declared:?}: {:?}",
            response.diagnostics
        );
    }
}

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
        .find_map(|stage| stage.inspect(input))
        .or_else(|| xpath::load(input, context))?;
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
            cem_presentation_primary_bytes(&request.context, stream, schema, scope, &uri, &request.input.bytes, "parse")
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
    source.diagnostics.extend(time_budget_diagnostics(
        &request.input.root_scope,
        &["inspectms", "inspecttimebudgetms"],
        started.elapsed().as_nanos(),
    ));
    project_diagnostic_uris(&mut source.diagnostics, &request.input, &request.context);
    let uri = input_uri(&request.input, &request.context);
    if (matches!(request.show, InspectView::Ast | InspectView::Tree) && source.ast.is_none())
        || (request.show == InspectView::Events && source.events.is_none())
    {
        request.context.ensure_active()?;
        return Ok(InspectResponse {
            view: request.show,
            body: Value::Null,
            primary_bytes: None,
            diagnostics: source.diagnostics,
        });
    }
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
            cem_presentation_primary_bytes(&request.context, stream, schema, scope, &uri, &request.input.bytes, "inspect")
        })
        .transpose()?;
    request.context.ensure_active()?;
    Ok(InspectResponse {
        diagnostics: source.diagnostics,
        view: request.show,
        body,
        primary_bytes,
    })
}
