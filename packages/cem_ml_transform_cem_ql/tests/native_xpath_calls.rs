//! XSLT-VIEW-NATIVE-CEMT: a typed XSLT XPath slot uses the generic runtime hook.
//! This is an integration fixture, not the still-pending XSLT-to-CEMT compiler.
use cem_ml::{
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::{
        xpath::{
            XPathAttachment, XPathDynamicContext, XPathEvaluationLimits, XPathEvaluationRequest,
            XPathExpressionAst, XPathInvocationHost, XPathNativeNode, XsltXPathInvocationAdapter,
        },
        xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    },
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{Item, ItemStream},
    native::{NativeFunctionRegistry, NativeQueryFunction, NativeQueryRequest},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    xpath::functions::XPathQueryItem,
};
use std::sync::Arc;

#[derive(Debug)]
struct Select(Arc<XPathExpressionAst>);
impl NativeQueryFunction for Select {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let [Item::Native(input)] = request.arguments[0].items.as_slice() else {
            return request.raise("fixture.input", "expected a native XPath node");
        };
        let input = input
            .downcast_ref::<XPathQueryItem>()
            .expect("retained native owner");
        let XPathAttachment::Host(host) = &self.0.attachment else {
            panic!("typed XSLT host required")
        };
        let registry = ResolverRegistry::new();
        let policy = ResolverPolicy::new();
        let result = XsltXPathInvocationAdapter
            .invoke_with_control(
                XPathEvaluationRequest {
                    invocation_host: XPathInvocationHost::Xslt,
                    expression: &self.0,
                    dynamic_context: XPathDynamicContext {
                        context_item: Some(input.xpath_item().clone()),
                        ..Default::default()
                    },
                    static_context: host.static_context.clone(),
                    expected_result: host.expected_result.clone(),
                    resolver_registry: &registry,
                    resolver_policy: &policy,
                    evaluation_limits: XPathEvaluationLimits {
                        max_sequence_items: Some(request.max_result_items),
                        ..Default::default()
                    },
                    safety_policy_stamp: "xslt-native-call-fixture",
                    module_resolution: request.module_resolution,
                },
                request.control,
                request.scope,
            )
            .expect("bounded XPath evaluates over the original owner");
        ItemStream::from_items(
            result
                .sequence
                .items
                .into_iter()
                .map(|item| {
                    XPathQueryItem::from_node(
                        item.native_node()
                            .expect("select returns native text")
                            .clone(),
                    )
                })
                .collect(),
        )
    }
}

fn expression() -> Arc<XPathExpressionAst> {
    let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
        bytes: br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><xsl:value-of select="/r/text()"/></xsl:template></xsl:stylesheet>"#,
        source_uri: "memory:native-call.xsl", content_type: Some("application/xslt+xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    Arc::new(
        stylesheet
            .unwrap()
            .xpath_expressions
            .into_iter()
            .find(|slot| slot.attribute_name == "select")
            .unwrap()
            .expression,
    )
}

fn input(source: &str) -> Item {
    XPathQueryItem::from_node(XPathNativeNode::cem_document(
        cem_ml::import::import_data(source, "xml", "cem", "memory:native-call.xml").unwrap(),
    ))
}

#[test]
fn cemt_native_hook_executes_the_same_xslt_ast_against_changed_xml_owners() {
    let expression = expression();
    let mut functions = NativeFunctionRegistry::default();
    functions
        .register("urn:fixture:select-1", 1, Select(expression.clone()))
        .unwrap();
    let artifact = compile_template(
        r#"{p | {$native:call("urn:fixture:select-1", input)}}"#,
        &CompileTemplateOptions {
            host_bindings: vec!["input".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for (xml, expected) in [
        ("<r>a<![CDATA[b]]>&amp;</r>", "<p>ab&amp;</p>"),
        ("<r>new<![CDATA[value]]></r>", "<p>newvalue</p>"),
    ] {
        let item = input(xml);
        let bindings =
            std::collections::BTreeMap::from([("input".into(), ItemStream::once(item.clone()))]);
        let query = compile(
            r#"native:call("urn:fixture:select-1", input)"#,
            &CompileContext {
                policy_bindings: bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let result = evaluate(
            &query,
            &EvaluationContext {
                policy_bindings: bindings.clone(),
                native_functions: functions.clone(),
                ..Default::default()
            },
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items.len(),
            1,
            "XPath coalesces text/CDATA/entity events"
        );
        let Item::Native(original) = &item else {
            unreachable!()
        };
        let Item::Native(returned) = &result.items[0] else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(
            original
                .downcast_ref::<XPathQueryItem>()
                .unwrap()
                .xpath_item()
                .native_node()
                .unwrap()
                .owner(),
            returned
                .downcast_ref::<XPathQueryItem>()
                .unwrap()
                .xpath_item()
                .native_node()
                .unwrap()
                .owner()
        ));
        assert!(result.items[0].source_map().is_some());
        let plan = render_compiled_template(
            &artifact,
            &TemplateData {
                bindings,
                native_functions: functions.clone(),
                ..Default::default()
            },
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan), expected);
    }
    assert!(
        Arc::strong_count(&expression) > 1,
        "the callback owns the compiled XPath AST"
    );
}
