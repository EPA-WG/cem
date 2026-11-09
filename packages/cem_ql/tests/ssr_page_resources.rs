use cem_ml::{
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    operation_control::ROOT_EXECUTION_SCOPE_ID,
    resolver::ResolvedRead,
};
use cem_ql::ssr_page::{prepare_inline_page_with_resources, SsrPageProgress, SsrPageState};
use std::sync::Arc;
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn capability() -> CemModuleUrlResolutionCapability {
    let h = CemResolutionContextHandle::new("page-resources");
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            h.clone(),
            CemModuleUrlContext {
                identity: "page".into(),
                resolver_identity: "test-explicit".into(),
                resource_policy_stamp: "test-css".into(),
                frames: vec![CemModuleUrlFrame::new(
                    "page",
                    "https://ssr.example.test/guide/",
                )],
            },
        )),
        h,
    )
}
fn response(css: &str) -> ResolvedRead {
    ResolvedRead {
        uri: "https://ssr.example.test/guide/shared.css".into(),
        bytes: css.as_bytes().to_vec(),
        content_type: Some("text/css".into()),
    }
}
fn source(body: &str) -> Arc<cem_ml::validation::html::HtmlDocumentAst> {
    owner(&format!("<cem-element tag='x-item'><template id='x-item' type='text/cem-ml'>{{style |```@import 'shared.css'; b {{color:red}}```}}{{b | Ready}}</template></cem-element>{body}"))
}
fn policy(
    source: &Arc<cem_ml::validation::html::HtmlDocumentAst>,
) -> cem_ml::ssr_artifact::SsrSettlePolicy {
    let mut p = page(source, "memory:output").policy().clone();
    p.max_resources = 16;
    p.max_wall_time_ms = 10000;
    p.max_cpu_time_ms = 10000;
    p
}
fn attr(tree: &cem_ml::parser::tree::RetainedCemTree, id: u32, name: &str) -> Option<String> {
    tree.node(id)?.attributes.iter().find_map(|id| {
        let n = tree.node(*id)?;
        (n.name.as_ref()?.local_name == name).then(|| n.value.clone())
    })
}
#[test]
fn declaration_import_and_owner_are_shared_but_publish_only_once() {
    let c = control();
    let source = source("<x-item></x-item><x-item></x-item>");
    let before = source.source_text.clone();
    let mut p = policy(&source);
    p.max_resources = 2;
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", p),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    let SsrPageProgress::Pending(request) = work.advance(&c).unwrap() else {
        panic!("missing import")
    };
    let SsrPageProgress::Pending(again) = work.advance(&c).unwrap() else {
        panic!("pending must be stable")
    };
    assert_eq!(
        request.request().resolution.resolved_url,
        again.request().resolution.resolved_url
    );
    work.deliver(&again, response("b {font-weight:bold}"), &c)
        .unwrap();
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Rendered { .. }
    ));
    assert!(work.deliver(&request, response("b {}"), &c).is_err());
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Rendered { .. }
    ));
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Quiescent
    ));
    let result = work.finish(&c).unwrap();
    assert_eq!(result.participants().len(), 2);
    assert_eq!(result.page().usage().resources, 2);
    assert_eq!(
        result.page().usage().total_bytes,
        source.source.byte_length as u64 + "b {font-weight:bold}".len() as u64
    );
    let a = result.participants()[0]
        .render_pass()
        .input()
        .declaration_stylesheets();
    let b = result.participants()[1]
        .render_pass()
        .input()
        .declaration_stylesheets();
    assert_eq!(a.len(), 1);
    assert!(Arc::ptr_eq(&a[0], &b[0]));
    let t = result.tree();
    assert_eq!(
        (0..t.ast().nodes.len() as u32)
            .filter(|id| attr(t, *id, "data-cem-declaration-style").is_some())
            .count(),
        1
    );
    let contexts = result
        .participants()
        .iter()
        .map(|p| attr(t, p.host_node(), "data-cem-css-context").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(contexts[0], contexts[1]);
    assert_eq!(source.source_text, before);
    assert!(!result.page().adoption_identity().hydration_ready());
}
#[test]
fn source_payload_leases_survive_parent_projection_and_sibling_continuation() {
    let c = control();
    let source=owner("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{section | {$ datadom.slots.heading}}</template></cem-element><cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{b | Ready}</template></cem-element><x-parent><x-child slot='heading'><template type='cem-ml'>{item | {$ 1 div 0}}</template></x-child></x-parent><x-child><template type='text/cem-ml'>{item | Sibling}</template></x-child>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let result = work.finish(&c).unwrap();
    assert_eq!(result.participants().len(), 3);
    let sources = result.participants()[1..]
        .iter()
        .map(|p| {
            let pass = p.render_pass();
            let admitted = pass
                .input()
                .bindings()
                .native_payload_source
                .as_ref()
                .unwrap();
            assert!(Arc::ptr_eq(
                admitted,
                pass.committed_island()
                    .bindings()
                    .native_payload_source
                    .as_ref()
                    .unwrap()
            ));
            assert!(admitted.matches_operation(&c));
            assert!(admitted
                .tree()
                .ast()
                .nodes
                .iter()
                .any(|n| matches!(n, cem_ml::parser::CemAstNode::Element { .. })));
            admitted.clone()
        })
        .collect::<Vec<_>>();
    assert!(!Arc::ptr_eq(&sources[0], &sources[1]));
    assert!(source.source_text.contains("{$ 1 div 0}"));
    for admitted in &sources {
        for id in 0..admitted.tree().ast().nodes.len() as u32 {
            assert_eq!(admitted.tree().source_line_number(id), None);
        }
    }
    let tree = result.tree().clone();
    drop(sources);
    drop(result);
    assert!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap() > 0);
    drop(tree);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn pending_cancellation_and_foreign_operation_never_publish_or_renew_page() {
    let c = control();
    let source = source("<x-item></x-item><x-item></x-item>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    let SsrPageProgress::Pending(request) = work.advance(&c).unwrap() else {
        panic!()
    };
    assert!(work
        .deliver(&request, response("b {}"), &control())
        .is_err());
    c.abort_signal().abort();
    assert!(work.deliver(&request, response("b {}"), &c).is_err());
    assert_eq!(work.state(), SsrPageState::Cancelled);
    assert!(work.finish(&c).is_err());
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
fn instance_source() -> Arc<cem_ml::validation::html::HtmlDocumentAst> {
    owner("<cem-element tag='x-item'><template id='x-item' type='text/cem-ml'>{b | Ready}</template></cem-element><x-item><template><style>@import 'shared.css'; b {color:red}</style></template></x-item><x-item><template><style>@import 'shared.css'; b {color:blue}</style></template></x-item>")
}
#[test]
fn instance_tickets_and_css_owners_stay_distinct_after_declaration_reuse() {
    let c = control();
    let source = instance_source();
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    let mut previous = None;
    for _ in 0..2 {
        let SsrPageProgress::Pending(request) = work.advance(&c).unwrap() else {
            panic!()
        };
        if let Some(old) = &previous {
            assert!(work.deliver(old, response("b {}"), &c).is_err());
            assert_eq!(work.state(), SsrPageState::Pending);
        }
        work.deliver(&request, response("b {font-weight:bold}"), &c)
            .unwrap();
        assert!(matches!(
            work.advance(&c).unwrap(),
            SsrPageProgress::Rendered { .. }
        ));
        previous = Some(request);
    }
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Quiescent
    ));
    let result = work.finish(&c).unwrap();
    assert_eq!(result.page().usage().resources, 4);
    let a = &result.participants()[0]
        .render_pass()
        .input()
        .instance_stylesheets()[0];
    let b = &result.participants()[1]
        .render_pass()
        .input()
        .instance_stylesheets()[0];
    assert!(!Arc::ptr_eq(a, b));
    assert_ne!(a.identity().cache_key, b.identity().cache_key);
    let t = result.tree();
    assert_eq!(
        (0..t.ast().nodes.len() as u32)
            .filter(|id| attr(t, *id, "data-cem-instance-style").is_some())
            .count(),
        2
    );
}
#[test]
fn aggregate_resource_and_byte_budgets_fail_on_later_participant_and_retire_every_lease() {
    for bytes in [false, true] {
        let c = control();
        let source = instance_source();
        let before = source.source_text.clone();
        let mut p = policy(&source);
        if bytes {
            p.max_total_bytes = source.source.byte_length as u64 + 6;
        } else {
            p.max_resources = 3;
        }
        let mut work = prepare_inline_page_with_resources(
            fixture::page_with_policy(&source, "memory:output", p),
            "page",
            capability(),
            &c,
        )
        .unwrap();
        let SsrPageProgress::Pending(first) = work.advance(&c).unwrap() else {
            panic!()
        };
        work.deliver(&first, response("b {}"), &c).unwrap();
        assert!(matches!(
            work.advance(&c).unwrap(),
            SsrPageProgress::Rendered { .. }
        ));
        if bytes {
            let SsrPageProgress::Pending(second) = work.advance(&c).unwrap() else {
                panic!()
            };
            assert!(work.deliver(&second, response("b {}"), &c).is_err());
        } else {
            assert!(work.advance(&c).is_err());
        }
        assert_eq!(work.state(), SsrPageState::Refused);
        assert_eq!(source.source_text, before);
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
        assert!(work.finish(&c).is_err());
    }
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn stylesheet_delivery_charges_parse_cpu_to_the_same_page_operation() {
    use cem_ml::operation_control::cpu::NativeCpuAccounting;
    let c = control();
    let source = source("<x-item></x-item>");
    let policy = policy(&source);
    let cpu = NativeCpuAccounting::new(&c, policy.max_cpu_time_ms).unwrap();
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    let SsrPageProgress::Pending(ticket) = work.advance(&c).unwrap() else {
        panic!()
    };
    let bytes = "b { color: red; margin: 1px }".repeat(128);
    assert!(work.resource_read_limit(&ticket, &c).unwrap() > bytes.len());
    assert!(work.resource_read_limit(&ticket, &control()).is_err());
    let before = cpu.usage_ms(&c).unwrap();
    if let Err(errors) = work.deliver(&ticket, response(&bytes), &c) {
        panic!("{errors:?} {:?}", work.resource_failure());
    }
    assert!(cpu.usage_ms(&c).unwrap() > before);
    assert!(cpu.is_idle());
    assert!(work.resource_read_limit(&ticket, &c).is_err());
}

#[test]
fn repeated_public_stylesheet_cannot_change_bytes_during_one_page() {
    let c = control();
    let source = instance_source();
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    let SsrPageProgress::Pending(first) = work.advance(&c).unwrap() else {
        panic!()
    };
    work.deliver(&first, response("b { color: red }"), &c)
        .unwrap();
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Rendered { .. }
    ));
    let SsrPageProgress::Pending(second) = work.advance(&c).unwrap() else {
        panic!()
    };
    let errors = work
        .deliver(&second, response("b { color: blue }"), &c)
        .unwrap_err();
    assert_eq!(errors[0].code, "cem.ssr.resource_changed");
    assert_eq!(work.state(), SsrPageState::Refused);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn foreign_page_tickets_and_transport_failure_cannot_replace_or_publish_work() {
    let c = control();
    let source = source("<x-item></x-item>");
    let new = || {
        prepare_inline_page_with_resources(
            fixture::page_with_policy(&source, "memory:output", policy(&source)),
            "page",
            capability(),
            &c,
        )
        .unwrap()
    };
    let mut a = new();
    let mut b = new();
    let SsrPageProgress::Pending(ra) = a.advance(&c).unwrap() else {
        panic!()
    };
    let SsrPageProgress::Pending(rb) = b.advance(&c).unwrap() else {
        panic!()
    };
    assert!(a.deliver(&rb, response("b {}"), &c).is_err());
    assert_eq!(a.state(), SsrPageState::Pending);
    assert!(a.fail_resource(&rb, "offline", &c).is_err());
    assert_eq!(a.state(), SsrPageState::Pending);
    assert!(a.fail_resource(&ra, "offline", &c).is_err());
    assert_eq!(a.state(), SsrPageState::Refused);
    assert_eq!(
        a.resource_failure().unwrap().code,
        "cem.css.import_load_failed"
    );
    assert!(!a.resource_failure().unwrap().source.frames.is_empty());
    assert!(a.finish(&c).is_err());
    assert!(b.finish(&c).is_err());
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
struct MutableResolver {
    inner: CemScopedModuleUrlResolver,
    revision: Arc<std::sync::atomic::AtomicUsize>,
}
impl cem_ml::module_resolution::CemModuleUrlResolver for MutableResolver {
    fn context_cache_identity(&self, context: &CemResolutionContextHandle) -> Option<String> {
        self.inner.context_cache_identity(context).map(|v| {
            format!(
                "{}:{v}",
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
        self.inner.resolve_module_url(request)
    }
}
#[test]
fn observed_resolver_revision_change_cannot_revive_cached_declaration_leases() {
    let c = control();
    let source = source("<x-item></x-item><x-item></x-item>");
    let revision = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let h = CemResolutionContextHandle::new("revision");
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(MutableResolver {
            revision: revision.clone(),
            inner: CemScopedModuleUrlResolver::new().with_context(
                h.clone(),
                CemModuleUrlContext {
                    identity: "page".into(),
                    resolver_identity: "test".into(),
                    resource_policy_stamp: "policy".into(),
                    frames: vec![CemModuleUrlFrame::new(
                        "page",
                        "https://ssr.example.test/guide/",
                    )],
                },
            ),
        }),
        h,
    );
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        cap,
        &c,
    )
    .unwrap();
    let SsrPageProgress::Pending(r) = work.advance(&c).unwrap() else {
        panic!()
    };
    work.deliver(&r, response("b {}"), &c).unwrap();
    assert!(matches!(
        work.advance(&c).unwrap(),
        SsrPageProgress::Rendered { .. }
    ));
    revision.store(1, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(work.state(), SsrPageState::Refused);
    revision.store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(work.advance(&c).is_err());
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn shared_scope_declaration_membership_and_css_lifetime_survive_nested_participants() {
    let c = control();
    let source=owner("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{x-item}</template></cem-element><cem-element tag='x-item' scope='shared-test'><template id='x-item' type='text/cem-ml'>{style @scope=shared-test |```b {color:red}```}{b | Ready}</template></cem-element><x-parent></x-parent><x-item></x-item>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let result = work.finish(&c).unwrap();
    assert_eq!(result.page().usage().resources, 1);
    for p in &result.participants()[1..] {
        assert_eq!(
            attr(result.tree(), p.host_node(), "scope").as_deref(),
            Some("shared-test")
        );
    }
    let record = result.participants()[1]
        .render_pass()
        .input()
        .declaration_stylesheets()[0]
        .clone();
    assert!(Arc::ptr_eq(
        &record,
        &result.participants()[2]
            .render_pass()
            .input()
            .declaration_stylesheets()[0]
    ));
    let tree = result.tree().clone();
    drop(result);
    assert!(record.emission().is_ok());
    assert!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap() > 0);
    drop(tree);
    assert!(record.emission().is_err());
    drop(record);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn projected_instance_css_retains_native_producer_maps_without_authored_coordinates() {
    let c = control();
    let source=owner("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{section | {$ datadom.slots.heading}}</template></cem-element><cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{b | Ready}</template></cem-element><x-parent><x-child slot='heading'><template><style>b {color:red}</style></template></x-child></x-parent>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let result = work.finish(&c).unwrap();
    let record = &result.participants()[1]
        .render_pass()
        .input()
        .instance_stylesheets()[0];
    let tree = record.source_tree(0).unwrap();
    for id in 0..tree.ast().nodes.len() as u32 {
        assert_eq!(tree.source_line_number(id), None);
    }
    assert!(tree.node_source_map(0).unwrap().frames.iter().any(|f|matches!(&f.transform,cem_ml::source_map::TransformKind::SourceBoundary{uri} if uri=="file:///source/page.html")));
    assert!(record.emission().unwrap().css().contains("color:red"));
    assert_eq!(result.page().usage().resources, 1);
}
#[test]
fn repeated_projection_shares_only_the_parsed_payload_view_and_keeps_distinct_source_leases() {
    let c = control();
    let source=owner("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{section | {$ datadom.slots.heading} {$ datadom.slots.heading}}</template></cem-element><cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{b | Ready}</template></cem-element><x-parent><x-child slot='heading'><template type='text/cem-ml'>{item | {$ 1 div 0}}</template></x-child></x-parent>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let result = work.finish(&c).unwrap();
    assert_eq!(result.participants().len(), 3);
    let a = result.participants()[1]
        .render_pass()
        .input()
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap();
    let b = result.participants()[2]
        .render_pass()
        .input()
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap();
    assert!(!Arc::ptr_eq(a, b));
    assert!(Arc::ptr_eq(a.tree(), b.tree()));
    assert!(!Arc::ptr_eq(a.source_tree(), b.source_tree()));
    assert!(a.matches_source(a.source_tree(), a.template_node()));
    assert!(b.matches_source(b.source_tree(), b.template_node()));
    let t = result.tree().clone();
    drop(result);
    drop(t);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn identical_payload_text_from_different_authored_origins_keeps_separate_views() {
    let c = control();
    let source=owner("<cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{b | Ready}</template></cem-element><x-child><template type='text/cem-ml'>{item | Same}</template></x-child><x-child><template type='text/cem-ml'>{item | Same}</template></x-child>");
    let mut work = prepare_inline_page_with_resources(
        fixture::page_with_policy(&source, "memory:output", policy(&source)),
        "page",
        capability(),
        &c,
    )
    .unwrap();
    while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let result = work.finish(&c).unwrap();
    let a = result.participants()[0]
        .render_pass()
        .input()
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap();
    let b = result.participants()[1]
        .render_pass()
        .input()
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap();
    assert!(!Arc::ptr_eq(a.tree(), b.tree()));
}
#[test]
fn payload_pool_revalidates_operation_limits_and_actual_type_on_cache_hits() {
    use cem_ml::{
        import::element_tree::RetainedCemPayloadSourcePool,
        parser::tree_apply::apply_attributes,
        value::{artifact::CemValueArtifactLimits, CemValueAttribute},
    };
    let c = control();
    {
        let source=owner("<cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{b | Ready}</template></cem-element><x-child><template type='text/cem-ml'>{item | Original}</template></x-child>");
        let mut work = prepare_inline_page_with_resources(
            fixture::page_with_policy(&source, "memory:output", policy(&source)),
            "page",
            capability(),
            &c,
        )
        .unwrap();
        while !matches!(work.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
        let result = work.finish(&c).unwrap();
        let original = result.participants()[0]
            .render_pass()
            .input()
            .bindings()
            .native_payload_source
            .as_ref()
            .unwrap();
        let pool = RetainedCemPayloadSourcePool::new(&c).unwrap();
        let limits = CemValueArtifactLimits::default();
        let a = pool
            .retain(
                original.source_tree().clone(),
                original.template_node(),
                &c,
                &limits,
            )
            .unwrap();
        let b = pool
            .retain(
                original.source_tree().clone(),
                original.template_node(),
                &c,
                &limits,
            )
            .unwrap();
        assert!(Arc::ptr_eq(a.tree(), b.tree()));
        assert!(pool
            .retain(
                original.source_tree().clone(),
                original.template_node(),
                &control(),
                &limits
            )
            .is_err());
        assert!(pool
            .retain(
                original.source_tree().clone(),
                original.template_node(),
                &c,
                &CemValueArtifactLimits {
                    max_values: 1,
                    ..limits
                }
            )
            .is_err());
        let changed = apply_attributes(
            original.source_tree().clone(),
            original.template_node(),
            Arc::new(vec![CemValueAttribute {
                name: "type".into(),
                namespace: None,
                qualified_name: None,
                value: "text/html".into(),
                value_stream: "text/html".to_owned(),
                contract: None,
                source_map: Default::default(),
            }]),
            Arc::new(()),
            limits,
            &c,
        )
        .unwrap();
        assert!(pool
            .retain(changed.tree().clone(), changed.output_target(), &c, &limits)
            .is_err());
    }
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
