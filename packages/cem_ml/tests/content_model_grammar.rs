use cem_ml::schema::document_model::content_model::{
    validate_with_check, GrammarError, GrammarLimits,
};

#[test]
fn grammar_accepts_shipped_forms_and_rejects_malformed_expressions() {
    for text in [
        "",
        " \t",
        "text",
        "head? body+ *",
        "ns:éclair*",
        "(animation-name-slot | animation-value-slot)*",
        "a | b c",
        "(a (b | c)?)+",
    ] {
        assert!(
            validate_with_check(text, GrammarLimits::default(), &mut || Ok::<_, ()>(())).is_ok(),
            "{text}"
        );
    }
    for text in [
        "a**",
        "a?+",
        "**",
        "()",
        "(a",
        "a)",
        "| a",
        "a |",
        "a || b",
        "(a | )",
        "a,b",
        "1name",
        "a:b:c",
        "a(b)",
        "a ?",
        "https://example.test",
    ] {
        assert!(
            matches!(
                validate_with_check(text, GrammarLimits::default(), &mut || Ok::<_, ()>(())),
                Err(GrammarError::Invalid { .. })
            ),
            "{text}"
        );
    }
}
#[test]
fn grammar_bounds_and_cancellation_are_distinct_from_invalid_syntax() {
    let mut limits = GrammarLimits::default();
    limits.max_bytes = 2;
    assert!(matches!(
        validate_with_check("abc", limits, &mut || Ok::<_, ()>(())),
        Err(GrammarError::Limit("bytes"))
    ));
    limits = GrammarLimits::default();
    limits.max_tokens = 2;
    assert!(matches!(
        validate_with_check("a b c", limits, &mut || Ok::<_, ()>(())),
        Err(GrammarError::Limit("tokens"))
    ));
    limits = GrammarLimits::default();
    limits.max_depth = 1;
    assert!(matches!(
        validate_with_check("((a))", limits, &mut || Ok::<_, ()>(())),
        Err(GrammarError::Limit("depth"))
    ));
    assert!(matches!(
        validate_with_check("a", GrammarLimits::default(), &mut || Err("stop")),
        Err(GrammarError::Interrupted("stop"))
    ));
}
#[test]
fn grammar_retains_decoded_error_offsets_and_polls_during_work() {
    let error = validate_with_check("éclair | )", GrammarLimits::default(), &mut || {
        Ok::<_, ()>(())
    })
    .unwrap_err();
    assert_eq!(error, GrammarError::Invalid { span: 10..11 });
    let mut checks = 0;
    let result = validate_with_check(&"name ".repeat(1000), GrammarLimits::default(), &mut || {
        checks += 1;
        if checks >= 4 {
            Err("cancel during scan")
        } else {
            Ok(())
        }
    });
    assert!(matches!(
        result,
        Err(GrammarError::Interrupted("cancel during scan"))
    ));
}

#[test]
fn every_shipped_children_expression_is_admitted() {
    fn visit(path: &std::path::Path, count: &mut usize) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "dist" {
                    visit(&path, count);
                }
            } else if path.extension().is_some_and(|s| s == "cem") {
                let text = std::fs::read_to_string(&path).unwrap();
                for rest in text.split("@children=\"").skip(1) {
                    let value = rest.split('"').next().unwrap();
                    assert!(
                        validate_with_check(value, GrammarLimits::default(), &mut || Ok::<_, ()>(
                            ()
                        ))
                        .is_ok(),
                        "{}: {value}",
                        path.display()
                    );
                    *count += 1;
                }
            }
        }
    }
    let mut count = 0;
    visit(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schema-packages"),
        &mut count,
    );
    assert!(count > 100);
}
#[test]
fn uri_contract_remains_absolute_and_prose_matches() {
    use cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype as T;
    for value in [
        "https://example.test/a",
        "urn:example:item",
        "cem+test://source",
    ] {
        assert_eq!(T::Uri.validate_lexical(value), Some(true));
    }
    for value in [
        "./a",
        "../a",
        "/a",
        "//example.test/a",
        "#part",
        "?q=1",
        "C:\\a",
    ] {
        assert_eq!(T::Uri.validate_lexical(value), Some(false));
    }
    let source = include_str!("../schema-packages/cem-ml/v1/schema/cem-ml-generic.cem");
    assert!(source.contains("@rule=\"absolute URI with scheme\""));
    assert!(!source.contains("absolute or relative URI reference"));
}
