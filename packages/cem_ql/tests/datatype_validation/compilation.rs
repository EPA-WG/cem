use super::*;

#[path = "function_limits.rs"]
mod function_limits;
#[path = "overrides.rs"]
mod overrides;
#[path = "list_serialization.rs"]
mod list_serialization;
#[path = "preparation_evidence.rs"]
mod preparation_evidence;
#[path = "pretyped_consumer.rs"]
mod pretyped_consumer;
#[path = "whole_list.rs"]
mod whole_list;
#[path = "preparation_replacement.rs"]
mod preparation_replacement;
#[path = "facet_replacement.rs"]
mod facet_replacement;
#[path = "external_typed.rs"]
mod external_typed;
#[path = "retained_constants.rs"]
mod retained_constants;

fn descriptor_fixture(
    text: &str,
    ready: bool,
    kind: DatatypeKind,
    representation: ValueRepresentation,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    cem_ml::schema::datatype_registry::DatatypeSource,
    cem_ql::datatype_compilation::DatatypeImplementations,
    DatatypeValidationRegistry,
) {
    use cem_ml::schema::{
        datatype_registry::DatatypeRegistry, reference_policy::ReferenceScopePolicy,
    };
    use cem_ql::datatype_compilation::{
        DatatypeImplementation, DatatypeImplementations, TokenizerBinding,
    };
    let src = source(text);
    let declaration = node(&src, "type");
    let mut types = DatatypeRegistry::default();
    types
        .insert(src.schema.clone(), declaration.clone())
        .unwrap();
    let source = types.source(&src.schema, "sample").unwrap();
    let tree = RetainedCemTree::from_shared(
        src.schema.document().clone(),
        "source.cem",
        "",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.register_datatype_source(source.clone()).unwrap();
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(DatatypeImplementation {
            source: source.clone(),
            kind,
            representation,
            accepted_bases: vec![],
            bounds: Default::default(),
            tokenizer: TokenizerBinding::Absent,
            validator: Some((src.schema.clone(), node(&src, "behavior"))),
        })
        .unwrap();
    let mut validations = DatatypeValidationRegistry::default();
    if ready {
        let mut sig = signature(false);
        sig.kind = kind;
        sig.value = representation;
        let c = DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
        validations
            .register_native(
                "urn:test:validate",
                c,
                adapter(),
                None,
                Outcome(RuleExecution::Complete(query(
                    "{ accepted: true, diagnostics: () }",
                ))),
            )
            .unwrap();
    }
    (host, source, implementations, validations)
}
#[test]
fn compiled_descriptors_require_registered_validation_and_retain_source_owners() {
    use cem_ql::datatype_compilation::{compile_datatypes, ExecutableDatatype};
    for ready in [false, true] {
        let (mut host, source, implementations, validations) = descriptor_fixture(
            &declaration(false),
            ready,
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        );
        let owner = source.declaration().document().clone();
        let result = compile_datatypes(
            owner.clone(),
            &[source],
            &mut host,
            &implementations,
            &validations,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits::schema_defaults()
                .unwrap(),
        );
        assert_eq!(result.is_ready(), ready);
        assert!(result.matches_owner(&owner));
        if ready {
            let descriptor = result.contracts[0]
                .as_any()
                .downcast_ref::<ExecutableDatatype>()
                .unwrap();
            let control = OperationControl::default();
            let runtime = ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            };
            assert_eq!(
                descriptor
                    .validate(&input(), &runtime, Default::default())
                    .accepted,
                Some(true)
            );
        } else {
            assert_eq!(result.issues[0].code, "validation-capability-unavailable");
        }
    }
}

