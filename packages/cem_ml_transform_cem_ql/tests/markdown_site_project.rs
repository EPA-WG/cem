//! The ordinary portable site project uses the same native graph as Studio.
use cem_ml::{
    command_service::VirtualResourceV1,
    engine::{CemMlEngine, FormatIdentity, TransformGraphResponse},
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    },
    source_map::{FrameSpan, TransformKind},
    transform_config::{
        parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
    },
    transform_graph_request::{
        lower_transform_graph_request, resolve_transform_graph_reference,
        ManifestTransformGraphResourceProvider,
    },
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;
use std::{collections::BTreeMap, path::PathBuf};

const ROOT: &str = "studio://markdown-site/";
const CONFIG: &str = "studio://markdown-site/graphs/build.cem";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/projects/markdown-site")
}
fn resources() -> BTreeMap<String, VirtualResourceV1> {
    let root = project_root();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("example.json")).unwrap()).unwrap();
    manifest["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| {
            let path = source["path"].as_str().unwrap();
            (
                format!("{ROOT}{path}"),
                VirtualResourceV1 {
                    bytes: std::fs::read(root.join(path)).unwrap(),
                    identity: Some(FormatIdentity {
                        content_type: source["contentType"].as_str().map(Into::into),
                        schema: source["schema"].as_str().map(Into::into),
                        ..Default::default()
                    }),
                },
            )
        })
        .collect()
}
struct ProjectResolver(BTreeMap<String, VirtualResourceV1>);
impl ResourceResolver for ProjectResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let uri = resolve_transform_graph_reference(
            request.base_uri.as_deref().unwrap_or(CONFIG),
            &request.uri,
        );
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
            .ok_or(ResolverDiagnostic::UnsupportedResolver {
                uri,
                purpose: request.purpose,
                direction: ResolveDirection::Read,
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
fn run(resources: BTreeMap<String, VirtualResourceV1>) -> Result<TransformGraphResponse, String> {
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: resources[CONFIG].bytes.clone(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
            ..Default::default()
        },
        base_uri: Some(CONFIG.into()),
    })
    .unwrap();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut context = engine_context_with_cem_ql_template_adapter();
    context.resolver_registry.register(
        "studio",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        ProjectResolver(resources.clone()),
    );
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request = lower_transform_graph_request(&context, &parsed.graph, &provider, CONFIG, true)
        .map_err(|error| format!("{error:?}"))?;
    RealCemMlEngine::new()
        .transform_graph(request)
        .map_err(|error| format!("{error:?}"))
}
fn replace(
    resources: &mut BTreeMap<String, VirtualResourceV1>,
    path: &str,
    before: &str,
    after: &str,
) {
    let resource = resources.get_mut(&format!("{ROOT}{path}")).unwrap();
    let text = String::from_utf8(resource.bytes.clone()).unwrap();
    assert!(text.contains(before));
    resource.bytes = text.replace(before, after).into_bytes();
}

#[test]
fn ordinary_site_matches_expected_outputs_and_preserves_content_and_partial_origins() {
    let sources = resources();
    let result = run(sources.clone()).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.artifacts.len(), 7);
    for artifact in &result.artifacts {
        let path = artifact
            .destination
            .as_ref()
            .unwrap()
            .strip_prefix(&format!("{ROOT}generated/"))
            .unwrap();
        if path.ends_with(".html") {
            let html = artifact.primary.as_str().unwrap();
            assert_eq!(
                html.as_bytes(),
                std::fs::read(project_root().join("expected").join(path)).unwrap(),
                "{path}"
            );
            let original = format!("{ROOT}content/{}", path.replace(".html", ".md"));
            let origins: Vec<_> = artifact
                .output_spans
                .iter()
                .flat_map(|span| &span.origin.frames)
                .filter_map(|frame| match &frame.transform {
                    TransformKind::SourceBoundary { uri } => Some(uri.as_str()),
                    _ => None,
                })
                .collect();
            assert!(origins.contains(&original.as_str()), "{path}: {origins:?}");
            assert!(origins.contains(&"studio://markdown-site/layouts/partials/chrome.cemt"));
            assert!(origins.contains(&"studio://markdown-site/layouts/partials/content.cemt"));
            assert!(!origins
                .iter()
                .any(|uri| uri.ends_with(".md") && *uri != original));
            assert!(!html.contains("title:"));
        } else {
            let input = if path.starts_with("assets/") {
                path.to_string()
            } else {
                format!("static/{path}")
            };
            assert_eq!(
                artifact.primary_bytes.as_ref().unwrap().bytes,
                sources[&format!("{ROOT}{input}")].bytes
            );
        }
    }
    let again = run(sources).unwrap();
    assert_eq!(publications(&result), publications(&again));
}

