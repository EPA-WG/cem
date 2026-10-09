use super::*;
use cem_ml::schema::datatype_contracts::CompiledDatatypeContract;
use cem_ml::schema::{
    datatype_contracts::LexicalInput, document_model::shipped_datatypes::ShippedDatatype as T,
};
use cem_ql::{
    datatype_conversion::{ConversionInput, ConversionValue, ConverterBinding},
    datatype_shipped,
};

pub(super) fn descriptor(ty: T) -> ExecutableDatatype {
    descriptor_with(ty, None)
}
pub(super) fn descriptor_with(ty: T, vocabulary: Option<&str>) -> ExecutableDatatype {
    let primitive = match ty {
        T::Boolean => "boolean",
        T::Integer => "integer",
        T::Number => "decimal",
        _ => "string",
    };
    let text = declaration(false)
        .replace(
            "urn:test:validate",
            &format!("cemml:datatype:{}", ty.name()),
        )
        .replace("@type=schema:string", &format!("@type=schema:{primitive}"));
    let profile_source = source(&text);
    let behavior = node(&profile_source, "behavior");
    let vocabulary_attr = vocabulary
        .map(|v| format!(" @values={v:?}"))
        .unwrap_or_default();
    let (mut host, sources) = types_fixture(&format!(
        "{{type @name=sample @kind={}{vocabulary_attr}}}",
        match ty.kind() {
            DatatypeKind::Lexical => "lexical",
            DatatypeKind::Reference => "reference",
            DatatypeKind::Grammar => "grammar",
            _ => "scalar",
        }
    ));
    let src = sources[0].clone();
    let mut implementations = DatatypeImplementations::default();
    let mut entry = implementation(&src, ty.kind(), ty.representation());
    entry.validator = Some((profile_source.schema.clone(), behavior.clone()));
    implementations.register(entry).unwrap();
    let mut validations = DatatypeValidationRegistry::default();
    if ty == T::ContentModel {
        let pending = compile_datatypes(
            src.declaration().document().clone(),
            &[src.clone()],
            &mut host,
            &implementations,
            &validations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(!pending.is_ready());
        assert_eq!(pending.issues[0].code, "validation-capability-unavailable");
    }

    let mut sig = signature(false);
    sig.kind = ty.kind();
    sig.value = ty.representation();
    let contract = DatatypeBehaviorContract::compile(&profile_source, &behavior, sig).unwrap();
    datatype_shipped::register_validation(&mut validations, ty, contract, adapter()).unwrap();
    if vocabulary.is_some() {
        implementations
            .select_constant_interpreter(
                src.clone(),
                cem_ql::datatype_enumeration::ConstantBinding::Ready(
                    datatype_shipped::constant_interpreter(src.clone(), ty).unwrap(),
                ),
            )
            .unwrap();
        implementations
            .select_equality(
                src.clone(),
                cem_ql::datatype_enumeration::EqualityBinding::Ready(equality(&src, ty)),
            )
            .unwrap();
    } else {
        implementations
            .select_converter(
                src.clone(),
                ConverterBinding::Ready(datatype_shipped::converter(src.clone(), ty).unwrap()),
            )
            .unwrap();
    }
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let result = cem_ql::datatype_compilation::compile_datatypes_with_runtime(
        src.declaration().document().clone(),
        &[src.clone()],
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    compiled(&result, &src).clone()
}
pub(super) fn convert(
    descriptor: &ExecutableDatatype,
    text: &str,
) -> cem_ql::datatype_conversion::DatatypeConversion {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    descriptor.convert(
        &ConversionInput {
            value: ConversionValue::Lexical(LexicalInput::new(Arc::from(text), Default::default())),
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        Default::default(),
    )
}
#[test]
fn shipped_converters_keep_typed_values_and_validate_the_result() {
    for (ty, text, expected) in [
        (
            T::String,
            " keep spaces ",
            AtomValue::String(" keep spaces ".into()),
        ),
        (T::Boolean, "1", AtomValue::Boolean(true)),
        (T::Integer, "003", AtomValue::Integer(3)),
        (T::Number, " 1.25 ", AtomValue::Decimal("1.25".into())),
    ] {
        let result = convert(&descriptor(ty), text);
        assert_eq!(result.accepted, Some(true), "{ty:?}: {result:?}");
        assert_eq!(result.value.unwrap()[0].atom(), Some(expected));
        assert_eq!(result.validation.unwrap().completed.len(), 1);
    }
}
#[test]
fn wide_integer_remains_integer_and_does_not_satisfy_decimal_contract() {
    let integer = descriptor(T::Integer);
    for text in [
        "9223372036854775808",
        "-9223372036854775809",
        "999999999999999999999999999999999999999999999999999999",
    ] {
        let result = convert(&integer, text);
        assert_eq!(result.accepted, Some(true), "{result:?}");
        let value = result.value.unwrap();
        let predicate = if text.starts_with('-') {
            "value < 0"
        } else {
            "value > 9223372036854775807"
        };
        assert_eq!(
            evaluate_with_value(predicate, value[0].clone()).items,
            vec![Item::Atomic(AtomValue::Boolean(true))]
        );
        assert_eq!(
            value[0].view().unwrap().field("datatype").unwrap(),
            query("\"integer\"").items
        );
        assert_eq!(
            validate_descriptor(&integer, value.clone()).accepted,
            Some(true)
        );
        assert_eq!(
            validate_descriptor(&descriptor(T::Number), value).accepted,
            None
        );
        assert_eq!(
            validate_descriptor(
                &integer,
                vec![Item::Atomic(AtomValue::Decimal(text.into()))]
            )
            .accepted,
            None
        );
    }
}

pub(super) fn equality(
    src: &DatatypeSource,
    ty: T,
) -> cem_ql::datatype_enumeration::RegisteredScalarEquality {
    use cem_ml::schema::datatype_enumeration::EqualityBehaviorContract;
    use cem_ql::datatype_enumeration::{DatatypeEqualityResultAdapter, RegisteredScalarEquality};
    let text = r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name=test @namespace=urn:test | {behaviors |
 {behavior @name=compare @implementation=function @function=eq @execution=datatype-equality |
  {inputs | {input-binding @name=left @source=left @type=integer @required=true} {input-binding @name=right @source=right @type=integer @required=true} {input-binding @name=datatype @source=datatype @type=node @required=true}}
  {result @type=schema:datatype-equality-result}
  {function @name=eq @returns=datatype-equality-result |
   {param @name=left @type=integer @required=true} {param @name=right @type=integer @required=true} {param @name=datatype @type=node @required=true}
   {body | {$ {equal: left == right, diagnostics: ()} }}
  }
 }
}}
"#;
    let ValueRepresentation::Scalar(representation) = ty.representation() else {
        panic!()
    };
    let primitive = match representation {
        ScalarRepresentation::Integer => "integer",
        ScalarRepresentation::Boolean => "boolean",
        ScalarRepresentation::Decimal => "decimal",
        _ => "string",
    };
    let text = text.replace("@type=integer", &format!("@type={primitive}"));
    let profile = source(&text);
    let contract = EqualityBehaviorContract::compile(
        &profile,
        &node(&profile, "behavior"),
        representation,
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
    )
    .unwrap();
    RegisteredScalarEquality::from_query(
        src.clone(),
        contract,
        DatatypeEqualityResultAdapter::new(
            value_contracts(),
            ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
            ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn value_contracts() -> Arc<ValueContracts> {
    Arc::new(
        ValueContracts::compile(
            &[source(
                cem_ml::schema::package_sources::builtin_schema_package_source("schema")
                    .unwrap()
                    .schema_source,
            )],
            Default::default(),
        )
        .unwrap(),
    )
}
#[test]
fn constant_preparation_preserves_wide_integers_and_query_equality_is_exact() {
    let integer = descriptor(T::Integer);
    let restricted = descriptor_with(
        T::Integer,
        Some("9223372036854775808 -9223372036854775809 3"),
    );
    assert!(restricted.converter().is_none());
    let constants = restricted.enumerations()[0].constants();
    assert_eq!(constants[0].token.text(), "9223372036854775808");
    assert_eq!(constants[0].token.span, 0..19);
    for (text, expected) in [
        ("9223372036854775808", true),
        ("9223372036854775809", false),
        ("-9223372036854775809", true),
        ("-9223372036854775810", false),
        ("003", true),
        ("4", false),
    ] {
        let value = convert(&integer, text).value.unwrap();
        assert_eq!(
            validate_descriptor(&restricted, value).accepted,
            Some(expected),
            "{text}"
        );
    }
}
fn evaluate_with_value(expression: &str, value: Item) -> ItemStream {
    use cem_ql::api::{
        evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext,
    };
    let context = StandaloneExpressionContext::default().with_binding(
        "value",
        StandaloneExpressionBinding::new(
            ItemStream::once(value),
            cem_ql::types::Type::Atom(cem_ql::types::AtomType::Integer),
        ),
    );
    evaluate_expression(expression, &context).unwrap().result
}
#[test]
fn checked_query_results_keep_wide_integer_identity_and_reject_decimal_substitutes() {
    use cem_ql::{
        datatype_conversion::DatatypeConversionResultAdapter,
        datatype_enumeration::DatatypeConstantResultAdapter,
    };
    let wide = convert(&descriptor(T::Integer), "9223372036854775808")
        .value
        .unwrap()
        .remove(0);
    let conversion = DatatypeConversionResultAdapter::new(
        value_contracts(),
        ContractName::new(CEM_SCHEMA_URI, "datatype-conversion-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap();
    let constants = DatatypeConstantResultAdapter::new(
        value_contracts(),
        ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap();
    let output = conversion
        .consume(
            evaluate_with_value(
                "{status: \"converted\", value: value, diagnostics: ()}",
                wide.clone(),
            ),
            &Default::default(),
            ValueRepresentation::Scalar(ScalarRepresentation::Integer),
            Default::default(),
        )
        .unwrap()
        .value
        .unwrap();
    assert_eq!(output[0].identity(), wide.identity());
    let output = constants
        .consume(
            evaluate_with_value(
                "{status: \"prepared\", value: value, diagnostics: ()}",
                wide.clone(),
            ),
            &Default::default(),
            ScalarRepresentation::Integer,
            Default::default(),
        )
        .unwrap()
        .value
        .unwrap();
    assert_eq!(output.identity(), wide.identity());
    for (item, expected) in [
        (wide.clone(), ScalarRepresentation::Decimal),
        (
            Item::Atomic(AtomValue::Decimal("9223372036854775808".into())),
            ScalarRepresentation::Integer,
        ),
    ] {
        assert!(conversion
            .consume(
                evaluate_with_value(
                    "{status: \"converted\", value: value, diagnostics: ()}",
                    item.clone()
                ),
                &Default::default(),
                ValueRepresentation::Scalar(expected),
                Default::default()
            )
            .is_err());
        assert!(constants
            .consume(
                evaluate_with_value(
                    "{status: \"prepared\", value: value, diagnostics: ()}",
                    item
                ),
                &Default::default(),
                expected,
                Default::default()
            )
            .is_err());
    }
}
#[test]
fn native_scalar_metadata_cannot_forge_integer_type_identity() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    #[derive(Debug)]
    struct Fake;
    impl QueryItemView for Fake {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Atomic
        }
        fn representation_id(&self) -> &'static str {
            "cem.typed-atomic"
        }
        fn identity(&self) -> String {
            "integer:9223372036854775808".into()
        }
        fn atom(&self) -> Option<AtomValue> {
            Some(AtomValue::Decimal("9223372036854775808".into()))
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            Some(vec![Item::Atomic(AtomValue::String(
                if name == "datatype" {
                    "integer"
                } else {
                    "9223372036854775808"
                }
                .into(),
            ))])
        }
    }
    assert_eq!(
        validate_descriptor(&descriptor(T::Integer), vec![Item::native(Fake)]).accepted,
        None
    );
}
#[test]
fn conversion_rejections_control_bounds_and_original_sources_are_preserved() {
    use cem_ql::datatype_conversion::{ConversionLimits, ConversionStop};
    let descriptor = descriptor(T::Integer);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let source = native(descriptor.source().declaration())
        .source_map()
        .unwrap();
    let mut input = ConversionInput {
        value: ConversionValue::Lexical(LexicalInput::new(
            Arc::from("9223372036854775808"),
            source.clone(),
        )),
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("input.cem".into()),
            ..Default::default()
        },
    };
    let result = descriptor.convert(&input, &runtime, Default::default());
    assert_eq!(result.value.unwrap()[0].source_map(), Some(source.clone()));
    let mut limits = ConversionLimits::default();
    limits.max_lexical_bytes = 1;
    assert!(matches!(
        descriptor.convert(&input, &runtime, limits).stopped,
        Some(ConversionStop::Limit(_))
    ));
    input.value = ConversionValue::Lexical(LexicalInput::new(Arc::from("invalid"), source));
    let rejected = descriptor.convert(&input, &runtime, Default::default());
    assert_eq!(rejected.accepted, Some(false));
    assert!(rejected.validation.is_none());
    assert_eq!(rejected.diagnostics[0].uri.as_deref(), Some("input.cem"));
    control.cancel_root(None, None).unwrap();
    assert!(matches!(
        descriptor
            .convert(&input, &runtime, Default::default())
            .stopped,
        Some(ConversionStop::Control(_))
    ));
    let (_, sources) = types_fixture("{type @name=sample @kind=scalar}");
    assert!(datatype_shipped::converter(sources[0].clone(), T::ContentModel).is_ok());
    assert!(datatype_shipped::constant_interpreter(sources[0].clone(), T::ContentModel).is_ok());
}