use cem_ml::schema::{
    datatype_contracts::ItemBounds,
    datatype_registry::{DatatypeRegistry, DatatypeSource},
    reference_policy::ReferenceScopePolicy,
    reference_traversal::ReferenceTraversalLimits,
};
use cem_ql::datatype_compilation::{
    compile_datatypes, BaseCompatibility, DatatypeImplementation, DatatypeImplementations,
    ExecutableDatatype, TokenizerBinding,
};
fn types_fixture(
    declarations: &str,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
) {
    let src = source(&format!(
        "{{schema @name=test @namespace=\"urn:test\" | {{types | {declarations}}} }}"
    ));
    types_fixture_source(src)
}
fn types_fixture_source(
    src: ValueContractSource,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
) {
    let mut registry = DatatypeRegistry::default();
    let mut sources = vec![];
    for n in &src.schema.document().nodes {
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            attributes,
            ..
        } = n
        {
            if expanded_name.local_name == "type" {
                let target =
                    SchemaDeclarationNode::new(src.schema.document().clone(), *node_id).unwrap();
                registry.insert(src.schema.clone(), target).unwrap();
                let name = attributes
                    .iter()
                    .find_map(|id| match src.schema.document().get(*id) {
                        Some(CemAstNode::Attribute {
                            expanded_name,
                            value,
                            ..
                        }) if expanded_name.local_name == "name" => value.as_deref(),
                        _ => None,
                    })
                    .unwrap();
                sources.push(registry.source(&src.schema, name).unwrap());
            }
        }
    }
    let tree = RetainedCemTree::from_shared(
        src.schema.document().clone(),
        "types.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    for s in &sources {
        host.register_datatype_source(s.clone()).unwrap();
        let CemAstNode::Attribute {
            value: Some(name), ..
        } = s.attribute("name").unwrap().node()
        else {
            panic!()
        };
        host.bind_literal_datatype(s.scope(), name, s.declaration().clone())
            .unwrap();
    }
    (host, sources)
}
fn implementation(
    source: &DatatypeSource,
    kind: DatatypeKind,
    representation: ValueRepresentation,
) -> DatatypeImplementation {
    DatatypeImplementation {
        source: source.clone(),
        kind,
        representation,
        accepted_bases: vec![],
        bounds: Default::default(),
        tokenizer: TokenizerBinding::Absent,
        validator: None,
    }
}
fn compiled<'a>(
    result: &'a cem_ml::schema::datatype_contracts::DatatypeCompilation,
    source: &DatatypeSource,
) -> &'a ExecutableDatatype {
    result
        .contracts
        .iter()
        .find(|c| c.source().declaration().identity() == source.declaration().identity())
        .unwrap()
        .as_any()
        .downcast_ref()
        .unwrap()
}
fn validate_descriptor(
    descriptor: &ExecutableDatatype,
    value: Vec<Item>,
) -> cem_ql::datatype_compilation::DatatypeValidation {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut request = input();
    request.value = value;
    descriptor.validate(&request, &runtime, Default::default())
}
#[test]
fn descriptor_list_bounds_and_tokenizer_are_explicit_and_do_not_convert() {
    let (mut host,sources)=types_fixture("{type @name=item @kind=scalar} {type @name=names @kind=list @base=item @min-items=0 @max-items=2}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    let mut list = implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    );
    list.bounds = ItemBounds::new(1, None).unwrap();
    list.tokenizer = TokenizerBinding::Ready(
        cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
    );
    implementations.register(list).unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let list = compiled(&result, &sources[1]);
    assert_eq!(list.bounds(), ItemBounds::new(1, Some(2)).unwrap());
    assert!(list.tokenizer().is_some());
    let empty = validate_descriptor(list, vec![]);
    assert_eq!(empty.accepted, Some(false));
    assert_eq!(
        empty.cardinality[0].source.identity(),
        sources[1].declaration().identity()
    );
    assert_eq!(
        validate_descriptor(list, query("(\"a\", \"a\")").items).accepted,
        Some(true)
    );
    assert_eq!(
        validate_descriptor(list, query("(\"a\", \"a\", \"a\")").items).accepted,
        Some(false)
    );
    assert_eq!(validate_descriptor(list, query("1").items).accepted, None);
}
#[test]
fn descriptor_node_inheritance_retains_bounds_owners_and_descendant_references() {
    let (mut host, sources) = types_fixture(
        "{type @name=nodes @kind=node @min-items=1} {type @name=derived @base=nodes @max-items=2}",
    );
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Node,
            ValueRepresentation::Nodes,
        ))
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = compiled(&result, &sources[1]);
    assert_eq!(descriptor.kind(), DatatypeKind::Node);
    use cem_ml::schema::datatype_contracts::CompiledDatatypeContract;
    assert_eq!(
        descriptor.base().unwrap().source().declaration().identity(),
        sources[0].declaration().identity()
    );
    assert_eq!(descriptor.bounds(), ItemBounds::new(1, Some(2)).unwrap());
    let target = source("{schema | {target | {#unavailable}}}");
    let target = node(&target, "target");
    let value = native(&target);
    let result = validate_descriptor(descriptor, vec![value.clone(), value.clone()]);
    assert_eq!(result.accepted, Some(true));
    assert!(Arc::ptr_eq(
        cem_ql::eval::retained_cem_node(&value)
            .unwrap()
            .owner()
            .ast_owner(),
        target.document()
    ));
    assert!(target
        .document()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
    assert_eq!(
        validate_descriptor(descriptor, vec![]).accepted,
        Some(false)
    );
    assert_eq!(
        validate_descriptor(descriptor, query("\"node\"").items).accepted,
        None
    );
}
#[test]
fn descriptor_bases_need_exact_compatibility_and_cannot_mix_nodes_with_scalars() {
    for (kind, representation, grant, ready) in [
        (
            DatatypeKind::Lexical,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
            false,
            false,
        ),
        (
            DatatypeKind::Lexical,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
            true,
            true,
        ),
        (DatatypeKind::Node, ValueRepresentation::Nodes, true, false),
    ] {
        let kind_name = if kind == DatatypeKind::Node {
            "node"
        } else {
            "lexical"
        };
        let (mut host, sources) = types_fixture(&format!(
            "{{type @name=base @kind=scalar}} {{type @name=derived @kind={kind_name} @base=base}}"
        ));
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                ValueRepresentation::Scalar(ScalarRepresentation::String),
            ))
            .unwrap();
        let mut derived = implementation(&sources[1], kind, representation);
        let validation_source = source(&declaration(false));
        let mut validations = DatatypeValidationRegistry::default();
        if kind == DatatypeKind::Lexical {
            let mut sig = signature(false);
            sig.kind = kind;
            validations
                .register_native(
                    "urn:test:validate",
                    DatatypeBehaviorContract::compile(
                        &validation_source,
                        &node(&validation_source, "behavior"),
                        sig,
                    )
                    .unwrap(),
                    adapter(),
                    None,
                    Outcome(RuleExecution::Complete(query(
                        "{ accepted: true, diagnostics: () }",
                    ))),
                )
                .unwrap();
            derived.validator = Some((
                validation_source.schema.clone(),
                node(&validation_source, "behavior"),
            ));
        }
        if grant {
            derived.accepted_bases.push(BaseCompatibility {
                kind: DatatypeKind::Scalar,
                representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
            });
        }
        implementations.register(derived).unwrap();
        let result = compile_datatypes(
            sources[0].declaration().document().clone(),
            &[sources[1].clone()],
            &mut host,
            &implementations,
            &validations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert_eq!(result.is_ready(), ready, "{:?}", result.issues);
        if !ready {
            assert_eq!(result.issues[0].code, "incompatible-datatype-base");
            assert_eq!(
                result.issues[0].related.as_ref().unwrap().identity(),
                sources[0].declaration().identity()
            );
        }
    }
}
#[test]
fn descriptors_keep_unsupported_facets_unavailable_names_and_limits_incomplete() {
    for declarations in [
        "{type @name=x @kind=node @values=one}",
        "{type @name=x @kind=node @pattern=a}",
        "{type @name=x @kind=node @min-items=-1}",
        "{type @name=x @kind=node @min-items=2 @max-items=1}",
        "{type @name=x @kind=node @unknown=a}",
        "{type @name=x @kind=node | {values | one}}",
        "{type @name=x @kind=node @base=unknown}",
        "{type @name=x @kind=node @base=x}",
    ] {
        let (mut host, sources) = types_fixture(declarations);
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Node,
                ValueRepresentation::Nodes,
            ))
            .unwrap();
        let result = compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(!result.is_ready(), "{declarations}");
        assert!(!result.issues.is_empty());
    }
    let (mut host, sources) = types_fixture("{type @name=x @kind=node}");
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &Default::default(),
        &Default::default(),
        ReferenceTraversalLimits {
            max_depth: 1,
            max_work: 1,
        },
    );
    assert!(!result.is_ready());
}

