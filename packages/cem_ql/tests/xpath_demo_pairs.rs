//! XPATH-CEMQL-DEMO-PAIRS: execute the authored examples through native CEMT.
use cem_ml::{
    legacy_custom_element::extract_html_template_fragments,
    resolver::{ResolverPolicy, ResolverRegistry},
};
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::{render_template, TemplateData},
    xpath::functions::CemtXPathFunctions,
};
use std::sync::Arc;

const PAGE: &str = include_str!("../../cem-elements/demo/xpath-functions.html");

fn render(legend: &str, name: &str, value: &str) -> String {
    let sample = PAGE
        .split(&format!("legend=\"{legend}\""))
        .nth(1)
        .unwrap()
        .split("</cem-demo-element>")
        .next()
        .unwrap();
    let outer = extract_html_template_fragments(sample).remove(0);
    let source = extract_html_template_fragments(&outer.body).remove(0).body;
    // The browser reads HTML template textContent. Decode these authored HTML
    // escapes only; external XML source remains input to the native importer.
    let source = source
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    // The browser coordinator extracts initial slice declarations before render.
    // Supply the edited slice bindings directly to the remaining authored body.
    let source = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("{slice @name="))
        .collect::<Vec<_>>()
        .join("\n");
    let mut data = TemplateData::default().with_binding(
        "datadom",
        ItemStream::once(Item::Record(
            [(
                "slices".into(),
                vec![Item::Record(
                    [(
                        name.into(),
                        vec![Item::Atomic(AtomValue::String(value.into()))],
                    )]
                    .into_iter()
                    .collect(),
                )],
            )]
            .into_iter()
            .collect(),
        )),
    );
    CemtXPathFunctions::compile(
        include_str!("../../cem-elements/demo/xpath-functions.cemt"),
        "memory:demo.cemt",
    )
    .unwrap()
    .install(
        &mut data.native_functions,
        Arc::new(ResolverRegistry::new()),
        Arc::new(ResolverPolicy::new()),
    )
    .unwrap();
    let result = render_template(&source, &data);
    assert!(
        result.diagnostics.is_empty(),
        "{legend}: {:?}",
        result.diagnostics
    );
    result.rendered
}
fn texts(html: &str, tag: &str) -> Vec<String> {
    html.split(&format!("<{tag}>"))
        .skip(1)
        .map(|part| {
            let content = part.split(&format!("</{tag}>")).next().unwrap();
            // Node interpolation may preserve an item subtree. Compare labels
            // through the shared import/text view, as the browser fixture does.
            let tree = cem_ml::import::import_data(
                &format!("<label>{content}</label>"), "xml", "cem", "memory:demo-label.xml",
            ).unwrap();
            tree.text_fragments(0).unwrap().collect::<String>()
        })
        .collect()
}

#[test]
fn scalar_and_predicate_pairs_match_for_empty_unicode_and_changed_values() {
    for value in ["Hello", "", "🍋", "cherry", "lemon"] {
        for (xpath, native) in [
            ("1. Named XPath function", "1a. CEM-QL string pair"),
            ("2. Shared XPath predicate", "2a. CEM-QL predicate pair"),
        ] {
            assert_eq!(
                texts(&render(xpath, "text", value), "output"),
                texts(&render(native, "text", value), "output")
            );
        }
    }
}

#[test]
fn node_pairs_share_numeric_matching_namespaces_and_descendant_text_rules() {
    for source in [
        "<r><item qty='2'>Cherry</item></r>",
        "<r><item qty=' 2.5 '>Cherry</item><item qty='1.5'>Lemon</item></r>",
        "<r><item qty='1'>Lemon</item><item qty='3'>Grape</item><item>Unknown</item></r>",
        "<r xmlns:p='urn:other'><p:item qty='2'>Skip</p:item><item p:qty='4'>Low</item><item qty='2'>A<b>B</b><![CDATA[C]]><!--skip--> D</item></r>",
    ] {
        let xpath = texts(&render("3. XML nodes and matching", "source", source), "li");
        let native = texts(&render("3a. CEM-QL native node pair", "source", source), "li");
        assert!(!xpath.is_empty());
        assert_eq!(xpath, native);
    }
    for legend in ["3. XML nodes and matching", "3a. CEM-QL native node pair"] {
        let output = render(legend, "source", "<r>");
        assert!(output.contains("role=\"alert\""), "{legend}: {output}");
        assert!(texts(&output, "li").is_empty());
    }
}
