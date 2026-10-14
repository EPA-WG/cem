use cem_ml::{
    css_import_snapshot::{CssSnapshotCaptureLimits, CssSnapshotSource, CssSourceSnapshot},
    engine::EngineContext,
    html_base::*,
    html_base_evidence::*,
    html_base_host::*,
    module_resolution::*,
    template_snapshot::*,
};
use cem_ql::{
    html_template_dependencies::HtmlTemplateDependencyLimits, template_execution_record::*,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

const ROOT: &str = "https://templates.test/root.cemt";
const CHILD: &str = "https://templates.test/cards/card.cemt";
const OUTPUT: &str = "https://output.test/docs/page.xhtml";
const BASE: &str = "https://output.test/assets/";
const ROOT_SOURCE: &str = r#"{style |```@import 'theme'; @import './local.css'; p {background:url(icon)}```}
{module-map | {resource @specifier=theme @target='./root.css'} {resource @specifier=icon @target='./root.svg'}}
{module | {import @as=cards @src='./cards/card.cemt'}
 {body | {html @xmlns='http://www.w3.org/1999/xhtml' | {head | {base @href='../assets/'}}
 {body | {call @from=cards @template=card} {p @style='background:url(./document.svg)' | Root}}}}}"#;
const CHILD_SOURCE: &str = r#"{style |```@import 'theme' layer(base) supports(display:grid) screen; @import 'theme'; p {background:url(icon)}```}
{module-map | {resource @specifier=theme @target='./child.css'} {resource @specifier=icon @target='./child.svg'}}
{module | {template @name=card @visibility=public | {body | {span | Child}}}}"#;

struct ObservedResolver {
    inner: CemModuleUrlResolutionCapability,
    body: bool,
    revision: Arc<AtomicUsize>,
    drift: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}
impl CemModuleUrlResolver for ObservedResolver {
    fn context_cache_identity(&self, _: &CemResolutionContextHandle) -> Option<String> {
        self.inner.cache_identity().map(|id| {
            format!(
                "{id}:{}",
                if self.body {
                    self.revision.load(Ordering::SeqCst)
                } else {
                    0
                }
            )
        })
    }
    fn derive_child_context(
        &self,
        _: &CemResolutionContextHandle,
        frame: CemModuleUrlFrame,
    ) -> Result<CemModuleUrlResolutionCapability, String> {
        let inner = self.inner.derive_child_context(frame)?;
        let handle = inner.context().clone();
        Ok(CemModuleUrlResolutionCapability::new(
            Arc::new(Self {
                inner,
                body: true,
                revision: self.revision.clone(),
                drift: self.drift.clone(),
                calls: self.calls.clone(),
            }),
            handle,
        ))
    }
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.body && self.drift.load(Ordering::SeqCst) {
            self.revision.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.resolve_from_context(
            request.purpose,
            &request.authored_specifier,
            &request.current_context,
            request.referrer.clone(),
            request.source_map.clone(),
        )
    }
}
fn cap(
    denied: bool,
    revision: Arc<AtomicUsize>,
    drift: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
) -> CemModuleUrlResolutionCapability {
    let mut frame = CemModuleUrlFrame::new("host", "https://host.test/maps/context.html");
    frame.allowed_schemes = Some(["https".into()].into());
    frame.specifiers.resources.insert(
        "theme".into(),
        if denied {
            CemModuleUrlMapping::blocked()
        } else {
            CemModuleUrlMapping::target("./wrong.css")
        },
    );
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("./host.svg"));
    let handle = CemResolutionContextHandle::new("host");
    let inner = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "host".into(),
                resolver_identity: "fixture".into(),
                resource_policy_stamp: "https:1".into(),
                frames: vec![frame],
            },
        )),
        handle.clone(),
    );
    CemModuleUrlResolutionCapability::new(
        Arc::new(ObservedResolver {
            inner,
            body: false,
            revision,
            drift,
            calls,
        }),
        handle,
    )
}
fn templates(root: &str, child: &str) -> Arc<TemplateSourceSnapshot> {
    TemplateSourceSnapshot::new(
        [(ROOT, root), (CHILD, child)]
            .into_iter()
            .map(|(uri, source)| TemplateSnapshotSource {
                uri: uri.into(),
                revision: "v1".into(),
                bytes: source.as_bytes().to_vec(),
                content_type: "text/cem-ml".into(),
            })
            .collect(),
        Default::default(),
    )
    .unwrap()
}
fn css_sources() -> Arc<CssSourceSnapshot> {
    CssSourceSnapshot::new(
        [
            (
                "https://templates.test/root.css",
                "https://cdn.test/root/root.css",
                "p {background:url(./root-leaf.svg)}",
            ),
            (
                "https://templates.test/local.css",
                "https://templates.test/local.css",
                "p {color:red}",
            ),
            (
                "https://templates.test/cards/child.css",
                "https://cdn.test/child/child.css",
                "@import './nested.css'; p {background:url(icon)}",
            ),
            (
                "https://cdn.test/child/nested.css",
                "https://cdn.test/child/nested.css",
                "p {background:url(./nested.svg)}",
            ),
        ]
        .into_iter()
        .map(|(requested, final_url, source)| CssSnapshotSource {
            requested_url: requested.into(),
            final_url: final_url.into(),
            revision: "css-v1".into(),
            bytes: source.as_bytes().to_vec(),
            content_type: "text/css".into(),
        })
        .collect(),
        Default::default(),
    )
    .unwrap()
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
struct Fixture {
    host: HtmlBaseExecutionHost,
    execution: Arc<SnapshotTemplateExecution>,
    prepared: Arc<ExecutedHtmlTemplatePreparation>,
    snapshot: Arc<TemplateSourceSnapshot>,
    context: Arc<EngineContext>,
    cap: CemModuleUrlResolutionCapability,
    revision: Arc<AtomicUsize>,
    drift: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}
fn fixture(root: &str, child: &str, denied: bool) -> Fixture {
    let snapshot = templates(root, child);
    let context = Arc::new(EngineContext::default());
    let revision = Arc::new(AtomicUsize::new(0));
    let drift = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let cap = cap(denied, revision.clone(), drift.clone(), calls.clone());
    let preflight =
        Arc::new(preflight_template_snapshot(snapshot.clone(), context.clone(), ROOT, 8).unwrap());
    let execution = execute_snapshot_xhtml_scoped(
        preflight,
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        HtmlTemplateDependencyLimits::default(),
    )
    .unwrap();
    let mut host = HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 1000,
            url_bytes: 4096,
        },
    )
    .unwrap();
    host.prepare(
        execution.tree().clone(),
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl(OUTPUT.into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "base:1".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let base = host.confirm(BASE.into()).unwrap();
    let document = Arc::new(
        host.inspect_dependencies(&base, &cap, 1000, slots, BASE)
            .unwrap(),
    );
    let prepared = Arc::new(
        execution
            .prepare_dependencies(&mut host, document, &snapshot, &context, &cap, slots, BASE)
            .unwrap(),
    );
    calls.store(0, Ordering::SeqCst);
    Fixture {
        host,
        execution,
        prepared,
        snapshot,
        context,
        cap,
        revision,
        drift,
        calls,
    }
}
fn capture(
    f: &mut Fixture,
    css: Arc<CssSourceSnapshot>,
    limits: CssSnapshotCaptureLimits,
) -> Result<ExecutedTemplateCssImports, String> {
    f.prepared.prepare_css_imports(
        &mut f.host,
        &f.execution,
        &f.snapshot,
        &f.context,
        &f.cap,
        css,
        limits,
        slots,
        BASE,
    )
}

#[test]
fn closures_use_recorded_body_contexts_final_bases_and_exact_native_roots() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let css = css_sources();
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    assert!(std::ptr::eq(result.preparation(), f.prepared.as_ref()));
    assert_eq!(result.bodies().len(), 2);
    for (index, body) in result.bodies().iter().enumerate() {
        let original = &f.execution.bodies().bodies[index];
        assert_eq!(body.body_uri, original.uri);
        assert_eq!(body.closures.len(), original.stylesheets.len());
        assert!(Arc::ptr_eq(
            &body.closures[0].closure().sheets()[0].resources.tree,
            original.stylesheets[0].css_tree()
        ));
        assert!(
            !body.closures[0].closure().sheets()[0].resources.references[0]
                .source
                .frames
                .is_empty()
        );
    }
    let root = result.bodies()[0].closures[0].closure();
    assert_eq!(root.sheets()[1].url, "https://cdn.test/root/root.css");
    assert_eq!(root.sheets()[2].url, "https://templates.test/local.css");
    assert_eq!(
        root.sheets()[1].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://cdn.test/root/root-leaf.svg"
    );
    let child = result.bodies()[1].closures[0].closure();
    assert_eq!(child.sheets().len(), 5); // root + two repeated imports with nested children
    assert!(matches!(
        &child.edges()[0].conditions,
        cem_ml::css_resources::CssResourceKind::Import {
            layer: Some(layer), supports: Some(supports), media: Some(media)
        } if layer == "base" && supports == "display:grid" && media == "screen"
    ));
    assert_eq!(
        child.sheets()[1].resources.references[1]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://templates.test/cards/child.svg"
    );
    assert_eq!(
        child.sheets()[2].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://cdn.test/child/nested.svg"
    );
    assert_eq!(
        result.bodies()[1].closures[0].acquisitions()[0]
            .source
            .revision,
        "css-v1"
    );
    assert_eq!(result.document().closures().len(), 1);
    assert_eq!(
        result.document().closures()[0].closure().sheets()[0]
            .resources
            .references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://output.test/assets/document.svg"
    );
    for discharged in [
        "stylesheet-owner-context-required",
        "template-stylesheet-import-closure-required",
        "css-import-and-runtime-closure-required",
    ] {
        assert!(
            f.prepared.blockers().iter().any(|b| b == discharged),
            "original {discharged}"
        );
        assert!(
            !result.blockers().iter().any(|b| b == discharged),
            "combined {discharged}"
        );
    }
    for retained in [
        "stylesheet-execution-history-required",
        "runtime-dependency-closure-required",
        "atomic-host-transaction-required",
    ] {
        assert!(
            result.blockers().iter().any(|b| b == retained),
            "{retained}"
        );
    }
    result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE,
        )
        .unwrap();
}

