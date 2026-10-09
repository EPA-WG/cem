use cem_ml::schema::document_model::{
    convert_attribute_value, shipped_datatypes::ShippedDatatype as T, AttributeModel,
    AttributeValueContract,
};

#[test]
fn shipped_validation_preserves_lexical_contracts_and_explicit_grammar_gap() {
    for (ty, valid, invalid) in [
        (T::Identifier, "a-b", "a:b"),
        (T::QualifiedName, "ns:item", "a:b:c"),
        (T::SymbolReference, "a.b-c", "a..b"),
        (T::WildcardName, "name:*", "a:b"),
        (T::Boolean, "true", "1"),
        (T::Integer, "+0003", "1.2"),
        (T::Number, "1.2e3", "NaN"),
        (T::Uri, "https://example.test/a", "./a"),
        (T::Semver, "1.2.3-rc.1+build", "01.2.3"),
        (T::MediaType, "text/plain; charset=utf-8", "a/b/c"),
        (T::Path, "./a", "../a"),
        (T::NameList, "a\tb-c", "a b:c"),
        (T::WildcardNameList, "a b:*", "a:b"),
        (T::TypeReference, "a:b", "a:b:c"),
        (T::WildcardTypeReference, "a:b:*", "plain"),
        (T::ReferenceUnresolvedDisposition, "warning", "warn"),
    ] {
        assert_eq!(ty.validate_lexical(valid), Some(true), "{ty:?}");
        assert_eq!(ty.validate_lexical(invalid), Some(false), "{ty:?}");
    }
    assert_eq!(T::String.validate_lexical(" any text "), Some(true));
    assert_eq!(T::Boolean.validate_lexical(""), Some(true));
    assert_eq!(T::NameList.validate_lexical(" \t "), Some(false));
    assert_eq!(T::ContentModel.validate_lexical("anything"), None);
    assert_eq!(T::ALL.len(), 18);
}
#[test]
fn explicit_conversion_matches_legacy_without_inventing_unsupported_capabilities() {
    for ty in T::ALL {
        for input in [
            "",
            " 1 ",
            "true",
            "false",
            "0",
            "003",
            "-0",
            "1.25",
            "1e3",
            "NaN",
            "a:b",
            "9223372036854775808",
            " keep spaces ",
        ] {
            let expected = convert_attribute_value(
                input,
                &AttributeValueContract {
                    model: AttributeModel {
                        value_type: Some(ty.name().into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &Default::default(),
            );
            match ty.convert_lexical(input, &Default::default(), &mut || Ok::<_, ()>(())) {
                Some(actual) => match (actual, expected) {
                    (Ok(a), Ok(b)) => assert_eq!(a, b),
                    (
                        Err(
                            cem_ml::schema::document_model::AttributeValueConversionError::Invalid(
                                a,
                            ),
                        ),
                        Err(b),
                    ) => assert_eq!(a, b),
                    other => panic!("{ty:?}: {other:?}"),
                },
                None => assert!(expected.is_err(), "{:?}", ty),
            }
        }
    }
}

#[test]
fn shipped_conversion_retains_wide_integer_lexical_and_interruption() {
    let wide = "9223372036854775808";
    let value = T::Integer
        .convert_lexical(wide, &Default::default(), &mut || Ok::<_, ()>(()))
        .unwrap()
        .unwrap();
    assert_eq!(value.datatype, "integer");
    assert_eq!(value.lexical, wide);
    let stopped = T::String
        .convert_lexical("value", &Default::default(), &mut || Err("cancelled"))
        .unwrap();
    assert!(matches!(
        stopped,
        Err(
            cem_ml::schema::document_model::AttributeValueConversionError::Interrupted("cancelled")
        )
    ));
}
