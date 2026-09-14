//! XSLT-VIEW-NATIVE-CEMT: a typed XSLT XPath slot uses the generic runtime hook.
//! This is an integration fixture, not the still-pending XSLT-to-CEMT compiler.
use cem_ml::{
    lifecycle::LoadedInputAstStream,
    resolver::{ResolverPolicy, ResolverRegistry},
    source_map::SourceMapStack,
    validation::{
        xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
        xpath::{
            XPathAttachment, XPathDynamicContext, XPathEvaluationLimits, XPathEvaluationRequest,
            XPathExpressionAst, XPathInvocationHost, XPathNativeNode, XPathResultItem,
            XsltXPathInvocationAdapter,
        },
        xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    },
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, Item, ItemStream, QueryItemView, QueryItemViewKind},
    native::{NativeFunctionRegistry, NativeQueryFunction, NativeQueryRequest},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
};
use std::{any::Any, sync::Arc};

#[derive(Debug)]
struct XmlNode(XPathNativeNode);
impl QueryItemView for XmlNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "test.xslt.xpath-node"
    }
    fn identity(&self) -> String {
        format!("{:p}:{:?}", Arc::as_ptr(self.0.owner()), self.0.handle())
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(self.0.source_map())
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(AtomValue::String(self.0.string_value()))
    }
}

#[derive(Debug)]
struct Select(Arc<XPathExpressionAst>);
impl NativeQueryFunction for Select {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let [Item::Native(input)] = request.arguments[0].items.as_slice() else {
            return request.raise("fixture.input", "expected a native XPath node");
        };
        let input = input
            .downcast_ref::<XmlNode>()
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
                        context_item: Some(XPathResultItem::from_native_node(input.0.clone())),
                        ..Default::default()
                    },
                    static_context: host.static_context.clone(),
                    expected_result: host.expected_result.clone(),
                    resolver_registry: &registry,
                    resolver_policy: &policy,
                    evaluation_limits: XPathEvaluationLimits {
                        max_sequence_items: Some(request.max_result_items),
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
                    Item::native(XmlNode(
                        item.native_node()
                            .expect("select returns native text")
                            .clone(),
                    ))
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
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:native-call.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    Item::native(XmlNode(
        XPathNativeNode::xml_document(Arc::new(LoadedInputAstStream::XmlDocument(
            document.unwrap(),
        )))
        .unwrap(),
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
            original.downcast_ref::<XmlNode>().unwrap().0.owner(),
            returned.downcast_ref::<XmlNode>().unwrap().0.owner()
        ));
        assert!(result.items[0].source_map().is_some());
        let plan = render_compiled_template(
            &artifact,
            &TemplateData {
                bindings,
                native_functions: functions.clone(),
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
