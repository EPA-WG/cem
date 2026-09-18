//! XSLT-BUNDLE-FOCUS-GATE: characterize the missing host focus before fixing
//! the bundle invocation contract. These are boundary probes, not acceptance
//! tests for a conforming XSLT loop. Replace the singleton expectation when
//! explicit host position/size support is approved and implemented.
use cem_ml::{
    content_cache::ContentHash,
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

fn evaluate(expression: &XPathExpressionAst, context: &XPathResultItem) -> Vec<XPathResultItem> {
    let XPathAttachment::Host(host) = &expression.attachment else {
        panic!("XSLT attachment must survive reload")
    };
    XsltXPathInvocationAdapter
        .invoke(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::Xslt,
            expression,
            dynamic_context: XPathDynamicContext {
                context_item: Some(context.clone()),
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
        .unwrap()
        .sequence
        .items
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
fn per_item_host_calls_cannot_supply_the_xslt_iteration_position_and_size() {
    let selectors = programs("/r/item");
    let labels = programs(LABEL);
    for source in [
        "<r><item>A</item><item>B</item></r>",
        "<r><item>C</item><item>D</item><item>E</item></r>",
    ] {
        let root = document(source);
        for (selector, label) in selectors.iter().zip(&labels) {
            let items = evaluate(selector, &root);
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
                let result = strings(evaluate(label, item));
                assert_eq!(result, [format!("{text}:1/1")]);
                actual.extend(result);
                required.push(format!("{text}:{}/{size}", index + 1));
            }
            assert_ne!(
                actual, required,
                "the bundle cannot promise XSLT loop focus yet"
            );
        }
    }
}

#[test]
fn expression_local_map_and_predicate_focus_work_before_and_after_reload() {
    let select = format!(
        "/r/item ! string-join(({LABEL}, ':', string((10, 20, 30)[position() = last()]), ':', string(position()), '/', string(last())), '')"
    );
    for expression in programs(&select) {
        assert_eq!(
            strings(evaluate(
                &expression,
                &document("<r><item>A</item><item>B</item></r>")
            )),
            ["A:1/2:30:1/2", "B:2/2:30:2/2"],
            "inner predicates establish their own focus and restore the simple-map focus"
        );
    }
}
