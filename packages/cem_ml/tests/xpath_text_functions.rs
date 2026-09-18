//! XPATH-DEMO-TEXT/BUDGET: shared limits and standard text semantics.
use cem_ml::{
    diagnostics::Diagnostic,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::*,
};

fn evaluate(
    source: &str,
    limits: XPathEvaluationLimits,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    evaluate_context(source, limits, XPathDynamicContext::default(), None)
}

fn evaluate_context(
    source: &str,
    limits: XPathEvaluationLimits,
    dynamic_context: XPathDynamicContext,
    control: Option<&cem_ml::operation_control::OperationControl>,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:text.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    let registry = ResolverRegistry::new();
    let policy = ResolverPolicy::new();
    let request = XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression: &expression,
        dynamic_context,
        static_context: XPathStaticContext::default(),
        expected_result: None,
        resolver_registry: &registry,
        resolver_policy: &policy,
        evaluation_limits: limits,
        safety_policy_stamp: "text-test",
        module_resolution: None,
    };
    match control {
        Some(control) => CemXPathEvaluator::default().evaluate_with_control(
            request,
            control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        ),
        None => CemXPathEvaluator::default().evaluate(request),
    }
}

fn values(source: &str) -> Vec<(String, String)> {
    evaluate(source, XPathEvaluationLimits::default())
        .unwrap_or_else(|e| panic!("{source}: {e:?}"))
        .sequence
        .items
        .into_iter()
        .map(|item| match item {
            XPathResultItem::Atomic { value, .. } => (value.type_name, value.lexical_value),
            _ => panic!("expected atomic result"),
        })
        .collect()
}

#[test]
fn text_functions_follow_xml_whitespace_codepoints_and_atomic_joining() {
    for (expression, ty, expected) in [
        ("normalize-space('  a\t a\r\nb  ')", "xs:string", "a a b"),
        (
            "normalize-space('a\u{a0}\u{2003}b')",
            "xs:string",
            "a\u{a0}\u{2003}b",
        ),
        ("normalize-space(())", "xs:string", ""),
        ("string-length('🍒e\u{301}')", "xs:integer", "3"),
        ("string-length(())", "xs:integer", "0"),
        (
            "string-join((1, true(), xs:decimal('2.50')), '|')",
            "xs:string",
            "1|true|2.5",
        ),
        ("string-join(('a', '', 'a'), ',')", "xs:string", "a,,a"),
        ("string-join(('a', 'b'))", "xs:string", "ab"),
        ("string-join((), ',')", "xs:string", ""),
        ("count(tokenize('  a\ta\nb  '))", "xs:integer", "3"),
        ("count(tokenize('a\u{a0}b'))", "xs:integer", "1"),
        ("count(tokenize(' \t\n'))", "xs:integer", "0"),
        ("count(tokenize(()))", "xs:integer", "0"),
    ] {
        assert_eq!(
            values(expression),
            vec![(ty.into(), expected.into())],
            "{expression}"
        );
    }
    assert_eq!(
        values("tokenize('a a b')"),
        vec![
            ("xs:string".into(), "a".into()),
            ("xs:string".into(), "a".into()),
            ("xs:string".into(), "b".into())
        ]
    );
}

#[test]
fn text_limits_cover_existing_operations_and_aggregate_sequences() {
    for source in [
        "'abcde'",
        "'abc' || 'def'",
        "('abc', 'def')",
        "string(12345)",
        "xs:string(12345)",
        "format-number(100, '0%')",
    ] {
        let errors = evaluate(
            source,
            XPathEvaluationLimits {
                max_text_bytes: Some(4),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == "cem.xpath.text_byte_limit_exceeded"),
            "{source}: {errors:?}"
        );
        assert!(errors
            .iter()
            .all(|e| e.uri.as_deref() == Some("memory:text.xpath")
                && e.byte_offset.is_some()
                && e.source_map.is_some()));
    }
    let result = evaluate(
        "'🍒'",
        XPathEvaluationLimits {
            max_text_bytes: Some(4),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.safety_policy_stamp.contains("xpath-text-bytes=4"));
    assert!(evaluate(
        "'🍒'",
        XPathEvaluationLimits {
            max_text_bytes: Some(3),
            ..Default::default()
        }
    )
    .is_err());
    let errors = evaluate(
        "string-join(tokenize('one two three'), '-')",
        XPathEvaluationLimits {
            max_work_units: Some(10),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "cem.xpath.work_limit_exceeded"),
        "{errors:?}"
    );
}

#[test]
fn invalid_text_arguments_report_the_argument_and_unsupported_arities_fail_closed() {
    for source in [
        "normalize-space(1)",
        "string-length(true())",
        "tokenize(('a', 'b'))",
        "string-join(('a'), ())",
    ] {
        let errors = evaluate(source, XPathEvaluationLimits::default()).unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("XPTY0004")),
            "{source}: {errors:?}"
        );
        assert!(
            errors[0].byte_offset.unwrap() > 0,
            "argument range: {errors:?}"
        );
    }
    for source in [
        "normalize-space()",
        "string-length()",
    ] {
        let errors = evaluate(source, XPathEvaluationLimits::default()).unwrap_err();
        let code = "cem.xpath.context_item_missing";
        assert_eq!(errors[0].code, code, "{source}: {errors:?}");
    }
}

