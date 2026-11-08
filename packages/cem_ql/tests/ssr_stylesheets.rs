use cem_ml::{
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    resolver::ResolvedRead,
};
use cem_ql::{
    ssr_island::prepare_inline_island,
    ssr_preparation::{prepare_inline_dce, prepare_inline_dce_with_stylesheets},
    ssr_render::render_and_apply_inline_dce,
    ssr_stylesheets::{begin_inline_stylesheets, SsrCssProgress, SsrCssState},
};
use std::sync::Arc;
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn capability() -> CemModuleUrlResolutionCapability {
    let h = CemResolutionContextHandle::new("ssr-css");
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            h.clone(),
            CemModuleUrlContext {
                identity: "page".into(),
                resolver_identity: "explicit-test-resolver".into(),
                resource_policy_stamp: "allowed-test-css".into(),
                frames: vec![CemModuleUrlFrame::new(
                    "page",
                    "https://ssr.example.test/guide/",
                )],
            },
        )),
        h,
    )
}
fn source(declaration: &str, instance: &str) -> Arc<cem_ml::validation::html::HtmlDocumentAst> {
    owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{declaration}{{p | Ready}}</template></cem-element><x-inline><template>{instance}<b>Original</b></template></x-inline>"))
}
fn response(css: &str, url: &str) -> ResolvedRead {
    ResolvedRead {
        uri: url.into(),
        bytes: css.as_bytes().to_vec(),
        content_type: Some("text/css".into()),
    }
}
#[test]
fn declaration_and_instance_css_wait_for_explicit_authority_then_publish_atomically() {
    let c = control();
    let source = source(
        "{style |```@import 'theme.css'; p {color:red}```}",
        "<style>b {color:blue}</style>",
    );
    let before = source.source_text.clone();
    assert!(prepare_inline_dce(page(&source, "memory:output"), &c).is_err());
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let mut coordinator = begin_inline_stylesheets(island, capability(), &c).unwrap();
    assert_eq!(coordinator.state(), SsrCssState::Pending);
    let SsrCssProgress::Pending(request) = coordinator.advance(&c).unwrap() else {
        panic!("expected import")
    };
    assert_eq!(
        request.request().resolution.resolved_url,
        "https://ssr.example.test/guide/theme.css"
    );
    coordinator
        .deliver(
            &request,
            response(
                "p {background:url(../images/paint.png)}",
                "https://ssr.example.test/redirect/theme.css",
            ),
            &c,
        )
        .unwrap();
    assert!(matches!(
        coordinator.advance(&c).unwrap(),
        SsrCssProgress::Ready
    ));
    let ready = coordinator.finish(&c).unwrap();
    let result = render_and_apply_inline_dce(ready, &c).unwrap();
    assert_eq!(source.source_text, before);
    let tree = result.application().tree();
    let declaration = result
        .application()
        .output_for_source(result.input().prepared().declaration_node())
        .unwrap();
    let node = tree.node(declaration).unwrap();
    let style = tree.node(*node.children.last().unwrap()).unwrap();
    assert_eq!(style.name.as_ref().unwrap().local_name, "style");
    let css = tree
        .text_fragments(*node.children.last().unwrap())
        .unwrap()
        .collect::<String>();
    assert!(
        css.contains("https://ssr.example.test/images/paint.png"),
        "{css}"
    );
    assert!(css.contains("data-cem-css-context"));
    let host = tree.node(result.application().output_target()).unwrap();
    assert!(host.attributes.iter().any(|id| tree
        .node(*id)
        .unwrap()
        .name
        .as_ref()
        .unwrap()
        .local_name
        == "data-cem-css-context"));
    assert_eq!(
        tree.node(*host.children.last().unwrap())
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "style"
    );
    assert!(!result.page().adoption_identity().hydration_ready());
}
#[test]
fn unresolved_stylesheets_cannot_render_or_forge_an_authority() {
    let c = control();
    let source = source("{style |```p {color:red}```}", "");
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    assert_eq!(
        render_and_apply_inline_dce(island, &c).unwrap_err()[0].code,
        "cem.ssr.css_unprepared"
    );
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let missing = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new()),
        CemResolutionContextHandle::new("missing"),
    );
    assert!(begin_inline_stylesheets(island, missing, &c).is_err());
}
fn pending_with(
    c: &cem_ml::operation_control::OperationControl,
    source: &Arc<cem_ml::validation::html::HtmlDocumentAst>,
) -> cem_ql::ssr_stylesheets::SsrStylesheetCoordinator {
    let p = prepare_inline_dce_with_stylesheets(page(source, "memory:output"), c).unwrap();
    begin_inline_stylesheets(
        prepare_inline_island(p, "inline-one", c).unwrap(),
        capability(),
        c,
    )
    .unwrap()
}
fn request(
    coordinator: &mut cem_ql::ssr_stylesheets::SsrStylesheetCoordinator,
    c: &cem_ml::operation_control::OperationControl,
) -> cem_ql::ssr_stylesheets::SsrCssRequest {
    let SsrCssProgress::Pending(request) = coordinator.advance(c).unwrap() else {
        panic!("expected pending CSS")
    };
    request
}
#[test]
fn requests_are_idempotent_and_cross_coordinator_or_duplicate_delivery_cannot_replace_current_work()
{
    let c = control();
    let source = source("", "<style>@import 'child.css';</style>");
    let mut first = pending_with(&c, &source);
    let mut second = pending_with(&c, &source);
    let a = request(&mut first, &c);
    let b = request(&mut second, &c);
    assert_eq!(request(&mut first, &c).request().id, a.request().id);
    assert!(first
        .deliver(
            &b,
            response("b {color:red}", "https://ssr.example.test/guide/child.css"),
            &c
        )
        .is_err());
    assert_eq!(first.state(), SsrCssState::Pending);
    first
        .deliver(
            &a,
            response("b {color:red}", "https://ssr.example.test/guide/child.css"),
            &c,
        )
        .unwrap();
    assert!(first
        .deliver(
            &a,
            response("b {color:blue}", "https://ssr.example.test/guide/child.css"),
            &c
        )
        .is_err());
    let ready = first.finish(&c).unwrap();
    assert!(render_and_apply_inline_dce(ready, &c).is_ok());
}
#[test]
fn bad_mime_transport_failure_and_cancelled_work_are_terminal_without_publication() {
    use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;
    for mode in 0..3 {
        let c = control();
        let source = source("", "<style>@import 'child.css';</style>");
        let before = source.source_text.clone();
        let mut coordinator = pending_with(&c, &source);
        let r = request(&mut coordinator, &c);
        match mode {
            0 => {
                let mut bad = response("b {}", "https://ssr.example.test/guide/child.css");
                bad.content_type = Some("text/html".into());
                assert!(coordinator.deliver(&r, bad, &c).is_err());
            }
            1 => {
                assert!(coordinator
                    .fail(&r, "offline", &c)
                    .unwrap_err()
                    .contains("cem.css.import_load_failed"));
            }
            _ => {
                c.abort_signal().abort();
                assert!(coordinator.advance(&c).is_err());
            }
        }
        assert_eq!(
            coordinator.state(),
            if mode == 2 {
                SsrCssState::Cancelled
            } else {
                SsrCssState::Refused
            }
        );
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
        assert_eq!(source.source_text, before);
        assert!(coordinator.finish(&c).is_err());
    }
}
#[test]
fn aggregate_bytes_and_new_resource_references_cannot_exceed_page_ceilings() {
    use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;
    for bytes in [true, false] {
        let c = control();
        let source = source("", "<style>@import 'child.css';</style>");
        let mut policy = page(&source, "memory:output").policy().clone();
        if bytes {
            policy.max_total_bytes = source.source_text.len() as u64 + 8;
        }
        let p = prepare_inline_dce_with_stylesheets(
            fixture::page_with_policy(&source, "memory:output", policy),
            &c,
        )
        .unwrap();
        let mut coordinator = begin_inline_stylesheets(
            prepare_inline_island(p, "inline-one", &c).unwrap(),
            capability(),
            &c,
        )
        .unwrap();
        let r = request(&mut coordinator, &c);
        let css = if bytes {
            "b {color:red}"
        } else {
            "a {background:url(a)} b {background:url(b)} c {background:url(c)}"
        };
        assert!(coordinator
            .deliver(
                &r,
                response(css, "https://ssr.example.test/guide/child.css"),
                &c
            )
            .is_err());
        assert_eq!(coordinator.state(), SsrCssState::Refused);
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
}
#[test]
fn stylesheet_preparation_and_ready_batch_cannot_move_to_a_new_operation() {
    let c = control();
    let source = source("{style |```p {color:red}```}", "");
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    assert!(prepare_inline_island(p, "inline-one", &control()).is_err());
    let mut coordinator = pending_with(&c, &source);
    assert!(coordinator.advance(&control()).is_err());
    let ready = coordinator.finish(&c).unwrap();
    assert_eq!(
        render_and_apply_inline_dce(ready, &control()).unwrap_err()[0].code,
        "cem.ssr.css_closure_stale"
    );
}
#[test]
fn native_css_context_cannot_authorize_an_authored_reserved_attribute_write() {
    let c = control();
    let source = source(
        "{style |```@import 'child.css';```}{attribute @name=data-cem-css-context | forged}",
        "",
    );
    let mut coordinator = pending_with(&c, &source);
    let r = request(&mut coordinator, &c);
    coordinator
        .deliver(
            &r,
            response("p {color:red}", "https://ssr.example.test/guide/child.css"),
            &c,
        )
        .unwrap();
    let ready = coordinator.finish(&c).unwrap();
    assert_eq!(
        render_and_apply_inline_dce(ready, &c).unwrap_err()[0].code,
        "cem.ssr.host_commit_refused"
    );
}
#[test]
fn shared_scope_requires_matching_declaration_membership_and_is_committed_to_host() {
    for scope in ["shared-test", "different"] {
        let c = control();
        let source=owner(&format!("<cem-element tag='x-inline' scope='{scope}'><template id='x-inline' type='text/cem-ml'>{{style @scope=shared-test |```p {{color:red}}```}}{{p | Scoped}}</template></cem-element><x-inline></x-inline>"));
        let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
        let result = begin_inline_stylesheets(
            prepare_inline_island(p, "inline-one", &c).unwrap(),
            capability(),
            &c,
        );
        if scope == "different" {
            assert!(result.is_err());
            continue;
        }
        let ready = result.unwrap().finish(&c).unwrap();
        let result = render_and_apply_inline_dce(ready, &c).unwrap();
        let tree = result.application().tree();
        let host = tree.node(result.application().output_target()).unwrap();
        assert!(host.attributes.iter().any(|id| {
            let a = tree.node(*id).unwrap();
            a.name.as_ref().unwrap().local_name == "scope" && a.value == scope
        }));
    }
}
#[test]
fn applied_owner_retains_import_memory_and_css_liveness_until_last_tree_release() {
    use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;
    let c = control();
    let source = source("", "<style>@import 'child.css';</style>");
    let mut coordinator = pending_with(&c, &source);
    let r = request(&mut coordinator, &c);
    coordinator
        .deliver(
            &r,
            response("b {color:red}", "https://ssr.example.test/guide/child.css"),
            &c,
        )
        .unwrap();
    let ready = coordinator.finish(&c).unwrap();
    let record = ready.instance_stylesheets()[0].clone();
    assert_eq!(
        record.source(1).unwrap().source_uri,
        "https://ssr.example.test/guide/child.css"
    );
    let result = render_and_apply_inline_dce(ready, &c).unwrap();
    assert_eq!(result.page().usage().resources, 2);
    assert!(result.page().usage().total_bytes > source.source_text.len() as u64);
    let retained = result.application().tree().clone();
    drop(result);
    assert!(record.emission().is_ok());
    drop(retained);
    assert!(record.emission().is_err());
    drop(record);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn elapsed_page_budget_uses_original_operation_including_pending_transport_time() {
    let c = control();
    let source = source("", "<style>@import 'child.css';</style>");
    let mut policy = page(&source, "memory:output").policy().clone();
    policy.max_wall_time_ms = 200;
    let p = prepare_inline_dce_with_stylesheets(
        fixture::page_with_policy(&source, "memory:output", policy),
        &c,
    )
    .unwrap();
    let mut coordinator = begin_inline_stylesheets(
        prepare_inline_island(p, "inline-one", &c).unwrap(),
        capability(),
        &c,
    )
    .unwrap();
    let _ = request(&mut coordinator, &c);
    std::thread::sleep(std::time::Duration::from_millis(250));
    assert!(coordinator.advance(&c).is_err());
    assert_eq!(coordinator.state(), SsrCssState::Refused);
}
struct MutableResolver {
    inner: CemScopedModuleUrlResolver,
    revision: Arc<std::sync::atomic::AtomicUsize>,
}
impl cem_ml::module_resolution::CemModuleUrlResolver for MutableResolver {
    fn context_cache_identity(&self, context: &CemResolutionContextHandle) -> Option<String> {
        use cem_ml::module_resolution::CemModuleUrlResolver;
        self.inner.context_cache_identity(context).map(|value| {
            format!(
                "{}:{value}",
                self.revision.load(std::sync::atomic::Ordering::Relaxed)
            )
        })
    }
    fn resolve_module_url(
        &self,
        request: &cem_ml::module_resolution::CemModuleUrlResolutionRequest,
    ) -> Result<
        cem_ml::module_resolution::CemModuleUrlResolution,
        cem_ml::module_resolution::CemModuleUrlResolutionError,
    > {
        use cem_ml::module_resolution::CemModuleUrlResolver;
        self.inner.resolve_module_url(request)
    }
}
#[test]
fn resolver_revision_changes_permanently_refuse_observed_work_and_invalidate_ready_batches() {
    for ready in [false, true] {
        let c = control();
        let source = source("{style |```p {color:red}```}", "");
        let h = CemResolutionContextHandle::new("mutable");
        let revision = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let resolver = MutableResolver {
            inner: CemScopedModuleUrlResolver::new().with_context(
                h.clone(),
                CemModuleUrlContext {
                    identity: "page".into(),
                    resolver_identity: "resolver".into(),
                    resource_policy_stamp: "policy".into(),
                    frames: vec![CemModuleUrlFrame::new(
                        "page",
                        "https://ssr.example.test/guide/",
                    )],
                },
            ),
            revision: revision.clone(),
        };
        let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
        let mut coordinator = begin_inline_stylesheets(
            prepare_inline_island(p, "inline-one", &c).unwrap(),
            CemModuleUrlResolutionCapability::new(Arc::new(resolver), h),
            &c,
        )
        .unwrap();
        if ready {
            let input = coordinator.finish(&c).unwrap();
            revision.store(1, std::sync::atomic::Ordering::Relaxed);
            assert_eq!(
                render_and_apply_inline_dce(input, &c).unwrap_err()[0].code,
                "cem.ssr.css_closure_stale"
            );
        } else {
            revision.store(1, std::sync::atomic::Ordering::Relaxed);
            assert_eq!(coordinator.state(), SsrCssState::Refused);
            revision.store(0, std::sync::atomic::Ordering::Relaxed);
            assert!(coordinator.advance(&c).is_err());
        }
    }
}
#[test]
fn import_integrity_failure_retains_native_import_location() {
    use cem_ml::module_resolution::CemModuleUrlMapping;
    let c = control();
    let source = source("", "<style>@import 'child.css';</style>");
    let h = CemResolutionContextHandle::new("integrity");
    let mut frame = CemModuleUrlFrame::new("page", "https://ssr.example.test/guide/");
    frame.specifiers.resources.insert(
        "child.css".into(),
        CemModuleUrlMapping::target("./child.css")
            .with_integrity("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
    );
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            h.clone(),
            CemModuleUrlContext {
                identity: "page".into(),
                resolver_identity: "resolver".into(),
                resource_policy_stamp: "policy".into(),
                frames: vec![frame],
            },
        )),
        h,
    );
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    let mut coordinator =
        begin_inline_stylesheets(prepare_inline_island(p, "inline-one", &c).unwrap(), cap, &c)
            .unwrap();
    let r = request(&mut coordinator, &c);
    assert!(coordinator
        .deliver(
            &r,
            response("b {color:red}", "https://ssr.example.test/guide/child.css"),
            &c
        )
        .is_err());
    let failure = coordinator.failure().unwrap();
    assert_eq!(failure.code, "cem.css.import_integrity");
    let location = failure.location.as_ref().unwrap();
    assert_eq!(location.source_uri, "file:///source/page.html");
    assert!(location.range.offset > 0);
}
#[test]
fn aggregate_import_bytes_are_shared_across_declaration_and_instance_owners() {
    use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;
    let c = control();
    let source = source(
        "{style |```@import 'first.css';```}",
        "<style>@import 'second.css';</style>",
    );
    let mut policy = page(&source, "memory:output").policy().clone();
    policy.max_total_bytes = source.source_text.len() as u64 + 20;
    let p = prepare_inline_dce_with_stylesheets(
        fixture::page_with_policy(&source, "memory:output", policy),
        &c,
    )
    .unwrap();
    let mut coordinator = begin_inline_stylesheets(
        prepare_inline_island(p, "inline-one", &c).unwrap(),
        capability(),
        &c,
    )
    .unwrap();
    let first = request(&mut coordinator, &c);
    coordinator
        .deliver(
            &first,
            response("p {color:red}", "https://ssr.example.test/guide/first.css"),
            &c,
        )
        .unwrap();
    let second = request(&mut coordinator, &c);
    assert!(coordinator
        .deliver(
            &second,
            response("b {color:red}", "https://ssr.example.test/guide/second.css"),
            &c
        )
        .is_err());
    assert_eq!(coordinator.state(), SsrCssState::Refused);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn declaration_workspace_is_admitted_before_any_transport_request() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::{AbortSignal, ScopePolicy},
    };
    let c = OperationControl::with_root_policy(
        AbortSignal::new(),
        ScopePolicy::host_root().with_memory_bytes(65_536),
    )
    .unwrap();
    let css = format!(
        "{{style |```/*{}*/ @import 'child.css';```}}",
        "comment ".repeat(1000)
    );
    let source = source(&css, "");
    let p = prepare_inline_dce_with_stylesheets(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    assert!(begin_inline_stylesheets(island, capability(), &c).is_err());
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
