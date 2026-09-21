//! XSLT-OUTPUT-PREREQUISITES: characterize the shared result-construction boundary.
//! These assertions preserve existing CEMT behavior, not XSLT conformance.
//! XSLT acceptance now uses an explicitly selected native result path.
use cem_ml::{import::import_data, validation::xpath::XPathNativeNode};
use cem_ql::{
    eval::ItemStream,
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        RenderPlan, RenderPlanNode, TemplateData,
    },
    xpath::functions::XPathQueryItem,
    xslt::compiler::compile_xslt_bundle,
};
use std::sync::Arc;

fn render(source: &str, data: &TemplateData) -> RenderPlan {
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template(&artifact, data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    plan
}

#[test]
fn native_xpath_subtrees_and_explicit_text_keep_distinct_output() {
    for (source, format, projection) in [
        ("<r><row id='a'>payload</row><row/></r>", "xml", "cem"),
        (r#"{"row":"payload","empty":null}"#, "json", "json-to-xml"),
    ] {
        let tree = import_data(source, format, projection, "memory:output-input").unwrap();
        let node = XPathNativeNode::cem_document(Arc::clone(&tree));
        assert!(Arc::ptr_eq(node.owner(), &tree));
        assert!(node.source_owner().is_some());
        let data = TemplateData::default().with_binding(
            "selected",
            ItemStream::once(XPathQueryItem::from_node(node)),
        );
        let native = render("{div | {$ selected}}", &data);
        let [RenderPlanNode::Element { children, .. }] = native.nodes.as_slice() else {
            panic!("expected result element");
        };
        let [RenderPlanNode::Reference { reference, .. }] = children.as_slice() else {
            panic!("expected retained native subtree reference");
        };
        assert_eq!(reference.values(), &data.bindings["selected"].items);
        assert!(render_plan_to_html(&native).contains("payload"));
        let plan = render("{div | {$dom:text(selected)}}", &data);
        assert_eq!(render_plan_to_html(&plan), "<div>payload</div>");
        let [RenderPlanNode::Element { children, .. }] = plan.nodes.as_slice() else {
            panic!("expected result element");
        };
        assert!(children
            .iter()
            .all(|n| matches!(n, RenderPlanNode::Text { .. })));
        // Both paths retain the input owner. Only explicit text conversion
        // omits its structure from the result.
        assert!(tree.native_owner().is_some());
    }
}

#[test]
fn atomics_and_explicit_text_become_indistinguishable_before_parent_construction() {
    let data = TemplateData::default();
    for source in [
        "{div | {$ 1}{$ 2}}",
        r#"{div | {$ "1"}{$ "2"}}"#,
        "{div | {$ (1, 2)}}",
    ] {
        assert_eq!(render_plan_to_html(&render(source, &data)), "<div>12</div>");
    }
    // XSLT complex content requires "1 2" for adjacent atomic items and "12"
    // for adjacent text nodes. Existing CEMT interpolation must stay unchanged.
}

#[test]
fn existing_cemt_constructors_collect_attributes_after_child_output() {
    let plan = render(
        r#"{element @name=div | {$ "body"}{attribute @name=title | {$ "late"}}}"#,
        &TemplateData::default(),
    );
    assert_eq!(
        render_plan_to_html(&plan),
        r#"<div title="late">body</div>"#
    );
    // The same result sequence requires XTDE0410 in XSLT. Reusing this CEMT
    // constructor directly would erase order before that check can occur.
}

#[test]
fn result_style_constructor_is_distinct_from_component_style_declaration() {
    let declaration = compile_template(
        "{style |``` .card { color: red; } ```}",
        &CompileTemplateOptions::default(),
    );
    assert!(
        declaration.diagnostics.is_empty(),
        "{:?}",
        declaration.diagnostics
    );
    assert_eq!(declaration.stylesheets.len(), 1);
    let result = compile_template(
        r#"{element @name=style | {$ ".card { color: red; }"}}"#,
        &CompileTemplateOptions::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.stylesheets.is_empty());
    let plan = render_compiled_template(&result, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan),
        "<style>.card { color: red; }</style>"
    );
}

#[test]
fn approved_output_forms_compile_without_altering_ordinary_cemt() {
    for body in [
        r#"<p title="{1 + 1}"/>"#,
        r#"<p xml:space="preserve"> </p>"#,
        r#"<style>.card { color: red; }</style>"#,
        r#"<xsl:try select="/*"><xsl:catch/></xsl:try>"#,
        r#"<xsl:try><xsl:value-of select="parse-xml('')"/><xsl:catch select="/*"/></xsl:try>"#,
    ] {
        let source = format!(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#
        );
        compile_xslt_bundle(&source, "memory:output-prerequisites.xslt").unwrap();
    }
}
