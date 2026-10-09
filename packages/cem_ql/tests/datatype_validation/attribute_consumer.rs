use super::*;
use cem_ml::schema::{
    datatype_contracts::LexicalInput,
    document_model::{attribute_facets::*, shipped_datatypes::ShippedDatatype as T},
};
use cem_ql::{
    attribute_datatypes::bind_attribute_datatype, attribute_validation::*, datatype_facets::*,
    datatype_preparation::*,
};

fn fixture(
    family: FacetFamily,
    fields: &str,
    prepare: bool,
    accepted: bool,
    warning: bool,
) -> BoundAttributeFacets {
    fixture_with_host(family, fields, prepare, accepted, warning).0
}
pub(super) fn fixture_with_host(
    family: FacetFamily,
    fields: &str,
    prepare: bool,
    accepted: bool,
    warning: bool,
) -> (
    BoundAttributeFacets,
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
) {
    let (kind, kind_name) = match family.representation() {
        ValueRepresentation::Scalar(_) => (DatatypeKind::Scalar, "scalar"),
        ValueRepresentation::Nodes => (DatatypeKind::Node, "node"),
        ValueRepresentation::List(_) => (DatatypeKind::List, "list"),
    };
    let (item, base) = if kind == DatatypeKind::List {
        ("{type @name=item @kind=scalar}", "@base=item")
    } else {
        ("", "")
    };
    let profile = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {item} {{type @name=sample @kind={kind_name} {base}}}}} {{attributes | {{attribute @name=value @type=sample {fields}}}}}}}"));
    let decl = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let selected = sources.iter().find(|s| matches!(s.attribute("name").unwrap().node(), CemAstNode::Attribute {value: Some(name),..} if name == "sample")).unwrap();
    let primitive = match family.representation() {
        ValueRepresentation::Scalar(ScalarRepresentation::Integer)
        | ValueRepresentation::List(ScalarRepresentation::Integer) => "integer",
        ValueRepresentation::Nodes => "node",
        _ => "string",
    };
    let mut text =
        declaration(false).replace("@type=schema:string", &format!("@type=schema:{primitive}"));
    if matches!(kind, DatatypeKind::Node | DatatypeKind::List) {
        text = text.replace(
            "@source=value @required=true @cardinality=one",
            "@source=value @required=true @cardinality=zero-or-more",
        );
    }
    let rule_source = source(&text);
    let rule = node(&rule_source, "behavior");
    let mut sig = signature(false);
    sig.kind = kind;
    sig.value = family.representation();
    let contract = DatatypeBehaviorContract::compile(&rule_source, &rule, sig).unwrap();
    let mut validations = DatatypeValidationRegistry::default();
    let diagnostics = if warning {
        "({code: \"rule-warning\", severity: \"warning\", message: \"rule ran\"})"
    } else {
        "()"
    };
    validations
        .register_native(
            "urn:test:validate",
            contract,
            adapter(),
            None,
            Outcome(RuleExecution::Complete(query(&format!(
                "{{accepted: {accepted}, diagnostics: {diagnostics}}}"
            )))),
        )
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    let mut registration = implementation(selected, kind, family.representation());
    registration.validator = Some((rule_source.schema, rule));
    if kind == DatatypeKind::List {
        registration.tokenizer = TokenizerBinding::Ready(
            cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
        );
        let item_source = sources
            .iter()
            .find(|s| s.declaration().identity() != selected.declaration().identity())
            .unwrap();
        implementations
            .register(implementation(
                item_source,
                DatatypeKind::Scalar,
                T::Integer.representation(),
            ))
            .unwrap();
        implementations
            .select_preparation(
                item_source.clone(),
                PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(item_source.clone(), T::Integer)
                        .unwrap(),
                ),
            )
            .unwrap();
    }
    implementations.register(registration).unwrap();
    implementations
        .select_facets(
            selected.clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(selected.clone(), "test:facets", family).unwrap(),
            ),
        )
        .unwrap();
    if prepare {
        let preparation = match family {
            FacetFamily::Shipped(ty) => {
                cem_ql::datatype_shipped::lexical_preparation(selected.clone(), ty).unwrap()
            }
            FacetFamily::List(item) => {
                RegisteredLexicalPreparation::list_items(selected.clone(), "test:list", item)
                    .unwrap()
            }
            _ => panic!(),
        };
        implementations
            .select_preparation(selected.clone(), PreparationBinding::Ready(preparation))
            .unwrap();
    }
    let compilation = compile_datatypes(
        selected.declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready(), "{:?}", compilation.issues);
    let CemAstNode::Element { attributes, .. } = decl.node() else {
        panic!()
    };
    let slot = attributes.iter().filter_map(|id|SchemaDeclarationNode::new(decl.document().clone(),*id)).find(|n|matches!(n.node(),CemAstNode::Attribute{expanded_name,..} if expanded_name.local_name=="type")).unwrap();
    host.bind_literal_attribute_type(slot, selected.declaration().clone())
        .unwrap();
    let bound = bind_attribute_datatype(
        decl,
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
    .compile_facets("urn:test", Default::default())
    .unwrap();
    (bound, host)
}
fn lexical(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn context<'a>(
    source: &'a CemAstNode,
    diagnostics: &'a BTreeMap<String, cem_ml::schema::document_model::DiagnosticBehavior>,
    attributes: &'a BTreeMap<String, String>,
) -> FacetContext<'a> {
    FacetContext {
        element_name: "sample",
        source,
        diagnostic_behaviors: diagnostics,
        attribute_values: attributes,
    }
}
#[test]
fn attribute_consumer_intersects_rules_and_facets_and_retains_lexical_input() {
    for (rule, text, expected) in [
        (true, "003", true),
        (false, "003", false),
        (true, "005", false),
    ] {
        let bound = fixture(
            FacetFamily::Shipped(T::Integer),
            "@maxInclusive=4",
            true,
            rule,
            false,
        );
        let control = OperationControl::default();
        let result = bound.validate_lexical(
            &lexical(text),
            context(
                bound.binding().declaration.node(),
                &Default::default(),
                &Default::default(),
            ),
            &ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
            Default::default(),
        );
        assert_eq!(result.accepted, Some(expected), "{result:?}");
        assert!(result.stopped.is_none());
        let AttributeDatatypePhase::Lexical(preparation) = result.datatype.unwrap() else {
            panic!()
        };
        assert_eq!(&*preparation.input.lexical.text, text);
        assert_eq!(
            preparation.value.unwrap()[0].atom(),
            Some(AtomValue::Integer(text.parse().unwrap()))
        );
        assert_eq!(preparation.validation.unwrap().accepted, Some(rule));
        assert_eq!(result.facets.unwrap().accepted, text == "003");
        assert!(bound.binding().datatype.converter().is_none());
    }
}
#[test]
fn attribute_consumer_shares_diagnostic_budget_between_rules_and_local_facets() {
    let bound = fixture(
        FacetFamily::Shipped(T::Integer),
        "@maxInclusive=4",
        true,
        true,
        true,
    );
    for (budget, expected) in [(0, None), (1, None), (2, Some(false))] {
        let control = OperationControl::default();
        let mut limits = AttributeValidationLimits::default();
        limits.preparation.validation.max_diagnostics = budget;
        let result = bound.validate_lexical(
            &lexical("005"),
            context(
                bound.binding().declaration.node(),
                &Default::default(),
                &Default::default(),
            ),
            &ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
            limits,
        );
        assert_eq!(result.accepted, expected, "budget {budget}: {result:?}");
        assert_eq!(result.stopped.is_some(), expected.is_none());
        if budget == 1 {
            let AttributeDatatypePhase::Lexical(preparation) = result.datatype.unwrap() else {
                panic!()
            };
            assert_eq!(
                preparation.validation.unwrap().completed[0]
                    .result
                    .diagnostics
                    .len(),
                1
            );
            assert!(matches!(
                result.stopped,
                Some(AttributeValidationStop::Facets(FacetExecutionError::Limit))
            ));
        }
    }
}
#[test]
fn attribute_consumer_nodes_preserve_targets_and_check_whole_sequence_count() {
    let bound = fixture(
        FacetFamily::Nodes,
        "@minItems=0 @maxItems=1",
        false,
        true,
        false,
    );
    let src = source("{schema | {target | {#nested}}}");
    let target = native(&node(&src, "target"));
    for values in [
        vec![],
        vec![target.clone()],
        vec![target.clone(), target.clone()],
    ] {
        let input = ValidationInput {
            value: values.clone(),
            candidate: vec![],
            fallback: Default::default(),
        };
        let control = OperationControl::default();
        let result = bound.validate_nodes(
            &input,
            context(
                bound.binding().declaration.node(),
                &Default::default(),
                &Default::default(),
            ),
            &ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
            Default::default(),
        );
        assert_eq!(result.accepted, Some(values.len() <= 1), "{result:?}");
        let AttributeDatatypePhase::Nodes {
            input: retained,
            validation,
        } = result.datatype.unwrap()
        else {
            panic!()
        };
        assert_eq!(validation.completed.len(), 1);
        assert_eq!(retained.value.len(), values.len());
        for value in retained.value {
            assert_eq!(value.identity(), target.identity());
            assert_eq!(value.source_map(), target.source_map());
            let original = cem_ql::eval::retained_cem_node(&target).unwrap();
            let retained = cem_ql::eval::retained_cem_node(&value).unwrap();
            assert!(Arc::ptr_eq(original.owner(), retained.owner()));
            assert!(Arc::ptr_eq(
                retained.owner().ast_owner(),
                src.schema.document()
            ));
        }
    }
    assert!(src
        .schema
        .document()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn attribute_consumer_never_accepts_unavailable_or_cancelled_phases() {
    let bound = fixture(
        FacetFamily::Shipped(T::Integer),
        "@minInclusive=1",
        false,
        true,
        false,
    );
    let control = OperationControl::default();
    let result = bound.validate_lexical(
        &lexical("003"),
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    );
    assert_eq!(result.accepted, None);
    assert!(result.facets.is_none());
    let AttributeDatatypePhase::Lexical(preparation) = result.datatype.unwrap() else {
        panic!()
    };
    assert!(matches!(
        preparation.stopped,
        Some(PreparationStop::NoPreparation)
    ));
    let ready = fixture(
        FacetFamily::Shipped(T::Integer),
        "@minInclusive=1",
        true,
        true,
        false,
    );
    let mut limits = AttributeValidationLimits::default();
    limits.max_facet_model_bytes = 0;
    let result = ready.validate_lexical(
        &lexical("003"),
        context(
            ready.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    );
    assert_eq!(result.accepted, None);
    assert!(matches!(
        result.stopped,
        Some(AttributeValidationStop::Facets(FacetExecutionError::Limit))
    ));
    let AttributeDatatypePhase::Lexical(preparation) = result.datatype.unwrap() else {
        panic!()
    };
    assert_eq!(preparation.accepted, Some(true));
    let signal = AbortSignal::default();
    signal.abort();
    let control = OperationControl::new(signal);
    let result = bound.validate_lexical(
        &lexical("003"),
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    );
    assert_eq!(result.accepted, None);
    assert!(result.stopped.is_some());
}

#[test]
fn attribute_consumer_lists_keep_prepared_tokens_and_shared_input_allowance() {
    let bound = fixture(
        FacetFamily::List(ScalarRepresentation::Integer),
        "@minItems=2",
        true,
        true,
        false,
    );
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (text, accepted) in [("003 003", true), ("003", false), ("", false)] {
        let report = bound.validate_lexical(
            &lexical(text),
            context(
                bound.binding().declaration.node(),
                &Default::default(),
                &Default::default(),
            ),
            &runtime,
            Default::default(),
        );
        assert_eq!(report.accepted, Some(accepted), "{report:?}");
        let AttributeDatatypePhase::Lexical(preparation) = report.datatype.unwrap() else {
            panic!()
        };
        assert_eq!(&*preparation.input.lexical.text, text);
        let count = text.split_whitespace().count();
        assert_eq!(preparation.token_spans.len(), count);
        assert_eq!(preparation.value.unwrap().len(), count);
    }
    let mut limits = AttributeValidationLimits::default();
    // Two scalar preparations and four datatype visits require six visits in total.
    limits.preparation.validation.max_input_values = 5;
    let report = bound.validate_lexical(
        &lexical("003 003"),
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &runtime,
        limits,
    );
    assert_eq!(report.accepted, None);
    assert!(report.facets.is_none());
    limits.preparation.validation.max_input_values = 6;
    let report = bound.validate_lexical(
        &lexical("003 003"),
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &runtime,
        limits,
    );
    assert_eq!(report.accepted, Some(true));
}

#[test]
fn attribute_consumer_rejects_wrong_ingress_without_scalar_fallback() {
    let bound = fixture(FacetFamily::Nodes, "", false, true, false);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let report = bound.validate_lexical(
        &lexical("target"),
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &runtime,
        Default::default(),
    );
    assert_eq!(report.accepted, None);
    assert!(matches!(
        report.stopped,
        Some(AttributeValidationStop::InvalidInput)
    ));
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("target".into()))],
        candidate: vec![],
        fallback: Default::default(),
    };
    let report = bound.validate_nodes(
        &input,
        context(
            bound.binding().declaration.node(),
            &Default::default(),
            &Default::default(),
        ),
        &runtime,
        Default::default(),
    );
    assert_eq!(report.accepted, None);
    assert!(report.facets.is_none());
}
