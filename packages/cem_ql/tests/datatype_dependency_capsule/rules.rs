use super::selection::limits;
use super::*;
use cem_ml::schema::{
    datatype_contracts::ItemBounds,
    datatype_validation::ScalarRepresentation,
    declaration_references::SchemaDeclarationNode,
    reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
    value_contracts::{ContractName, ValueContractSource, ValueContracts},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        NativeDatatypeValidator, RuleExecution, ValidationCall, ValidationInput,
    },
    eval::{AtomValue, Item, ItemStream, RetainedCemNode},
    selected_registry::{SelectedBuiltin, SelectedFunctionRegistry},
    structural_receiver::{
        DatatypeDependencyRuleRegistration, FreshDatatypeRuleBinding, FreshStructuralHost,
    },
};
const CONSUMER: &str = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=root @kind=scalar @rule={#selected}}}}";
fn verified(query: bool, expression: &str) -> (Arc<capsule::VerifiedDatatypeDependencies>, String) {
    verified_for(CONSUMER, query, expression)
}
fn verified_for(
    text: &str,
    query: bool,
    expression: &str,
) -> (Arc<capsule::VerifiedDatatypeDependencies>, String) {
    let (target, artifact) = rule_artifact_body(query, expression);
    let id = artifact.id().to_owned();
    let artifacts = [artifact];
    let c = capture(text);
    let b = capsule::export(
        &c,
        identity(text, "file:///app/consumer.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&c, "root")]),
        c.document()
            .nodes
            .iter()
            .filter_map(|n| match n {
                CemAstNode::Attribute {
                    node_id,
                    expanded_name,
                    value,
                    ..
                } => match expanded_name.local_name.as_str() {
                    "rule" => Some((*node_id, target.clone())),
                    "base" | "list-base" => Some((
                        *node_id,
                        capsule::DependencyTarget::LocalDatatype(declaration(
                            &c,
                            value.as_ref().unwrap(),
                        )),
                    )),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
        &artifacts,
        &OperationControl::default(),
    )
    .unwrap();
    (
        Arc::new(
            capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap(),
        ),
        id,
    )
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
    let source = ValueContractSource::new(
        node(&c, "schema"),
        ns,
        BTreeMap::from([("schema".into(), ns.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[source], Default::default()).unwrap()),
        ContractName::new(ns, "datatype-validation-result"),
        ContractName::new(ns, "datatype-diagnostic"),
    )
    .unwrap()
}
fn primitive(c: &capsule::VerifiedDatatypeDependencies) -> Vec<DatatypeImplementation> {
    use cem_ml::schema::{
        datatype_registry::{DatatypeKind, DatatypeKindSource},
        datatype_validation::ValueRepresentation,
    };
    c.sources()
        .values()
        .filter_map(|source| {
            let DatatypeKindSource::Explicit(kind) = source.plan().kind else {
                return None;
            };
            Some(DatatypeImplementation {
                source: source.clone(),
                kind,
                representation: if kind == DatatypeKind::List {
                    ValueRepresentation::List(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: ItemBounds::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
        })
        .collect()
}
fn functions() -> SelectedFunctionRegistry {
    SelectedFunctionRegistry::new([SelectedBuiltin::StringLength]).unwrap()
}
fn registration(query: bool) -> DatatypeDependencyRuleRegistration {
    if query {
        DatatypeDependencyRuleRegistration::Query {
            functions: functions(),
            adapter: adapter(),
        }
    } else {
        DatatypeDependencyRuleRegistration::NativeValidator { adapter: adapter() }
    }
}
fn input(value: &str) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String(value.into()))],
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn prepared(
    c: &Arc<capsule::VerifiedDatatypeDependencies>,
    id: &str,
    control: OperationControl,
) -> (
    FreshStructuralHost,
    cem_ql::schema_references::DeclarationScope,
    BTreeMap<String, cem_ql::schema_references::DeclarationScope>,
) {
    let mut h = FreshStructuralHost::new(&hash(b"registry"), control).unwrap();
    let (root, scopes) = register_sources(&mut h, c, id, true);
    (h, root, scopes)
}
fn register_sources(
    h: &mut FreshStructuralHost,
    c: &Arc<capsule::VerifiedDatatypeDependencies>,
    id: &str,
    crossing: bool,
) -> (
    cem_ql::schema_references::DeclarationScope,
    BTreeMap<String, cem_ql::schema_references::DeclarationScope>,
) {
    let root = h
        .register_dependency_scope(
            c,
            None,
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let module = h
        .register_dependency_scope(
            c,
            Some(id),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let behavior = node(c.dependency_capture(id).unwrap(), "behavior").node_id();
    let item = RetainedCemNode::new(c.dependency_source(id).unwrap().clone(), behavior)
        .unwrap()
        .query_item();
    let ctx = StandaloneExpressionContext::default().with_binding(
        "selected",
        StandaloneExpressionBinding::any(ItemStream::once(item)),
    );
    h.prepare_dependency_lexical_scopes(c, None, |_, _, _| {
        (Some(ctx.clone()), ReferenceScopePolicyOverrides::default())
    })
    .unwrap();
    if crossing {
        assert!(h.allow_scope_crossing(root, module));
    }
    (root, BTreeMap::from([(id.into(), module)]))
}
#[test]
fn imported_query_rules_rebind_original_datatype_and_evaluate_freshly() {
    let (c,id)=verified(true,"{accepted: str:length(value) > 2, diagnostics: {code: \"query.note\", severity: \"warning\", message: \"original\", source: datatype}}");
    let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
    let owner = Arc::new(
        h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(true))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    let compiled = h
        .compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits(),
        )
        .unwrap();
    for (value, expected) in [("a", false), ("same", true)] {
        let result = compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &input(value),
                Default::default(),
            )
            .unwrap();
        assert_eq!(result.accepted, Some(expected));
        assert_eq!(result.completed.len(), 1);
        assert!(Arc::ptr_eq(
            result.completed[0].datatype.document(),
            c.capture().document()
        ));
        assert!(Arc::ptr_eq(
            result.completed[0].behavior.document(),
            c.dependency_capture(&id).unwrap().document()
        ));
        let diagnostic = &result.completed[0].result.diagnostics[0];
        assert_eq!(diagnostic.code, "query.note");
        let expected = cem_ql::datatype_results::DiagnosticAttribution::from_node(
            &RetainedCemNode::new(c.source().clone(), declaration(c.capture(), "root"))
                .unwrap()
                .query_item(),
        );
        assert_eq!(
            (
                &diagnostic.uri,
                &diagnostic.source_map,
                diagnostic.byte_offset
            ),
            (&expected.uri, &expected.source_map, expected.byte_offset)
        );
    }
}

use cem_ql::{eval::QueryContextScope, native_registrations::ImplementationIdentity};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex, Weak,
};
fn native_identity() -> ImplementationIdentity {
    use cem_ml::schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, ResultRepresentation, ValidationSignature, ValueRepresentation,
        },
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
#[derive(Debug)]
struct NativeRule {
    calls: Arc<AtomicUsize>,
    expected_scope: u32,
    expected_queue: u32,
    owner: Arc<Mutex<Option<Weak<FreshDatatypeRuleBinding>>>>,
    action: Arc<AtomicUsize>,
    lifetime: Arc<()>,
}
impl NativeDatatypeValidator for NativeRule {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            call.runtime.query.scope,
            QueryContextScope(self.expected_scope)
        );
        assert_eq!(
            call.runtime.query.scope_policy.queue_size,
            self.expected_queue
        );
        assert!(call.runtime.query.module_resolution.is_none());
        let _ = &self.lifetime;
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
                call.runtime
                    .control
                    .cancel_root(Some("rule fixture".into()), None)
                    .unwrap();
            }
            _ => {}
        }
        RuleExecution::Complete(ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(
                    call.value[0].atom() == Some(AtomValue::String("same".into())),
                ))],
            ),
            (
                "diagnostics".into(),
                vec![Item::Record(BTreeMap::from([
                    (
                        "code".into(),
                        vec![Item::Atomic(AtomValue::String("native.note".into()))],
                    ),
                    (
                        "severity".into(),
                        vec![Item::Atomic(AtomValue::String("warning".into()))],
                    ),
                    (
                        "message".into(),
                        vec![Item::Atomic(AtomValue::String("original".into()))],
                    ),
                    ("source".into(), vec![call.datatype.clone()]),
                ]))],
            ),
        ]))))
    }
}
fn register_native(
    h: &mut FreshStructuralHost,
    calls: Arc<AtomicUsize>,
    owner: Arc<Mutex<Option<Weak<FreshDatatypeRuleBinding>>>>,
    action: Arc<AtomicUsize>,
    lifetime: Arc<()>,
) {
    h.register_native_validator(
        native_identity(),
        NativeRule {
            calls,
            expected_scope: 73,
            expected_queue: 128,
            owner,
            action,
            lifetime,
        },
    )
    .unwrap();
}
fn source_context(
    h: &mut FreshStructuralHost,
    scope: cem_ql::schema_references::DeclarationScope,
    queue: u32,
) {
    let mut context = StandaloneExpressionContext::default();
    context.scope = QueryContextScope(73);
    context.scope_policy = context.scope_policy.with_queue_size(queue);
    assert!(h.set_context(scope, Some(context)));
}
#[test]
fn imported_native_rules_use_actual_source_context_and_do_not_cache_acceptance() {
    let (c, id) = verified(false, "");
    let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
    source_context(&mut h, scopes[&id], 128);
    let calls = Arc::new(AtomicUsize::new(0));
    register_native(
        &mut h,
        calls.clone(),
        Default::default(),
        Default::default(),
        Arc::new(()),
    );
    let owner = Arc::new(
        h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    let compiled = h
        .compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits(),
        )
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "compilation must not execute the rule"
    );
    for (value, expected) in [("a", false), ("same", true)] {
        let result = compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &input(value),
                Default::default(),
            )
            .unwrap();
        assert_eq!(result.accepted, Some(expected));
        assert!(Arc::ptr_eq(
            result.completed[0].datatype.document(),
            c.capture().document()
        ));
        assert!(Arc::ptr_eq(
            result.completed[0].behavior.document(),
            c.dependency_capture(&id).unwrap().document()
        ));
        assert_eq!(
            result.completed[0].result.diagnostics[0].code,
            "native.note"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[test]
fn imported_rules_need_exact_kind_functions_implementation_and_source_owners() {
    for query in [false, true] {
        let (c, id) = verified(query, "{accepted: str:length(value)>2, diagnostics: ()}");
        let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
        assert!(h
            .link_datatype_dependency_rule(&c, &id, scopes[&id], registration(!query))
            .is_err());
        assert!(h
            .link_datatype_dependency_rule(&c, &id, root, registration(query))
            .is_err());
        if !query {
            assert!(h
                .link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
                .is_err());
            let mut wrong = native_identity();
            wrong.module_hash = hash(b"wrong module");
            h.register_native_validator(
                wrong,
                NativeRule {
                    calls: Default::default(),
                    expected_scope: 0,
                    expected_queue: 64,
                    owner: Default::default(),
                    action: Default::default(),
                    lifetime: Arc::new(()),
                },
            )
            .unwrap();
            assert!(h
                .link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
                .is_err());
            h.remove_native_implementation("urn:test:validate").unwrap();
            source_context(&mut h, scopes[&id], 128);
            register_native(
                &mut h,
                Default::default(),
                Default::default(),
                Default::default(),
                Arc::new(()),
            );
        } else {
            assert!(h
                .link_datatype_dependency_rule(
                    &c,
                    &id,
                    scopes[&id],
                    DatatypeDependencyRuleRegistration::Query {
                        functions: SelectedFunctionRegistry::new([]).unwrap(),
                        adapter: adapter()
                    }
                )
                .is_err());
        }
        let owner = Arc::new(
            h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(query))
                .unwrap(),
        );
        let rules = BTreeMap::from([(id.clone(), owner.clone())]);
        let compiled = h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits(),
            )
            .unwrap();
        let wrong = BTreeMap::from([("unknown".into(), owner.clone())]);
        assert!(h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &wrong,
                limits()
            )
            .is_err());
        let mut extra = rules.clone();
        extra.insert("unknown".into(), owner.clone());
        assert!(h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &extra,
                limits()
            )
            .is_err());
        assert!(compiled.is_live(&h));
        let (copy, copy_id) = verified(query, "{accepted: str:length(value)>2, diagnostics: ()}");
        assert_eq!(id, copy_id);
        let (mut foreign, other_root, other_scopes) =
            prepared(&copy, &copy_id, OperationControl::default());
        assert!(foreign
            .compile_datatype_dependencies_with_rules(
                copy.clone(),
                other_root,
                &other_scopes,
                &[],
                &primitive(&copy),
                &rules,
                limits()
            )
            .is_err());
        assert!(!compiled.is_live(&foreign));
        assert!(h.set_context(scopes[&id], None));
        assert!(!owner.is_live(&h));
        assert!(!compiled.is_live(&h));
        assert!(h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits()
            )
            .is_err());
    }
}
#[test]
fn rule_retirement_and_dropped_owners_disable_retained_query_and_native_snapshots() {
    for query in [false, true] {
        for retire in [false, true] {
            let (c, id) = verified(query, "{accepted: true, diagnostics: ()}");
            let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
            if !query {
                source_context(&mut h, scopes[&id], 128);
                register_native(
                    &mut h,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Arc::new(()),
                );
            }
            let owner = Arc::new(
                h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(query))
                    .unwrap(),
            );
            let weak = Arc::downgrade(&owner);
            let mut rules = BTreeMap::from([(id.clone(), owner)]);
            let compiled = h
                .compile_datatype_dependencies_with_rules(
                    c.clone(),
                    root,
                    &scopes,
                    &[],
                    &primitive(&c),
                    &rules,
                    limits(),
                )
                .unwrap();
            assert!(compiled.is_live(&h));
            if retire {
                rules[&id].retire();
            } else {
                rules.clear();
                assert!(weak.upgrade().is_none());
            }
            assert!(!compiled.is_live(&h));
            assert!(compiled
                .validate(
                    &h,
                    declaration(c.capture(), "root"),
                    &input("same"),
                    Default::default()
                )
                .is_err());
        }
    }
}
#[test]
fn callback_retirement_and_stop_cannot_publish_native_rule_success() {
    for action_code in [1, 2] {
        let (c, id) = verified(false, "");
        let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
        source_context(&mut h, scopes[&id], 128);
        let calls = Arc::new(AtomicUsize::new(0));
        let slot = Arc::new(Mutex::new(None));
        let action = Arc::new(AtomicUsize::new(action_code));
        register_native(&mut h, calls.clone(), slot.clone(), action, Arc::new(()));
        let owner = Arc::new(
            h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
                .unwrap(),
        );
        *slot.lock().unwrap() = Some(Arc::downgrade(&owner));
        let rules = BTreeMap::from([(id.clone(), owner)]);
        let compiled = h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits(),
            )
            .unwrap();
        assert!(compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &input("same"),
                Default::default()
            )
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!compiled.is_live(&h));
    }
}
#[test]
fn retained_rule_snapshots_release_native_callbacks_with_the_actual_host() {
    for remove in [false, true] {
        let (c, id) = verified(false, "");
        let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
        source_context(&mut h, scopes[&id], 128);
        let lifetime = Arc::new(());
        let weak = Arc::downgrade(&lifetime);
        register_native(
            &mut h,
            Default::default(),
            Default::default(),
            Default::default(),
            lifetime,
        );
        let owner = Arc::new(
            h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
                .unwrap(),
        );
        let rules = BTreeMap::from([(id.clone(), owner)]);
        let compiled = h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits(),
            )
            .unwrap();
        assert!(weak.upgrade().is_some());
        if remove {
            h.remove_native_implementation("urn:test:validate").unwrap();
            assert!(!compiled.is_live(&h));
            assert!(!rules[&id].is_live(&h));
        }
        drop(h);
        assert!(
            weak.upgrade().is_none(),
            "snapshot or rule owner retained a callback"
        );
        drop(compiled);
        drop(rules);
    }
}
#[test]
fn original_rule_source_policy_is_used_instead_of_consumer_policy() {
    for (consumer_queue, rule_queue, expected) in [(128, 1, None), (1, 128, Some(true))] {
        let declarations = (0..8)
            .map(|i| {
                let name = if i == 7 {
                    "root".into()
                } else {
                    format!("type{i}")
                };
                let base = if i == 0 {
                    "@kind=scalar".into()
                } else {
                    format!("@base=type{}", i - 1)
                };
                format!("{{type @name={name} {base} @rule={{#selected}}}}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:consumer | {{types | {declarations}}}}}");
        let (c, id) = verified_for(
            &text,
            true,
            r#"{accepted: str:length(value)>2, diagnostics: {code: "policy.note", severity: "warning", message: "original", source: datatype}}"#,
        );
        let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
        source_context(&mut h, root, consumer_queue);
        source_context(&mut h, scopes[&id], rule_queue);
        let owner = Arc::new(
            h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(true))
                .unwrap(),
        );
        let rules = BTreeMap::from([(id.clone(), owner)]);
        let compiled = h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits(),
            )
            .unwrap();
        let result = compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &input("same"),
                Default::default(),
            )
            .unwrap();
        assert_eq!(
            result.accepted,
            expected,
            "consumer={consumer_queue} rule={rule_queue} completed={}",
            result.completed.len()
        );
        if expected.is_none() {
            assert!(result.stopped.is_some());
        }
    }
}
#[test]
fn imported_item_rules_share_query_and_rule_budgets_through_list_inheritance() {
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=root @list-base=words} {type @name=words @kind=list @base=term} {type @name=term @kind=scalar @rule={#selected}}}}";
    let (c, id) = verified_for(
        text,
        true,
        "{accepted: str:length(value)>2, diagnostics: ()}",
    );
    let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
    source_context(&mut h, scopes[&id], 1);
    let owner = Arc::new(
        h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(true))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    let compiled = h
        .compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits(),
        )
        .unwrap();
    let root_id = declaration(c.capture(), "root");
    assert_eq!(
        compiled
            .validate(&h, root_id, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let mut many = input("same");
    many.value = vec![many.value[0].clone(); 64];
    let outcome = compiled
        .validate(&h, root_id, &many, Default::default())
        .unwrap();
    assert_eq!(
        outcome.accepted, None,
        "query work must not restart for each item"
    );
    assert!(outcome.completed.len() < 64);
    let mut low = cem_ql::datatype_validation::ValidationLimits::default();
    low.max_rules = 1;
    let stopped = compiled.validate(&h, root_id, &many, low).unwrap();
    assert!(stopped.completed.is_empty());
    assert!(stopped.stopped.is_some());
    assert_eq!(
        compiled
            .validate(&h, root_id, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}

#[derive(Debug)]
struct ReentrantRule {
    calls: Arc<AtomicUsize>,
    target: Arc<
        Mutex<
            Option<(
                Weak<FreshStructuralHost>,
                Weak<cem_ql::structural_receiver::FreshDependencyDatatypeCompilation>,
                u32,
            )>,
        >,
    >,
}
impl NativeDatatypeValidator for ReentrantRule {
    fn validate(&self, _call: ValidationCall<'_>) -> RuleExecution {
        let count = self.calls.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            let target = self.target.lock().unwrap().clone().unwrap();
            let (host, compiled) = (target.0.upgrade().unwrap(), target.1.upgrade().unwrap());
            assert!(
                compiled
                    .validate(&host, target.2, &input("same"), Default::default())
                    .is_err(),
                "reentry must refuse before executing a second callback"
            );
        }
        RuleExecution::Complete(ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(true))],
            ),
            ("diagnostics".into(), vec![]),
        ]))))
    }
}
#[test]
fn reentrant_imported_rule_validation_refuses_without_callback_replay() {
    let (c, id) = verified(false, "");
    let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
    let calls = Arc::new(AtomicUsize::new(0));
    let target = Arc::new(Mutex::new(None));
    h.register_native_validator(
        native_identity(),
        ReentrantRule {
            calls: calls.clone(),
            target: target.clone(),
        },
    )
    .unwrap();
    let owner = Arc::new(
        h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(false))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    let compiled = Arc::new(
        h.compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits(),
        )
        .unwrap(),
    );
    let h = Arc::new(h);
    let root_id = declaration(c.capture(), "root");
    *target.lock().unwrap() = Some((Arc::downgrade(&h), Arc::downgrade(&compiled), root_id));
    assert_eq!(
        compiled
            .validate(&h, root_id, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        compiled
            .validate(&h, root_id, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true),
        "entry guard must release after validation"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn linking_rules_grants_no_crossing_and_equal_source_pins_do_not_replace_same_host_owners() {
    let expr = "{accepted: str:length(value)>2, diagnostics: ()}";
    let (c, id) = verified(true, expr);
    let (copy, copy_id) = verified(true, expr);
    assert_eq!(id, copy_id);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let (root, scopes) = register_sources(&mut h, &c, &id, true);
    let (copy_root, copy_scopes) = register_sources(&mut h, &copy, &copy_id, true);
    let owner = Arc::new(
        h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(true))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    assert!(rules[&id].is_live(&h));
    assert!(h
        .compile_datatype_dependencies_with_rules(
            copy.clone(),
            copy_root,
            &copy_scopes,
            &[],
            &primitive(&copy),
            &rules,
            limits()
        )
        .is_err());
    assert!(
        rules[&id].is_live(&h),
        "failed admission cannot retire the real owner"
    );
    assert!(h
        .compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits()
        )
        .is_ok());
    let mut denied =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let (root, scopes) = register_sources(&mut denied, &c, &id, false);
    let owner = Arc::new(
        denied
            .link_datatype_dependency_rule(&c, &id, scopes[&id], registration(true))
            .unwrap(),
    );
    let rules = BTreeMap::from([(id.clone(), owner)]);
    assert!(
        denied
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits()
            )
            .is_err(),
        "linking metadata must not manufacture a directed grant"
    );
}
#[test]
fn incompatible_imported_rule_signature_keeps_original_diagnostic_owner_and_prior_snapshot() {
    for query in [false, true] {
        let (c, id) = verified(query, "{accepted: true, diagnostics: ()}");
        let (mut h, root, scopes) = prepared(&c, &id, OperationControl::default());
        if !query {
            source_context(&mut h, scopes[&id], 128);
            register_native(
                &mut h,
                Default::default(),
                Default::default(),
                Default::default(),
                Arc::new(()),
            );
        }
        let owner = Arc::new(
            h.link_datatype_dependency_rule(&c, &id, scopes[&id], registration(query))
                .unwrap(),
        );
        let rules = BTreeMap::from([(id.clone(), owner)]);
        let compiled = h
            .compile_datatype_dependencies_with_rules(
                c.clone(),
                root,
                &scopes,
                &[],
                &primitive(&c),
                &rules,
                limits(),
            )
            .unwrap();
        let mut wrong = primitive(&c);
        wrong[0].representation = cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
            ScalarRepresentation::Integer,
        );
        let Err(error) = h.compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &wrong,
            &rules,
            limits(),
        ) else {
            panic!("wrong rule signature compiled")
        };
        assert!(error
            .issues
            .iter()
            .any(|i| i.code == "validation-signature-incompatible"
                && Arc::ptr_eq(
                    i.source.document(),
                    c.dependency_capture(&id).unwrap().document()
                )));
        assert!(compiled.is_live(&h));
        assert_eq!(
            compiled
                .validate(
                    &h,
                    declaration(c.capture(), "root"),
                    &input("same"),
                    Default::default()
                )
                .unwrap()
                .accepted,
            Some(true)
        );
    }
}