#[test]
fn intermediate_conversions_and_formatting_cannot_hide_oversized_text() {
    for source in [
        "string-length('abc' || 'def')",
        "string-length(xs:decimal(1e100))",
        "1e100 castable as xs:decimal",
        "string-length(format-number(100, '0%'))",
        "string-length(format-integer(1234, '٠'))",
        "string-join(('aa', 'bb'), '...')",
    ] {
        let errors = evaluate(
            source,
            XPathEvaluationLimits {
                max_text_bytes: Some(5),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            errors[0].code, "cem.xpath.text_byte_limit_exceeded",
            "{source}: {errors:?}"
        );
    }
    let errors = evaluate(
        "format-integer(12345678901234567890, 'A')",
        XPathEvaluationLimits {
            max_work_units: Some(300),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.work_limit_exceeded");
    let errors = evaluate(
        "tokenize('a a a')",
        XPathEvaluationLimits {
            max_sequence_items: Some(2),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.sequence_item_limit_exceeded");
}

#[test]
fn native_xml_atomization_is_bounded_and_preserves_owners_and_text_rules() {
    use cem_ml::{
        lifecycle::LoadedInputAstStream,
        validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
    };
    use std::sync::Arc;
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: b"<r a='a&#x9;b'>a<![CDATA[b]]>&amp;&#x1F352;<i>c</i></r>",
        source_uri: "memory:text.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty());
    let owner = Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()));
    let context = XPathDynamicContext {
        context_item: Some(XPathResultItem::from_native_node(
            XPathNativeNode::xml_document(owner.clone()).unwrap(),
        )),
        ..Default::default()
    };
    for source in [
        "string(.)",
        "data(/r)",
        "normalize-space(/r)",
        "string-length(/r)",
        "xs:string(/r)",
        "/r = 'abc'",
        "/r || ''",
    ] {
        let errors = evaluate_context(
            source,
            XPathEvaluationLimits {
                max_text_bytes: Some(7),
                ..Default::default()
            },
            context.clone(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            errors[0].code, "cem.xpath.text_byte_limit_exceeded",
            "{source}: {errors:?}"
        );
        assert_eq!(errors[0].uri.as_deref(), Some("memory:text.xpath"));
    }
    for (source, expected) in [
        ("string(/r)", "ab&🍒c"),
        ("normalize-space(/r/@a)", "a b"),
        ("string-length(/r)", "5"),
    ] {
        let result = evaluate_context(
            source,
            XPathEvaluationLimits::default(),
            context.clone(),
            None,
        )
        .unwrap();
        let XPathResultItem::Atomic { value, .. } = &result.sequence.items[0] else {
            panic!()
        };
        assert_eq!(value.lexical_value, expected);
    }
    let result = evaluate_context(
        "/r",
        XPathEvaluationLimits {
            max_text_bytes: Some(0),
            ..Default::default()
        },
        context,
        None,
    )
    .unwrap();
    assert!(Arc::ptr_eq(
        result.sequence.items[0]
            .native_node()
            .unwrap()
            .source_owner()
            .as_ref()
            .unwrap(),
        &owner
    ));
}

#[test]
fn binding_copies_default_context_and_control_share_the_contract() {
    let item = XPathResultItem::Atomic {
        value: XPathAtomicValue {
            type_name: "xs:string".into(),
            lexical_value: "  a  🍒  ".into(),
            namespace_uri: None,
            local_name: None,
        },
        source_map: Default::default(),
    };
    let context = XPathDynamicContext {
        context_item: Some(item.clone()),
        variable_bindings: std::collections::BTreeMap::from([(
            XPathExpandedName::new(None::<String>, "text"),
            XPathResultSequence {
                sequence_type: "xs:string".into(),
                items: vec![item],
            },
        )]),
        ..Default::default()
    };
    for source in ["normalize-space()", "string-length()", "$text", "."] {
        let errors = evaluate_context(
            source,
            XPathEvaluationLimits {
                max_text_bytes: Some(4),
                ..Default::default()
            },
            context.clone(),
            None,
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "cem.xpath.text_byte_limit_exceeded");
    }
    let result = evaluate_context(
        "normalize-space()",
        XPathEvaluationLimits::default(),
        context.clone(),
        None,
    )
    .unwrap();
    let XPathResultItem::Atomic { value, .. } = &result.sequence.items[0] else {
        panic!()
    };
    assert_eq!(value.lexical_value, "a 🍒");
    let control = cem_ml::operation_control::OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let errors = evaluate_context(
        "string-join(tokenize($text), '-')",
        XPathEvaluationLimits::default(),
        context,
        Some(&control),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.control_failure");
}

#[test]
fn scope_budget_names_propagate_independently_to_query_execution() {
    use cem_ml::{
        query::{query_execution_limits, QueryLanguage},
        run_config::ScopeConfig,
    };
    let scope = ScopeConfig {
        budgets: std::collections::BTreeMap::from([
            ("xpathItems".into(), "16".into()),
            ("xpathTextBytes".into(), "128".into()),
            ("xpathWorkUnits".into(), "1024".into()),
        ]),
        ..Default::default()
    };
    assert_eq!(scope.xpath_items_budget().unwrap(), Some(16));
    assert_eq!(scope.xpath_text_bytes_budget().unwrap(), Some(128));
    assert_eq!(scope.xpath_work_units_budget().unwrap(), Some(1024));
    let limits = query_execution_limits(QueryLanguage::XPath, &scope).unwrap();
    assert_eq!(
        (
            limits.max_result_items,
            limits.max_text_bytes,
            limits.max_work_units
        ),
        (Some(16), Some(128), Some(1024))
    );
    let mut invalid = scope;
    invalid.budgets.insert("xpathTextBytes".into(), "no".into());
    assert!(invalid.xpath_text_bytes_budget().is_err());
    assert!(query_execution_limits(QueryLanguage::XPath, &invalid).is_err());
}