#[test]
fn replaced_owners_and_stale_hosts_cannot_consume_css_preparation() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let css = css_sources();
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css_sources(),
            slots,
            BASE
        )
        .is_err());
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &templates(ROOT_SOURCE, CHILD_SOURCE),
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    let other = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    assert!(result
        .validate(
            &mut f.host,
            &other.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &other.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE,
        )
        .unwrap();
    f.host.invalidate().unwrap();
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
}

#[test]
fn all_root_bounds_precede_resolution_and_total_occurrences_and_bytes_are_shared() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    for mode in ["roots", "nodes", "sheets", "bytes"] {
        let mut limits = CssSnapshotCaptureLimits::default();
        if mode == "roots" {
            limits.imports.max_sheets = 2;
        } // three retained roots before import expansion
        if mode == "nodes" {
            limits.max_root_nodes = 1;
        }
        if mode == "sheets" {
            limits.imports.max_sheets = 8;
        } // document 1 + root 3 + child 5
        if mode == "bytes" {
            limits.responses.max_total_bytes = 120;
        }
        f.calls.store(0, Ordering::SeqCst);
        let error = capture(&mut f, css_sources(), limits).err().unwrap();
        if matches!(mode, "roots" | "nodes") {
            assert_eq!(f.calls.load(Ordering::SeqCst), 0, "{error}");
        } else {
            assert!(
                error.contains(if mode == "sheets" {
                    "cem.css.import_limit"
                } else {
                    "cem.css.import_byte_limit"
                }),
                "{error}"
            );
        }
    }
    let css = css_sources();
    let total_bytes = [
        ("https://templates.test/root.css", 1),
        ("https://templates.test/local.css", 1),
        ("https://templates.test/cards/child.css", 2),
        ("https://cdn.test/child/nested.css", 2),
    ]
    .iter()
    .map(|(uri, count)| css.source(uri).unwrap().bytes.len() * count)
    .sum();
    let root_nodes = f
        .prepared
        .preparation()
        .document()
        .stylesheets()
        .iter()
        .map(|sheet| sheet.resources.as_ref().unwrap().tree.ast().nodes.len())
        .sum::<usize>()
        + f.execution
            .bodies()
            .bodies
            .iter()
            .flat_map(|body| &body.stylesheets)
            .map(|sheet| sheet.css_tree().ast().nodes.len())
            .sum::<usize>();
    let mut exact = CssSnapshotCaptureLimits::default();
    exact.imports.max_sheets = 9;
    exact.responses.max_total_bytes = total_bytes;
    exact.max_root_nodes = root_nodes;
    let result = capture(&mut f, css, exact).unwrap();
    assert_eq!(
        result
            .document()
            .closures()
            .iter()
            .chain(result.bodies().iter().flat_map(|body| &body.closures))
            .map(|closure| closure.closure().received_bytes())
            .sum::<usize>(),
        total_bytes
    );
}

