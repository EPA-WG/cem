use cem_ml::{
    engine::{EngineContext, TemplateInput},
    html_base::*,
    html_base_evidence::*,
    html_base_host::*,
    html_execution_inventory::HtmlExecutionInventory,
    module_resolution::*,
    resolver::*,
    run_config::ScopeConfig,
};
use cem_ql::html_template_dependencies::*;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const BASE: &str = "https://document.test/assets/";
const ROOT: &str = "https://templates.test/root.cemt";
const CHILD: &str = "https://templates.test/imports/card.cemt";
const SOURCE: &str = r#"{module |
 {import @as=card @src="https://templates.test/imports/card.cemt"}
 {template @name=main |
  {style |```p { background: url(./root.svg) }```}
  {div @title='{module:module_url("./asset.svg")}'
       @class='{module:module_url("./explicit.svg", "https://explicit.test/base/")}' |
   {cem:variable @name=data @select='ct:read("file:///never-read.xml")'}
  }
 }
}"#;
const IMPORT: &[u8] = br#"{module | {template @name=card |
 {style |```p { background: url(./child.svg) }```}
 {span @title='{module:module_url("./child.svg")}' @class='{module:module_url("denied")}' | Card}
}}"#;

struct Loader {
    reads: Arc<AtomicUsize>,
    drift: Option<Arc<AtomicUsize>>,
}
impl ResourceResolver for Loader {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if let Some(revision) = &self.drift {
            revision.fetch_add(1, Ordering::SeqCst);
        }
        assert_eq!(
            request.uri, CHILD,
            "inspection must never execute content reads"
        );
        Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes: IMPORT.to_vec(),
            content_type: Some("text/cem-ml".into()),
        })
    }
    fn write(
        &self,
        request: &ResolveRequest,
        _: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        panic!("inspection must not write {}", request.uri)
    }
}
struct Resolver {
    inner: CemScopedModuleUrlResolver,
    revision: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    drift: bool,
}
impl CemModuleUrlResolver for Resolver {
    fn context_cache_identity(&self, context: &CemResolutionContextHandle) -> Option<String> {
        self.inner
            .context_cache_identity(context)
            .map(|id| format!("{id}:{}", self.revision.load(Ordering::SeqCst)))
    }
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.drift {
            self.revision.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.resolve_module_url(request)
    }
}
fn cap(
    name: &str,
    revision: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    drift: bool,
) -> CemModuleUrlResolutionCapability {
    let handle = CemResolutionContextHandle::new(name);
    let mut frame =
        CemModuleUrlFrame::new(name, format!("https://execution.test/{name}/start.html"));
    frame
        .specifiers
        .resources
        .insert("denied".into(), CemModuleUrlMapping::blocked());
    CemModuleUrlResolutionCapability::new(
        Arc::new(Resolver {
            inner: CemScopedModuleUrlResolver::new().with_context(
                handle.clone(),
                CemModuleUrlContext {
                    identity: name.into(),
                    resolver_identity: "resolver".into(),
                    resource_policy_stamp: "resource-policy".into(),
                    frames: vec![frame],
                },
            ),
            revision,
            calls,
            drift,
        }),
        handle,
    )
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
struct Fixture {
    host: HtmlBaseExecutionHost,
    document: Arc<HtmlExecutionInventory>,
    context: Arc<EngineContext>,
    root: Arc<TemplateInput>,
    document_cap: CemModuleUrlResolutionCapability,
    bodies: BTreeMap<String, CemModuleUrlResolutionCapability>,
    revision: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
}
fn fixture(preflight_drift: bool, resolution_drift: bool) -> Fixture {
    let revision = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let mut context = EngineContext::default();
    context.resolver_registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Loader {
            reads: reads.clone(),
            drift: preflight_drift.then(|| revision.clone()),
        },
    );
    let mut host = HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 100,
            url_bytes: 4096,
        },
    )
    .unwrap();
    let tree = cem_ml::import::import_data_bytes(br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><base href="../assets/"/></head><body><img src="image.svg"/></body></html>"#,
        "application/xml", "cem", "studio://retained-output.xhtml").unwrap();
    host.prepare(
        tree,
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://document.test/docs/page.html".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "base-policy".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let evidence = host.confirm(BASE.into()).unwrap();
    let document_cap = cap(
        "document",
        Arc::new(AtomicUsize::new(0)),
        calls.clone(),
        false,
    );
    let document = host
        .inspect_dependencies(&evidence, &document_cap, 100, slots, BASE)
        .unwrap();
    Fixture {
        host,
        document: Arc::new(document),
        context: Arc::new(context),
        root: Arc::new(TemplateInput {
            uri: ROOT.into(),
            bytes: SOURCE.as_bytes().to_vec(),
            identity: None,
            root_scope: ScopeConfig::default(),
        }),
        document_cap,
        bodies: BTreeMap::from([
            (
                ROOT.into(),
                cap("root", revision.clone(), calls.clone(), resolution_drift),
            ),
            (
                CHILD.into(),
                cap("child", revision.clone(), calls.clone(), false),
            ),
        ]),
        revision,
        calls,
        reads,
    }
}
fn prepare(
    f: &mut Fixture,
    limits: HtmlTemplateDependencyLimits,
) -> Result<HtmlTemplateDependencyPreparation, String> {
    prepare_html_template_dependencies(
        &mut f.host,
        f.document.clone(),
        f.context.clone(),
        f.root.clone(),
        &f.document_cap,
        &f.bodies,
        &Default::default(),
        limits,
        slots,
        BASE,
    )
}
fn validate(f: &mut Fixture, p: &HtmlTemplateDependencyPreparation) -> Result<(), String> {
    p.validate(
        &mut f.host,
        &f.context,
        &f.root,
        &f.document_cap,
        &f.bodies,
        slots,
        BASE,
    )
}
#[test]
fn native_preparation_retains_sources_and_distinct_execution_bases_without_reads() {
    let mut f = fixture(false, false);
    let p = prepare(&mut f, Default::default()).unwrap();
    assert_eq!(f.reads.load(Ordering::SeqCst), 1);
    assert!(Arc::ptr_eq(
        &p.document().document().tree,
        &f.document.document().tree
    ));
    assert_eq!(
        p.document().references()[0].resolved_uri.as_ref().unwrap(),
        "https://document.test/assets/image.svg"
    );
    assert_eq!(
        p.preflight().preflight.as_ref().unwrap().resolved_imports[0].bytes,
        IMPORT
    );
    assert_eq!(p.bodies().bodies.len(), 2);
    for (body, resolutions) in p.bodies().bodies.iter().zip(p.resolutions()) {
        for (sheet, plan) in body.stylesheets.iter().zip(&resolutions.stylesheets) {
            assert!(Arc::ptr_eq(sheet.css_tree(), &plan.tree));
        }
        for (expression, resolved) in body.expressions.iter().zip(&resolutions.expressions) {
            for (index, _) in &resolved.resolutions {
                let candidate = &resolved.inspection.uses[*index];
                assert!(expression
                    .query()
                    .unwrap()
                    .tree
                    .node(candidate.node_id)
                    .is_some());
                assert!(!candidate.source.frames.is_empty());
            }
        }
    }
    let root = &p.resolutions()[0];
    assert_eq!(
        root.stylesheets[0].references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://templates.test/root.svg"
    );
    let urls: Vec<_> = root
        .expressions
        .iter()
        .flat_map(|e| &e.resolutions)
        .map(|(_, r)| r.as_ref().unwrap().resolved_url.as_str())
        .collect();
    assert_eq!(
        urls,
        [
            "https://execution.test/root/asset.svg",
            "https://explicit.test/base/explicit.svg"
        ]
    );
    let child = &p.resolutions()[1];
    assert_eq!(
        child.stylesheets[0].references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://templates.test/imports/child.svg"
    );
    assert_eq!(
        child.expressions[0].resolutions[0]
            .1
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://execution.test/child/child.svg"
    );
    assert!(child.expressions[1].resolutions[0].1.is_err());
    for body in &p.bodies().bodies {
        for expression in &body.expressions {
            assert!(!expression.host_source.frames.is_empty());
            assert!(expression.query().is_some());
            assert_eq!(expression.template_uri, body.uri);
        }
    }
    assert!(p
        .blockers()
        .iter()
        .any(|b| b == "template-module-resolution-incomplete"));
    assert!(p
        .blockers()
        .iter()
        .any(|b| b == "template-output-correspondence-required"));
    assert!(p
        .blockers()
        .iter()
        .any(|b| b == "template-acquisition-freshness-required"));
    assert!(p
        .blockers()
        .iter()
        .any(|b| b == "resource-read-policy-required"));
    validate(&mut f, &p).unwrap();
    assert_eq!(
        f.reads.load(Ordering::SeqCst),
        1,
        "validation must not reacquire imports"
    );
}
#[test]
fn native_preparation_rejects_replaced_roots_contexts_resolvers_and_stale_hosts() {
    let mut f = fixture(false, false);
    let p = prepare(&mut f, Default::default()).unwrap();
    let root = f.root.clone();
    f.root = Arc::new((*root).clone());
    assert!(validate(&mut f, &p).is_err());
    f.root = root;
    let context = f.context.clone();
    f.context = Arc::new((*context).clone());
    assert!(validate(&mut f, &p).is_err());
    f.context = context;
    let original = f.bodies[ROOT].clone();
    f.bodies.insert(
        ROOT.into(),
        cap("root", f.revision.clone(), f.calls.clone(), false),
    );
    assert!(validate(&mut f, &p).is_err());
    f.bodies.insert(ROOT.into(), original);
    validate(&mut f, &p).unwrap();
    f.revision.fetch_add(1, Ordering::SeqCst);
    assert!(validate(&mut f, &p).is_err());
    f.host.invalidate().unwrap();
    assert!(validate(&mut f, &p).is_err());
}
#[test]
fn native_preparation_rejects_preflight_and_resolution_callback_drift() {
    for (preflight_drift, resolution_drift) in [(true, false), (false, true)] {
        let mut f = fixture(preflight_drift, resolution_drift);
        assert!(prepare(&mut f, Default::default()).is_err());
    }
}
#[test]
fn native_preparation_requires_all_contexts_and_complete_bounded_analysis_before_resolution() {
    for case in 0..8 {
        let mut f = fixture(false, false);
        let mut limits = HtmlTemplateDependencyLimits::default();
        match case {
            0 => {
                f.bodies.remove(CHILD);
            }
            1 => {
                limits.max_bodies = 1;
            }
            2 => {
                limits.per_body_budget = 0;
            }
            3 => {
                limits.max_import_depth = 0;
            }
            4 => {
                limits.per_body_budget = 5;
            }
            5 => {
                f.bodies.remove(ROOT);
            }
            6 => {
                f.bodies.insert(
                    "https://unvisited.test/extra.cemt".into(),
                    f.bodies[ROOT].clone(),
                );
            }
            _ => {
                Arc::make_mut(&mut f.root).bytes = b"{div @title='{'".to_vec();
            }
        }
        assert!(prepare(&mut f, limits).is_err(), "case {case}");
        assert_eq!(f.calls.load(Ordering::SeqCst), 0, "case {case}");
    }
}

