//! XPATH-HOST-FOCUS: caller-supplied focus is runtime state, not program IR.
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::Diagnostic,
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{artifact::*, *},
};

fn parse(source: &str) -> XPathExpressionAst {
    xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:host-focus.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    )
}

fn context(position: Option<u64>, size: Option<u64>) -> XPathDynamicContext {
    XPathDynamicContext {
        context_item: Some(XPathResultItem::from_native_node(
            XPathNativeNode::cem_document(
                import_data(
                    "<r><item>A</item><item>B</item></r>",
                    "xml",
                    "cem",
                    "memory:focus.xml",
                )
                .unwrap(),
            ),
        )),
        context_position: position,
        context_size: size,
        ..Default::default()
    }
}

fn evaluate(
    expression: &XPathExpressionAst,
    dynamic_context: XPathDynamicContext,
    limits: XPathEvaluationLimits,
    control: &OperationControl,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    CemXPathEvaluator::default().evaluate_with_control(
        XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::StandaloneTransform,
            expression,
            dynamic_context,
            static_context: Default::default(),
            expected_result: None,
            resolver_registry: &ResolverRegistry::new(),
            resolver_policy: &ResolverPolicy::new(),
            evaluation_limits: limits,
            safety_policy_stamp: "host-focus-test",
            module_resolution: None,
        },
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
}

fn values(source: &str, context: XPathDynamicContext) -> Vec<String> {
    evaluate(
        &parse(source),
        context,
        Default::default(),
        &Default::default(),
    )
    .unwrap_or_else(|errors| panic!("{source}: {errors:?}"))
    .sequence
    .items
    .into_iter()
    .map(|item| match item {
        XPathResultItem::Atomic { value, .. } => value.lexical_value,
        other => panic!("expected atomic value, got {other:?}"),
    })
    .collect()
}

#[test]
fn omitted_coordinates_preserve_singleton_and_absent_focus() {
    assert_eq!(
        values("(position(), last())", context(None, None)),
        ["1", "1"]
    );
    assert_eq!(values("42", Default::default()), ["42"]);
    for source in [".", "position()", "last()"] {
        let errors = evaluate(
            &parse(source),
            Default::default(),
            Default::default(),
            &Default::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "cem.xpath.context_item_missing");
    }
}

#[test]
fn explicit_focus_is_exact_u64_metadata_not_a_materialized_sequence() {
    let expression = parse("(position(), last())");
    for (position, size) in [(1, 1), (2, 9), (1, u64::MAX), (u64::MAX, u64::MAX)] {
        let result = evaluate(
            &expression,
            context(Some(position), Some(size)),
            XPathEvaluationLimits {
                max_sequence_items: Some(2),
                ..Default::default()
            },
            &Default::default(),
        )
        .unwrap();
        assert_eq!(result.sequence.items.len(), 2);
        for (item, expected) in result.sequence.items.iter().zip([position, size]) {
            let XPathResultItem::Atomic { value, .. } = item else {
                panic!()
            };
            assert_eq!(value.type_name, "xs:integer");
            assert_eq!(value.lexical_value, expected.to_string());
        }
    }
}

#[test]
fn invalid_or_partial_focus_fails_before_even_a_focus_independent_expression() {
    let expression = parse("42");
    for present in [false, true] {
        for (position, size) in [
            (Some(0), Some(1)),
            (Some(1), Some(0)),
            (Some(2), Some(1)),
            (Some(1), None),
            (None, Some(1)),
            (Some(u64::MAX), Some(u64::MAX - 1)),
            (Some(1), Some(1)),
        ] {
            if present && position == Some(1) && size == Some(1) {
                continue;
            }
            let mut dynamic = context(position, size);
            if !present {
                dynamic.context_item = None;
            }
            let errors = evaluate(
                &expression,
                dynamic,
                Default::default(),
                &Default::default(),
            )
            .unwrap_err();
            assert_eq!(errors[0].code, "cem.xpath.focus_invalid");
            assert_eq!(errors[0].uri.as_deref(), Some("memory:host-focus.xpath"));
            assert_eq!(errors[0].byte_offset, Some(0));
            assert!(errors[0]
                .source_map
                .as_ref()
                .is_some_and(|map| !map.frames.is_empty()));
        }
    }
}

