use super::*;
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_compilation::compile_datatypes_with_runtime,
    datatype_enumeration::*,
};

#[derive(Debug)]
struct Interpret;
impl NativeConstantInterpreter for Interpret {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        let candidate = cem_ql::eval::retained_cem_node(call.candidate).unwrap();
        assert_eq!(candidate.node_id(), call.token.source.node_id());
        assert!(Arc::ptr_eq(
            candidate.owner().ast_owner(),
            call.token.source.document()
        ));
        ConstantExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
            diagnostics: vec![],
        }
    }
}
#[derive(Debug)]
struct Equal;
impl NativeScalarEquality for Equal {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        EqualityExecution::Complete {
            equal: call.left == call.right,
            diagnostics: vec![],
        }
    }
}
fn registry(sources: &[DatatypeSource]) -> DatatypeImplementations {
    let mut regs = DatatypeImplementations::default();
    regs.register(implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    ))
    .unwrap();
    regs.select_equality(
        sources[0].clone(),
        EqualityBinding::Ready(
            RegisteredScalarEquality::new(
                sources[0].clone(),
                "retained-equality",
                ScalarRepresentation::String,
                Equal,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    regs.select_constant_interpreter(
        sources[0].clone(),
        ConstantBinding::Ready(
            RegisteredConstantInterpreter::new(
                sources[0].clone(),
                "retained-interpreter",
                ScalarRepresentation::String,
                Interpret,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    regs
}
fn run(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    roots: &[DatatypeSource],
    regs: &DatatypeImplementations,
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    run_with(
        host,
        roots,
        regs,
        &OperationControl::default(),
        Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
}
fn run_with(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    roots: &[DatatypeSource],
    regs: &DatatypeImplementations,
    control: &OperationControl,
    limits: ConstantPreparationLimits,
    traversal: ReferenceTraversalLimits,
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    compile_datatypes_with_runtime(
        roots[0].declaration().document().clone(),
        roots,
        host,
        regs,
        &Default::default(),
        traversal,
        &ValidationRuntime {
            control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    )
}
fn accepted(
    result: &cem_ml::schema::datatype_contracts::DatatypeCompilation,
    source: &DatatypeSource,
    value: &str,
) -> Option<bool> {
    validate_descriptor(
        compiled(result, source),
        vec![Item::Atomic(AtomValue::String(value.into()))],
    )
    .accepted
}

#[test]
fn retained_constants_preserve_full_empty_unicode_values_and_original_attributes() {
    let (mut host, sources) = types_fixture(
        r#"{type @name=sample @kind=scalar | {constant @value=" In progress "} {constant @value=""} {constant @value="日本 語"}}"#,
    );
    let result = run(&mut host, &sources, &registry(&sources));
    assert!(result.is_ready(), "{:?}", result.issues);
    for value in [" In progress ", "", "日本 語"] {
        assert_eq!(accepted(&result, &sources[0], value), Some(true));
    }
    for value in ["In", "progress", "In progress", "日本"] {
        assert_eq!(accepted(&result, &sources[0], value), Some(false));
    }
    let constants = compiled(&result, &sources[0]).enumerations()[0].constants();
    assert_eq!(constants.len(), 3);
    for constant in constants {
        assert_eq!(constant.token.span, 0..constant.token.lexical.len());
        assert!(
            matches!(constant.token.source.node(), CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "value")
        );
        assert!(Arc::ptr_eq(
            constant.token.source.document(),
            sources[0].declaration().document()
        ));
    }
}

#[test]
fn retained_constants_reject_mixed_forms_and_malformed_declarations() {
    for (body, code) in [
        (
            r#"@values="old" | {constant @value="new"}"#,
            "datatype-mixed-vocabulary",
        ),
        ("| {constant}", "datatype-constant-value-required"),
        ("| {constant @value}", "datatype-constant-require-literal"),
        (
            "| {constant @value={#value}}",
            "datatype-constant-require-literal",
        ),
        (
            "| {constant @value={$value}}",
            "datatype-constant-require-literal",
        ),
        (
            "| {constant @value=x @extra=y}",
            "unsupported-datatype-constant-field",
        ),
        (
            "| {constant @value=x @value=y}",
            "duplicate-datatype-constant-field",
        ),
        (
            "| {constant @value=x | text}",
            "unsupported-datatype-constant-child",
        ),
        (
            "| {constant @value=x | {constant @value=y}}",
            "unsupported-datatype-constant-child",
        ),
        ("| {value @value=x}", "unsupported-datatype-child"),
        ("| {$ value }", "unsupported-datatype-child"),
    ] {
        let (mut host, sources) =
            types_fixture(&format!("{{type @name=sample @kind=scalar {body}}}"));
        let result = run(&mut host, &sources, &registry(&sources));
        assert!(!result.is_ready(), "{body}");
        assert!(
            result.issues.iter().any(|e| e.code == code),
            "{body}: {:?}",
            result.issues
        );
    }
}

#[test]
fn retained_constants_intersect_inherited_tokens_and_do_not_reinterpret_them() {
    for (base, derived, expected) in [
        (
            r#"@values="In progress""#,
            r#"| {constant @value="In progress"}"#,
            false,
        ),
        (
            r#"| {constant @value="In progress"} {constant @value="done"}"#,
            r#"| {constant @value="In progress"}"#,
            true,
        ),
        (r#"| {constant @value="done"}"#, "@values=done", true),
    ] {
        let (mut host, sources) = types_fixture(&format!(
            "{{type @name=base @kind=scalar {base}}} {{type @name=derived @base=base {derived}}}"
        ));
        let result = run(&mut host, &[sources[1].clone()], &registry(&sources));
        assert_eq!(result.is_ready(), expected, "{:?}", result.issues);
        if expected {
            let descriptor = compiled(&result, &sources[1]);
            assert_eq!(descriptor.enumerations().len(), 2);
            assert_eq!(
                descriptor.enumerations()[0]
                    .source()
                    .declaration()
                    .identity(),
                sources[0].declaration().identity()
            );
            assert_eq!(
                descriptor.enumerations()[0]
                    .interpreter()
                    .source
                    .declaration()
                    .identity(),
                sources[0].declaration().identity()
            );
        } else {
            assert_eq!(result.issues[0].code, "datatype-constant-contract-rejected");
        }
    }
}

#[test]
fn retained_constants_share_count_text_validation_and_operation_limits() {
    let text = r#"{type @name=sample @kind=scalar | {constant @value="one two"} {constant @value="three"}}"#;
    for (limits, code) in [
        (
            ConstantPreparationLimits {
                max_constants: 1,
                ..Default::default()
            },
            "datatype-constant-count-limit",
        ),
        (
            ConstantPreparationLimits {
                max_lexical_bytes: 8,
                ..Default::default()
            },
            "datatype-constant-byte-limit",
        ),
        (
            ConstantPreparationLimits {
                validation: ValidationLimits {
                    max_input_values: 3,
                    ..Default::default()
                },
                ..Default::default()
            },
            "datatype-constant-value-limit",
        ),
    ] {
        let (mut host, sources) = types_fixture(text);
        let result = run_with(
            &mut host,
            &sources,
            &registry(&sources),
            &OperationControl::default(),
            limits,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(!result.is_ready());
        assert_eq!(result.issues[0].code, code);
    }
    let (mut host, sources) = types_fixture(text);
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(!run_with(
        &mut host,
        &sources,
        &registry(&sources),
        &control,
        Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap()
    )
    .is_ready());
}

fn reference_fixture(
    values: Vec<Item>,
    ready: bool,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
    cem_ql::schema_references::DeclarationScope,
) {
    let (original, sources) = types_fixture("{type @name=sample @kind=scalar | {#chosen}}");
    let tree = cem_ml::schema::declaration_references::SchemaDeclarationHost::input_source_tree(
        &original,
        sources[0].declaration(),
    )
    .unwrap();
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(
        tree,
        if ready {
            Some(StandaloneExpressionContext::default().with_binding(
                "chosen",
                StandaloneExpressionBinding::any(ItemStream::from_items(values)),
            ))
        } else {
            None
        },
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    (host, sources, scope)
}
#[test]
fn retained_constant_references_require_original_nodes_and_preserve_readiness() {
    let foreign = source("{schema | original text}");
    let retained = native(&node(&foreign, "schema"));
    let tree = cem_ql::eval::retained_cem_node(&retained)
        .unwrap()
        .owner()
        .clone();
    let text_id = tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Text { node_id, data, .. } if !data.trim().is_empty() => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let text = RetainedCemNode::new(tree.clone(), text_id)
        .unwrap()
        .query_item();
    let (mut host, sources, from) = reference_fixture(vec![text], true);
    let to = host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.allow_scope_crossing(from, to);
    let result = run(&mut host, &sources, &registry(&sources));
    assert_eq!(
        result.issues[0].code,
        "datatype-constant-retained-target-required"
    );
    assert_eq!(
        result.issues[0].state,
        cem_ml::schema::datatype_contracts::DatatypeIssueState::Invalid
    );

    for values in [
        vec![],
        vec![Item::Atomic(AtomValue::String("value".into()))],
        vec![query("{ value: \"value\" }").items[0].clone()],
    ] {
        let (mut host, sources, _) = reference_fixture(values, true);
        let result = run(&mut host, &sources, &registry(&sources));
        assert!(!result.is_ready(), "{:?}", result.issues);
    }
    let (mut host, sources, _) = reference_fixture(vec![], false);
    let result = run(&mut host, &sources, &registry(&sources));
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|e| e.state == cem_ml::schema::datatype_contracts::DatatypeIssueState::Pending));
}

#[test]
fn retained_constant_references_preserve_cross_owner_order_duplicates_and_grants() {
    let foreign =
        source(r#"{schema | {constant @value="second value"} {constant @value="first value"}}"#);
    let tree = cem_ql::eval::retained_cem_node(&native(&node(&foreign, "constant")))
        .unwrap()
        .owner()
        .clone();
    let nodes: Vec<_> = tree
        .ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "constant" => Some(
                RetainedCemNode::new(tree.clone(), *node_id)
                    .unwrap()
                    .query_item(),
            ),
            _ => None,
        })
        .collect();
    let (mut host, sources, from) = reference_fixture(
        vec![nodes[1].clone(), nodes[0].clone(), nodes[1].clone()],
        true,
    );
    let to = host.register_scope(
        tree.clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let regs = registry(&sources);
    let denied = run(&mut host, &sources, &regs);
    assert!(!denied.is_ready());
    assert!(denied.reference_issues.iter().any(|e| e.kind
        == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::ScopeDenied));
    assert!(host.allow_scope_crossing(from, to));
    let result = run(&mut host, &sources, &regs);
    assert!(result.is_ready(), "{:?}", result.issues);
    let constants = compiled(&result, &sources[0]).enumerations()[0].constants();
    assert_eq!(
        constants.iter().map(|c| c.token.text()).collect::<Vec<_>>(),
        ["first value", "second value", "first value"]
    );
    assert_eq!(
        constants[0].declaration.as_ref().unwrap().identity(),
        constants[2].declaration.as_ref().unwrap().identity()
    );
    for constant in constants {
        assert!(Arc::ptr_eq(
            constant.token.source.document(),
            tree.ast_owner()
        ));
        assert_eq!(constant.token.form, ConstantForm::RetainedLiteral);
    }
    drop(host);
    drop(tree);
    assert_eq!(accepted(&result, &sources[0], "first value"), Some(true));
}

#[test]
fn retained_constant_reference_cycles_depth_and_scope_work_never_activate() {
    let foreign = source("{schema | {#next} {constant @value=x}}");
    let target = native(&node(&foreign, "constant"));
    let tree = cem_ql::eval::retained_cem_node(&target)
        .unwrap()
        .owner()
        .clone();
    let next = tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => Some(
                RetainedCemNode::new(tree.clone(), *node_id)
                    .unwrap()
                    .query_item(),
            ),
            _ => None,
        })
        .unwrap();
    let (mut host, sources, from) = reference_fixture(vec![next], true);
    let to = host.register_scope(
        tree,
        Some(StandaloneExpressionContext::default().with_binding(
            "next",
            StandaloneExpressionBinding::any(ItemStream::once(target)),
        )),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.allow_scope_crossing(from, to);
    let result = run_with(
        &mut host,
        &sources,
        &registry(&sources),
        &OperationControl::default(),
        Default::default(),
        ReferenceTraversalLimits {
            max_depth: 1,
            max_work: 100,
        },
    );
    assert!(!result.is_ready());
    assert!(result
        .reference_issues
        .iter()
        .any(|e| e.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::DepthLimit));
    assert!(run(&mut host, &sources, &registry(&sources)).is_ready());

    let (mut host, sources, scope) = reference_fixture(vec![], true);
    let reference = sources[0].plan().constant_slots[0].clone();
    let tree = cem_ml::schema::declaration_references::SchemaDeclarationHost::input_source_tree(
        &host, &reference,
    )
    .unwrap();
    let cycle = RetainedCemNode::new(tree.clone(), reference.node_id())
        .unwrap()
        .query_item();
    host.set_context(
        scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "chosen",
            StandaloneExpressionBinding::any(ItemStream::once(cycle)),
        )),
    );
    let result = run(&mut host, &sources, &registry(&sources));
    assert!(!result.is_ready());
    assert!(result.reference_issues.iter().any(
        |e| e.kind == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::Cycle
    ));
    // A reference to a type is not a request to recursively collect its constants.
    let wrong = RetainedCemNode::new(tree, sources[0].declaration().node_id())
        .unwrap()
        .query_item();
    host.set_context(
        scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "chosen",
            StandaloneExpressionBinding::any(ItemStream::once(wrong)),
        )),
    );
    assert!(!run(&mut host, &sources, &registry(&sources)).is_ready());

    let foreign = source("{schema | {constant @value=x}}");
    let target = native(&node(&foreign, "constant"));
    let tree = cem_ql::eval::retained_cem_node(&target)
        .unwrap()
        .owner()
        .clone();
    let (mut host, sources, from) = reference_fixture(vec![target.clone(); 4], true);
    let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
    policy.limits.max_work = 2;
    let to = host.register_scope(tree, Some(Default::default()), policy);
    host.allow_scope_crossing(from, to);
    let result = run(&mut host, &sources, &registry(&sources));
    assert!(!result.is_ready());
    assert!(result
        .reference_issues
        .iter()
        .any(|e| e.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit));
    let (mut host, sources) =
        types_fixture("{type @name=sample @kind=scalar | {constant @value=x}}");
    let result = run_with(
        &mut host,
        &sources,
        &registry(&sources),
        &OperationControl::default(),
        Default::default(),
        ReferenceTraversalLimits {
            max_depth: 1,
            max_work: 6,
        },
    );
    assert!(!result.is_ready());
}

#[derive(Debug)]
struct Output(ConstantExecution, bool);
impl NativeConstantInterpreter for Output {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        if self.1 {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.0.clone()
    }
}
#[test]
fn retained_constants_require_one_checked_scalar_and_recheck_control_after_callbacks() {
    for (output, cancel, ready) in [
        (
            ConstantExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::String("typed value".into()))],
                diagnostics: vec![],
            },
            false,
            true,
        ),
        (
            ConstantExecution::Prepared {
                value: vec![],
                diagnostics: vec![],
            },
            false,
            false,
        ),
        (
            ConstantExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Boolean(true))],
                diagnostics: vec![],
            },
            false,
            false,
        ),
        (
            ConstantExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::String("typed value".into()))],
                diagnostics: vec![],
            },
            true,
            false,
        ),
        (ConstantExecution::Rejected(vec![]), false, false),
        (ConstantExecution::Pending(vec![]), false, false),
        (ConstantExecution::Unavailable(vec![]), false, false),
        (ConstantExecution::Failed(vec![]), false, false),
    ] {
        let (mut host, sources) = types_fixture(
            r#"{type @name=sample @kind=scalar | {constant @value="source spelling"}}"#,
        );
        let mut regs = DatatypeImplementations::default();
        regs.register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
        regs.select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "eq",
                    ScalarRepresentation::String,
                    Equal,
                )
                .unwrap(),
            ),
        )
        .unwrap();
        regs.select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "interpret",
                    ScalarRepresentation::String,
                    Output(output, cancel),
                )
                .unwrap(),
            ),
        )
        .unwrap();
        let result = run(&mut host, &sources, &regs);
        assert_eq!(result.is_ready(), ready, "{:?}", result.issues);
        if ready {
            assert_eq!(accepted(&result, &sources[0], "typed value"), Some(true));
            assert_eq!(
                accepted(&result, &sources[0], "source spelling"),
                Some(false)
            );
        }
    }
}

