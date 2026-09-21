//! CEMT-CONSTRUCTOR-REFERENCES: first-class references keep their shape across transport.
use cem_ml::value::{artifact::CemValueArtifactLimits, CemReference};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{output::output_nodes, portable::*, values::reference, AtomValue, Item, ItemStream},
    render::RenderPlanNode,
};

fn context(values: ItemStream) -> EvaluationContext {
    let mut context = EvaluationContext::default();
    context.policy_bindings.insert("values".into(), values);
    context
}

fn run(query: &str, values: ItemStream) -> ItemStream {
    let context = context(values);
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    evaluate(&compiled, &context)
}

fn ok(query: &str, values: ItemStream) -> ItemStream {
    let result = run(query, values);
    assert!(result.error.is_none(), "{query}: {result:?}");
    result
}

fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view().unwrap().field(name).unwrap_or_default()
}

fn kind(item: &Item) -> String {
    let Some(AtomValue::String(kind)) = field(item, "kind")[0].atom() else {
        panic!()
    };
    kind
}

fn round_trip(values: &ItemStream) -> ItemStream {
    let limits = CemValueArtifactLimits::default();
    decode_values(&encode_values(values, &limits).unwrap(), &limits).unwrap()
}

fn source() -> Item {
    ok(r#"data:read("<source xmlns:p=\"urn:names\"><p:name rank=\"2\">ivy<em>saur</em></p:name></source>", "xml").root.children.children"#,
        ItemStream::empty()).items.remove(0)
}

fn representations(targets: Vec<Item>) -> Vec<ItemStream> {
    let occurrence = RenderPlanNode::Reference {
        reference: CemReference::new(targets.clone()),
        source_map: Default::default(),
    };
    let parent = output_nodes(vec![RenderPlanNode::Element {
        tag: "output".into(),
        namespace: None,
        qualified_name: None,
        attributes: vec![],
        children: vec![occurrence.clone()],
        source_map: Default::default(),
    }]);
    let direct = [
        ItemStream::once(reference(targets)),
        output_nodes(vec![occurrence]),
        ItemStream::from_items(field(&parent.items[0], "children")),
    ];
    direct
        .iter()
        .cloned()
        .chain(direct.iter().map(round_trip))
        .collect()
}

#[test]
fn cloning_references_preserves_kind_aliases_and_detached_ownership() {
    let name = source();
    for values in representations(vec![name.clone(), name]) {
        let original = &values.items[0];
        let old_targets = field(original, "targets");
        let old_parent = ok("dom:parent(seq:first(values.targets)).name", values.clone());
        let cloned = ok("dom:clone(values)", values.clone());
        assert_eq!(cloned.items.len(), 1);
        let copy = &cloned.items[0];
        assert_eq!(kind(copy), "reference");
        assert_ne!(copy.identity(), original.identity());
        let occurrence = |item: &Item| {
            field(item, "occurrence").first().and_then(Item::atom) == Some(AtomValue::Boolean(true))
        };
        assert_eq!(occurrence(copy), occurrence(original));
        let targets = field(copy, "targets");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].identity(), targets[1].identity());
        assert_ne!(targets[0].identity(), old_targets[0].identity());
        assert_eq!(targets[0].source_map(), old_targets[0].source_map());
        let origin = targets[0].view().unwrap().provenance().unwrap();
        let old_origin = old_targets[0].view().unwrap().provenance().unwrap();
        assert_eq!(origin.source_key, old_origin.source_key);
        assert_eq!(origin.line_number, old_origin.line_number);
        assert!(ok("dom:parent(values)", cloned.clone()).items.is_empty());
        assert!(ok("dom:parent(seq:first(values.targets))", cloned.clone())
            .items
            .is_empty());
        let children = field(&targets[0], "children");
        assert_eq!(
            ok("dom:parent(values)", ItemStream::once(children[0].clone())).items[0].identity(),
            targets[0].identity()
        );
        assert_eq!(
            ok("dom:text(values)", cloned.clone()).items[0].atom(),
            Some(AtomValue::String("ivysaurivysaur".into()))
        );
        assert_eq!(
            ok("dom:parent(seq:first(values.targets)).name", values).items,
            old_parent.items
        );
        assert_eq!(
            ok("dom:text(values)", round_trip(&cloned)).items[0].atom(),
            Some(AtomValue::String("ivysaurivysaur".into()))
        );
    }
}