#[test]
fn missing_or_denied_css_and_callback_drift_cannot_publish() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let missing = CssSourceSnapshot::new(vec![], Default::default()).unwrap();
    assert!(capture(&mut f, missing, Default::default())
        .err()
        .unwrap()
        .contains("cem.css.snapshot_missing"));
    f.drift.store(true, Ordering::SeqCst);
    assert!(capture(&mut f, css_sources(), Default::default()).is_err());
    let mut denied = fixture(ROOT_SOURCE, CHILD_SOURCE, true);
    assert!(capture(&mut denied, css_sources(), Default::default())
        .err()
        .unwrap()
        .contains("cem.css.import_resolution_failed"));
}

#[test]
fn derived_policy_drift_is_permanent_and_cancellation_rejects_retained_records() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let css = css_sources();
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    let host_fingerprint = f.cap.cache_identity();
    f.revision.store(1, Ordering::SeqCst);
    assert_eq!(f.cap.cache_identity(), host_fingerprint);
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    f.revision.store(0, Ordering::SeqCst);
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    f.context.operation_control.abort_signal().abort();
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    assert!(capture(&mut f, css, Default::default()).is_err());
}

#[test]
fn empty_stylesheet_sets_still_bind_snapshot_and_all_body_contexts() {
    let root = ROOT_SOURCE
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(" @style='background:url(./document.svg)'", "");
    let child = CHILD_SOURCE.lines().skip(1).collect::<Vec<_>>().join("\n");
    let mut f = fixture(&root, &child, false);
    let css = CssSourceSnapshot::new(vec![], Default::default()).unwrap();
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    assert!(result.bodies().iter().all(|b| b.closures.is_empty()));
    assert!(result.document().closures().is_empty());
    f.revision.store(1, Ordering::SeqCst);
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
    f.revision.store(0, Ordering::SeqCst);
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
}

