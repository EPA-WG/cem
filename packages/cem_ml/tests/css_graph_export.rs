//! Typed CSS graph exports use the CSS writer after native CSS/SCSS loading.
use cem_ml::{
    command_service::VirtualResourceV1,
    engine::{CemMlEngine, EngineContext, FormatIdentity, TransformGraphResponse},
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    },
    source_map::{ScssOriginKind, TransformKind},
    transform_config::{
        parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
    },
    transform_graph_request::{
        lower_transform_graph_request, ManifestTransformGraphResourceProvider,
    },
};
use std::collections::BTreeMap;

const ROOT: &str = "studio://styles/";
const CONFIG: &str = "studio://styles/graphs/build.cem";
const CSS: &str = "/* Café */\n@scope (.profile-card) { h2 { color: #036; } }\n";
const SCSS: &str =
    "@use \"tokens\";\n.card { color: tokens.$accent; .title { font-weight: 700; } }\n";
const MODULE: &str = "$accent: #036;\n";

#[derive(Clone)]
struct DeclaredResources(BTreeMap<String, VirtualResourceV1>);
impl ResourceResolver for DeclaredResources {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let uri = if request.uri.contains("://") {
            request.uri.clone()
        } else {
            url::Url::parse(request.base_uri.as_deref().unwrap_or(ROOT))
                .unwrap()
                .join(&request.uri)
                .unwrap()
                .to_string()
        };
        self.0
            .get(&uri)
            .map(|resource| ResolvedRead {
                uri: uri.clone(),
                bytes: resource.bytes.clone(),
                content_type: resource
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.clone()),
            })
            .ok_or_else(|| ResolverDiagnostic::Io {
                uri,
                message: "not declared".into(),
            })
    }
    fn write(
        &self,
        request: &ResolveRequest,
        _: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        Err(ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::Write,
        })
    }
}

fn execute(
    source: &str,
    scss: bool,
    grant: bool,
    target_type: &str,
    target_schema: &str,
    color: Option<&str>,
) -> TransformGraphResponse {
    let (file, content_type, schema) = if scss {
        (
            "main.scss",
            "text/vnd.cem.scss",
            "https://cem.dev/ns/data/scss/1",
        )
    } else {
        ("main.css", "text/css", "https://cem.dev/ns/data/css/1")
    };
    let resources = BTreeMap::from([
        (
            format!("{ROOT}styles/{file}"),
            VirtualResourceV1 {
                bytes: source.as_bytes().to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some(content_type.into()),
                    schema: Some(schema.into()),
                    ..Default::default()
                }),
            },
        ),
        (
            format!("{ROOT}styles/_tokens.scss"),
            VirtualResourceV1 {
                bytes: MODULE.as_bytes().to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some("text/vnd.cem.scss".into()),
                    schema: Some("https://cem.dev/ns/data/scss/1".into()),
                    ..Default::default()
                }),
            },
        ),
    ]);
    let graph = format!("{{run | {{import @id=source @src=\"../styles/{file}\" @content-type={content_type} @schema={schema} | {{export @id=css @out=\"../generated/site.css\" @content-type={target_type} @schema={target_schema}}}}}}}");
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: graph.into_bytes(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
            ..Default::default()
        },
        base_uri: Some(CONFIG.into()),
    })
    .unwrap();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut context = EngineContext::default();
    if grant {
        context.resolver_registry.register(
            "studio",
            ResolvePurpose::Input,
            ResolveDirection::Read,
            DeclaredResources(resources.clone()),
        );
    }
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let mut request =
        lower_transform_graph_request(&context, &parsed.graph, &provider, CONFIG, true).unwrap();
    request.exports[0].target_scope.cemt_color_profile = color.map(str::to_owned);
    RealCemMlEngine::new().transform_graph(request).unwrap()
}

fn css(source: &str, scss: bool, grant: bool) -> TransformGraphResponse {
    execute(
        source,
        scss,
        grant,
        "text/css",
        "https://cem.dev/ns/data/css/1",
        None,
    )
}