#[test]
fn authored_native_rule_requires_lifecycle_selection_and_exact_registered_target() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::ReferenceResolutionHost,
    };
    use cem_ql::api::{StandaloneExpressionBinding, StandaloneExpressionContext};
    let (mut host, source, implementations, validations) = descriptor_fixture(
        &declaration(false).replace("@kind=scalar}", "@kind=scalar @rule={#selected}}"),
        true,
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    );
    let compile = |host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost| {
        compile_datatypes(
            source.declaration().document().clone(),
            &[source.clone()],
            host,
            &implementations,
            &validations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    assert!(!compile(&mut host).is_ready());
    let behavior = source
        .declaration()
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "behavior" => {
                SchemaDeclarationNode::new(source.declaration().document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let scope = host
        .scope(&host.source_reference(source.declaration().clone()))
        .unwrap();
    host.set_context(
        scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "selected",
            StandaloneExpressionBinding::any(ItemStream::once(native(&behavior))),
        )),
    );
    let result = compile(&mut host);
    assert!(result.is_ready(), "{:?}", result.issues);
    assert_eq!(compiled(&result, &source).rules().len(), 1);
    assert_eq!(
        validate_descriptor(compiled(&result, &source), input().value).accepted,
        Some(true)
    );
    host.set_context(
        scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "selected",
            StandaloneExpressionBinding::any(ItemStream::once(native(source.declaration()))),
        )),
    );
    assert!(!compile(&mut host).is_ready());
}
#[test]
fn descriptor_node_rule_checks_the_complete_native_sequence_once() {
    let text = declaration(false)
        .replace("@kind=scalar", "@kind=node")
        .replace(
            "@name=value @type=schema:string @source=value @required=true @cardinality=one",
            "@name=value @type=schema:node @source=value @required=true @cardinality=zero-or-more",
        );
    let (mut host, source, implementations, validations) =
        descriptor_fixture(&text, true, DatatypeKind::Node, ValueRepresentation::Nodes);
    let result = compile_datatypes(
        source.declaration().document().clone(),
        &[source.clone()],
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let value = native(source.declaration());
    let result = validate_descriptor(compiled(&result, &source), vec![value.clone(), value]);
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.completed.len(), 1);
}

