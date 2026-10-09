use super::shipped_conversion::{convert, descriptor, descriptor_with};
use super::*;
use cem_ml::schema::{
    datatype_contracts::{CompiledDatatypeContract, LexicalInput},
    document_model::shipped_datatypes::ShippedDatatype as T,
};
use cem_ql::{
    datatype_conversion::{
        ConversionInput, ConversionLimits, ConversionStop, ConversionValue, ConverterBinding,
    },
    datatype_shipped,
};

#[test]
fn explicit_lexical_conversion_trims_without_rewriting_or_resolving() {
    for (ty, valid, invalid) in [
        (T::Identifier, "a-b", "a:b"),
        (T::QualifiedName, "ns:item", "a:b:c"),
        (T::SymbolReference, "a.b-c", "a..b"),
        (T::WildcardName, "name:*", "a:b"),
        (T::Uri, "HTTPS://Example.test/a%2Fb", "./a"),
        (T::Semver, "1.2.3-rc.1+build", "01.2.3"),
        (T::MediaType, "Text/Plain; charset=UTF-8", "a/b/c"),
        (T::Path, "./a", "../a"),
        (T::TypeReference, "unbound:item", "a:b:c"),
        (T::WildcardTypeReference, "unbound:item:*", "plain"),
        (T::ReferenceUnresolvedDisposition, "warning", "warn"),
    ] {
        let d = descriptor(ty);
        let good = convert(&d, &format!(" \t{valid}\n"));
        assert_eq!(good.accepted, Some(true), "{ty:?}: {good:?}");
        assert_eq!(
            good.value.unwrap()[0].atom(),
            Some(AtomValue::String(valid.into()))
        );
        let bad = convert(&d, invalid);
        assert_eq!(bad.accepted, Some(false), "{ty:?}: {bad:?}");
        assert!(bad.validation.is_none());
        assert!(datatype_shipped::constant_interpreter(d.source().clone(), ty).is_ok());
    }
}

fn list_descriptor(ty: T, tokenizer: Option<TokenizerBinding>) -> ExecutableDatatype {
    let item = ty.item().unwrap();
    let text = declaration(false).replace(
        "urn:test:validate",
        &format!("cemml:datatype:{}", item.name()),
    );
    let profile = source(&text);
    let behavior = node(&profile, "behavior");
    let (mut host, sources) =
        types_fixture("{type @name=item @kind=lexical} {type @name=names @kind=list @base=item @min-items=0 @max-items=3}");
    let mut implementations = DatatypeImplementations::default();
    let mut entry = implementation(&sources[0], item.kind(), item.representation());
    entry.validator = Some((profile.schema.clone(), behavior.clone()));
    implementations.register(entry).unwrap();
    let mut list = datatype_shipped::list_implementation(sources[1].clone(), ty).unwrap();
    if let Some(binding) = tokenizer {
        list.tokenizer = binding;
    }
    implementations.register(list).unwrap();
    implementations
        .select_converter(
            sources[1].clone(),
            ConverterBinding::Ready(datatype_shipped::converter(sources[1].clone(), ty).unwrap()),
        )
        .unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    let mut sig = signature(false);
    sig.kind = item.kind();
    sig.value = item.representation();
    datatype_shipped::register_validation(
        &mut registry,
        item,
        DatatypeBehaviorContract::compile(&profile, &behavior, sig).unwrap(),
        adapter(),
    )
    .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &registry,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let d = compiled(&result, &sources[1]).clone();
    assert_eq!(
        d.item().unwrap().source().declaration().identity(),
        sources[0].declaration().identity()
    );
    d
}