#[test]
fn consumption_latches_body_policy_changes_observed_during_dom_callbacks() {
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let css = css_sources();
    let result = capture(&mut f, css.clone(), Default::default()).unwrap();
    let revision = f.revision.clone();
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            |slot| {
                revision.store(1, Ordering::SeqCst);
                slots(slot)
            },
            BASE,
        )
        .is_err());
    f.revision.store(0, Ordering::SeqCst);
    // The host and original preparation remain current; it is the captured
    // CSS result that permanently remembers the observed body-policy change.
    f.prepared
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            slots,
            BASE,
        )
        .unwrap();
    assert!(result
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            slots,
            BASE
        )
        .is_err());
}

#[test]
fn stylesheet_execution_joins_document_link_and_body_owners() {
    use cem_ml::stylesheet_execution::*;
    struct Observer;
    impl StylesheetExecutionObserver for Observer {
        fn environment_revision(&mut self) -> Result<String, String> {
            Ok("viewport:1;cssom:1".into())
        }
        fn observe(
            &mut self,
            slot: StylesheetExecutionSlot<'_>,
        ) -> Result<StylesheetObservation, String> {
            Ok(StylesheetObservation {
                selected: true,
                conditions_match: true,
                loaded_source: Some(slot.source_key.into()),
                installation: Some((slot.root * 100 + slot.sheet + 1) as u64),
            })
        }
    }
    let root = ROOT_SOURCE.replace(
        "{head |",
        "{head | {link @rel=stylesheet @href='https://templates.test/root.css'}",
    );
    let mut f = fixture(&root, CHILD_SOURCE, false);
    let css = css_sources();
    let captured = Arc::new(capture(&mut f, css.clone(), Default::default()).unwrap());
    let mut host = StylesheetExecutionHost::default();
    let evidence = captured
        .record_execution(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            &mut host,
            &mut Observer,
            100,
            slots,
            BASE,
        )
        .unwrap();
    assert_eq!(evidence.execution().observations().len(), 10);
    assert!(!evidence
        .blockers()
        .iter()
        .any(|b| b == "stylesheet-execution-history-required"));
    assert!(evidence
        .blockers()
        .iter()
        .any(|b| b == "css-runtime-dependency-closure-required"));
    evidence
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            &mut host,
            &mut Observer,
            slots,
            BASE,
        )
        .unwrap();
    f.revision.store(1, Ordering::SeqCst);
    assert!(evidence
        .validate(
            &mut f.host,
            &f.execution,
            &f.snapshot,
            &f.context,
            &f.cap,
            &css,
            &mut host,
            &mut Observer,
            slots,
            BASE
        )
        .is_err());
}