#[test]
fn ordinary_site_keeps_metadata_title_and_body_heading_origins_independent() {
    let mut sources = resources();
    let path = "content/articles/first-build.md";
    let source_uri = format!("{ROOT}{path}");
    replace(
        &mut sources,
        path,
        "title: Build a small site",
        "title: Café publishing title",
    );
    let source = String::from_utf8(sources[&source_uri].bytes.clone()).unwrap();
    let result = run(sources).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.artifacts.len(), 7);
    let artifact = result
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.destination.as_deref()
                == Some("studio://markdown-site/generated/articles/first-build.html")
        })
        .unwrap();
    let html = artifact.primary.as_str().unwrap();
    assert!(html.contains("<title>Café publishing title</title>"));
    assert!(html.contains("<h1 id=\"build-a-small-site\" tabindex=\"-1\">Build a small site</h1>"));

    for (text, source_offset) in [
        (
            "Café publishing title",
            source.find("Café publishing title").unwrap(),
        ),
        (
            "Build a small site",
            source.find("# Build a small site").unwrap() + 2,
        ),
    ] {
        let output_offset = html.find(text).unwrap() as u64;
        let source_offset = source_offset as u64;
        let origins: Vec<_> = artifact
            .output_spans
            .iter()
            .filter(|span| {
                span.output_range.start <= output_offset
                    && output_offset < span.output_range.start + span.output_range.len as u64
            })
            .map(|span| &span.origin)
            .collect();
        assert!(origins.iter().any(|origin| {
            origin.frames.iter().any(|frame| {
                matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == &source_uri)
                && matches!(&frame.span, FrameSpan::Single(range)
                    if range.start <= source_offset
                        && source_offset < range.start + range.len as u64)
            })
        }), "{text} must point to its own original Markdown bytes: {origins:?}");
    }
    for partial in ["chrome.cemt", "content.cemt"] {
        let uri = format!("{ROOT}layouts/partials/{partial}");
        assert!(artifact.output_spans.iter().any(|span| {
            span.origin.frames.iter().any(|frame| {
                matches!(&frame.transform, TransformKind::SourceBoundary { uri: source } if source == &uri)
            })
        }), "the shared partial keeps its own origin: {partial}");
    }
    assert!(!artifact.output_spans.iter().any(|span| {
        span.origin.frames.iter().any(|frame| {
            matches!(&frame.transform, TransformKind::SourceBoundary { uri }
                if uri.ends_with(".md") && uri != &source_uri)
        })
    }));
}

fn publications(
    result: &TransformGraphResponse,
) -> Vec<(&Option<String>, &serde_json::Value, Option<&[u8]>)> {
    result
        .artifacts
        .iter()
        .map(|artifact| {
            (
                &artifact.destination,
                &artifact.primary,
                artifact
                    .primary_bytes
                    .as_ref()
                    .map(|value| value.bytes.as_slice()),
            )
        })
        .collect()
}

#[test]
fn ordinary_site_edits_failure_repair_and_missing_dependencies_use_native_semantics() {
    let mut sources = resources();
    replace(
        &mut sources,
        "content/articles/first-build.md",
        "Build a small site",
        "Build an edited site",
    );
    replace(
        &mut sources,
        "layouts/partials/chrome.cemt",
        "Built from saved project resources.",
        "Shared layout edited.",
    );
    let edited = run(sources.clone()).unwrap();
    assert!(edited.diagnostics.is_empty(), "{:?}", edited.diagnostics);
    assert_eq!(
        edited
            .artifacts
            .iter()
            .filter(|artifact| artifact
                .primary
                .as_str()
                .is_some_and(|html| html.contains("Shared layout edited.")))
            .count(),
        4
    );
    assert!(edited.artifacts.iter().any(|artifact| artifact
        .primary
        .as_str()
        .is_some_and(|html| html.contains("Build an edited site"))));
    let valid = sources.clone();
    replace(
        &mut sources,
        "content/articles/first-build.md",
        "title: Build an edited site",
        "title: [broken",
    );
    let failed = run(sources).unwrap();
    assert!(failed.artifacts.is_empty());
    assert!(failed.diagnostics.iter().any(|diagnostic| diagnostic.code
        == "cem.markdown.front_matter_invalid"
        && diagnostic.uri.as_deref()
            == Some("studio://markdown-site/content/articles/first-build.md")));
    assert_eq!(
        publications(&run(valid.clone()).unwrap()),
        publications(&edited)
    );
    let mut missing = valid;
    missing.remove("studio://markdown-site/layouts/partials/chrome.cemt");
    match run(missing) {
        Err(error) => assert!(error.contains("chrome.cemt"), "{error}"),
        Ok(result) => {
            assert!(result.artifacts.is_empty());
            assert!(!result.diagnostics.is_empty());
        }
    }
}
