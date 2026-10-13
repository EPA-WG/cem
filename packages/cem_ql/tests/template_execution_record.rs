use cem_ml::{
    engine::EngineContext, html_base::*, html_base_evidence::*, html_base_host::*,
    module_resolution::*, template_snapshot::*,
};
use cem_ql::{
    html_template_dependencies::HtmlTemplateDependencyLimits, template_execution_record::*,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
const ROOT: &str = "https://templates.test/root.cemt";
const OUTPUT: &str = "https://output.test/docs/page.xhtml";
const BASE: &str = "https://output.test/assets/";
const SOURCE: &str = r#"{module |
 {import @as=cards @src="./card.cemt"}
 {body |
  {html @xmlns="http://www.w3.org/1999/xhtml" |
   {head | {base @href="../assets/"}}
   {body | {call @from=cards @template=card} {p | {$ label}}}
  }
 }
}"#;
const CHILD: &str = r#"{module | {template @name=card @visibility=public | {body | {img @src='{module:module_url("./image.svg")}'}}}}"#;
fn sources(root: &str, child: &str) -> Arc<TemplateSourceSnapshot> {
    TemplateSourceSnapshot::new(
        vec![
            TemplateSnapshotSource {
                uri: ROOT.into(),
                revision: "1".into(),
                bytes: root.as_bytes().to_vec(),
                content_type: "text/cem-ml".into(),
            },
            TemplateSnapshotSource {
                uri: "https://templates.test/card.cemt".into(),
                revision: "1".into(),
                bytes: child.as_bytes().to_vec(),
                content_type: "text/cem-ml".into(),
            },
        ],
        Default::default(),
    )
    .unwrap()
}
struct Resolver {
    inner: CemScopedModuleUrlResolver,
    revision: Arc<AtomicUsize>,
    drift: bool,
}
impl CemModuleUrlResolver for Resolver {
    fn context_cache_identity(&self, context: &CemResolutionContextHandle) -> Option<String> {
        self.inner
            .context_cache_identity(context)
            .map(|s| format!("{s}:{}", self.revision.load(Ordering::SeqCst)))
    }
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        if self.drift {
            self.revision.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.resolve_module_url(request)
    }
}
fn capability(revision: Arc<AtomicUsize>, drift: bool) -> CemModuleUrlResolutionCapability {
    let context = CemResolutionContextHandle::new("execution");
    CemModuleUrlResolutionCapability::new(
        Arc::new(Resolver {
            inner: CemScopedModuleUrlResolver::new().with_context(
                context.clone(),
                CemModuleUrlContext {
                    identity: "execution".into(),
                    resolver_identity: "resolver".into(),
                    resource_policy_stamp: "policy".into(),
                    frames: vec![CemModuleUrlFrame::new(
                        "execution",
                        "https://runtime.test/shared/index.html",
                    )],
                },
            ),
            revision,
            drift,
        }),
        context,
    )
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
fn capture(
    tree: Arc<cem_ml::parser::tree::RetainedCemTree>,
    cap: &CemModuleUrlResolutionCapability,
) -> (
    HtmlBaseExecutionHost,
    Arc<cem_ml::html_execution_inventory::HtmlExecutionInventory>,
) {
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
        tree,
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl(OUTPUT.into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "base-policy".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let base = host.confirm(BASE.into()).unwrap();
    let inventory = Arc::new(
        host.inspect_dependencies(&base, cap, 1000, slots, BASE)
            .unwrap(),
    );
    (host, inventory)
}
#[test]
fn recorded_execution_binds_actual_output_and_snapshot_without_reacquisition() {
    let sources = sources(SOURCE, CHILD);
    let context = Arc::new(EngineContext::default());
    let preflight =
        Arc::new(preflight_template_snapshot(sources.clone(), context.clone(), ROOT, 8).unwrap());
    let cap = capability(Arc::new(AtomicUsize::new(0)), false);
    let mut bindings = BTreeMap::from([("label".into(), "Snapshot output".into())]);
    let record =
        execute_snapshot_xhtml(preflight, &cap, &bindings, OUTPUT, Default::default()).unwrap();
    assert!(record
        .output()
        .rendered
        .contains("https://runtime.test/shared/image.svg"));
    assert!(record.output().rendered.contains("Snapshot output"));
    bindings.insert("label".into(), "Changed caller".into());
    assert_eq!(record.scalar_bindings()["label"], "Snapshot output");
    assert!(!record.output().output_spans.is_empty());
    let (mut host, inventory) = capture(record.tree().clone(), &cap);
    let prepared = record
        .prepare_dependencies(&mut host, inventory, &sources, &context, &cap, slots, BASE)
        .unwrap();
    assert!(std::ptr::eq(
        prepared.preparation().bodies(),
        record.bodies()
    ));
    assert!(std::ptr::eq(
        prepared.preparation().preflight(),
        record.preflight().inspection()
    ));
    for discharged in [
        "template-output-correspondence-required",
        "template-acquisition-freshness-required",
        "template-output-document-analysis-required",
    ] {
        assert!(
            !prepared.blockers().iter().any(|b| b == discharged),
            "{discharged}"
        );
    }
    for retained in [
        "runtime-dependency-closure-required",
        "atomic-host-transaction-required",
        "general-document-reference-coverage-required",
    ] {
        assert!(
            prepared.blockers().iter().any(|b| b == retained),
            "{retained}"
        );
    }
    prepared
        .validate(&mut host, &record, &sources, &context, &cap, slots, BASE)
        .unwrap();
    let replaced = sources_clone();
    assert!(prepared
        .validate(&mut host, &record, &replaced, &context, &cap, slots, BASE)
        .is_err());
    host.invalidate().unwrap();
    assert!(prepared
        .validate(&mut host, &record, &sources, &context, &cap, slots, BASE)
        .is_err());
}
fn sources_clone() -> Arc<TemplateSourceSnapshot> {
    sources(SOURCE, CHILD)
}
#[test]
fn recorded_execution_rejects_equal_reimported_output_and_other_execution() {
    let sources = sources(SOURCE, CHILD);
    let context = Arc::new(EngineContext::default());
    let preflight =
        Arc::new(preflight_template_snapshot(sources.clone(), context.clone(), ROOT, 8).unwrap());
    let cap = capability(Arc::new(AtomicUsize::new(0)), false);
    let bindings = BTreeMap::from([("label".into(), "Same".into())]);
    let record = execute_snapshot_xhtml(
        preflight.clone(),
        &cap,
        &bindings,
        OUTPUT,
        Default::default(),
    )
    .unwrap();
    let copied = cem_ml::import::import_data_bytes(
        record.output().rendered.as_bytes(),
        "application/xml",
        "cem",
        OUTPUT,
    )
    .unwrap();
    let (mut host, inventory) = capture(copied, &cap);
    assert!(record
        .prepare_dependencies(&mut host, inventory, &sources, &context, &cap, slots, BASE)
        .is_err());
    let (mut host, inventory) = capture(record.tree().clone(), &cap);
    let prepared = record
        .prepare_dependencies(&mut host, inventory, &sources, &context, &cap, slots, BASE)
        .unwrap();
    let other =
        execute_snapshot_xhtml(preflight, &cap, &bindings, OUTPUT, Default::default()).unwrap();
    assert!(prepared
        .validate(&mut host, &other, &sources, &context, &cap, slots, BASE)
        .is_err());
    let other_cap = capability(Arc::new(AtomicUsize::new(0)), false);
    assert!(prepared
        .validate(&mut host, &record, &sources, &context, &other_cap, slots, BASE)
        .is_err());
}
#[test]
fn recorded_execution_rejects_unrecorded_maps_reads_drift_and_bounds() {
    for (child, expected) in [
        (
            r#"{module-map | {resource @specifier=icon @target="./icon.svg"}}{module | {template @name=card @visibility=public | {body | {span | Mapped}}}}"#,
            "body module maps",
        ),
        (
            r#"{module | {module-map | {resource @specifier=icon @target="./icon.svg"}} {template @name=card @visibility=public | {body | {span | Mapped}}}}"#,
            "body module maps",
        ),
        (
            r#"{module | {template @name=card | {div @title='{ct:read("file:///never-read.xml")}'}}}"#,
            "runtime reads",
        ),
    ] {
        let sources = sources(SOURCE, child);
        let context = Arc::new(EngineContext::default());
        let preflight = Arc::new(preflight_template_snapshot(sources, context, ROOT, 8).unwrap());
        let cap = capability(Arc::new(AtomicUsize::new(0)), false);
        let rejected = execute_snapshot_xhtml(
            preflight,
            &cap,
            &BTreeMap::from([("label".into(), "x".into())]),
            OUTPUT,
            Default::default(),
        )
        .err()
        .unwrap();
        assert!(rejected.contains(expected), "{rejected}");
    }
    let preflight = Arc::new(
        preflight_template_snapshot(
            sources(SOURCE, CHILD),
            Arc::new(EngineContext::default()),
            ROOT,
            8,
        )
        .unwrap(),
    );
    let cap = capability(Arc::new(AtomicUsize::new(0)), true);
    assert!(execute_snapshot_xhtml(
        preflight.clone(),
        &cap,
        &BTreeMap::from([("label".into(), "x".into())]),
        OUTPUT,
        Default::default()
    )
    .is_err());
    let cap = capability(Arc::new(AtomicUsize::new(0)), false);
    assert!(execute_snapshot_xhtml(
        preflight,
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        HtmlTemplateDependencyLimits {
            per_body_budget: 0,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn recorded_execution_requires_an_actual_xhtml_document_root() {
    let snapshot = sources("{div | Not an XHTML document}", CHILD);
    let preflight = Arc::new(
        preflight_template_snapshot(snapshot, Arc::new(EngineContext::default()), ROOT, 8).unwrap(),
    );
    let cap = capability(Arc::new(AtomicUsize::new(0)), false);
    let result = execute_snapshot_xhtml(
        preflight,
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    );
    assert!(result.is_err());
}

fn native_capability(frame: CemModuleUrlFrame) -> CemModuleUrlResolutionCapability {
    let handle = CemResolutionContextHandle::new("native-host");
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "native-host".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "host-policy".into(),
                frames: vec![frame],
            },
        )),
        handle,
    )
}
fn host_frame() -> CemModuleUrlFrame {
    CemModuleUrlFrame::new("host", "https://host.test/context/")
}
const MAPPED_ROOT: &str = r#"{module-map | {resource @specifier=root @target="./root.svg"}}
{module |
 {import @as=cards @src="./card.cemt"}
 {body | {html @xmlns="http://www.w3.org/1999/xhtml" |
  {head | {base @href="../assets/"}}
  {body |
   {img @src='{module:module_url("root")}' }
   {call @from=cards @template=card @with:label='{module:module_url("./argument.svg")}'}
   {img @src='{module:module_url("./after.svg")}' }
  }
 }}
}"#;
const MAPPED_CHILD: &str = r#"{module-map | {resource @specifier=icon @target="./child.svg"}}
{module | {template @name=card @visibility=public | {param @name=label} {body |
 {img @src='{module:module_url("icon")}' }
 {img @src='{module:module_url("root")}' }
 {img @src='{module:module_url("./local.svg")}' }
 {img @src='{module:module_url("./explicit.svg", "https://referrer.test/else/")}' }
 {p | {$ label}}
}}}"#;
#[test]
fn scoped_execution_installs_body_maps_and_restores_caller_context() {
    let snapshot = sources(MAPPED_ROOT, MAPPED_CHILD);
    let context = Arc::new(EngineContext::default());
    let preflight =
        Arc::new(preflight_template_snapshot(snapshot.clone(), context.clone(), ROOT, 8).unwrap());
    let cap = native_capability(host_frame());
    let record = execute_snapshot_xhtml_scoped(
        preflight,
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    )
    .unwrap();
    for url in [
        "https://templates.test/root.svg",
        "https://templates.test/child.svg",
        "https://templates.test/local.svg",
        "https://templates.test/argument.svg",
        "https://templates.test/after.svg",
        "https://referrer.test/else/explicit.svg",
    ] {
        assert!(
            record.output().rendered.contains(url),
            "{url}: {}",
            record.output().rendered
        );
    }
    assert!(!record.output().rendered.contains("module-map"));
    let (mut host, inventory) = capture(record.tree().clone(), &cap);
    let prepared = record
        .prepare_dependencies(&mut host, inventory, &snapshot, &context, &cap, slots, BASE)
        .unwrap();
    assert!(!prepared
        .blockers()
        .iter()
        .any(|b| b == "template-module-map-context-required"
            || b == "module-resolution-context-required"));
    let urls: Vec<_> = prepared
        .preparation()
        .resolutions()
        .iter()
        .flat_map(|body| &body.expressions)
        .flat_map(|e| &e.resolutions)
        .map(|(_, r)| r.as_ref().unwrap().resolved_url.as_str())
        .collect();
    assert!(urls.contains(&"https://templates.test/child.svg"));
    prepared
        .validate(&mut host, &record, &snapshot, &context, &cap, slots, BASE)
        .unwrap();
}
#[test]
fn scoped_execution_preserves_ancestor_restrictions_and_css_precedence() {
    let child = format!("{{style |```p {{ background: url(icon) }}```}}{MAPPED_CHILD}");
    let snapshot = sources(MAPPED_ROOT, &child);
    let context = Arc::new(EngineContext::default());
    let preflight =
        Arc::new(preflight_template_snapshot(snapshot.clone(), context.clone(), ROOT, 8).unwrap());
    let mut frame = host_frame();
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("./outer.svg"));
    let cap = native_capability(frame.clone());
    let record = execute_snapshot_xhtml_scoped(
        preflight.clone(),
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    )
    .unwrap();
    assert!(record
        .output()
        .rendered
        .contains("https://host.test/context/outer.svg"));
    let (mut host, inventory) = capture(record.tree().clone(), &cap);
    let prepared = record
        .prepare_dependencies(&mut host, inventory, &snapshot, &context, &cap, slots, BASE)
        .unwrap();
    let child_css = &prepared.preparation().resolutions()[1].stylesheets[0];
    assert_eq!(
        child_css.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://templates.test/child.svg"
    );
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::blocked());
    assert!(execute_snapshot_xhtml_scoped(
        preflight.clone(),
        &native_capability(frame),
        &BTreeMap::new(),
        OUTPUT,
        Default::default()
    )
    .is_err());
    let mut frame = host_frame();
    frame.allowed_schemes = Some(["file".into()].into());
    assert!(execute_snapshot_xhtml_scoped(
        preflight,
        &native_capability(frame),
        &BTreeMap::new(),
        OUTPUT,
        Default::default()
    )
    .is_err());
}
#[test]
fn scoped_execution_rejects_unsupported_resolvers_and_unapplied_nested_maps() {
    let preflight = Arc::new(
        preflight_template_snapshot(
            sources(MAPPED_ROOT, MAPPED_CHILD),
            Arc::new(EngineContext::default()),
            ROOT,
            8,
        )
        .unwrap(),
    );
    assert!(execute_snapshot_xhtml_scoped(
        preflight,
        &capability(Arc::new(AtomicUsize::new(0)), false),
        &BTreeMap::new(),
        OUTPUT,
        Default::default()
    )
    .is_err());
    let nested = r#"{module | {module-map | {resource @specifier=icon @target="./bad.svg"}} {template @name=card @visibility=public | {body | {span | x}}}}"#;
    let preflight = Arc::new(
        preflight_template_snapshot(
            sources(MAPPED_ROOT, nested),
            Arc::new(EngineContext::default()),
            ROOT,
            8,
        )
        .unwrap(),
    );
    let error = execute_snapshot_xhtml_scoped(
        preflight,
        &native_capability(host_frame()),
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    )
    .err()
    .unwrap();
    assert!(error.contains("unapplied"), "{error}");
}
#[test]
fn scoped_execution_records_grandchild_context_and_rejects_ambiguous_ancestry() {
    let root = MAPPED_ROOT.replace("./card.cemt", "./cards/card.cemt");
    let child = r#"{module-map | {resource @specifier=parent @target="./parent.svg"}}
    {module | {import @as=leaf @src="./deep/leaf.cemt"} {template @name=card @visibility=public | {body |
    {call @from=leaf @template=leaf} {img @src='{module:module_url("./after.svg")}' } }}}"#;
    let leaf = r#"{module-map | {resource @specifier=leaf @target="./leaf.svg"}}
    {module | {template @name=leaf @visibility=public | {body |
    {img @src='{module:module_url("leaf")}' } {img @src='{module:module_url("parent")}' }
    }}}"#;
    let run = |root: &str| {
        let snapshot = TemplateSourceSnapshot::new(
            vec![
                (ROOT, root),
                ("https://templates.test/cards/card.cemt", child),
                ("https://templates.test/cards/deep/leaf.cemt", leaf),
            ]
            .into_iter()
            .map(|(uri, text)| TemplateSnapshotSource {
                uri: uri.into(),
                revision: "1".into(),
                bytes: text.as_bytes().to_vec(),
                content_type: "text/cem-ml".into(),
            })
            .collect(),
            Default::default(),
        )
        .unwrap();
        let preflight = Arc::new(
            preflight_template_snapshot(snapshot, Arc::new(EngineContext::default()), ROOT, 8)
                .unwrap(),
        );
        execute_snapshot_xhtml_scoped(
            preflight,
            &native_capability(host_frame()),
            &BTreeMap::new(),
            OUTPUT,
            Default::default(),
        )
    };
    let record = run(&root).unwrap();
    for url in [
        "https://templates.test/cards/deep/leaf.svg",
        "https://templates.test/cards/parent.svg",
        "https://templates.test/cards/after.svg",
        "https://templates.test/after.svg",
    ] {
        assert!(
            record.output().rendered.contains(url),
            "{url}: {}",
            record.output().rendered
        );
    }
    let repeated = MAPPED_ROOT.replace(
        "{body | {html",
        "{import @as=again @src='./card.cemt'} {body | {html",
    );
    let preflight = Arc::new(
        preflight_template_snapshot(
            sources(&repeated, MAPPED_CHILD),
            Arc::new(EngineContext::default()),
            ROOT,
            8,
        )
        .unwrap(),
    );
    assert!(execute_snapshot_xhtml_scoped(
        preflight,
        &native_capability(host_frame()),
        &BTreeMap::new(),
        OUTPUT,
        Default::default()
    )
    .is_ok());
    let ambiguous = root.replace(
        "{body | {html",
        "{import @as=direct @src='./cards/deep/leaf.cemt'} {body | {html",
    );
    let error = run(&ambiguous).err().unwrap();
    assert!(error.contains("ancestry"), "{error}");
}