#[test]
fn list_preflight_covers_item_requirements_and_limits_before_execution() {
    let (mut host, sources) =
        types_fixture("{type @name=item @kind=scalar} {type @name=names @kind=list @base=item}");
    let src = source(&declaration(true));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut validations = DatatypeValidationRegistry::default();
    validations
        .register_native(
            "urn:test:validate",
            contract(&src, true),
            adapter(),
            None,
            Check {
                calls: calls.clone(),
                accepted: false,
            },
        )
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    let mut item = implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    );
    item.validator = Some((src.schema.clone(), node(&src, "behavior")));
    implementations.register(item).unwrap();
    implementations
        .register(implementation(
            &sources[1],
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        ))
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = compiled(&result, &sources[1]);
    assert_eq!(validate_descriptor(descriptor, vec![]).accepted, Some(true));
    assert!(descriptor.tokenizer().is_none());
    assert_eq!(
        validate_descriptor(descriptor, input().value).accepted,
        None
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut request = input();
    request.candidate = vec![native(sources[0].declaration())];
    request.value.extend(request.value.clone());
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert!(descriptor
        .validate(
            &request,
            &runtime,
            ValidationLimits {
                max_rules: 1,
                ..Default::default()
            }
        )
        .accepted
        .is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let checked = descriptor.validate(&request, &runtime, Default::default());
    assert_eq!(checked.accepted, Some(false));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(checked.completed.len(), 2);
    assert!(checked
        .completed
        .iter()
        .all(|r| r.datatype.identity() == sources[0].declaration().identity()));
}
#[test]
fn scalar_enumeration_cannot_activate_before_equality_and_constants() {
    let (mut host, sources) = types_fixture("{type @name=x @kind=scalar @values=one}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "datatype-enumeration-unavailable");
}

#[test]
fn compilation_retains_reference_failure_locations_and_rejects_scope_rebinding() {
    let (mut host, sources) = types_fixture("{type @name=x @kind=node @base=unknown}");
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &Default::default(),
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(result.dependency_sites.len(), 1);
    assert!(!result.reference_issues.is_empty());
    assert_eq!(
        result.reference_issues[0]
            .source
            .as_ref()
            .unwrap()
            .identity(),
        sources[0].attribute("base").unwrap().identity()
    );
    let (mut host, sources) = types_fixture("{type @name=x @kind=node}");
    let mut other = DatatypeRegistry::default();
    other
        .insert(
            sources[0].declaration().clone(),
            sources[0].declaration().clone(),
        )
        .unwrap();
    let other = other.source(sources[0].declaration(), "x").unwrap();
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Node,
            ValueRepresentation::Nodes,
        ))
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[0].clone(), other],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "conflicting-datatype-scope");
}

#[path = "conversion.rs"]
mod conversion;

#[path = "enumeration.rs"]
mod enumeration;

#[path = "names.rs"]
mod names;

#[path = "conversion_source.rs"]
mod conversion_source;

#[path = "enumeration_source.rs"]
mod enumeration_source;

#[path = "shipped_conversion.rs"]
mod shipped_conversion;

#[path = "shipped_lists.rs"]
mod shipped_lists;

#[path = "shipped_inventory.rs"]
mod shipped_inventory;

#[path = "attribute_types.rs"]
mod attribute_types;

#[path = "preparation.rs"]
mod preparation;

#[path = "facets.rs"]
mod facets;

#[path = "attribute_consumer.rs"]
mod attribute_consumer;

#[path = "attribute_readiness.rs"]
mod attribute_readiness;