#[test]
fn shipped_lists_keep_original_token_spans_order_duplicates_and_item_validation() {
    for (ty, text, expected) in [
        (T::NameList, "\u{2003}β a β\t", vec!["β", "a", "β"]),
        (T::WildcardNameList, "a:* a a:*", vec!["a:*", "a", "a:*"]),
    ] {
        let d = list_descriptor(ty, None);
        let map = native(d.source().declaration()).source_map().unwrap();
        let input = LexicalInput::new(Arc::from(text), map.clone());
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let result = d.convert(
            &ConversionInput {
                value: ConversionValue::Lexical(input.clone()),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(result.accepted, Some(true), "{result:?}");
        let values = result.value.unwrap();
        for (value, expected) in values.iter().zip(expected) {
            assert_eq!(value.atom(), Some(AtomValue::String(expected.into())));
            let (original, span) = datatype_shipped::token_source(value).unwrap();
            assert!(Arc::ptr_eq(&original.text, &input.text));
            assert_eq!(&original.text[span], expected);
            assert_eq!(value.source_map(), Some(map.clone()));
        }
        assert_eq!(values.len(), 3);
        assert_eq!(result.validation.unwrap().completed.len(), 3);
        assert_eq!(convert(&d, " \t ").accepted, Some(false));
        assert_eq!(convert(&d, "a b:c").accepted, Some(false));
        assert_eq!(convert(&d, "a b c d").accepted, Some(false));
        assert!(datatype_shipped::constant_interpreter(d.source().clone(), ty).is_err());
    }
}

#[test]
fn shipped_list_conversion_uses_descriptor_tokenizer_and_bounded_control() {
    let missing = list_descriptor(T::NameList, Some(TokenizerBinding::Absent));
    assert!(matches!(
        convert(&missing, "a").stopped,
        Some(ConversionStop::Unavailable)
    ));
    let d = list_descriptor(T::NameList, None);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let input = ConversionInput {
        value: ConversionValue::Lexical(LexicalInput::new(Arc::from("a b c"), Default::default())),
        candidate: vec![],
        fallback: Default::default(),
    };
    let mut limits = ConversionLimits::default();
    limits.max_output_values = 2;
    let result = d.convert(&input, &runtime, limits);
    assert!(
        matches!(result.stopped, Some(ConversionStop::Limit(_))),
        "{result:?}"
    );
    assert!(result.value.is_none());
    control.cancel_root(None, None).unwrap();
    assert!(matches!(
        d.convert(&input, &runtime, Default::default()).stopped,
        Some(ConversionStop::Control(_))
    ));
}

#[test]
fn lexical_constants_are_prepared_without_runtime_conversion() {
    for (ty, token) in [
        (T::QualifiedName, "unbound:item"),
        (T::Uri, "https://example.test/a%2Fb"),
        (T::MediaType, "Text/Plain"),
    ] {
        let d = descriptor_with(ty, Some(token));
        assert!(d.converter().is_none());
        assert_eq!(d.enumerations()[0].constants()[0].token.text(), token);
        assert_eq!(
            validate_descriptor(&d, vec![Item::Atomic(AtomValue::String(token.into()))]).accepted,
            Some(true)
        );
        assert_eq!(
            validate_descriptor(
                &d,
                vec![Item::Atomic(AtomValue::String("different".into()))]
            )
            .accepted,
            Some(false)
        );
    }
}

#[test]
fn list_converter_does_not_substitute_a_second_tokenizer() {
    use cem_ml::schema::datatype_contracts::{
        RegisteredTokenizer, TokenizationError, TokenizationRequest, Tokenizer,
    };
    #[derive(Debug)]
    struct Whole;
    impl Tokenizer for Whole {
        fn tokenize(
            &self,
            r: TokenizationRequest<'_>,
        ) -> Result<Vec<std::ops::Range<usize>>, TokenizationError> {
            Ok(vec![0..r.input.text.len()])
        }
    }
    let d = list_descriptor(
        T::NameList,
        Some(TokenizerBinding::Ready(
            RegisteredTokenizer::new("whole", Whole).unwrap(),
        )),
    );
    let result = convert(&d, "a b");
    assert_eq!(result.accepted, Some(false));
    assert_eq!(result.value.unwrap().len(), 1);
    assert_eq!(result.validation.unwrap().completed.len(), 1);
}

#[test]
fn grammar_conversion_validates_syntax_and_retains_typed_output() {
    let d = descriptor(T::ContentModel);
    let result = convert(&d, "  (a | ns:b)*  ");
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(
        result.value.unwrap()[0].atom(),
        Some(AtomValue::String("(a | ns:b)*".into()))
    );
    assert_eq!(result.validation.unwrap().completed.len(), 1);
    let invalid = convert(&d, "(a | )*");
    assert_eq!(invalid.accepted, Some(false));
    assert!(invalid.validation.is_none());
    let deep = format!("{}a{}", "(".repeat(65), ")".repeat(65));
    let result = convert(&d, &deep);
    assert!(
        matches!(result.stopped, Some(ConversionStop::Limit(_))),
        "{result:?}"
    );
    assert!(result.value.is_none());
}

#[test]
fn grammar_constants_keep_existing_token_syntax_without_runtime_converter() {
    let d = descriptor_with(T::ContentModel, Some("(a|b)* text?"));
    assert!(d.converter().is_none());
    assert_eq!(d.enumerations()[0].constants().len(), 2);
    assert_eq!(
        validate_descriptor(&d, vec![Item::Atomic(AtomValue::String("(a|b)*".into()))]).accepted,
        Some(true)
    );
    assert_eq!(
        validate_descriptor(&d, vec![Item::Atomic(AtomValue::String("(a | b)*".into()))]).accepted,
        Some(false)
    );
}