#[test]
fn scoped_execution_keeps_lexical_context_in_match_hooks_and_control_expressions() {
    let root = r#"{module | {import @as=cards @src="./cards/card.cemt"}
     {body | {html @xmlns="http://www.w3.org/1999/xhtml" | {head | {base @href="../assets/"}}
      {body | {call @from=cards @template=card} {p | {$module:module_url("./root-after.svg")}}}
     }}}
    "#;
    let child = r#"{module |
     {template @on=expression @into=content @match='value == "trigger"' |
      {result-sequence @select='module:module_url("./hook.svg")'}}
     {template @name=matched @mode=asset @match='module:module_url("./test") == module:module_url("https://templates.test/cards/test")' |
      {span | {$module:module_url("./match.svg")}}}
     {template @name=card @visibility=public | {body |
      {cem:variable @name=asset @select='module:module_url("./variable.svg")'}
      {p | {$asset}} {p | {$"trigger"}}
      {cem:if @test='module:module_url("./test") == module:module_url("https://templates.test/cards/test")' |
       {b | conditional}}
      {cem:choose | {cem:when @test='module:module_url("./test") == module:module_url("https://templates.test/cards/test")' | {b | choice}}}
      {cem:for-each @select='module:module_url("./loop.svg")' @as=item | {p | {$item}}}
      {apply-templates @select='"subject"' @mode=asset}
     }}}
    "#;
    let snapshot = TemplateSourceSnapshot::new(
        vec![
            (ROOT, root),
            ("https://templates.test/cards/card.cemt", child),
        ]
        .into_iter()
        .map(|(uri, text)| TemplateSnapshotSource {
            uri: uri.into(),
            revision: "1".into(),
            bytes: text.as_bytes().to_vec(),
            content_type: "text/cem-ml".into(),
        })
        .collect(),
        Default::default(),
    )
    .unwrap();
    let preflight = Arc::new(
        preflight_template_snapshot(snapshot, Arc::new(EngineContext::default()), ROOT, 8).unwrap(),
    );
    let record = execute_snapshot_xhtml_scoped(
        preflight,
        &native_capability(host_frame()),
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    )
    .unwrap();
    for text in [
        "https://templates.test/cards/hook.svg",
        "https://templates.test/cards/match.svg",
        "https://templates.test/cards/variable.svg",
        "https://templates.test/cards/loop.svg",
        "conditional",
        "choice",
        "https://templates.test/root-after.svg",
    ] {
        assert!(
            record.output().rendered.contains(text),
            "{text}: {}",
            record.output().rendered
        );
    }
}