#[test]
fn element_shells_require_explicit_target_selection() {
    let name = source();
    for values in representations(vec![name.clone(), name]) {
        let denied = run("dom:element(values)", values.clone());
        assert!(denied.error.is_some());
        assert!(denied.items.is_empty());
        assert!(denied
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.type_error"));
        let shells = ok("dom:element(values.targets)", values.clone());
        assert_eq!(shells.items.len(), 2);
        assert_ne!(shells.items[0].identity(), shells.items[1].identity());
        for shell in &shells.items {
            assert_eq!(kind(shell), "element");
            assert_eq!(
                field(shell, "namespace")[0].atom(),
                Some(AtomValue::String("urn:names".into()))
            );
            assert!(field(shell, "children").is_empty());
            assert!(field(shell, "attributes").is_empty());
        }
        let copies = ok("dom:clone(values.targets)", values);
        assert_eq!(copies.items.len(), 2);
        assert_eq!(kind(&copies.items[0]), "element");
        assert_ne!(copies.items[0].identity(), copies.items[1].identity());
    }
}

#[test]
fn empty_nested_and_scalar_references_keep_their_selected_structure() {
    for values in representations(vec![]) {
        let cloned = ok("dom:clone(values)", values.clone());
        assert_eq!(cloned.items.len(), 1);
        assert_eq!(kind(&cloned.items[0]), "reference");
        assert!(field(&cloned.items[0], "targets").is_empty());
        assert!(ok("dom:element(values.targets)", values.clone())
            .items
            .is_empty());
        assert!(run("dom:element(values)", values).error.is_some());
    }
    let inner = reference(vec![source(), Item::Atomic(AtomValue::Integer(2))]);
    for values in representations(vec![inner.clone(), inner]) {
        let cloned = ok("dom:clone(values)", values.clone());
        assert_eq!(cloned.items.len(), 1);
        let targets = field(&cloned.items[0], "targets");
        assert_eq!(targets.len(), 2);
        assert_eq!(kind(&targets[0]), "reference");
        assert_eq!(targets[0].identity(), targets[1].identity());
        let contents = field(&targets[0], "targets");
        assert_eq!(contents[1].atom(), Some(AtomValue::Integer(2)));
        assert_eq!(
            ok("dom:text(values)", cloned).items[0].atom(),
            Some(AtomValue::String("ivysaur2ivysaur2".into()))
        );
        let rejected = run("dom:element(values.targets.targets)", values);
        assert!(rejected.error.is_some());
        assert!(
            rejected.items.is_empty(),
            "no shell prefix escapes invalid scalar input"
        );
    }
}

#[test]
fn references_to_each_import_format_share_the_constructor_contract() {
    for (format, text) in [
        ("xml", "<r><name>ivy</name></r>"),
        ("json", r#"{"name":"ivy"}"#),
        ("yaml", "name: ivy\n"),
        ("csv", "name\nivy\n"),
    ] {
        let target = ok(
            &format!(r#"data:read(dom:text(values), "{format}").root.children"#),
            ItemStream::once(Item::Atomic(AtomValue::String(text.into()))),
        )
        .items
        .remove(0);
        for values in representations(vec![target.clone(), target.clone()]) {
            let cloned = ok("dom:clone(values)", values.clone());
            assert_eq!(cloned.items.len(), 1, "{format}");
            assert_eq!(kind(&cloned.items[0]), "reference");
            assert_eq!(
                ok("dom:text(values)", cloned).items,
                ok("dom:text(values)", values.clone()).items
            );
            assert_eq!(ok("dom:element(values.targets)", values).items.len(), 2);
        }
    }
}

#[derive(Debug)]
struct ScopedText;
impl cem_ql::eval::QueryItemView for ScopedText {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "test.clone-scoped-text"
    }
    fn identity(&self) -> String {
        "scoped".into()
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Node
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let value = match name {
            "kind" => "text",
            "value" => "restricted",
            _ => return None,
        };
        Some(vec![Item::Atomic(AtomValue::String(value.into()))])
    }
    fn parent(
        &self,
        scope: cem_ql::eval::QueryContextScope,
    ) -> Result<Option<Item>, cem_ql::eval::QueryNodeAccessError> {
        if scope.0 == 7 {
            Ok(None)
        } else {
            Err(cem_ql::eval::QueryNodeAccessError::ScopeViolation)
        }
    }
}

#[test]
fn clone_detaches_ancestry_without_bypassing_source_access_checks() {
    for values in [
        ItemStream::once(reference(vec![Item::native(ScopedText)])),
        output_nodes(vec![RenderPlanNode::Reference {
            reference: CemReference::new(vec![Item::native(ScopedText)]),
            source_map: Default::default(),
        }]),
    ] {
        let denied = run("dom:clone((42, values))", values.clone());
        assert!(denied.error.is_some(), "{denied:?}");
        assert!(denied.items.is_empty());
        let mut context = context(values);
        context.scope = cem_ql::eval::QueryContextScope(7);
        let query = compile(
            "dom:clone(values)",
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let allowed = evaluate(&query, &context);
        assert!(allowed.error.is_none(), "{allowed:?}");
        assert_eq!(kind(&allowed.items[0]), "reference");
        assert_eq!(
            ok("dom:text(values)", allowed).items[0].atom(),
            Some(AtomValue::String("restricted".into()))
        );
    }
}

#[test]
fn cloning_references_obeys_child_limits_and_cancellation_without_partial_output() {
    use cem_ml::{
        operation_control::{
            ExecutionScopeKind, ExecutionScopeRegistration, OperationControl,
            ROOT_EXECUTION_SCOPE_ID,
        },
        scheduler::ScopePolicy,
    };
    use cem_ql::api::evaluate_with_control;
    let mut nested = source();
    for _ in 0..20 {
        // Two targets retain a layer instead of the constructor's single-reference identity shortcut.
        nested = reference(vec![nested, Item::Atomic(AtomValue::Integer(0))]);
    }
    let values = ItemStream::once(nested);
    for values in [values.clone(), round_trip(&values)] {
        let context = context(values);
        let query = compile(
            r#"try { dom:clone((42, values)) } catch (code, message) { "caught" }"#,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let control = OperationControl::default();
        for (name, policy) in [
            ("depth", ScopePolicy::host_root().with_stack_depth(8)),
            ("memory", ScopePolicy::host_root().with_memory_bytes(32)),
            ("items", ScopePolicy::host_root().with_queue_size(4)),
            ("cancelled", ScopePolicy::host_root()),
        ] {
            let child = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Template,
                        name,
                        policy,
                    ),
                )
                .unwrap();
            if name == "cancelled" {
                control.cancel_scope(child, None, None).unwrap();
            }
            let result = evaluate_with_control(&query, &context, &control, child);
            assert!(result.error.is_some(), "{name}: {result:?}");
            assert!(result.items.is_empty());
            assert_eq!(control.memory_charged(child).unwrap(), 0);
            assert!(control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok());
        }
        let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(result.items[0].atom(), Some(AtomValue::Integer(42)));
        assert_eq!(kind(&result.items[1]), "reference");
    }
}
