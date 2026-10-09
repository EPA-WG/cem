use super::*;
use cem_ml::schema::{
    datatype_contracts::CompiledDatatypeContract,
    document_model::shipped_datatypes::ShippedDatatype as T,
};
use cem_ql::{
    datatype_conversion::ConverterBinding,
    datatype_enumeration::{ConstantBinding, EqualityBinding},
    datatype_shipped,
};

#[test]
fn all_original_shipped_declarations_compile_with_explicit_capabilities() {
    let profile = source(include_str!(
        "../../../cem_ml/schema-packages/cem-ml/v1/schema/cem-ml-generic.cem"
    ));
    let owner = profile.schema.document().clone();
    let (mut host, sources) = types_fixture_source(profile);
    let unregistered = compile_datatypes(
        owner.clone(),
        &sources,
        &mut host,
        &Default::default(),
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(
        !unregistered.is_ready(),
        "shipped names grant no implementation authority"
    );
    let mut implementations = DatatypeImplementations::default();
    let mut validations = DatatypeValidationRegistry::default();
    for ty in T::ALL {
        let src = sources.iter().find(|s| matches!(s.attribute("name").unwrap().node(), CemAstNode::Attribute {value: Some(name),..} if name == ty.name())).unwrap();
        let primitive = match ty.representation() {
            ValueRepresentation::Scalar(ScalarRepresentation::Boolean) => "boolean",
            ValueRepresentation::Scalar(ScalarRepresentation::Integer) => "integer",
            ValueRepresentation::Scalar(ScalarRepresentation::Decimal) => "decimal",
            _ => "string",
        };
        let mut text = declaration(false)
            .replace(
                "urn:test:validate",
                &format!("cemml:datatype:{}", ty.name()),
            )
            .replace("@type=schema:string", &format!("@type=schema:{primitive}"));
        if ty.item().is_some() {
            text = text.replace(
                "@source=value @required=true @cardinality=one",
                "@source=value @required=true @cardinality=zero-or-more",
            );
        }
        let profile = source(&text);
        let behavior = node(&profile, "behavior");
        let mut sig = signature(false);
        sig.kind = ty.kind();
        sig.value = ty.representation();
        datatype_shipped::register_validation(
            &mut validations,
            ty,
            DatatypeBehaviorContract::compile(&profile, &behavior, sig).unwrap(),
            adapter(),
        )
        .unwrap();
        let entry =
            datatype_shipped::implementation(src.clone(), ty, (profile.schema.clone(), behavior));
        implementations.register(entry).unwrap();
        if !matches!(ty, T::ContentModel | T::TypeReference) {
            implementations
                .select_preparation(
                    src.clone(),
                    cem_ql::datatype_preparation::PreparationBinding::Ready(
                        datatype_shipped::lexical_preparation(src.clone(), ty).unwrap(),
                    ),
                )
                .unwrap();
        }
        implementations
            .select_converter(
                src.clone(),
                ConverterBinding::Ready(datatype_shipped::converter(src.clone(), ty).unwrap()),
            )
            .unwrap();
        if ty.item().is_none() {
            implementations
                .select_constant_interpreter(
                    src.clone(),
                    ConstantBinding::Ready(
                        datatype_shipped::constant_interpreter(src.clone(), ty).unwrap(),
                    ),
                )
                .unwrap();
            implementations
                .select_equality(
                    src.clone(),
                    EqualityBinding::Ready(super::shipped_conversion::equality(src, ty)),
                )
                .unwrap();
        }
    }
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let result = cem_ql::datatype_compilation::compile_datatypes_with_runtime(
        owner.clone(),
        &sources,
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    assert_eq!(result.contracts.len(), 18);
    for (ty, text) in [
        (T::Identifier, "a"),
        (T::QualifiedName, "n:a"),
        (T::SymbolReference, "a.b"),
        (T::WildcardName, "a:*"),
        (T::String, " keep "),
        (T::Boolean, "true"),
        (T::ReferenceUnresolvedDisposition, " warning "),
        (T::Integer, "003"),
        (T::Number, "1.2"),
        (T::Uri, "urn:example:a"),
        (T::Semver, "1.2.3"),
        (T::MediaType, "text/plain"),
        (T::Path, "./a"),
        (T::NameList, "a b a"),
        (T::WildcardNameList, "a b:*"),
        (T::ContentModel, "(a|b)*"),
        (T::TypeReference, "n:a"),
        (T::WildcardTypeReference, "n:a:*"),
    ] {
        let d = result.contracts.iter().find(|c| matches!(c.source().attribute("name").unwrap().node(), CemAstNode::Attribute {value: Some(name),..} if name == ty.name())).unwrap().as_any().downcast_ref::<ExecutableDatatype>().unwrap();
        assert!(Arc::ptr_eq(d.source().declaration().document(), &owner));
        let converted = super::shipped_conversion::convert(d, text);
        assert_eq!(converted.accepted, Some(true), "{ty:?}");
        let validation = d.validate(
            &ValidationInput {
                value: converted.value.unwrap(),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(validation.accepted, Some(true), "{ty:?}: {validation:?}");
        let prepared = d.prepare_lexical(
            &cem_ql::datatype_preparation::PreparationInput {
                lexical: cem_ml::schema::datatype_contracts::LexicalInput::new(
                    Arc::from(text),
                    Default::default(),
                ),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(prepared.accepted, Some(true), "{ty:?}: {prepared:?}");
        assert_eq!(&*prepared.input.lexical.text, text);
        if let Some(base) = d.base() {
            assert!(Arc::ptr_eq(base.source().declaration().document(), &owner));
        }
        if let Some(item) = d.item() {
            assert!(Arc::ptr_eq(item.source().declaration().document(), &owner));
        }
    }
}