#[test]
fn graph_exports_native_css_bytes_and_original_scoped_source_ranges() {
    let response = css(CSS, false, false);
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    assert_eq!(response.artifacts.len(), 1);
    let artifact = &response.artifacts[0];
    let bytes = artifact
        .primary_bytes
        .as_ref()
        .expect("typed CSS writer bytes");
    assert_eq!(bytes.bytes, CSS.as_bytes());
    assert_eq!(bytes.content_type, "text/css");
    assert_eq!(
        bytes.schema.as_deref(),
        Some("https://cem.dev/ns/data/css/1")
    );
    assert_eq!(
        artifact.destination.as_deref(),
        Some("studio://styles/generated/site.css")
    );
    assert!(
        !artifact.output_spans.is_empty(),
        "CSS export retains writer source spans"
    );
    assert!(artifact.output_spans.iter().flat_map(|span| &span.origin.frames).any(|frame|
        matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == "studio://styles/styles/main.css")));
    assert!(artifact
        .output_spans
        .iter()
        .all(
            |span| (span.output_range.start + u64::from(span.output_range.len)) as usize
                <= bytes.bytes.len()
        ));
}

#[test]
fn graph_exports_native_scss_handoff_with_module_and_expansion_origins() {
    let response = css(SCSS, true, true);
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let artifact = &response.artifacts[0];
    let bytes = artifact
        .primary_bytes
        .as_ref()
        .expect("SCSS handoff CSS bytes");
    assert_eq!(
        bytes.bytes,
        b".card {\n  color: #036;\n}\n.card .title {\n  font-weight: 700;\n}\n"
    );
    assert_eq!(bytes.content_type, "text/css");
    for origin in [
        ScssOriginKind::Module,
        ScssOriginKind::Definition,
        ScssOriginKind::Expansion,
    ] {
        assert!(artifact.output_spans.iter().flat_map(|span| &span.origin.frames).any(|frame|
            matches!(&frame.transform, TransformKind::ScssOrigin { origin_kind, .. } if *origin_kind == origin)), "missing {origin:?}");
    }
    assert!(artifact.output_spans.iter().flat_map(|span| &span.origin.frames).any(|frame|
        matches!(&frame.transform, TransformKind::ScssOrigin { module_uri, .. } if module_uri == "studio://styles/styles/_tokens.scss")));
    assert!(artifact
        .output_spans
        .iter()
        .all(
            |span| (span.output_range.start + u64::from(span.output_range.len)) as usize
                <= bytes.bytes.len()
        ));
}

#[test]
fn graph_does_not_infer_module_read_authority() {
    let response = css(SCSS, true, false);
    assert!(response.artifacts.is_empty());
    assert!(response
        .diagnostics
        .iter()
        .any(|item| item.code == "cem.scss.resolver_denied" && item.severity.is_hard_violation()));
    assert!(response
        .diagnostics
        .iter()
        .any(|item| item.uri.as_deref() == Some("studio://styles/styles/main.scss")));
}

#[test]
fn graph_keeps_stylesheet_parse_and_resource_policy_refusals() {
    for (source, scss, code) in [
        (".card { color: red;\n", false, "cem.css.invalid_token"),
        (
            ".card { background: url(https://example.test/image); }\n",
            false,
            "cem.css.url_rejected",
        ),
        (".card\n  color: red\n", true, "cem.scss.parse_error"),
        (
            "@use \"../outside\";\n.card { color: red; }",
            true,
            "cem.scss.resolver_denied",
        ),
    ] {
        let response = css(source, scss, true);
        assert!(response.artifacts.is_empty(), "{code}");
        assert!(
            response
                .diagnostics
                .iter()
                .any(|item| item.code == code && item.severity.is_hard_violation()),
            "{code}: {:?}",
            response.diagnostics
        );
    }
}

#[test]
fn graph_refuses_css_target_identity_mismatch_and_raw_scss_relabeling() {
    for (content_type, schema) in [
        ("text/css", "https://cem.dev/ns/data/scss/1"),
        ("text/vnd.cem.scss", "https://cem.dev/ns/data/css/1"),
        ("text/vnd.cem.scss", "https://cem.dev/ns/data/scss/1"),
    ] {
        let response = execute(CSS, false, false, content_type, schema, None);
        assert!(response.artifacts.is_empty(), "{content_type}, {schema}");
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item.severity.is_hard_violation()));
    }
}

#[test]
fn graph_browser_css_refuses_presentation_coloring() {
    for profile in ["terminal", "html", "md"] {
        let response = execute(
            CSS,
            false,
            false,
            "text/css",
            "https://cem.dev/ns/data/css/1",
            Some(profile),
        );
        assert!(response.artifacts.is_empty());
        assert!(
            response
                .diagnostics
                .iter()
                .any(|item| item.severity.is_hard_violation()
                    && item
                        .message
                        .contains("browser-facing CSS export cannot apply")),
            "{:?}",
            response.diagnostics
        );
    }
    assert!(execute(
        CSS,
        false,
        false,
        "text/css",
        "https://cem.dev/ns/data/css/1",
        Some("none")
    )
    .diagnostics
    .is_empty());
}
