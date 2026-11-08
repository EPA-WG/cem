use cem_ml::{
    content_cache::ContentHash,
    dependency_identity::{
        DependencyClosureState, DependencyLayer, DependencySnapshot, DependencyVersion,
    },
    operation_control::OperationControl,
    scheduler::AbortSignal,
    ssr_artifact::{CemSsrPageArtifact, SsrExecutionIdentity, SsrSettlePolicy, SsrSourceMapMode},
    ssr_route::{preflight_ssr_page_routes, SsrPageRouteRequest},
    transform_artifact::TransformArtifactBody,
    validation::html::{
        html_document_ast_from_source_bytes, HtmlDocumentAst, HtmlSourceValidationRequest,
    },
};
use std::{collections::BTreeMap, sync::Arc};
pub fn owner(body: &str) -> Arc<HtmlDocumentAst> {
    let html = format!(
        "<!doctype html><html><head><title>Inline page</title></head><body>{body}</body></html>"
    );
    Arc::new(
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: html.as_bytes(),
            source_uri: "file:///source/page.html",
            content_type: Some("text/html"),
        })
        .0
        .unwrap(),
    )
}
pub fn page_with_policy(
    source: &Arc<HtmlDocumentAst>,
    output: &str,
    policy: SsrSettlePolicy,
) -> CemSsrPageArtifact {
    page_with_document(
        source,
        output,
        policy,
        TransformArtifactBody::HtmlDomProjection(Arc::clone(source)),
    )
}
#[allow(dead_code)] // Explicit alternate-source fixture profile, used by page coordination.
pub fn page_with_document(
    source: &Arc<HtmlDocumentAst>,
    output: &str,
    policy: SsrSettlePolicy,
    document: TransformArtifactBody,
) -> CemSsrPageArtifact {
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        source,
        "https://ssr.example.test/guide/",
        Some(output),
    )])
    .unwrap();
    let identity = SsrExecutionIdentity {
        dependencies: DependencySnapshot {
            layer: DependencyLayer::RenderProjection,
            engine_version: cem_ml::VERSION.into(),
            format_version: "ssr/1".into(),
            closure_state: DependencyClosureState::Pending,
            root_uri: source.source.uri.clone(),
            sources: BTreeMap::new(),
            imports: vec![],
            options: BTreeMap::new(),
            policy: DependencyVersion::Opaque,
            resolution: DependencyVersion::Opaque,
            scope: DependencyVersion::Opaque,
            inputs: BTreeMap::new(),
        },
        lifecycle_version: "0.2.0".into(),
        island_schema_version: "0.2.0".into(),
        render_plan_version: "1.7.0".into(),
        cem_ql_version: cem_ql::VERSION.into(),
        entrypoint: "ssr:render-page".into(),
        parameters_hash: ContentHash::from_blake3(b"native params"),
        source_map_mode: SsrSourceMapMode::Retained,
        formatter_profile: "dom-safe/1".into(),
    };
    CemSsrPageArtifact::unrendered(Arc::clone(source), &routes[0], document, identity, policy)
        .unwrap()
}
pub fn page(source: &Arc<HtmlDocumentAst>, output: &str) -> CemSsrPageArtifact {
    page_with_policy(
        source,
        output,
        SsrSettlePolicy {
            max_declaration_depth: 8,
            max_instances: 4,
            max_render_passes: 4,
            max_resources: 4,
            max_total_bytes: 1024 * 1024,
            max_wall_time_ms: 1000,
            max_cpu_time_ms: 1000,
            allow_static_fallback: false,
        },
    )
}
pub fn control() -> OperationControl {
    OperationControl::new(AbortSignal::new())
}
