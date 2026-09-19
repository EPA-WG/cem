//! XSLT group context is supplied by the host, never captured by functions.
use super::*;

pub(super) fn validate(
    group: Option<&XPathXsltGroupContext>,
    host: XPathInvocationHost,
    range: XPathSourceRange,
    runtime: &mut XPathEvaluationRuntime,
) -> Result<(), XPathEvaluationError> {
    if let Some(group) = group {
        if host != XPathInvocationHost::Xslt {
            return Err(XPathEvaluationError::dynamic(
                "cem.xpath.xslt_context_host",
                "XSLT group context requires the XSLT invocation host",
                range,
            ));
        }
        if let Some(key) = &group.current_grouping_key {
            runtime.enforce_sequence_items(key.items.len(), range)?;
            for item in &key.items {
                runtime.poll(range)?;
                if !matches!(item, XPathResultItem::Atomic { .. }) {
                    return Err(XPathEvaluationError::dynamic(
                        "cem.xpath.xslt_group_key_type",
                        "XSLT grouping keys must contain only atomic values",
                        range,
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn evaluate(
    function: XPathNativeFunction,
    focus: XPathFocus<'_>,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    if !focus.xslt_host {
        return Err(XPathEvaluationError::unsupported(
            "XSLT group functions require the XSLT invocation host",
            range,
        ));
    }
    let (value, code, message) = if function == XPathNativeFunction::CurrentGroup {
        (
            focus.xslt_group.and_then(|g| g.current_group.as_ref()),
            "cem.xslt.current_group_absent",
            "err:XTDE1061: the current group is absent",
        )
    } else {
        (
            focus
                .xslt_group
                .and_then(|g| g.current_grouping_key.as_ref()),
            "cem.xslt.current_grouping_key_absent",
            "err:XTDE1071: the current grouping key is absent",
        )
    };
    let value = value.ok_or_else(|| XPathEvaluationError::dynamic(code, message, range))?;
    runtime.enforce_sequence_items(value.items.len(), range)?;
    runtime.check_items_text(&value.items, range)?;
    text::charge_items_copy(&value.items, runtime, range)?;
    Ok(value.items.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(items: Vec<XPathResultItem>) -> XPathResultSequence {
        XPathResultSequence {
            sequence_type: "item()*".into(),
            items,
        }
    }

    fn context() -> XPathXsltGroupContext {
        let tree = crate::import::import_data("<r><n>A</n></r>", "xml", "cem", "memory:group-data")
            .unwrap();
        XPathXsltGroupContext {
            current_group: Some(sequence(vec![XPathResultItem::from_native_node(
                XPathNativeNode::cem_document(tree),
            )])),
            current_grouping_key: Some(sequence(Vec::new())),
        }
    }

    fn evaluate(
        code: &str,
        context: Option<XPathXsltGroupContext>,
        host: XPathInvocationHost,
        limit: Option<u64>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
        let expression = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: code.as_bytes(),
                source_uri: "memory:group-expression",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::StandaloneStaticContext {
                source_id: 1,
                static_context: Default::default(),
            },
        );
        CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
            expression: &expression,
            invocation_host: host,
            dynamic_context: XPathDynamicContext {
                xslt_group: context,
                ..Default::default()
            },
            static_context: Default::default(),
            expected_result: None,
            resolver_registry: &ResolverRegistry::new(),
            resolver_policy: &ResolverPolicy::new(),
            evaluation_limits: XPathEvaluationLimits {
                max_sequence_items: limit,
                ..Default::default()
            },
            safety_policy_stamp: "test",
            module_resolution: None,
        })
    }

    #[test]
    fn native_group_survives_focus_changes_and_preserves_node_identity() {
        let group = context();
        let expected = group.current_group.as_ref().unwrap().items[0].clone();
        for code in [
            "current-group()",
            "(1, 2)[count(current-group()) = 1] ! current-group()",
            "current-group()/*[count(current-group()) = 1]/..",
        ] {
            let result =
                evaluate(code, Some(group.clone()), XPathInvocationHost::Xslt, None).unwrap();
            assert!(!result.sequence.items.is_empty());
            assert!(result.sequence.items.iter().all(|v| v == &expected));
        }
    }

    #[test]
    fn absent_group_is_lazy_and_distinct_from_present_empty() {
        let mut empty = context();
        empty.current_group = Some(sequence(Vec::new()));
        assert!(evaluate(
            "current-group()",
            Some(empty),
            XPathInvocationHost::Xslt,
            None
        )
        .unwrap()
        .sequence
        .items
        .is_empty());
        let result = evaluate(
            "current-grouping-key()",
            Some(context()),
            XPathInvocationHost::Xslt,
            None,
        )
        .unwrap();
        assert!(result.sequence.items.is_empty());
        assert!(evaluate(
            "if (false()) then current-group() else ()",
            None,
            XPathInvocationHost::Xslt,
            None
        )
        .unwrap()
        .sequence
        .items
        .is_empty());
        for (code, error) in [
            ("current-group()", "XTDE1061"),
            ("current-grouping-key()", "XTDE1071"),
        ] {
            let diagnostics = evaluate(code, None, XPathInvocationHost::Xslt, None).unwrap_err();
            assert!(diagnostics[0].message.contains(error), "{diagnostics:?}");
            assert_eq!(
                diagnostics[0].uri.as_deref(),
                Some("memory:group-expression")
            );
            assert!(diagnostics[0].source_map.is_some());
        }
    }

    #[test]
    fn function_bodies_clear_groups_but_explicit_variables_remain_lexical() {
        let result = evaluate(
            "let $g := current-group() return (function() { $g })()",
            Some(context()),
            XPathInvocationHost::Xslt,
            None,
        )
        .unwrap();
        assert_eq!(result.sequence.items.len(), 1);
        for (code, error) in [
            ("(function() { current-group() })()", "XTDE1061"),
            ("(function() { current-grouping-key() })()", "XTDE1071"),
        ] {
            let diagnostics =
                evaluate(code, Some(context()), XPathInvocationHost::Xslt, None).unwrap_err();
            assert!(diagnostics[0].message.contains(error), "{diagnostics:?}");
        }
    }

    #[test]
    fn context_is_host_specific_and_access_obeys_resource_limits() {
        for host in [
            XPathInvocationHost::Query,
            XPathInvocationHost::Cemt,
            XPathInvocationHost::StandaloneTransform,
        ] {
            assert!(evaluate("current-group()", None, host, None).is_err());
            assert!(evaluate("1", Some(context()), host, None).is_err());
        }
        let errors = evaluate(
            "current-group()",
            Some(context()),
            XPathInvocationHost::Xslt,
            Some(0),
        )
        .unwrap_err();
        assert!(errors[0].code.contains("limit"), "{errors:?}");
        let mut invalid = context();
        invalid.current_grouping_key = invalid.current_group.clone();
        assert!(evaluate("1", Some(invalid), XPathInvocationHost::Xslt, None).is_err());
    }
}