#[test]
fn combined_query_and_native_rules_intersect_with_original_owners_and_shared_limits() {
    let (query_target, query_artifact) = rule_artifact_at(
        true,
        r#"{accepted: str:length(value)>2, diagnostics: {code: "query.note", severity: "warning", message: "original", source: datatype}}"#,
        "file:///app/query.cem",
    );
    let (native_target, native_artifact) = rule_artifact_at(false, "", "file:///app/native.cem");
    let (query_id, native_id) = (
        query_artifact.id().to_owned(),
        native_artifact.id().to_owned(),
    );
    let artifacts = [query_artifact, native_artifact];
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=base @kind=scalar @rule={#query}} {type @name=root @base=base @rule={#native}}}}";
    let capture = capture(text);
    let b = capsule::export(
        &capture,
        identity(text, "file:///app/consumer.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&capture, "root")]),
        BTreeMap::from([
            (
                field(&capture, "root", "base"),
                capsule::DependencyTarget::LocalDatatype(declaration(&capture, "base")),
            ),
            (field(&capture, "root", "rule"), native_target),
            (field(&capture, "base", "rule"), query_target),
        ]),
        &artifacts,
        &OperationControl::default(),
    )
    .unwrap();
    let c = Arc::new(
        capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap(),
    );
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let root = h
        .register_dependency_scope(
            &c,
            None,
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut scopes = BTreeMap::new();
    let mut ctx = StandaloneExpressionContext::default();
    for (name, id) in [("query", &query_id), ("native", &native_id)] {
        let scope = h
            .register_dependency_scope(
                &c,
                Some(id),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        scopes.insert(id.clone(), scope);
        let item = RetainedCemNode::new(
            c.dependency_source(id).unwrap().clone(),
            node(c.dependency_capture(id).unwrap(), "behavior").node_id(),
        )
        .unwrap()
        .query_item();
        ctx = ctx.with_binding(
            name,
            StandaloneExpressionBinding::any(ItemStream::once(item)),
        );
        assert!(h.allow_scope_crossing(root, scope));
    }
    h.prepare_dependency_lexical_scopes(&c, None, |_, _, _| {
        (Some(ctx.clone()), ReferenceScopePolicyOverrides::default())
    })
    .unwrap();
    source_context(&mut h, scopes[&native_id], 128);
    let calls = Arc::new(AtomicUsize::new(0));
    register_native(
        &mut h,
        calls.clone(),
        Default::default(),
        Default::default(),
        Arc::new(()),
    );
    let rules = BTreeMap::from([
        (
            query_id.clone(),
            Arc::new(
                h.link_datatype_dependency_rule(
                    &c,
                    &query_id,
                    scopes[&query_id],
                    registration(true),
                )
                .unwrap(),
            ),
        ),
        (
            native_id.clone(),
            Arc::new(
                h.link_datatype_dependency_rule(
                    &c,
                    &native_id,
                    scopes[&native_id],
                    registration(false),
                )
                .unwrap(),
            ),
        ),
    ]);
    let compiled = h
        .compile_datatype_dependencies_with_rules(
            c.clone(),
            root,
            &scopes,
            &[],
            &primitive(&c),
            &rules,
            limits(),
        )
        .unwrap();
    let id = declaration(c.capture(), "root");
    for (value, accepted) in [("a", false), ("long", false), ("same", true)] {
        let result = compiled
            .validate(&h, id, &input(value), Default::default())
            .unwrap();
        assert_eq!(result.accepted, Some(accepted));
        assert_eq!(result.completed.len(), 2);
        assert_eq!(
            result.completed[0].datatype.node_id(),
            declaration(c.capture(), "base")
        );
        assert_eq!(result.completed[1].datatype.node_id(), id);
        for (i, module) in [query_id.as_str(), native_id.as_str()].iter().enumerate() {
            assert!(Arc::ptr_eq(
                result.completed[i].behavior.document(),
                c.dependency_capture(module).unwrap().document()
            ));
            assert!(Arc::ptr_eq(
                result.completed[i].datatype.document(),
                c.capture().document()
            ));
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let mut low = cem_ql::datatype_validation::ValidationLimits::default();
    low.max_rules = 1;
    let result = compiled.validate(&h, id, &input("same"), low).unwrap();
    assert_eq!(result.accepted, None);
    assert!(result.completed.is_empty());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "rule count must be preflighted"
    );
    let mut low = cem_ql::datatype_validation::ValidationLimits::default();
    low.max_diagnostics = 1;
    assert_eq!(
        compiled
            .validate(&h, id, &input("same"), low)
            .unwrap()
            .accepted,
        None
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(compiled.is_live(&h));
}
