//! XPATH-HOST-FOCUS: XSLT-owned programs accept explicit host focus before and
//! after binary reload. This verifies the invocation contract, not the pending
//! stylesheet-to-CEMT loop compiler or bundle loader.
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::Diagnostic,
    import::import_data,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::{
        xpath::{
            artifact::{XPathArtifactLoadContext, XPathCompiledArtifact},
            XPathAttachment, XPathDynamicContext, XPathEvaluationRequest, XPathExpressionAst,
            XPathInvocationAdapter, XPathInvocationHost, XPathNativeNode, XPathResultItem,
            XsltXPathInvocationAdapter,
        },
        xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    },
};

const LABEL: &str = "string-join((string(.), ':', string(position()), '/', string(last())), '')";

fn programs(select: &str) -> [XPathExpressionAst; 2] {
    // This is stylesheet authoring input. Runtime documents below use only
    // the shared CEM import boundary, never a format-specific query view.
    let source = format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><xsl:for-each select="/r/item"><xsl:value-of select="{select}"/></xsl:for-each></xsl:template></xsl:stylesheet>"#
    );
    let (stylesheet, diagnostics) =
        xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:bundle-focus.xslt",
            content_type: Some("application/xslt+xml"),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let expression = stylesheet
        .unwrap()
        .xpath_expressions
        .into_iter()
        .last()
        .expect("typed value-of select")
        .expression;
    let hash = ContentHash::from_blake3(source.as_bytes());
    let compiled = XPathCompiledArtifact::compile(&expression, hash.clone()).unwrap();
    let reloaded =
        XPathCompiledArtifact::from_bytes(compiled.bytes().to_vec(), compiled.content_hash())
            .unwrap()
            .reload(&XPathArtifactLoadContext {
                expected_source_hash: hash,
                invocation_host: XPathInvocationHost::Xslt,
            })
            .unwrap();
    assert!(reloaded.source_text.is_none());
    assert!(reloaded.tokens.is_empty());
    assert_eq!(reloaded.source, expression.source);
    assert_eq!(reloaded.attachment, expression.attachment);
    [expression, reloaded]
}

fn document(source: &str) -> XPathResultItem {
    XPathResultItem::from_native_node(XPathNativeNode::cem_document(
        import_data(source, "xml", "cem", "memory:bundle-focus-input.xml").unwrap(),
    ))
}

fn evaluate(
    expression: &XPathExpressionAst,
    context: &XPathResultItem,
    coordinates: Option<(u64, u64)>,
) -> Result<Vec<XPathResultItem>, Vec<Diagnostic>> {
    let XPathAttachment::Host(host) = &expression.attachment else {
        panic!("XSLT attachment must survive reload")
    };
    XsltXPathInvocationAdapter
        .invoke(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::Xslt,
            expression,
            dynamic_context: XPathDynamicContext {
                context_item: Some(context.clone()),
                context_position: coordinates.map(|(position, _)| position),
                context_size: coordinates.map(|(_, size)| size),
                ..Default::default()
            },
            static_context: host.static_context.clone(),
            expected_result: host.expected_result.clone(),
            resolver_registry: &ResolverRegistry::new(),
            resolver_policy: &ResolverPolicy::new(),
            evaluation_limits: Default::default(),
            safety_policy_stamp: "xslt-bundle-focus-gate",
            module_resolution: None,
        })
        .map(|result| result.sequence.items)
}

fn strings(items: Vec<XPathResultItem>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| match item {
            XPathResultItem::Atomic { value, .. } => value.lexical_value,
            _ => panic!("expected an atomic result"),
        })
        .collect()
}

#[test]
fn per_item_host_calls_supply_the_xslt_iteration_position_and_size() {
    let selectors = programs("/r/item");
    let labels = programs(LABEL);
    for source in [
        "<r><item>A</item><item>B</item></r>",
        "<r><item>C</item><item>D</item><item>E</item></r>",
    ] {
        let root = document(source);
        for (selector, label) in selectors.iter().zip(&labels) {
            let items = evaluate(selector, &root, None).unwrap();
            let size = items.len();
            let mut actual = Vec::new();
            let mut required = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let node = item.native_node().unwrap();
                assert!(std::sync::Arc::ptr_eq(
                    node.owner(),
                    root.native_node().unwrap().owner(),
                ));
                let text = node.string_value();
                assert_eq!(
                    strings(evaluate(label, item, None).unwrap()),
                    [format!("{text}:1/1")]
                );
                let result =
                    strings(evaluate(label, item, Some((index as u64 + 1, size as u64))).unwrap());
                actual.extend(result);
                required.push(format!("{text}:{}/{size}", index + 1));
            }
            assert_eq!(
                actual, required,
                "the host can supply XSLT loop focus without rewriting XPath"
            );
        }
    }
}

#[test]
fn expression_local_map_and_predicate_focus_work_before_and_after_reload() {
    let select = format!(
        "(/r/item ! string-join(({LABEL}, ':', string((10, 20, 30)[position() = last()]), ':', string(position()), '/', string(last())), ''), position(), last())"
    );
    for expression in programs(&select) {
        assert_eq!(
            strings(
                evaluate(
                    &expression,
                    &document("<r><item>A</item><item>B</item></r>"),
                    Some((4, 9)),
                )
                .unwrap()
            ),
            ["A:1/2:30:1/2", "B:2/2:30:2/2", "4", "9"],
            "inner predicates establish their own focus and restore the simple-map focus"
        );
    }
}

#[test]
fn invalid_host_focus_keeps_the_original_stylesheet_range_after_reload() {
    for expression in programs("42") {
        let errors = evaluate(&expression, &document("<r/>"), Some((3, 2))).unwrap_err();
        assert_eq!(errors[0].code, "cem.xpath.focus_invalid");
        assert_eq!(errors[0].uri.as_deref(), Some("memory:bundle-focus.xslt"));
        let range = expression.syntax_ast.as_ref().unwrap().root.source_range;
        assert!(range.start.byte_offset > 0);
        assert_eq!(errors[0].byte_offset, Some(range.start.byte_offset));
        assert!(!errors[0].source_map.as_ref().unwrap().frames.is_empty());
    }
}
