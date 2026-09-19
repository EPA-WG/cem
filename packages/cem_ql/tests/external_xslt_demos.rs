use cem_ml::legacy_custom_element::{convert_template_source, extract_html_template_fragments};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

const STYLESHEET: &str = include_str!("../../cem-elements/demo/data-island-tree.xsl");
const FRAGMENTS: &str = include_str!("../../cem-elements/demo/html-template.xhtml");

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(name, items)| (name.into(), items))
            .collect(),
    )
}

fn element(tag: &str, children: Vec<Item>) -> Item {
    record([
        ("kind", vec![string("element")]),
        ("tag", vec![string(tag)]),
        ("attributes", vec![record([])]),
        ("children", children),
    ])
}

fn text(value: &str) -> Item {
    record([
        ("kind", vec![string("text")]),
        ("text", vec![string(value)]),
    ])
}

fn render(source: &str, payload: Vec<Item>) -> String {
    let converted = convert_template_source(source);
    assert!(
        converted.diagnostics.is_empty(),
        "{:?}",
        converted.diagnostics
    );
    assert!(!converted.source.contains("xsl:"));
    let datadom = record([("payload", vec![record([("nodes", payload)])])]);
    let result = render_template(
        &converted.source,
        &TemplateData::default().with_binding("datadom", ItemStream::once(datadom)),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn whole_file_xslt_renders_explicit_source_without_initial_focus() {
    use cem_ml::validation::xpath::XPathExpandedName;
    use cem_ql::{
        render::render_plan_to_html,
        xslt::{
            compiler::{compile_xslt_bundle_with_options, XsltCompileOptions},
            XsltBundle,
        },
    };
    let compiled = compile_xslt_bundle_with_options(
        STYLESHEET,
        "memory:data-island-tree.xsl",
        &XsltCompileOptions {
            entrypoint: Some(XPathExpandedName::unqualified("tree")),
            parameters: [(XPathExpandedName::unqualified("source"), "source".into())].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    for label in ["🍒 &amp; 🍋", "Changed"] {
        let source =
            format!("<catalog><section><item><leaf>{label}</leaf></item></section></catalog>");
        let plan = bundle.render(
            &TemplateData::default().with_binding("source", ItemStream::once(string(&source))),
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        let output = render_plan_to_html(&plan);
        assert!(output.contains("XSLT XML payload tree"));
        assert!(output.contains(label));
        assert_eq!(output.matches("<details").count(), 4);
        assert_eq!(output.matches("<summary").count(), 4);
    }
}

#[test]
fn embedded_xslt_fragment_is_explicit_and_lowers_real_stylesheet_bytes() {
    let fragments = extract_html_template_fragments(FRAGMENTS);
    let fragment = fragments
        .iter()
        .find(|fragment| fragment.attributes.get("id").map(String::as_str) == Some("embedded-xslt"))
        .expect("the library must contain a genuine embedded XSLT fragment");
    assert_eq!(
        fragment.attributes.get("lang").map(String::as_str),
        Some("custom-element-v0")
    );
    assert!(!fragment.attributes.contains_key("type"));
    assert!(fragment.body.contains("<xsl:stylesheet"));
    assert!(fragment
        .body
        .contains("xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\""));
    let output = render(
        &fragment.body,
        vec![element(
            "basket",
            vec![
                element("fruit", vec![text("🍒")]),
                element("fruit", vec![text("🍋")]),
            ],
        )],
    );
    assert!(output.contains("Embedded XSLT fruit tree"));
    assert!(output.contains("basket"));
    assert!(output.contains("🍒"));
    assert!(output.contains("🍋"));
    assert_eq!(output.matches("<details").count(), 1);
    assert_eq!(output.matches("<li").count(), 2);
    assert!(!output.contains("embedded-xsl data island tree"));
}

#[test]
fn unsupported_embedded_stylesheet_instruction_is_diagnosed() {
    let converted = convert_template_source(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"><xsl:template match="/"><xsl:script>never execute</xsl:script></xsl:template></xsl:stylesheet>"#,
    );
    assert!(converted
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "legacy_xslt.unsupported_construct"));
    assert!(!converted.source.contains("never execute"));
}