#[derive(Debug)]
struct MutableView;
impl cem_ql::eval::QueryItemView for MutableView {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.typed-atomic"
    }
    fn identity(&self) -> String {
        "claimed-constant".into()
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        panic!("unadmitted view must not be atomized")
    }
}
#[test]
fn retained_constants_admit_immutable_scalar_storage_with_cumulative_output_bounds() {
    for (value, bytes, expected) in [
        (
            Item::Atomic(AtomValue::String("expanded value".into())),
            100,
            true,
        ),
        (
            Item::Atomic(AtomValue::String("expanded value".into())),
            20,
            false,
        ),
        (Item::native(MutableView), 100, false),
    ] {
        let (mut host, sources) = types_fixture(
            "{type @name=sample @kind=scalar | {constant @value=x} {constant @value=y}}",
        );
        let mut regs = DatatypeImplementations::default();
        regs.register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
        regs.select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "eq",
                    ScalarRepresentation::String,
                    Equal,
                )
                .unwrap(),
            ),
        )
        .unwrap();
        regs.select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "interpret",
                    ScalarRepresentation::String,
                    Output(
                        ConstantExecution::Prepared {
                            value: vec![value],
                            diagnostics: vec![],
                        },
                        false,
                    ),
                )
                .unwrap(),
            ),
        )
        .unwrap();
        let result = run_with(
            &mut host,
            &sources,
            &regs,
            &OperationControl::default(),
            ConstantPreparationLimits {
                max_retained_value_bytes: bytes,
                ..Default::default()
            },
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert_eq!(result.is_ready(), expected, "{:?}", result.issues);
    }
}