struct CompiledObserver;
impl cem_ql::stylesheet_installation::CompiledStylesheetObserver for CompiledObserver {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok("fixture:1".into())
    }
    fn observe(
        &mut self,
        slot: cem_ql::stylesheet_installation::CompiledStylesheetSlot<'_>,
    ) -> Result<cem_ql::stylesheet_installation::CompiledStylesheetObservation, String> {
        assert!(slot.css.starts_with("@scope"));
        Ok(
            cem_ql::stylesheet_installation::CompiledStylesheetObservation {
                installation: slot.index as u32 + 1,
                selected: true,
                source_matches: true,
            },
        )
    }
}

#[test]
fn recorded_body_installations_compile_exact_scopes_and_keep_body_policy_live() {
    use cem_ml::css_emission::CssManagedScope;
    use cem_ql::template_execution_record::BodyStylesheetInstallation;
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let css = Arc::new(capture(&mut f, css_sources(), Default::default()).unwrap());
    let choices: Vec<_> = f
        .execution
        .bodies()
        .bodies
        .iter()
        .flat_map(|body| {
            body.stylesheets
                .iter()
                .enumerate()
                .map(|(index, _)| BodyStylesheetInstallation {
                    body_uri: body.uri.clone(),
                    index,
                    scope: CssManagedScope::Private {
                        tag: "cem-card".into(),
                        context: None,
                    },
                })
        })
        .collect();
    assert!(css.compile_installations(&choices[..1], 100000).is_err());
    assert!(css.compile_installations(&choices, 1).is_err());
    let mut choices = choices;
    choices.swap(0, 1);
    assert!(css.compile_installations(&choices, 100000).is_err());
    choices.swap(0, 1);
    let compiled = css.compile_installations(&choices, 100000).unwrap();
    let mut host = cem_ql::stylesheet_installation::CompiledStylesheetHost::default();
    let evidence = compiled
        .capture(&mut host, &mut CompiledObserver, 100000)
        .unwrap();
    evidence.validate(&mut host, &mut CompiledObserver).unwrap();
    assert!(evidence
        .blockers()
        .iter()
        .any(|b| b == "atomic-host-transaction-required"));
    assert_eq!(compiled.len(), 2);
    assert!(compiled
        .css(0)
        .unwrap()
        .contains("https://templates.test/root.svg"));
    assert!(compiled.css(1).unwrap().contains("@supports"));
    assert!(compiled.css(1).unwrap().contains("@scope"));
    f.revision.store(1, Ordering::SeqCst);
    assert!(compiled.css(0).is_err());
    assert!(evidence.validate(&mut host, &mut CompiledObserver).is_err());
    f.revision.store(0, Ordering::SeqCst);
    assert!(compiled.css(0).is_err());
}

#[test]
fn suppressed_body_emission_never_becomes_installation_evidence() {
    use cem_ml::css_emission::CssManagedScope;
    let mut f = fixture(ROOT_SOURCE, CHILD_SOURCE, false);
    let css = Arc::new(capture(&mut f, css_sources(), Default::default()).unwrap());
    let choices: Vec<_> = f
        .execution
        .bodies()
        .bodies
        .iter()
        .flat_map(|body| {
            body.stylesheets
                .iter()
                .enumerate()
                .map(|(index, _)| BodyStylesheetInstallation {
                    body_uri: body.uri.clone(),
                    index,
                    scope: CssManagedScope::Private {
                        tag: "cem-card".into(),
                        context: None,
                    },
                })
        })
        .collect();
    assert!(css
        .compile_installations(&choices, 100000)
        .err()
        .unwrap()
        .contains("layer_unsupported"));
}

