use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default()).expect("native data query compiles");
    evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root().with_queue_size(8192),
            ..EvaluationContext::default()
        },
    )
}

fn read(source: &str, format: &str) -> Item {
    // Explicit query-string literals only; no AST serialization boundary.
    let result = eval(&format!(
        "data:read({}, {})",
        serde_json::to_string(source).unwrap(),
        serde_json::to_string(format).unwrap()
    ));
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    result.items.into_iter().next().unwrap()
}

fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view()
        .expect("import retains a native owner")
        .field(name)
        .unwrap_or_default()
}

fn text(item: &Item, name: &str) -> String {
    match field(item, name).as_slice() {
        [Item::Atomic(AtomValue::String(value))] => value.clone(),
        other => panic!("expected {name} string: {other:?}"),
    }
}

#[test]
fn xml_import_preserves_cem_nodes_namespaces_order_and_provenance() {
    let source = "<?inspect inert?><root xmlns:a=\"urn:one\" xmlns:b=\"urn:one\"><a:item empty=\"\">before<b/>after<!--note--><![CDATA[<raw>]]></a:item><b:item>&amp;&#x1F352;</b:item></root>";
    let report = read(source, "xml");
    assert_eq!(text(&report, "error"), "");
    let root = field(&report, "root").remove(0);
    assert_eq!(text(&root, "kind"), "document");
    let children = field(&root, "children");
    assert_eq!(text(&children[0], "kind"), "processing-instruction");
    let elements = field(&children[1], "children");
    assert_eq!(text(&elements[0], "namespace"), "urn:one");
    assert_eq!(text(&elements[1], "namespace"), "urn:one");
    assert_eq!(text(&elements[0], "name"), "item");
    let mixed = field(&elements[0], "children");
    assert_eq!(
        mixed
            .iter()
            .map(|node| text(node, "kind"))
            .collect::<Vec<_>>(),
        ["text", "element", "text", "comment", "cdata"]
    );
    assert_eq!(text(&field(&elements[0], "attributes")[0], "value"), "");
    assert!(elements[0].view().unwrap().source_map().is_some());
    assert!(!text(&elements[0], "id").is_empty());
    assert_eq!(text(&field(&elements[1], "children")[0], "value"), "&🍒");
}

#[test]
fn data_formats_import_structures_not_precomputed_presentations() {
    for (format, source) in [
        (
            "json",
            "[{\"qty\":10,\"fruit\":\"🍒\"},{\"qty\":2,\"fruit\":\"🍋\"}]",
        ),
        ("yaml", "- qty: 10\n  fruit: 🍒\n- qty: 2\n  fruit: 🍋\n"),
        ("csv", "qty,fruit\n10,🍒\n2,🍋\n"),
    ] {
        let report = read(source, format);
        assert_eq!(text(&report, "error"), "", "{format}");
        assert!(report.view().unwrap().field("tables").is_none());
        let root = field(&report, "root").remove(0);
        let array = field(&root, "children").remove(0);
        assert_eq!(text(&array, "name"), "array");
        assert_eq!(text(&array, "namespace"), "cem:generic-data");
        let objects = field(&array, "children");
        assert_eq!(objects.len(), 2);
        assert_eq!(text(&objects[0], "name"), "object");
        let properties = field(&objects[0], "children");
        assert_eq!(text(&properties[0], "name"), "property");
        assert_eq!(
            text(&field(&properties[0], "attributes")[0], "value"),
            "qty"
        );
        let value = field(&properties[0], "children").remove(0);
        assert_eq!(
            text(&value, "name"),
            if format == "csv" { "string" } else { "number" }
        );
        assert_eq!(text(&field(&value, "children")[0], "value"), "10");
    }
}

#[test]
fn generic_queries_keep_imported_node_identity_when_grouping_and_sorting() {
    let source = "<r><x n=\"10\"/><x n=\"2\"/><y n=\"3\"/></r>";
    let report = read(source, "xml");
    let root = field(&report, "root").remove(0);
    let expected = field(&field(&root, "children")[0], "children");
    let binding = ItemStream::once(root);
    let context = CompileContext {
        policy_bindings: std::collections::BTreeMap::from([("doc".into(), binding.clone())]),
        ..CompileContext::default()
    };
    let query = compile("seq:sorted(doc.children.children, fn(node) => node.attributes.value, \"ascending\", \"number\")", &context).unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: context.policy_bindings,
            ..EvaluationContext::default()
        },
    );
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    assert_eq!(
        result.items,
        vec![
            expected[1].clone(),
            expected[2].clone(),
            expected[0].clone()
        ]
    );
}

#[test]
fn malformed_unresolved_and_excessive_sources_fail_without_partial_ast() {
    for (format, source) in [
        ("xml", "<root>".into()),
        ("json", "[oops]".into()),
        ("csv", "a,b\n\"oops".into()),
        ("yaml", "x: [oops".into()),
        (
            "xml",
            "<!DOCTYPE r SYSTEM 'https://invalid.test/dtd'><r/>".into(),
        ),
        ("xml", "<r>&external;</r>".into()),
        ("yaml", "a: &anchor [1,2]\nb: *anchor".into()),
        ("json", format!("{}0{}", "[".repeat(65), "]".repeat(65))),
        ("xml", format!("{}x{}", "<r>".repeat(65), "</r>".repeat(65))),
        ("json", " ".repeat(32769)),
        ("unknown", "hello".into()),
    ] {
        let report = read(&source, format);
        assert!(
            !text(&report, "error").is_empty(),
            "must reject {format}: {source}"
        );
        assert!(field(&report, "root").is_empty());
    }
}

#[test]
fn declarative_reader_binds_native_ast_and_can_be_used_without_a_viewer() {
    use cem_ql::render::{render_template, TemplateData};
    let data = TemplateData::default().with_binding(
        "source",
        ItemStream::once(Item::Atomic(AtomValue::String(
            "<r><value>🍒</value></r>".into(),
        ))),
    );
    let rendered = render_template(
        r#"
        {cem-data @name=document @select=source @type=application/xml}
        {output | {$document.root.children.children.children.value}}
    "#,
        &data,
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(rendered.rendered.trim(), "<output>🍒</output>");
    assert!(!rendered.rendered.contains("cem-data"));
}

#[test]
fn declarative_reader_rejects_missing_contract_fields_and_visible_children() {
    use cem_ql::render::{render_template, TemplateData};
    for source in [
        r#"{cem-data @select='"[]"' @type=json}"#,
        r#"{cem-data @name="bad name" @select='"[]"' @type=json}"#,
        r#"{cem-data @name=document @type=json}"#,
        r#"{cem-data @name=document @select='"[]"'}"#,
        r#"{cem-data @name=document @select='"[]"' @type=json | {p | ignored}}"#,
    ] {
        let result = render_template(source, &TemplateData::default());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "cem.ql.render.data_reader_invalid"),
            "{source}: {:?}",
            result.diagnostics
        );
        assert!(!result.rendered.contains("<cem-data"));
    }
}