#[test]
fn scoped_execution_bounds_extracted_module_map_entries() {
    let entries = (0..80)
        .map(|n| format!("{{resource @specifier=unused{n} @target='./{n}.svg'}}"))
        .collect::<String>();
    let root = format!("{{module-map | {entries}}}{{html @xmlns='http://www.w3.org/1999/xhtml'}}");
    let preflight = Arc::new(
        preflight_template_snapshot(
            sources(&root, CHILD),
            Arc::new(EngineContext::default()),
            ROOT,
            8,
        )
        .unwrap(),
    );
    let error = execute_snapshot_xhtml_scoped(
        preflight,
        &native_capability(host_frame()),
        &BTreeMap::new(),
        OUTPUT,
        HtmlTemplateDependencyLimits {
            per_body_budget: 64,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("module-map budget"), "{error}");
}

struct DerivedPolicy {
    inner: CemModuleUrlResolutionCapability,
    revision: Arc<AtomicUsize>,
    body: bool,
}
impl CemModuleUrlResolver for DerivedPolicy {
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
                revision: self.revision.clone(),
                body: true,
            }),
            handle,
        ))
    }
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.inner.resolve_from_context(
            request.purpose,
            request.authored_specifier.clone(),
            &request.current_context,
            request.referrer.clone(),
            request.source_map.clone(),
        )
    }
}
#[test]
fn scoped_execution_rechecks_derived_policy_when_host_policy_is_unchanged() {
    let inner = native_capability(host_frame());
    let handle = inner.context().clone();
    let revision = Arc::new(AtomicUsize::new(0));
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(DerivedPolicy {
            inner,
            revision: revision.clone(),
            body: false,
        }),
        handle,
    );
    let snapshot = sources(MAPPED_ROOT, MAPPED_CHILD);
    let context = Arc::new(EngineContext::default());
    let preflight =
        Arc::new(preflight_template_snapshot(snapshot.clone(), context.clone(), ROOT, 8).unwrap());
    let record = execute_snapshot_xhtml_scoped(
        preflight,
        &cap,
        &BTreeMap::new(),
        OUTPUT,
        Default::default(),
    )
    .unwrap();
    let (mut host, inventory) = capture(record.tree().clone(), &cap);
    let prepared = record
        .prepare_dependencies(&mut host, inventory, &snapshot, &context, &cap, slots, BASE)
        .unwrap();
    let host_fingerprint = cap.cache_identity();
    revision.fetch_add(1, Ordering::SeqCst);
    assert_eq!(cap.cache_identity(), host_fingerprint);
    let error = prepared
        .validate(&mut host, &record, &snapshot, &context, &cap, slots, BASE)
        .unwrap_err();
    assert!(error.contains("body resolver changed"), "{error}");
}
