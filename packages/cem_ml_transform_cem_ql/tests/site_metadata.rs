//! Native rooted site composition: common metadata, shared layouts and origins.
use cem_ml::{
    command_service::VirtualResourceV1,
    engine::{CemMlEngine, FormatIdentity},
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
use std::collections::BTreeMap;

const CONFIG: &str = "studio://site/graphs/build.cem";
const PARTIAL_URI: &str = "studio://site/layouts/partials/content.cemt";
const PARTIAL: &[u8] = include_bytes!("fixtures/site-metadata/layouts/partials/content.cemt");
const LAYOUT: &[u8] = include_bytes!("fixtures/site-metadata/layouts/page.cemt");
const GRAPH: &str = include_str!("fixtures/site-metadata/graphs/build.cem");
struct PartialResolver;
impl ResourceResolver for PartialResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let uri = resolve_transform_graph_reference(
            request.base_uri.as_deref().unwrap_or(CONFIG),
            &request.uri,
        );
        if uri == PARTIAL_URI {
            return Ok(ResolvedRead {
                uri,
                bytes: PARTIAL.to_vec(),
                content_type: Some("application/vnd.cem.transform+cem".into()),
            });
        }
        Err(ResolverDiagnostic::UnsupportedResolver {
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
fn run(first: &str) -> cem_ml::engine::TransformGraphResponse {
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: GRAPH.as_bytes().to_vec(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
            ..Default::default()
        },
        base_uri: Some(CONFIG.into()),
    })
    .unwrap();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let resource = |bytes: &[u8]| VirtualResourceV1 {
        bytes: bytes.to_vec(),
        identity: None,
    };
    let resources = BTreeMap::from([
        (
            "studio://site/content/index.md".into(),
            resource(first.as_bytes()),
        ),
        (
            "studio://site/content/posts/next.md".into(),
            resource(b"---\ntitle: Page B\n---\n# Body B\n"),
        ),
        (
            "studio://site/data/site.json".into(),
            resource(include_bytes!("fixtures/site-metadata/data/site.json")),
        ),
        ("studio://site/layouts/page.cemt".into(), resource(LAYOUT)),
        (PARTIAL_URI.into(), resource(PARTIAL)),
    ]);
    let mut context = engine_context_with_cem_ql_template_adapter();
    context.resolver_registry.register(
        "studio",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        PartialResolver,
    );
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request =
        lower_transform_graph_request(&context, &parsed.graph, &provider, CONFIG, true).unwrap();
    RealCemMlEngine::new().transform_graph(request).unwrap()
}
const FIRST: &str = "---\r\ntitle: Page A\r\ndescription: Café\r\ndraft: false\r\nweight: 2\r\ntags: [native, site]\r\n---\r\n# Body A\r\n\r\nA **native** article.\r\n";

#[test]
fn rooted_site_layout_retains_distinct_content_metadata_and_partial_sources() {
    let result = run(FIRST);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.artifacts.len(), 2);
    for (artifact, (path, title, body, source)) in result.artifacts.iter().zip([
        ("index.html", "Page A", "Body A", FIRST),
        (
            "posts/next.html",
            "Page B",
            "Body B",
            "---\ntitle: Page B\n---\n# Body B\n",
        ),
    ]) {
        let html = artifact.primary.as_str().expect("HTML export");
        assert!(html.contains(&format!("<title>{title}</title>")), "{html}");
        assert!(html.contains("Site navigation<a href=\"/generated/index.html\">Home</a><a href=\"/generated/posts/next.html\">Next</a></nav>"), "{html}");
        assert!(html.contains("<footer>Shared footer</footer>"), "{html}");
        assert!(!html.contains("title:"));
        assert_eq!(
            artifact.destination.as_deref(),
            Some(format!("studio://site/generated/{path}").as_str())
        );
        let source_uri = format!("studio://site/content/{}", path.replace(".html", ".md"));
        for text in [title, body] {
            let start = html.find(text).unwrap() as u64;
            let origins: Vec<_> = artifact
                .output_spans
                .iter()
                .filter(|span| {
                    span.output_range.start <= start
                        && start < span.output_range.start + span.output_range.len as u64
                })
                .collect();
            assert!(origins.iter().any(|span| span.origin.frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == &source_uri))), "{text}: {origins:?}");
            assert!(origins.iter().any(|span| span.origin.frames.iter().any(|frame| matches!(&frame.span, FrameSpan::Single(range) if range.start <= source.find(text).unwrap() as u64 && (source.find(text).unwrap() as u64) < range.start + range.len as u64))), "original range for {text}: {origins:?}");
        }
        assert!(artifact.output_spans.iter().any(|span| span.origin.frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == PARTIAL_URI))), "partial source missing: {:?}", artifact.output_spans);
        assert!(!artifact.output_spans.iter().any(|span| span.origin.frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri.ends_with(".md") && uri != &source_uri))));
    }
}

#[test]
fn invalid_metadata_fails_before_any_site_artifact_is_admitted() {
    let result = run("---\ntitle: [broken\n---\n# Body\n");
    assert!(result.artifacts.is_empty());
    assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code
        == "cem.markdown.front_matter_invalid"
        && diagnostic.uri.as_deref() == Some("studio://site/content/index.md")));
}
