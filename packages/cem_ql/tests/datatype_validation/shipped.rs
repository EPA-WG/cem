use super::*;
use cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype as T;
use cem_ql::datatype_shipped::register_validation;

fn bound(ty: T) -> cem_ql::datatype_validation::BoundDatatypeRule {
    bound_with_limits(ty, None)
}
fn bound_with_limits(
    ty: T,
    limits: Option<cem_ml::schema::document_model::content_model::GrammarLimits>,
) -> cem_ql::datatype_validation::BoundDatatypeRule {
    let (primitive, cardinality) = match ty.representation() {
        ValueRepresentation::List(_) => ("string", "zero-or-more"),
        ValueRepresentation::Scalar(ScalarRepresentation::Boolean) => ("boolean", "one"),
        ValueRepresentation::Scalar(ScalarRepresentation::Integer) => ("integer", "one"),
        ValueRepresentation::Scalar(ScalarRepresentation::Decimal) => ("decimal", "one"),
        _ => ("string", "one"),
    };
    let text = declaration(false).replace("urn:test:validate", &format!("cemml:datatype:{}", ty.name()))
        .replace("@name=value @type=schema:string @source=value @required=true @cardinality=one", &format!("@name=value @type=schema:{primitive} @source=value @required=true @cardinality={cardinality}"));
    let src = source(&text);
    let mut sig = signature(false);
    sig.kind = ty.kind();
    sig.value = ty.representation();
    let contract = DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    assert!(registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            ty.kind()
        )
        .is_err());
    if let Some(limits) = limits {
        cem_ql::datatype_shipped::register_content_model_validation(
            &mut registry,
            contract,
            adapter(),
            limits,
        )
        .unwrap();
    } else {
        register_validation(&mut registry, ty, contract, adapter()).unwrap();
    }
    registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            ty.kind(),
        )
        .unwrap()
}
#[test]
fn shipped_native_validation_uses_exact_representations_without_conversion() {
    for (ty, accepted, rejected) in [
        (
            T::Identifier,
            query("\"a-b\"").items,
            query("\"a:b\"").items,
        ),
        (
            T::Uri,
            query("\"https://example.test/a\"").items,
            query("\"./a\"").items,
        ),
        (
            T::Semver,
            query("\"1.2.3\"").items,
            query("\"01.2.3\"").items,
        ),
        (
            T::MediaType,
            query("\"text/plain\"").items,
            query("\"a/b/c\"").items,
        ),
        (T::Path, query("\"./a\"").items, query("\"../a\"").items),
        (
            T::TypeReference,
            query("\"a:b\"").items,
            query("\"a:b:c\"").items,
        ),
        (
            T::NameList,
            query("(\" a \", \"b\")").items,
            query("\"a b\"").items,
        ),
        (T::WildcardNameList, query("(\"a\", \"b:*\")").items, vec![]),
        (
            T::Number,
            vec![Item::Atomic(AtomValue::Decimal("1.25".into()))],
            vec![Item::Atomic(AtomValue::Decimal("NaN".into()))],
        ),
    ] {
        let rule = bound(ty);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        for (value, expected) in [(accepted, true), (rejected, false)] {
            let original = value.clone();
            let input = ValidationInput {
                value,
                candidate: vec![],
                fallback: DiagnosticAttribution {
                    uri: Some("candidate.cem".into()),
                    ..Default::default()
                },
            };
            let result = validate_rules(&[rule.clone()], &input, &runtime, Default::default());
            assert_eq!(result.accepted, Some(expected), "{ty:?}: {result:?}");
            assert_eq!(input.value, original);
            if !expected {
                assert_eq!(
                    result.completed[0].result.diagnostics[0].uri.as_deref(),
                    Some("candidate.cem")
                );
            }
        }
    }
    for (ty, value) in [
        (T::Boolean, Item::Atomic(AtomValue::Boolean(true))),
        (T::Integer, Item::Atomic(AtomValue::Integer(3))),
    ] {
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        assert_eq!(
            validate_rules(
                &[bound(ty)],
                &ValidationInput {
                    value: vec![value],
                    ..input()
                },
                &runtime,
                Default::default()
            )
            .accepted,
            Some(true)
        );
        assert_eq!(
            validate_rules(
                &[bound(ty)],
                &ValidationInput {
                    value: query("\"1\"").items,
                    ..input()
                },
                &runtime,
                Default::default()
            )
            .accepted,
            None
        );
    }
}
#[test]
fn shipped_registration_rejects_wrong_signatures_and_primitive_identity() {
    let src = source(&declaration(false));
    let mut registry = DatatypeValidationRegistry::default();
    assert!(
        register_validation(&mut registry, T::Integer, contract(&src, false), adapter()).is_err()
    );
    let mut sig = signature(false);
    sig.kind = DatatypeKind::Grammar;
    let grammar = DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
    assert!(register_validation(&mut registry, T::ContentModel, grammar, adapter()).is_err());
}

#[test]
fn shipped_native_validation_keeps_control_and_sequence_limits() {
    let rule = bound(T::NameList);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let input = ValidationInput {
        value: query("(\"a\", \"b\")").items,
        ..input()
    };
    let result = validate_rules(
        &[rule.clone()],
        &input,
        &runtime,
        ValidationLimits {
            max_input_values: 1,
            ..Default::default()
        },
    );
    assert_eq!(result.accepted, None);
    assert!(result.completed.is_empty());
    control.cancel_root(None, None).unwrap();
    let result = validate_rules(&[rule], &input, &runtime, Default::default());
    assert_eq!(result.accepted, None);
    assert!(matches!(
        result.stopped.unwrap().reason,
        cem_ql::datatype_validation::ValidationStopReason::Control(_)
    ));
}

#[test]
fn shipped_grammar_requires_registration_and_preserves_controlled_incompleteness() {
    let rule = bound(T::ContentModel); // helper first verifies missing registration
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (text, accepted) in [("(a | b)*", Some(true)), ("a??", Some(false))] {
        let result = validate_rules(
            &[rule.clone()],
            &ValidationInput {
                value: vec![Item::Atomic(AtomValue::String(text.into()))],
                ..input()
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(result.accepted, accepted, "{result:?}");
    }
    let deep = format!("{}a{}", "(".repeat(65), ")".repeat(65));
    let result = validate_rules(
        &[rule.clone()],
        &ValidationInput {
            value: vec![Item::Atomic(AtomValue::String(deep))],
            ..input()
        },
        &runtime,
        Default::default(),
    );
    assert_eq!(result.accepted, None);
    let expanded = bound_with_limits(
        T::ContentModel,
        Some(
            cem_ml::schema::document_model::content_model::GrammarLimits {
                max_depth: 128,
                ..Default::default()
            },
        ),
    );
    let deep = format!("{}a{}", "(".repeat(65), ")".repeat(65));
    let result = validate_rules(
        &[expanded],
        &ValidationInput {
            value: vec![Item::Atomic(AtomValue::String(deep))],
            ..input()
        },
        &runtime,
        Default::default(),
    );
    assert_eq!(result.accepted, Some(true));
    control.cancel_root(None, None).unwrap();
    let result = validate_rules(&[rule], &input(), &runtime, Default::default());
    assert_eq!(result.accepted, None);
}
