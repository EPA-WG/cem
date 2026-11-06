use cem_ml::{import::dom::DomCapture, value::artifact::CemValueArtifactLimits};
use cem_ql::{
    eval::{
        portable::{decode_values, encode_values},
        Item, ItemStream,
    },
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
};
use std::collections::BTreeMap;

#[test]
fn native_dom_payload_slots_survive_transport_and_owner_release() {
    let limits = CemValueArtifactLimits::default();
    let mut capture = DomCapture::new(
        "cem-dom://runtime/original",
        "https://example.test/",
        "https://example.test/",
        limits,
    )
    .unwrap();
    capture
        .push(0, "capture-document", "document", "", "", "")
        .unwrap();
    let container = capture
        .push(
            0,
            "container",
            "element",
            "http://www.w3.org/1999/xhtml",
            "cem-payload:payload",
            "",
        )
        .unwrap();
    for (key, text) in [("first", "One & two"), ("second", "<three>")] {
        let b = capture
            .push(
                container,
                key,
                "element",
                "http://www.w3.org/1999/xhtml",
                "b",
                "",
            )
            .unwrap();
        capture
            .push(
                b,
                &format!("{key}-slot"),
                "attribute",
                "",
                "slot",
                "heading",
            )
            .unwrap();
        capture
            .push(b, &format!("{key}-text"), "text", "", "", text)
            .unwrap();
    }
    capture
        .push(
            container,
            "comment",
            "comment",
            "",
            "",
            "untouched & literal",
        )
        .unwrap();
    let tree = capture.finish().unwrap();
    let original = cem_ql::eval::RetainedCemNode::new(tree.clone(), container)
        .unwrap()
        .query_item();
    let bytes = encode_values(&ItemStream::once(original.clone()), &limits).unwrap();
    let decoded = decode_values(&bytes, &limits).unwrap().items.remove(0);
    let mut data = TemplateData::default();
    data.bind_native_payload(decoded).unwrap();
    drop(tree);
    drop(original);
    drop(bytes);
    let artifact = compile_template(
        "{section | {$ datadom.slots.heading} {$ datadom.slots.heading}}",
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let html = render_plan_to_html(&plan);
    assert_eq!(html.matches("One &amp; two").count(), 2);
    assert_eq!(html.matches("&lt;three&gt;").count(), 2);
    let [Item::Record(fields)] = data.bindings["datadom"].items.as_slice() else {
        panic!()
    };
    let [Item::Record(slots)] = fields["slots"].as_slice() else {
        panic!()
    };
    assert_eq!(
        slots[""][0]
            .view()
            .unwrap()
            .provenance()
            .unwrap()
            .source_key
            .as_deref(),
        Some("comment")
    );
    let first = slots["heading"][0].view().unwrap();
    assert_eq!(
        first.provenance().unwrap().source_key.as_deref(),
        Some("first")
    );
    assert_eq!(first.provenance().unwrap().line_number, None);
    let parent = first
        .parent(cem_ql::eval::QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        parent
            .view()
            .unwrap()
            .provenance()
            .unwrap()
            .source_key
            .as_deref(),
        Some("container")
    );
}
#[test]
fn ordinary_records_remain_invalid_payload_and_inserted_content() {
    let mut data = TemplateData::default();
    assert!(data
        .bind_native_payload(Item::Record(BTreeMap::new()))
        .is_err());
    data.bindings.insert(
        "fake".into(),
        ItemStream::once(Item::Record(BTreeMap::new())),
    );
    let a = compile_template(
        "{p | {$ fake}}",
        &CompileTemplateOptions {
            host_bindings: vec!["fake".into()],
            ..Default::default()
        },
    );
    assert!(render_compiled_template(&a, &data)
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.render.expression_type"));
}