#[test]
fn empty_body_installations_bind_policy_and_explicit_release() {
    let root = ROOT_SOURCE
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(" @style='background:url(./document.svg)'", "");
    let child = CHILD_SOURCE.lines().skip(1).collect::<Vec<_>>().join("\n");
    let mut f = fixture(&root, &child, false);
    let sources = CssSourceSnapshot::new(vec![], Default::default()).unwrap();
    let css = Arc::new(capture(&mut f, sources, Default::default()).unwrap());
    let compiled = css.compile_installations(&[], 0).unwrap();
    assert!(compiled.is_empty());
    let mut host = cem_ql::stylesheet_installation::CompiledStylesheetHost::default();
    let evidence = compiled
        .capture(&mut host, &mut CompiledObserver, 0)
        .unwrap();
    evidence.validate(&mut host, &mut CompiledObserver).unwrap();
    let released = css.compile_installations(&[], 0).unwrap();
    released.release();
    assert!(released
        .capture(&mut Default::default(), &mut CompiledObserver, 0)
        .is_err());
    f.revision.store(1, Ordering::SeqCst);
    assert!(evidence.validate(&mut host, &mut CompiledObserver).is_err());
    f.revision.store(0, Ordering::SeqCst);
    assert!(compiled
        .capture(&mut host, &mut CompiledObserver, 0)
        .is_err());
}