#[test]
fn retained_constants_use_registered_wide_integer_interpretation_and_exact_equality() {
    use cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype;
    let (mut host, sources) = types_fixture(
        r#"{type @name=sample @kind=scalar | {constant @value="123456789012345678901234567890"}}"#,
    );
    let mut regs = DatatypeImplementations::default();
    regs.register(implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::Integer),
    ))
    .unwrap();
    regs.select_equality(
        sources[0].clone(),
        EqualityBinding::Ready(
            RegisteredScalarEquality::new(
                sources[0].clone(),
                "exact-wide-value",
                ScalarRepresentation::Integer,
                Equal,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    regs.select_constant_interpreter(
        sources[0].clone(),
        ConstantBinding::Ready(
            cem_ql::datatype_shipped::constant_interpreter(
                sources[0].clone(),
                ShippedDatatype::Integer,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    let result = run(&mut host, &sources, &regs);
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = compiled(&result, &sources[0]);
    let constant = &descriptor.enumerations()[0].constants()[0];
    assert_eq!(constant.token.text(), "123456789012345678901234567890");
    assert!(constant.value.view().is_some());
    assert_eq!(
        validate_descriptor(descriptor, vec![constant.value.clone()]).accepted,
        Some(true)
    );
    assert_eq!(
        validate_descriptor(descriptor, vec![Item::Atomic(AtomValue::Integer(1))]).accepted,
        Some(false)
    );
}
