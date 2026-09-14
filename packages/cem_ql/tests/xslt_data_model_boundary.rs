//! XSLT-VIEW-GATE: characterize existing CEM values before promising XDM lowering.
//! These are current CEM contracts, not assertions of XPath/XSLT conformance.
//! See docs/xslt-data-table-parity.md for the approval boundary.
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item};
use cem_ql::ir::IrNode;

fn eval(source: &str) -> Vec<Item> {
    let compiled = compile(source, &CompileContext::default()).expect("probe must compile");
    let result = evaluate(&compiled, &EvaluationContext::default());
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.items
}

fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view()
        .expect("data import must retain a native view")
        .field(name)
        .unwrap_or_default()
}

fn text(item: &Item, name: &str) -> String {
    match field(item, name).as_slice() {
        [Item::Atomic(AtomValue::String(value))] => value.clone(),
        other => panic!("expected {name} string, got {other:?}"),
    }
}

#[test]
fn cem_array_ir_flattens_sequences_including_empty_members() {
    // There is no CEM-QL surface array constructor. Probe the existing array
    // IR directly instead of assuming XPath square-constructor syntax exists.
    let mut query = compile("((), (1, 2), ())", &CompileContext::default()).unwrap();
    let root = query.tree.root.0 as usize;
    let IrNode::Sequence(members) = query.tree.nodes[root].clone() else {
        panic!("expected sequence IR");
    };
    query.tree.nodes[root] = IrNode::Array(members);
    let result = evaluate(&query, &EvaluationContext::default());
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let items = result.items;
    let [Item::Array(members)] = items.as_slice() else {
        panic!("expected a CEM array, got {items:?}");
    };
    // XDM's square array constructor has three sequence-valued members here.
    // The existing CEM constructor intentionally collects a flat item stream.
    assert_eq!(
        members,
        &[
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(2)),
        ]
    );
}

#[test]
fn native_json_tree_retains_null_slots_order_and_provenance_in_cem_shape() {
    let reports = eval(r#"data:read("{\"z\":[null,\"\"],\"a\":true}", "json")"#);
    let report = &reports[0];
    assert_eq!(text(report, "error"), "");
    let document = field(report, "root").remove(0);
    assert_eq!(text(&document, "kind"), "document");
    let object = field(&document, "children").remove(0);
    assert_eq!(text(&object, "namespace"), "cem:generic-data");
    assert_eq!(text(&object, "name"), "object");
    let properties = field(&object, "children");
    assert_eq!(properties.len(), 2);
    assert_eq!(text(&properties[0], "name"), "property");
    assert_eq!(text(&field(&properties[0], "attributes")[0], "value"), "z");
    assert_eq!(text(&field(&properties[1], "attributes")[0], "value"), "a");

    let array = field(&properties[0], "children").remove(0);
    let members = field(&array, "children");
    assert_eq!(text(&array, "name"), "array");
    assert_eq!(members.len(), 2);
    assert_eq!(text(&members[0], "name"), "null");
    assert_eq!(text(&members[1], "name"), "string");
    for member in &members {
        assert!(member.identity().is_some());
        assert!(member.source_map().is_some());
    }
    assert_ne!(members[0].identity(), members[1].identity());
    // Neither returning this CEM tree as fn:parse-json(), nor relabeling it as
    // fn:json-to-xml(), supplies the standard result types and node structure.
}

#[test]
fn ordinary_record_projection_retains_subject_but_is_not_a_native_node_view() {
    let items = eval(
        r#"for subject in data:read("{\"a\":1}", "json").root.children {
            {kind: "element", name: "map",
             namespace: "http://www.w3.org/2005/xpath-functions", source: subject}
        }"#,
    );
    let [Item::Record(fields)] = items.as_slice() else {
        panic!("expected an ordinary projection record, got {items:?}");
    };
    assert!(items[0].view().is_none());
    assert!(items[0].identity().is_none());
    assert!(items[0].source_map().is_none());
    let source = &fields["source"][0];
    assert!(source.identity().is_some());
    assert!(source.source_map().is_some());
    assert_eq!(text(source, "name"), "object");
}

#[test]
fn explicit_json_xml_projection_is_native_and_does_not_change_the_default() {
    let items = eval(r#"data:read("{\"rows\":[null,\"\"]}", "json", "json-to-xml").root"#);
    let root = &items[0];
    let map = field(root, "children").remove(0);
    assert_eq!(
        text(&map, "namespace"),
        "http://www.w3.org/2005/xpath-functions"
    );
    assert_eq!(text(&map, "name"), "map");
    let array = field(&map, "children").remove(0);
    assert_eq!(text(&array, "name"), "array");
    let key = field(&array, "attributes").remove(0);
    assert_eq!(text(&key, "name"), "key");
    assert_eq!(text(&key, "value"), "rows");
    let members = field(&array, "children");
    assert_eq!(members.len(), 2);
    assert_eq!(text(&members[0], "name"), "null");
    assert_eq!(text(&members[1], "name"), "string");
    assert!(members
        .iter()
        .all(|node| node.identity().is_some() && node.source_map().is_some()));
    let ordinary = eval(r#"data:read("{\"rows\":[null,\"\"]}", "json").root.children"#);
    assert_eq!(text(&ordinary[0], "name"), "object");
    assert_ne!(
        ordinary[0].identity(),
        map.identity(),
        "projection is part of identity"
    );
}

#[test]
fn explicit_projection_rejects_unknown_profiles_and_incompatible_formats() {
    for query in [
        r#"data:read("{}", "json", "typo")"#,
        r#"data:read("<root/>", "xml", "json-to-xml")"#,
        r#"data:read("[", "json", "json-to-xml")"#,
    ] {
        let items = eval(query);
        assert!(!text(&items[0], "error").is_empty());
        assert!(field(&items[0], "root").is_empty());
    }
}

#[test]
fn declarative_reader_can_select_the_standard_json_node_projection() {
    use cem_ql::eval::ItemStream;
    use cem_ql::render::{render_template, TemplateData};
    for projection in ["json-to-xml", "{profile}"] {
        let source = format!(
            r#"{{cem-data @name=document @select='"{{\"a\":null}}"' @type=json @projection="{projection}"}}
                {{p | {{$document.root.children.name}}:
                    {{$document.root.children.children.name}}:
                    {{$document.root.children.children.attributes.value}}}}"#
        );
        let result = render_template(
            &source,
            &TemplateData::default().with_binding(
                "profile",
                ItemStream::once(Item::Atomic(AtomValue::String("json-to-xml".into()))),
            ),
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.rendered.split_whitespace().collect::<String>(),
            "<p>map:null:a</p>"
        );
    }
}
