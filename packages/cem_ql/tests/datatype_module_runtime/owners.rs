use super::*;
use cem_ml::schema::{
    declaration_references::SchemaDeclarationNode,
    value_contracts::{ContractName, ValueContractSource, ValueContracts},
};
use cem_ql::{
    datatype_dependency_capsule as capsule,
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{NativeDatatypeValidator, RuleExecution, ValidationCall},
    eval::QueryContextScope,
    native_registrations::ImplementationIdentity,
    selected_registry::{SelectedBuiltin, SelectedFunctionRegistry},
    structural_receiver::{
        DatatypeDependencyRuleRegistration, FreshDatatypeRuleBinding,
        FreshDependencyDatatypeCompilation,
    },
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex, Weak,
};
#[path = "../support/datatype_rule_artifacts.rs"]
mod rule_artifacts;
fn identity(text: &str, uri: &str) -> SourceIdentity {
    SourceIdentity::new(uri, SourceId(1), text.as_bytes())
}
fn node(c: &LexicallyScopedDocument, name: &str) -> SchemaDeclarationNode {
    c.document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(c.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn adapter() -> DatatypeResultAdapter {
    let c = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let ns = cem_ml::schema::registry::CEM_SCHEMA_URI;
    let s = ValueContractSource::new(
        node(&c, "schema"),
        ns,
        BTreeMap::from([("schema".into(), ns.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[s], Default::default()).unwrap()),
        ContractName::new(ns, "datatype-validation-result"),
        ContractName::new(ns, "datatype-diagnostic"),
    )
    .unwrap()
}
fn native_identity() -> ImplementationIdentity {
    use cem_ml::schema::datatype_validation::{
        CandidateRequirement, ResultRepresentation, ValidationSignature,
    };
    ImplementationIdentity::validator(
        "urn:test:validate",
        hash(b"module"),
        "validate",
        ValidationSignature {
            kind: DatatypeKind::Scalar,
            value: ValueRepresentation::Scalar(ScalarRepresentation::String),
            candidate: CandidateRequirement::Optional,
            result: ResultRepresentation::Accepted(ContractName::new(
                cem_ml::schema::registry::CEM_SCHEMA_URI,
                "datatype-validation-result",
            )),
        },
    )
}
fn registration(query: bool) -> DatatypeDependencyRuleRegistration {
    if query {
        DatatypeDependencyRuleRegistration::Query {
            functions: SelectedFunctionRegistry::new([SelectedBuiltin::StringLength]).unwrap(),
            adapter: adapter(),
        }
    } else {
        DatatypeDependencyRuleRegistration::NativeValidator { adapter: adapter() }
    }
}
fn graph(query: bool, unused: bool) -> (Arc<VerifiedDatatypeModules>, String) {
    graph_with(query, unused, false)
}
fn graph_with(query: bool, unused: bool, list: bool) -> (Arc<VerifiedDatatypeModules>, String) {
    let (target, artifact) = if unused {
        rule_artifacts::rule_artifact(query)
    } else {
        rule_artifacts::rule_artifact_body(
            query,
            r#"{accepted: str:length(value)>2, diagnostics: {code: "module.note", severity: "warning", message: "original", source: datatype}}"#,
        )
    };
    let id = artifact.id().to_owned();
    let text = if unused {
        "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar} {type @name=unused @kind=scalar @rule={#check}}}}"
    } else {
        "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar @rule={#check}}}}"
    };
    let mut core = source("file:///app/types/core.cem", text, "scalar");
    if unused {
        core.roots.insert(datatype(&core.capture, "unused"));
    }
    let target = match target {
        capsule::DependencyTarget::Query(p) => ModuleTarget::Query(p),
        capsule::DependencyTarget::Native(p) => ModuleTarget::Native(p),
        _ => unreachable!(),
    };
    core.bindings.insert(
        field(
            &core.capture,
            if unused { "unused" } else { "scalar" },
            "rule",
        ),
        target,
    );
    let mut vendor = derived("vendor", "core", datatype(&core.capture, "scalar"));
    if list {
        vendor=source("file:///app/types/vendor.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:vendor | {types | {type @name=value @kind=list @base={#selected}}}}","value");
        vendor.bindings.insert(
            field(&vendor.capture, "value", "base"),
            ModuleTarget::ModuleDatatype {
                module: "core".into(),
                root: datatype(&core.capture, "scalar"),
            },
        );
    }
    let mut app = derived("app", "vendor", *vendor.roots.first().unwrap());
    if list {
        app=source("file:///app/types/app.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @list-base={#selected}}}}","value");
        app.bindings.insert(
            field(&app.capture, "value", "list-base"),
            ModuleTarget::ModuleDatatype {
                module: "vendor".into(),
                root: *vendor.roots.first().unwrap(),
            },
        );
    }
    let input = BTreeMap::from([
        ("app".into(), app),
        ("vendor".into(), vendor),
        ("core".into(), core),
    ]);
    let artifacts = [artifact];
    let b = modules::export(
        "app",
        &input,
        &artifacts,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    (
        Arc::new(
            modules::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap(),
        ),
        id,
    )
}
fn prepare_rule(
    h: &mut FreshStructuralHost,
    g: &VerifiedDatatypeModules,
    id: &str,
    scopes: &BTreeMap<String, DeclarationScope>,
) {
    let item = RetainedCemNode::new(
        g.leaf_source(id).unwrap().clone(),
        node(g.leaf_capture(id).unwrap(), "behavior").node_id(),
    )
    .unwrap()
    .query_item();
    let c = StandaloneExpressionContext::default().with_binding(
        "check",
        StandaloneExpressionBinding::any(ItemStream::once(item)),
    );
    h.prepare_datatype_module_lexical_scopes(g, "core", |_, _, _| {
        (Some(c.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(scopes["core"], scopes[id]);
    let mut ctx = StandaloneExpressionContext::default();
    ctx.scope = QueryContextScope(73);
    ctx.scope_policy = ctx.scope_policy.with_queue_size(128);
    h.set_context(scopes[id], Some(ctx));
}
type Reentry = Arc<
    Mutex<
        Option<(
            Weak<FreshStructuralHost>,
            Weak<FreshDependencyDatatypeCompilation>,
            AstNodeId,
        )>,
    >,
>;
#[derive(Debug, Default)]
struct Rule {
    calls: Arc<AtomicUsize>,
    action: Arc<AtomicUsize>,
    owner: Arc<Mutex<Option<Weak<FreshDatatypeRuleBinding>>>>,
    reentry: Reentry,
    marker: Arc<()>,
}
impl NativeDatatypeValidator for Rule {
    fn validate(&self, c: ValidationCall<'_>) -> RuleExecution {
        let count = self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(c.runtime.query.scope, QueryContextScope(73));
        assert_eq!(c.runtime.query.scope_policy.queue_size, 128);
        assert!(c.runtime.query.module_resolution.is_none());
        let _ = &self.marker;
        match self.action.load(Ordering::SeqCst) {
            1 => self
                .owner
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap()
                .retire(),
            2 => {
                c.runtime.control.cancel_root(None, None).unwrap();
            }
            3 if count == 0 => {
                let t = self.reentry.lock().unwrap().clone().unwrap();
                assert!(t
                    .1
                    .upgrade()
                    .unwrap()
                    .validate(&t.0.upgrade().unwrap(), t.2, &input(1), Default::default())
                    .is_err());
            }
            _ => {}
        }
        RuleExecution::Complete(ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(
                    c.value[0].atom() == Some(AtomValue::String("same".into())),
                ))],
            ),
            (
                "diagnostics".into(),
                vec![Item::Record(BTreeMap::from([
                    (
                        "code".into(),
                        vec![Item::Atomic(AtomValue::String("module.note".into()))],
                    ),
                    (
                        "severity".into(),
                        vec![Item::Atomic(AtomValue::String("warning".into()))],
                    ),
                    (
                        "message".into(),
                        vec![Item::Atomic(AtomValue::String("original".into()))],
                    ),
                    ("source".into(), vec![c.datatype.clone()]),
                ]))],
            ),
        ]))))
    }
}
#[test]
fn transitive_query_and_native_rules_require_owned_bindings_and_keep_original_datatypes() {
    for query in [false, true] {
        let (g, id) = graph(query, false);
        let (mut h, root, scopes) = prepared(&g, true);
        prepare_rule(&mut h, &g, &id, &scopes);
        let calls = Arc::new(AtomicUsize::new(0));
        if !query {
            h.register_native_validator(
                native_identity(),
                Rule {
                    calls: calls.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let selected = h
            .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
            .unwrap();
        assert_eq!(selected.site_count(&h).unwrap(), 3);
        assert!(h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &BTreeMap::new(),
                limits()
            )
            .is_err());
        assert!(h
            .link_datatype_module_rule(&g, &id, root, registration(query))
            .is_err());
        let owner = Arc::new(
            h.link_datatype_module_rule(&g, &id, scopes[&id], registration(query))
                .unwrap(),
        );
        let bindings = BTreeMap::from([(id.clone(), owner)]);
        let compiled = h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &bindings,
                limits(),
            )
            .unwrap();
        let root_id = *g.module("app").unwrap().roots().first().unwrap();
        for (value, accepted) in [("a", false), ("same", true)] {
            let mut data = input(1);
            data.value[0] = Item::Atomic(AtomValue::String(value.into()));
            let r = compiled
                .validate(&h, root_id, &data, Default::default())
                .unwrap();
            assert_eq!(r.accepted, Some(accepted));
            assert!(Arc::ptr_eq(
                r.completed[0].datatype.document(),
                g.module("core").unwrap().capture().document()
            ));
            assert!(Arc::ptr_eq(
                r.completed[0].behavior.document(),
                g.leaf_capture(&id).unwrap().document()
            ));
            assert_eq!(r.completed[0].result.diagnostics[0].code, "module.note");
        }
        assert_eq!(calls.load(Ordering::SeqCst), if query { 0 } else { 2 });
        bindings[&id].retire();
        assert!(!compiled.is_live(&h));
        assert!(compiled
            .validate(&h, root_id, &input(1), Default::default())
            .is_err());
    }
}
#[test]
fn unused_exported_rule_does_not_require_an_owner_or_a_prepared_expression() {
    for query in [false, true] {
        let (g, _) = graph(query, true);
        let (mut h, root, scopes) = prepared(&g, true);
        let selected = h
            .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
            .unwrap();
        assert_eq!(selected.site_count(&h).unwrap(), 2);
        let compiled = h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &BTreeMap::new(),
                limits(),
            )
            .unwrap();
        assert_eq!(
            compiled
                .validate(
                    &h,
                    *g.module("app").unwrap().roots().first().unwrap(),
                    &input(1),
                    Default::default()
                )
                .unwrap()
                .accepted,
            Some(true)
        );
    }
}
#[test]
fn callback_retirement_stop_and_reentry_obey_the_shared_owner_guards() {
    for action in [1, 2, 3] {
        let (g, id) = graph(false, false);
        let (mut h, root, scopes) = prepared(&g, true);
        prepare_rule(&mut h, &g, &id, &scopes);
        let action = Arc::new(AtomicUsize::new(action));
        let calls = Arc::new(AtomicUsize::new(0));
        let owner = Arc::new(Mutex::new(None));
        let reentry = Reentry::default();
        h.register_native_validator(
            native_identity(),
            Rule {
                calls: calls.clone(),
                action: action.clone(),
                owner: owner.clone(),
                reentry: reentry.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let binding = Arc::new(
            h.link_datatype_module_rule(&g, &id, scopes[&id], registration(false))
                .unwrap(),
        );
        *owner.lock().unwrap() = Some(Arc::downgrade(&binding));
        let bindings = BTreeMap::from([(id.clone(), binding)]);
        let compiled = Arc::new(
            h.compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &bindings,
                limits(),
            )
            .unwrap(),
        );
        let h = Arc::new(h);
        let root_id = *g.module("app").unwrap().roots().first().unwrap();
        *reentry.lock().unwrap() = Some((Arc::downgrade(&h), Arc::downgrade(&compiled), root_id));
        let result = compiled.validate(&h, root_id, &input(1), Default::default());
        if action.load(Ordering::SeqCst) == 3 {
            assert_eq!(result.unwrap().accepted, Some(true));
            assert!(compiled.is_live(&h));
        } else {
            assert!(result.is_err());
            assert!(!compiled.is_live(&h));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn weak_owners_and_host_retirement_release_callbacks_and_reject_foreign_bindings() {
    for query in [false, true] {
        let (g, id) = graph(query, false);
        let (mut h, root, scopes) = prepared(&g, true);
        prepare_rule(&mut h, &g, &id, &scopes);
        let marker = Arc::new(());
        let weak = Arc::downgrade(&marker);
        if !query {
            h.register_native_validator(
                native_identity(),
                Rule {
                    marker: marker.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        drop(marker);
        let binding = Arc::new(
            h.link_datatype_module_rule(&g, &id, scopes[&id], registration(query))
                .unwrap(),
        );
        let bindings = BTreeMap::from([(id.clone(), binding)]);
        let compiled = h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &bindings,
                limits(),
            )
            .unwrap();
        let (copy, copy_id) = graph(query, false);
        assert_eq!(copy_id, id);
        let (mut foreign, other_root, other_scopes) = prepared(&copy, true);
        prepare_rule(&mut foreign, &copy, &id, &other_scopes);
        assert!(foreign
            .compile_datatype_modules(
                copy.clone(),
                other_root,
                &other_scopes,
                &[],
                &implementations(&copy),
                &bindings,
                limits()
            )
            .is_err());
        assert!(compiled.is_live(&h));
        if !query {
            h.remove_native_implementation("urn:test:validate").unwrap();
            assert!(!compiled.is_live(&h));
            assert!(weak.upgrade().is_none());
        }
        drop(h);
        assert!(weak.upgrade().is_none());
        drop(compiled);
        drop(bindings);
    }
}
#[test]
fn transitive_item_rules_share_preflight_and_query_diagnostic_work_limits() {
    for query in [false, true] {
        let (g, id) = graph_with(query, false, true);
        let (mut h, root, scopes) = prepared(&g, true);
        prepare_rule(&mut h, &g, &id, &scopes);
        let calls = Arc::new(AtomicUsize::new(0));
        if !query {
            h.register_native_validator(
                native_identity(),
                Rule {
                    calls: calls.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        if query {
            let mut ctx = StandaloneExpressionContext::default();
            ctx.scope = QueryContextScope(73);
            ctx.scope_policy = ctx.scope_policy.with_queue_size(1);
            h.set_context(scopes[&id], Some(ctx));
        }
        let bindings = BTreeMap::from([(
            id.clone(),
            Arc::new(
                h.link_datatype_module_rule(&g, &id, scopes[&id], registration(query))
                    .unwrap(),
            ),
        )]);
        let compiled = h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &bindings,
                limits(),
            )
            .unwrap();
        let root_id = *g.module("app").unwrap().roots().first().unwrap();
        let mut low = cem_ql::datatype_validation::ValidationLimits::default();
        low.max_rules = 1;
        let report = compiled.validate(&h, root_id, &input(2), low).unwrap();
        assert_eq!(report.accepted, None);
        assert!(report.completed.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let mut low = cem_ql::datatype_validation::ValidationLimits::default();
        low.max_diagnostics = 1;
        if query {
            let outcome = compiled
                .validate(&h, root_id, &input(128), Default::default())
                .unwrap();
            assert_eq!(outcome.accepted, None);
            assert!(outcome.completed.len() < 128);
            let cem_ql::datatype_validation::ValidationStopReason::Result(
                cem_ql::datatype_results::DatatypeResultError::Execution(stream),
            ) = outcome.stopped.unwrap().reason
            else {
                panic!("expected shared query work exhaustion")
            };
            assert_eq!(
                stream.error,
                Some(cem_ql::eval::EvalError::BudgetExceeded(
                    cem_ql::eval::BudgetAxis::ValidatorWork
                ))
            );
        }
        assert_eq!(
            compiled
                .validate(&h, root_id, &input(2), low)
                .unwrap()
                .accepted,
            None
        );
        assert_eq!(
            compiled
                .validate(&h, root_id, &input(1), Default::default())
                .unwrap()
                .accepted,
            Some(true)
        );
    }
}