#[test]
fn predicates_and_maps_override_then_restore_host_focus() {
    assert_eq!(values(
        "(position(), last(), (10,20,30)[position() = last()], position(), last(), (7,8) ! (position(), last()), position(), last())",
        context(Some(2), Some(9)),
    ), ["2", "9", "30", "2", "9", "1", "2", "2", "2", "2", "9"]);
    assert_eq!(
        values(
            "(/r/item ! (string(.), position(), last()), position(), last())",
            context(Some(2), Some(9)),
        ),
        ["A", "1", "2", "B", "2", "2", "2", "9"]
    );
}

#[test]
fn inline_functions_do_not_capture_caller_focus_but_can_capture_explicit_values() {
    for body in [".", "position()", "last()"] {
        let expression = parse(&format!("(function() {{ {body} }})()"));
        let errors = evaluate(
            &expression,
            context(Some(2), Some(9)),
            Default::default(),
            &Default::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "cem.xpath.context_item_missing");
    }
    assert_eq!(values(
        "let $p := position(), $s := last(), $f := function() { ($p, $s) } return ($f(), position(), last())",
        context(Some(2), Some(9)),
    ), ["2", "9", "2", "9"]);
    assert_eq!(
        values(
            "((function() { (4,5) ! position() })(), position(), last())",
            context(Some(2), Some(9)),
        ),
        ["1", "2", "2", "9"]
    );
}

#[test]
fn reloaded_programs_accept_changed_focus_without_capturing_nodes_or_coordinates() {
    let source = "(., position(), last())";
    let original = parse(source);
    let hash = ContentHash::from_blake3(source.as_bytes());
    let compiled = XPathCompiledArtifact::compile(&original, hash.clone()).unwrap();
    let bytes = compiled.bytes().to_vec();
    let loaded = XPathCompiledArtifact::from_bytes(bytes.clone(), compiled.content_hash())
        .unwrap()
        .reload(&XPathArtifactLoadContext {
            expected_source_hash: hash.clone(),
            invocation_host: XPathInvocationHost::StandaloneTransform,
        })
        .unwrap();
    assert!(loaded.source_text.is_none());
    assert!(loaded.tokens.is_empty());
    for (position, size) in [(2, 9), (u64::MAX, u64::MAX)] {
        let dynamic = context(Some(position), Some(size));
        let owner = dynamic
            .context_item
            .as_ref()
            .unwrap()
            .native_node()
            .unwrap()
            .owner()
            .clone();
        let result = evaluate(&loaded, dynamic, Default::default(), &Default::default()).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            result.sequence.items[0].native_node().unwrap().owner(),
            &owner
        ));
        let coords: Vec<_> = result.sequence.items[1..]
            .iter()
            .map(|item| match item {
                XPathResultItem::Atomic { value, .. } => value.lexical_value.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(coords, [position.to_string(), size.to_string()]);
        assert_eq!(
            XPathCompiledArtifact::compile(&loaded, hash.clone())
                .unwrap()
                .bytes(),
            bytes
        );
    }
}

#[test]
fn host_focus_preserves_work_item_limits_and_cancellation() {
    let expression = parse("(position(), last())");
    for (limits, code) in [
        (
            XPathEvaluationLimits {
                max_sequence_items: Some(1),
                ..Default::default()
            },
            "cem.xpath.sequence_item_limit_exceeded",
        ),
        (
            XPathEvaluationLimits {
                max_work_units: Some(0),
                ..Default::default()
            },
            "cem.xpath.work_limit_exceeded",
        ),
    ] {
        let errors = evaluate(
            &expression,
            context(Some(2), Some(9)),
            limits,
            &Default::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, code);
    }
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let errors = evaluate(
        &expression,
        context(Some(2), Some(9)),
        Default::default(),
        &control,
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.control_failure");
}