fn element(tree: &cem_ml::parser::tree::RetainedCemTree, name: &str) -> u32 {
    (0..tree.ast().nodes.len() as u32)
        .find(|id| {
            tree.node(*id).is_some_and(|n| {
                n.name.as_ref().is_some_and(|n| n.local_name == name)
                    && n.kind == cem_ml::parser::tree::CemTreeNodeKind::Element
            })
        })
        .unwrap()
}
fn composed(
    f: &mut Fixture,
) -> (
    Arc<RecordedBodyStylesheetInstallations>,
    Arc<RecordedTemplateOutput>,
) {
    let css = Arc::new(capture(f, css_sources(), Default::default()).unwrap());
    let choices = f
        .execution
        .bodies()
        .bodies
        .iter()
        .flat_map(|body| {
            body.stylesheets
                .iter()
                .enumerate()
                .map(|(index, _)| BodyStylesheetInstallation {
                    body_uri: body.uri.clone(),
                    index,
                    scope: cem_ml::css_emission::CssManagedScope::Private {
                        tag: "cem-card".into(),
                        context: None,
                    },
                })
        })
        .collect::<Vec<_>>();
    let compiled = css.compile_installations(&choices, 100000).unwrap();
    let head = element(f.execution.tree(), "head");
    let output = compiled
        .compose_output(&vec![head; compiled.len()], 10000, 100000)
        .unwrap();
    (compiled, output)
}
fn composed_document(
    f: &mut Fixture,
    output: &RecordedTemplateOutput,
) -> Arc<cem_ml::html_execution_inventory::HtmlExecutionInventory> {
    f.host
        .prepare(
            output.tree().clone(),
            HtmlBaseContext {
                fallback: HtmlFallbackBase::DocumentUrl(OUTPUT.into()),
                encoding: HtmlBaseEncoding::Utf8,
                policy_stamp: "base:1".into(),
            },
            slots,
            |_| Ok(HtmlBasePolicyDecision::Allow),
        )
        .unwrap();
    let base = f.host.confirm(BASE.into()).unwrap();
    Arc::new(
        f.host
            .inspect_dependencies(&base, &f.cap, 10000, slots, BASE)
            .unwrap(),
    )
}
struct OutputObserver {
    wrong: bool,
}
impl cem_ml::stylesheet_execution::StylesheetExecutionObserver for OutputObserver {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok("output:1".into())
    }
    fn observe(
        &mut self,
        slot: cem_ml::stylesheet_execution::StylesheetExecutionSlot<'_>,
    ) -> Result<cem_ml::stylesheet_execution::StylesheetObservation, String> {
        Ok(cem_ml::stylesheet_execution::StylesheetObservation {
            selected: true,
            conditions_match: true,
            loaded_source: Some(if self.wrong {
                "wrong".into()
            } else {
                slot.source_key.into()
            }),
            installation: Some((slot.root * 100 + slot.sheet + 1) as u64),
        })
    }
}
#[test]
fn complete_output_composes_retained_emissions_and_joins_live_document_and_installations() {
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (compiled, output) = composed(&mut f);
    assert_eq!(output.stylesheet_nodes().len(), 2);
    assert_ne!(Arc::as_ptr(output.tree()), Arc::as_ptr(f.execution.tree()));
    assert!(output.output().rendered.contains("<style>"));
    let style_start = output.output().rendered.find("<style>").unwrap() as u64 + 7;
    let style_end = output.output().rendered.find("</style>").unwrap() as u64;
    assert!(
        output
            .output()
            .output_spans
            .iter()
            .any(|span| span.output_range.start >= style_start
                && span.output_range.end() <= style_end
                && span.output_range.len > 0
                && !span.origin.frames.is_empty()),
        "emitted CSS must preserve its own source frames, independently of body spans"
    );
    assert_eq!(
        output.original_execution().output().rendered,
        f.execution.output().rendered
    );
    let document = composed_document(&mut f, &output);
    let mut host = RecordedTemplateOutputHost::default();
    let mut observer = OutputObserver { wrong: false };
    let evidence = output
        .capture(
            &mut f.host,
            document,
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    evidence
        .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
        .unwrap();
    assert!(!evidence
        .blockers()
        .iter()
        .any(|s| s == "stylesheet-execution-history-required"));
    assert!(evidence
        .blockers()
        .iter()
        .any(|s| s == "css-runtime-dependency-closure-required"));
    assert!(evidence
        .blockers()
        .iter()
        .any(|s| s == "atomic-host-transaction-required"));
    compiled.release();
    assert!(evidence
        .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
        .is_err());
}
#[test]
fn whole_output_rejects_foreign_html_and_changed_non_stylesheet_slots() {
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (_, output) = composed(&mut f);
    let mut host = RecordedTemplateOutputHost::default();
    let mut observer = OutputObserver { wrong: false };
    // The original, uncomposed output cannot certify newly installed styles.
    let original = Arc::new(
        f.host
            .inspect_dependencies(
                &f.host.current().unwrap().clone(),
                &f.cap,
                10000,
                slots,
                BASE,
            )
            .unwrap(),
    );
    assert!(output
        .capture(
            &mut f.host,
            original,
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default()
        )
        .is_err());
    let doc = composed_document(&mut f, &output);
    let evidence = output
        .capture(
            &mut f.host,
            doc,
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    let body = element(output.tree(), "p");
    assert!(evidence
        .validate(
            &mut f.host,
            &mut host,
            &mut observer,
            |slot| Ok(if slot.id == body { 100000 } else { slot.id + 1 }),
            BASE
        )
        .is_err());
    assert!(evidence
        .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
        .is_err());
}
#[test]
fn whole_output_latches_css_and_body_policy_drift_and_rejects_cancellation() {
    for failure in ["css", "policy", "cancel"] {
        let mut f = fixture(
            ROOT_SOURCE,
            &CHILD_SOURCE.replace(" layer(base)", ""),
            false,
        );
        let (_, output) = composed(&mut f);
        let doc = composed_document(&mut f, &output);
        let mut host = RecordedTemplateOutputHost::default();
        let mut observer = OutputObserver { wrong: false };
        let evidence = output
            .capture(
                &mut f.host,
                doc,
                &mut host,
                &mut observer,
                slots,
                BASE,
                Default::default(),
            )
            .unwrap();
        match failure {
            "css" => observer.wrong = true,
            "policy" => f.revision.store(1, Ordering::SeqCst),
            _ => f.context.operation_control.abort_signal().abort(),
        }
        assert!(evidence
            .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
            .is_err());
        observer.wrong = false;
        f.revision.store(0, Ordering::SeqCst);
        assert!(evidence
            .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
            .is_err());
    }
}
#[test]
fn whole_output_bounds_complete_placements_and_preserves_declaration_scope_contract() {
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (compiled, _) = composed(&mut f);
    let head = element(f.execution.tree(), "head");
    let body = element(f.execution.tree(), "p");
    assert!(compiled.compose_output(&[head], 10000, 100000).is_err());
    assert!(compiled.compose_output(&[head, head], 1, 100000).is_err());
    assert!(compiled.compose_output(&[head, head], 10000, 1).is_err());
    assert!(compiled
        .compose_output(&[body, head], 10000, 100000)
        .is_err());
    let css = Arc::new(capture(&mut f, css_sources(), Default::default()).unwrap());
    let choices = f
        .execution
        .bodies()
        .bodies
        .iter()
        .flat_map(|body| {
            body.stylesheets
                .iter()
                .enumerate()
                .map(|(index, _)| BodyStylesheetInstallation {
                    body_uri: body.uri.clone(),
                    index,
                    scope: cem_ml::css_emission::CssManagedScope::Instance,
                })
        })
        .collect::<Vec<_>>();
    assert!(css.compile_installations(&choices, 100000).is_err());
}

#[test]
fn composed_equal_bytes_do_not_substitute_native_owners_or_html_epochs() {
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (compiled, output) = composed(&mut f);
    let head = element(f.execution.tree(), "head");
    let other = compiled
        .compose_output(&[head, head], 10000, 100000)
        .unwrap();
    assert_eq!(output.output().rendered, other.output().rendered);
    let doc = composed_document(&mut f, &output);
    let mut host = RecordedTemplateOutputHost::default();
    let mut observer = OutputObserver { wrong: false };
    assert!(other
        .capture(
            &mut f.host,
            doc.clone(),
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default()
        )
        .is_err());
    let evidence = output
        .capture(
            &mut f.host,
            doc,
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    // Even a same-output recapture is a different live HTML epoch.
    composed_document(&mut f, &output);
    assert!(evidence
        .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
        .is_err());
}
#[test]
fn empty_composed_output_keeps_all_body_policies_and_callback_invalidation() {
    let root = ROOT_SOURCE
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(" @style='background:url(./document.svg)'", "");
    let child = CHILD_SOURCE.lines().skip(1).collect::<Vec<_>>().join("\n");
    let mut f = fixture(&root, &child, false);
    let css = Arc::new(
        capture(
            &mut f,
            CssSourceSnapshot::new(vec![], Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap(),
    );
    let compiled = css.compile_installations(&[], 0).unwrap();
    let output = compiled.compose_output(&[], 10000, 100000).unwrap();
    assert!(output.stylesheet_nodes().is_empty());
    assert_eq!(output.output().rendered, f.execution.output().rendered);
    let doc = composed_document(&mut f, &output);
    let mut host = RecordedTemplateOutputHost::default();
    let mut observer = OutputObserver { wrong: false };
    let evidence = output
        .capture(
            &mut f.host,
            doc,
            &mut host,
            &mut observer,
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    let revision = f.revision.clone();
    assert!(evidence
        .validate(
            &mut f.host,
            &mut host,
            &mut observer,
            |slot| {
                revision.store(1, Ordering::SeqCst);
                slots(slot)
            },
            BASE
        )
        .is_err());
    f.revision.store(0, Ordering::SeqCst);
    assert!(evidence
        .validate(&mut f.host, &mut host, &mut observer, slots, BASE)
        .is_err());
}

#[test]
fn recorded_html_delivery_keeps_empty_templates_closed_and_rechecks_native_policy() {
    let root = ROOT_SOURCE.replace(
        "{head |",
        "{head | {result-element @name=template @namespace='http://www.w3.org/1999/xhtml'}",
    );
    let f = fixture(&root, CHILD_SOURCE, false);
    let before = f.execution.output().rendered.clone();
    let html = f.execution.html_output().unwrap();
    assert!(html.rendered.contains("</template>"), "{}", html.rendered);
    assert_eq!(html.rendered, f.execution.html_output().unwrap().rendered);
    assert_eq!(before, f.execution.output().rendered);
    assert!(!html.output_spans.is_empty());
    f.revision.store(1, Ordering::SeqCst);
    assert!(f.execution.html_output().is_err());
}
#[path = "template_css_imports/worker_preparation.rs"]
mod worker_preparation;