#[test]
fn native_preparation_requires_fingerprints_and_all_function_bodies() {
    struct Uncacheable;
    impl CemModuleUrlResolver for Uncacheable {
        fn resolve_module_url(
            &self,
            _: &CemModuleUrlResolutionRequest,
        ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
            panic!("uncacheable body capability must be rejected before resolution")
        }
    }
    let mut f = fixture(false, false);
    f.bodies.insert(
        ROOT.into(),
        CemModuleUrlResolutionCapability::new(
            Arc::new(Uncacheable),
            CemResolutionContextHandle::new("root"),
        ),
    );
    assert!(prepare(&mut f, Default::default()).is_err());
    assert_eq!(f.reads.load(Ordering::SeqCst), 0);
    let mut f = fixture(false, false);
    let result = prepare_html_template_dependencies(
        &mut f.host,
        f.document.clone(),
        f.context.clone(),
        f.root.clone(),
        &f.document_cap,
        &f.bodies,
        &cem_ql::render::CompileTemplateOptions {
            skip_cemt_function_bodies: true,
            ..Default::default()
        },
        Default::default(),
        slots,
        BASE,
    );
    assert!(result.is_err());
    assert_eq!(f.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn native_preparation_rejects_foreign_and_changed_dom_owners() {
    let mut f = fixture(false, false);
    let p = prepare(&mut f, Default::default()).unwrap();
    let mut foreign = fixture(false, false);
    assert!(p
        .validate(
            &mut foreign.host,
            &f.context,
            &f.root,
            &f.document_cap,
            &f.bodies,
            slots,
            BASE
        )
        .is_err());
    // Change the fixture DOM predicate after preflight performs its first read.
    let reads = f.reads.clone();
    let result = prepare_html_template_dependencies(
        &mut f.host,
        f.document.clone(),
        f.context.clone(),
        f.root.clone(),
        &f.document_cap,
        &f.bodies,
        &Default::default(),
        Default::default(),
        |slot| {
            if reads.load(Ordering::SeqCst) > 1 {
                Err("fixture DOM changed during preflight".into())
            } else {
                slots(slot)
            }
        },
        BASE,
    );
    assert!(result.is_err());
    assert!(
        validate(&mut f, &p).is_err(),
        "DOM mismatch must revoke the previous preparation too"
    );
}

#[test]
fn native_preparation_bounds_retained_css_before_any_expression_resolution() {
    let mut f = fixture(false, false);
    f.bodies.remove(CHILD);
    Arc::make_mut(&mut f.root).bytes = format!(
        "{{style |```{}```}}{{div @title='{{module:module_url(\"./asset.svg\")}}' | Ready}}",
        "p { background: url(./large.svg) }".repeat(40)
    )
    .into_bytes();
    let result = prepare(
        &mut f,
        HtmlTemplateDependencyLimits {
            per_body_budget: 32,
            ..Default::default()
        },
    );
    assert!(
        result.is_err(),
        "retained stylesheet analysis must fit the same body budget"
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
