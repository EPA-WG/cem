#![cfg(any(target_os = "linux", windows))]
use cem_ml::{
    content_cache::ContentHash,
    dependency_identity::{
        DependencyClosureState, DependencyVersion, SourceDependency, RENDER_DEPENDENCY_INPUTS,
    },
    ssr_artifact::CemSsrPageArtifact,
    ssr_route::{preflight_ssr_page_routes, SsrPageRouteRequest},
    transform_artifact::TransformArtifactBody,
};
use cem_ql::ssr_page::{
    prepare_inline_page, SsrPageProgress, SsrReadinessContext, SSR_READY_RENDER_PLAN,
};
use std::{collections::BTreeMap, sync::Arc};
#[path = "support/ssr_page.rs"]
mod fixture;
fn ready_page(source: &Arc<cem_ml::validation::html::HtmlDocumentAst>) -> CemSsrPageArtifact {
    let initial = fixture::page(source, "memory:output");
    let mut identity = initial.execution_identity().clone();
    identity.render_plan_version = SSR_READY_RENDER_PLAN.into();
    let d = &mut identity.dependencies;
    d.closure_state = DependencyClosureState::Ready;
    let stamp = || DependencyVersion::Content(ContentHash::from_blake3(b"explicit-test-context"));
    d.policy = stamp();
    d.resolution = stamp();
    d.scope = stamp();
    d.sources.insert(
        source.source.uri.clone(),
        SourceDependency {
            content_type: "text/html".into(),
            schema: Some(cem_ml::schema::registry::HTML_SCHEMA_URI.into()),
            version: DependencyVersion::Content(ContentHash::from_blake3(
                source.source_text.as_bytes(),
            )),
        },
    );
    d.inputs = RENDER_DEPENDENCY_INPUTS
        .iter()
        .map(|i| (*i, stamp()))
        .collect();
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        source,
        "https://ssr.example.test/guide/",
        Some("memory:output"),
    )])
    .unwrap();
    let mut policy = initial.policy().clone();
    policy.max_wall_time_ms = 10000;
    policy.max_cpu_time_ms = 1000;
    CemSsrPageArtifact::unrendered(
        source.clone(),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(source.clone()),
        identity,
        policy,
    )
    .unwrap()
}
fn source(version: &str) -> Arc<cem_ml::validation::html::HtmlDocumentAst> {
    fixture::owner(&format!("<cem-element tag='x-parent' version='{version}'><template id='x-parent' type='text/cem-ml'>{{section | {{x-child}}}}</template></cem-element><cem-element tag='x-child' version='0.2.3'><template id='x-child' type='text/cem-ml'>{{b | Ready}}</template></cem-element><x-parent></x-parent>"))
}
fn versions(parent: &str, child: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("x-parent".into(), parent.into()),
        ("x-child".into(), child.into()),
    ])
}
fn settled(
    page: CemSsrPageArtifact,
    c: &cem_ml::operation_control::OperationControl,
) -> cem_ql::ssr_page::QuiescentInlinePage {
    let mut work = prepare_inline_page(page, "ready", c).unwrap();
    while !matches!(work.advance(c).unwrap(), SsrPageProgress::Quiescent) {}
    work.finish(c).unwrap()
}
#[test]
fn complete_nested_native_page_has_fresh_opaque_readiness_without_enabling_export_adoption() {
    let c = fixture::control();
    let source = source("1.2.3");
    let page = ready_page(&source);
    let context = SsrReadinessContext::new(&page, versions("1.9.0", "0.2.9")).unwrap();
    let ready = settled(page, &c).prove_ready(&context, &c).unwrap();
    assert_eq!(ready.participants().len(), 2);
    let observation = ready.validate_current(ready.tree(), &context, &c).unwrap();
    assert!(observation.usage().cpu_time_ms <= ready.page().policy().max_cpu_time_ms);
    assert!(!ready.page().adoption_identity().hydration_ready());
    assert!(ready
        .validate_current(ready.tree(), &context, &fixture::control())
        .is_err());
    c.abort_signal().abort();
    assert!(ready.validate_current(ready.tree(), &context, &c).is_err());
}
#[test]
fn incompatible_or_missing_versions_never_prove_readiness() {
    for target in ["2.0.0", "1.2.3-beta.1", ""] {
        let c = fixture::control();
        let page = ready_page(&source("1.2.3"));
        let context = SsrReadinessContext::new(&page, versions(target, "0.2.9")).unwrap();
        assert!(settled(page, &c).prove_ready(&context, &c).is_err());
    }
    let c = fixture::control();
    let source=fixture::owner("<cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{b | Missing}</template></cem-element><x-a></x-a>");
    let page = ready_page(&source);
    let context =
        SsrReadinessContext::new(&page, BTreeMap::from([("x-a".into(), "1.0.0".into())])).unwrap();
    assert!(settled(page, &c).prove_ready(&context, &c).is_err());
}
#[test]
fn opaque_dependencies_and_changed_execution_context_are_not_readiness_authority() {
    let source = source("1.2.3");
    assert!(SsrReadinessContext::new(
        &fixture::page(&source, "memory:output"),
        versions("1.2.3", "0.2.3")
    )
    .is_err());
    let c = fixture::control();
    let page = ready_page(&source);
    let context = SsrReadinessContext::new(&page, versions("1.2.3", "0.2.3")).unwrap();
    let ready = settled(page, &c).prove_ready(&context, &c).unwrap();
    let mut changed = ready_page(&source);
    let mut p = changed.policy().clone();
    p.max_instances = 3;
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &source,
        "https://ssr.example.test/guide/",
        Some("memory:output"),
    )])
    .unwrap();
    changed = CemSsrPageArtifact::unrendered(
        source.clone(),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(source.clone()),
        changed.execution_identity().clone(),
        p,
    )
    .unwrap();
    let other = SsrReadinessContext::new(&changed, versions("1.2.3", "0.2.3")).unwrap();
    assert!(ready.validate_current(ready.tree(), &other, &c).is_err());
}
fn capability(
    resolver: Option<Arc<std::sync::atomic::AtomicUsize>>,
) -> cem_ml::module_resolution::CemModuleUrlResolutionCapability {
    use cem_ml::module_resolution::*;
    let h = CemResolutionContextHandle::new("ready-policy");
    let inner = CemScopedModuleUrlResolver::new().with_context(
        h.clone(),
        CemModuleUrlContext {
            identity: "page".into(),
            resolver_identity: "explicit".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![CemModuleUrlFrame::new(
                "page",
                "https://ssr.example.test/guide/",
            )],
        },
    );
    match resolver {
        Some(revision) => {
            CemModuleUrlResolutionCapability::new(Arc::new(MutableResolver { inner, revision }), h)
        }
        None => CemModuleUrlResolutionCapability::new(Arc::new(inner), h),
    }
}
struct MutableResolver {
    inner: cem_ml::module_resolution::CemScopedModuleUrlResolver,
    revision: Arc<std::sync::atomic::AtomicUsize>,
}
impl cem_ml::module_resolution::CemModuleUrlResolver for MutableResolver {
    fn context_cache_identity(
        &self,
        h: &cem_ml::module_resolution::CemResolutionContextHandle,
    ) -> Option<String> {
        self.inner.context_cache_identity(h).map(|s| {
            format!(
                "{}:{s}",
                self.revision.load(std::sync::atomic::Ordering::Relaxed)
            )
        })
    }
    fn resolve_module_url(
        &self,
        r: &cem_ml::module_resolution::CemModuleUrlResolutionRequest,
    ) -> Result<
        cem_ml::module_resolution::CemModuleUrlResolution,
        cem_ml::module_resolution::CemModuleUrlResolutionError,
    > {
        self.inner.resolve_module_url(r)
    }
}
#[test]
fn resource_wait_is_excluded_from_cpu_and_closed_css_payload_leases_prove_ready() {
    let c = fixture::control();
    let source=fixture::owner("<cem-element tag='x-a' version='1.2.3'><template id='x-a' type='text/cem-ml'>{style |```@import 'ready.css'; b {color:red}```}{b | Ready}</template></cem-element><x-a><template type='cem-ml'>{item | {$ 1 div 0}}</template></x-a>");
    let initial = ready_page(&source);
    let mut p = initial.policy().clone();
    p.max_cpu_time_ms = 100;
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &source,
        "https://ssr.example.test/guide/",
        Some("memory:output"),
    )])
    .unwrap();
    let page = CemSsrPageArtifact::unrendered(
        source.clone(),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(source.clone()),
        initial.execution_identity().clone(),
        p,
    )
    .unwrap();
    let context =
        SsrReadinessContext::new(&page, BTreeMap::from([("x-a".into(), "1.9.0".into())])).unwrap();
    let mut work =
        cem_ql::ssr_page::prepare_inline_page_with_resources(page, "ready", capability(None), &c)
            .unwrap();
    let SsrPageProgress::Pending(request) = work.advance(&c).unwrap() else {
        panic!()
    };
    std::thread::sleep(std::time::Duration::from_millis(150));
    work.deliver(
        &request,
        cem_ml::resolver::ResolvedRead {
            uri: request.request().resolution.resolved_url.clone(),
            bytes: b"b {font-weight:bold}".to_vec(),
            content_type: Some("text/css".into()),
        },
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let ready = work.finish(&c).unwrap().prove_ready(&context, &c).unwrap();
    let observation = ready.validate_current(ready.tree(), &context, &c).unwrap();
    assert!(observation.usage().wall_time_ms >= 150);
    assert!(observation.usage().cpu_time_ms < 100);
    assert_eq!(observation.usage().resources, 2);
}
#[test]
fn resolver_revision_invalidates_ready_proof_and_cannot_be_restored() {
    let c = fixture::control();
    let source=fixture::owner("<cem-element tag='x-a' version='1.0.0'><template id='x-a' type='text/cem-ml'>{style |```b {color:red}```}{b | Ready}</template></cem-element><x-a></x-a>");
    let page = ready_page(&source);
    let context =
        SsrReadinessContext::new(&page, BTreeMap::from([("x-a".into(), "1.2.0".into())])).unwrap();
    let revision = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut work = cem_ql::ssr_page::prepare_inline_page_with_resources(
        page,
        "ready",
        capability(Some(revision.clone())),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let ready = work.finish(&c).unwrap().prove_ready(&context, &c).unwrap();
    revision.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(ready.validate_current(ready.tree(), &context, &c).is_err());
    revision.store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(ready.validate_current(ready.tree(), &context, &c).is_err());
}
#[test]
fn unsupported_protocol_malformed_declaration_and_replaced_native_trees_are_refused() {
    let c = fixture::control();
    let source = source("1.2.3");
    let original = ready_page(&source);
    let mut identity = original.execution_identity().clone();
    identity.render_plan_version = "cem-native-render-plan/99".into();
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &source,
        "https://ssr.example.test/guide/",
        Some("memory:output"),
    )])
    .unwrap();
    let page = CemSsrPageArtifact::unrendered(
        source.clone(),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(source.clone()),
        identity,
        original.policy().clone(),
    )
    .unwrap();
    let context = SsrReadinessContext::new(&page, versions("1.2.3", "0.2.3")).unwrap();
    assert!(settled(page, &c).prove_ready(&context, &c).is_err());
    let c = fixture::control();
    let context = SsrReadinessContext::new(&original, versions("1.2.3", "0.2.3")).unwrap();
    let ready = settled(original, &c).prove_ready(&context, &c).unwrap();
    let copied = cem_ml::parser::tree_apply::apply_attributes(
        ready.tree().clone(),
        ready.participants()[0].host_node(),
        Arc::new(vec![]
            as Vec<
                cem_ml::value::CemValueAttribute<cem_ql::eval::ItemStream>,
            >),
        Arc::new(()),
        Default::default(),
        &c,
    )
    .unwrap();
    assert!(ready.validate_current(copied.tree(), &context, &c).is_err());
    for version in ["01.2.3", "1.0", "1.2.3-01", " 1.2.3 "] {
        let source = crate::source(version);
        assert!(prepare_inline_page(ready_page(&source), "ready", &fixture::control()).is_err());
    }
}
#[test]
fn duplicate_live_ids_never_authorize_a_ready_page() {
    let c = fixture::control();
    let source=fixture::owner("<cem-element tag='x-a' version='1.0.0'><template id='x-a' type='text/cem-ml'>{b @id=duplicate | Ready}</template></cem-element><x-a></x-a><x-a></x-a>");
    let page = ready_page(&source);
    let context =
        SsrReadinessContext::new(&page, BTreeMap::from([("x-a".into(), "1.0.0".into())])).unwrap();
    let errors = settled(page, &c).prove_ready(&context, &c).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.code == "cem.ssr.readiness_duplicate_id"));
}
