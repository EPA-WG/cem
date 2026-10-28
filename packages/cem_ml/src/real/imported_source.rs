//! Source operations over the shared retained data import, never a second parser.
use super::*;
use crate::parser::tree::RetainedCemTree;
use crate::schema::registry::{
    CSV_SCHEMA_URI, HTML_SCHEMA_URI, JSON_SCHEMA_SCHEMA_URI, JSON_VALUE_SCHEMA_URI,
    MATHML_SCHEMA_URI, SVG_SCHEMA_URI, XHTML_SCHEMA_URI, XML_SCHEMA_URI, YAML_SCHEMA_URI,
};

type ImportedSource = (SourceInspection, Option<Arc<RetainedCemTree>>);

pub(super) fn load(
    input: &EngineInput,
    context: &EngineContext,
    loaded: &LoadedInput,
    show: Option<InspectView>,
) -> Option<ImportedSource> {
    let mut identity = effective_input_identity(input, context);
    // A refused native admission must not fall back to CEM tokenization either.
    let projection_identity = crate::import::projection::ProjectionKind::identity(identity.schema.as_deref(), identity.content_type.as_deref()).is_some();
    let refused = loaded
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
        && (matches!(
            identity.schema.as_deref().map(str::trim),
            Some(
                JSON_VALUE_SCHEMA_URI
                    | JSON_SCHEMA_SCHEMA_URI
                    | YAML_SCHEMA_URI
                    | XML_SCHEMA_URI
                    | HTML_SCHEMA_URI
                    | XHTML_SCHEMA_URI
                    | SVG_SCHEMA_URI
                    | MATHML_SCHEMA_URI
                    | CSV_SCHEMA_URI
            )
        ) || matches!(
            identity
                .content_type
                .as_deref()
                .map(content_type_essence)
                .as_deref(),
            Some(
                "application/json"
                    | "text/json"
                    | "application/schema+json"
                    | "application/yaml"
                    | "application/x-yaml"
                    | "text/yaml"
                    | "text/x-yaml"
                    | "application/xml"
                    | "text/xml"
                    | "text/html"
                    | "application/xhtml+xml"
                    | "image/svg+xml"
                    | "application/mathml+xml"
                    | "text/csv"
            )
        ));
    let defaults = match loaded.ast_stream.as_ref() {
        Some(LoadedInputAstStream::ProjectionDocument(doc)) => (doc.kind.schema(), doc.content_type.as_str()),
        Some(LoadedInputAstStream::ControlDocument(doc)) => (doc.schema, doc.content_type.as_str()),
        Some(LoadedInputAstStream::JsonDocument(doc)) => {
            (JSON_VALUE_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::JsonSchemaDocument(doc)) => {
            (JSON_SCHEMA_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::YamlDocument(doc)) => {
            (YAML_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::XmlDocument(doc)) => {
            (XML_SCHEMA_URI, doc.source.media_type.as_str())
        }
        Some(LoadedInputAstStream::HtmlDocument(doc)) => {
            (HTML_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::XhtmlDocument(doc)) => {
            (XHTML_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::SvgDocument(doc)) => {
            (SVG_SCHEMA_URI, doc.source.content_type.as_str())
        }
        Some(LoadedInputAstStream::MathMlDocument(doc)) => (
            MATHML_SCHEMA_URI,
            doc.xml_document.source.content_type.as_str(),
        ),
        Some(LoadedInputAstStream::CsvDocument(doc)) => {
            (CSV_SCHEMA_URI, doc.source.media_type.as_str())
        }
        Some(LoadedInputAstStream::CssDocument(doc)) if show == Some(InspectView::Tree) => {
            (CSS_SCHEMA_URI, doc.source.media_type.as_str())
        }
        _ if loaded
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
            && matches!(
                loaded.adapter_id,
                Some(
                    "json"
                        | "json-schema"
                        | "yaml"
                        | "xml"
                        | "html"
                        | "xhtml"
                        | "svg"
                        | "mathml"
                        | "csv"
                )
            ) =>
        {
            ("", "")
        }
        _ if refused || projection_identity || crate::control_source::is_control_identity(&identity) => ("", ""),
        _ => return None,
    };
    if identity
        .schema
        .as_deref()
        .is_none_or(|s| s.trim().is_empty())
    {
        identity.schema = Some(defaults.0.into());
    }
    if identity
        .content_type
        .as_deref()
        .is_none_or(|s| s.trim().is_empty())
    {
        identity.content_type = Some(defaults.1.into());
    }
    let mut diagnostics = loaded.diagnostics.clone();
    diagnostics.extend(root_scope_execution_diagnostics(
        &input.uri,
        &input.root_scope,
        "input",
    ));
    if crate::control_source::is_control_identity(&identity)
        && !matches!(
            loaded.ast_stream,
            Some(LoadedInputAstStream::ControlDocument(_))
        )
        && !diagnostics.iter().any(|d| d.severity.is_hard_violation())
    {
        diagnostics.push(Diagnostic {
            uri: Some(input.uri.clone()),
            code: "cem.source.identity_unsupported".into(),
            severity: Severity::Error,
            message: "Control source tooling requires its registered source identity".into(),
            ..Default::default()
        });
    }
    if projection_identity && loaded.ast_stream.is_none() && !diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
        diagnostics.push(Diagnostic { uri: Some(input.uri.clone()), code: "cem.source.identity_unsupported".into(), severity: Severity::Error,
            message: "Projection source requires its matching native MIME/schema adapter".into(), ..Default::default() });
    }
    let owner = if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
        None
    } else {
        loaded.ast_stream.as_ref().and_then(|native| {
            match crate::import::retain_lifecycle(Arc::new(native.clone())) {
                Ok(owner) => Some(owner),
                Err(message) => {
                    diagnostics.push(Diagnostic {
                        uri: Some(input_uri(input, context)),
                        code: "cem.source.import_failed".into(),
                        severity: Severity::Error,
                        message,
                        ..Default::default()
                    });
                    None
                }
            }
        })
    };
    project_diagnostics_for_source(&mut diagnostics, &input.bytes);
    project_diagnostic_uris(&mut diagnostics, input, context);
    let schema = identity.schema.as_deref().unwrap_or_default();
    let mime = identity.content_type.as_deref().unwrap_or_default();
    let ast = owner
        .as_ref()
        .filter(|_| matches!(show, Some(InspectView::Ast | InspectView::Tree)))
        .map(|owner| projection::cem_tree_inspection_with_identity(owner.clone(), schema, mime));
    let events = owner
        .as_ref()
        .filter(|_| show == Some(InspectView::Events))
        .map(|owner| projection::cem_tree_events_with_identity(owner.clone(), schema, mime));
    let offsets = owner
        .as_ref()
        .map(|owner| {
            owner
                .ast()
                .iter()
                .enumerate()
                .filter_map(|(id, _)| {
                    owner.source_node_range(id as u32).map(|range| {
                        ByteRange::new(range.offset, range.length.try_into().unwrap_or(u32::MAX))
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some((
        SourceInspection {
            identity,
            ast,
            events,
            offsets,
            diagnostics,
        },
        owner,
    ))
}

pub(super) fn parse(
    request: &ParseRequest,
    (mut source, owner): ImportedSource,
    started: Instant,
) -> EngineResult<ParseResponse> {
    source.diagnostics.extend(parse_budget_diagnostics(
        &request.input.root_scope,
        started.elapsed().as_nanos(),
    ));
    project_diagnostic_uris(&mut source.diagnostics, &request.input, &request.context);
    request.context.ensure_active()?;
    let Some(owner) = owner else {
        return Ok(ParseResponse {
            primary: Value::Null,
            primary_bytes: None,
            diagnostics: source.diagnostics,
        });
    };
    if matches!(
        request.projection,
        ParseProjection::DomJson | ParseProjection::Json
    ) {
        let primary = projection::dom_json(owner.ast());
        request.context.ensure_active()?;
        return Ok(ParseResponse {
            primary,
            primary_bytes: None,
            diagnostics: source.diagnostics,
        });
    }
    source_inspection::parse(request, source)
}
