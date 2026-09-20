//! CEMQL-STRING-IMPORT-PARITY and CEMQL-URI-PARITY.
use cem_ml::{import::import_data, validation::xpath::XPathNativeNode};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    ir::{deserialize::IrDeserializer, serialize::IrSerializer},
    xpath::functions::{CemtXPathFunctions, XPathQueryItem},
};
use std::{collections::BTreeMap, sync::Arc};

fn run(query: &str, bindings: &[(&str, Item)]) -> ItemStream {
    let bindings: BTreeMap<_, _> = bindings
        .iter()
        .map(|(name, item)| ((*name).into(), ItemStream::once(item.clone())))
        .collect();
    let query = compile(
        query,
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .expect(query);
    // The new functions must survive ordinary portable query loading.
    let bytes = IrSerializer::serialize(&query);
    let query = IrDeserializer::deserialize(&bytes).unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: bindings,
            ..Default::default()
        },
    )
}
fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn ok(query: &str, bindings: &[(&str, Item)]) -> Vec<Item> {
    let result = run(query, bindings);
    assert!(result.error.is_none(), "{query}: {:?}", result.diagnostics);
    result.items
}

#[test]
fn string_profiles_return_native_cem_nodes_usable_by_xpath_without_reimport() {
    let library = CemtXPathFunctions::compile(
        r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=test.root @visibility=public @returns=any |
    {param @name=doc @type=any @required=true}
    {body | {xpath @context=doc @sequence-type="node()" | {expression | .}}}}}"#,
        "memory:parity.cemt",
    )
    .unwrap();
    for (format, source, name, value) in [
        (
            "xml",
            "<?xml version='1.0' encoding='UTF-16'?><r>🍒</r>",
            "r",
            "🍒",
        ),
        ("json", "\u{feff}[\"\\uDEAD\",1e999]", "array", "�1e999"),
        ("csv", "fruit\nCherry", "array", "fruitCherry"),
        ("yaml", "fruit: Cherry", "object", "Cherry"),
    ] {
        let bindings = [("source", string(source)), ("format", string(format))];
        let parsed = ok("data:parse(source, format)", &bindings).remove(0);
        assert_eq!(
            parsed
                .view()
                .unwrap()
                .field("children")
                .unwrap()
                .iter()
                .find(|item| item.view().unwrap().field("kind").unwrap() == vec![string("element")])
                .unwrap()
                .view()
                .unwrap()
                .field("name")
                .unwrap(),
            vec![string(name)]
        );
        let mut context = EvaluationContext {
            policy_bindings: BTreeMap::from([("doc".into(), ItemStream::once(parsed.clone()))]),
            ..Default::default()
        };
        library
            .install(
                &mut context.native_functions,
                Arc::new(cem_ml::resolver::ResolverRegistry::new()),
                Arc::new(cem_ml::resolver::ResolverPolicy::new()),
            )
            .unwrap();
        let query = compile(
            "native:call(\"test.root\", doc)",
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let result = evaluate(&query, &context);
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        let root = result.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(root.string_value(), value);
        assert!(root.source_owner().is_some());
        assert!(parsed.view().unwrap().source_map().is_some());
        assert_eq!(
            ok("data:node_key(doc)", &[("doc", parsed)]),
            vec![string(&root.source_key().unwrap())]
        );
    }
}

#[test]
fn options_and_typed_recovery_preserve_existing_reader_contract() {
    assert_eq!(
        ok(
            r#"seq:count(data:parse(source, "json", {duplicates: "use-first"}).children.children)"#,
            &[("source", string(r#"{"x":1,"x":2}"#))]
        ),
        vec![Item::Atomic(AtomValue::Integer(1))]
    );
    assert_eq!(
        ok(
            r#"data:parse(source, "json", {escape: true}).children.children.value"#,
            &[("source", string(r#""\uDEAD""#))]
        ),
        vec![string(r#"\udead"#)]
    );
    assert_eq!(
        ok(
            r#"seq:count(data:parse("fruit\nCherry", "csv", {header: "present"}).children.children)"#,
            &[]
        ),
        vec![Item::Atomic(AtomValue::Integer(1))]
    );
    for (query, code) in [
        (r#"data:parse("<r>", "xml")"#, "cem.ql.import_malformed"),
        (
            r#"data:parse("{\"x\":1,\"x\":2}", "json", {duplicates: "reject"})"#,
            "cem.ql.import_duplicate_key",
        ),
        (
            r#"data:parse("{}", "json", {escape: "yes"})"#,
            "cem.ql.import_options",
        ),
        (
            r#"data:parse("a", "csv", {header: "guess"})"#,
            "cem.ql.import_options",
        ),
        (
            r#"data:parse("<r/>", "xml", {unknown: true})"#,
            "cem.ql.import_options",
        ),
    ] {
        assert_eq!(
            ok(
                &format!("try {{ (1, {query}) }} catch (code, message) {{ code }}"),
                &[]
            ),
            vec![string(code)]
        );
    }
    assert!(ok(r#"data:parse((), "xml")"#, &[]).is_empty());
    let failure = run(r#"data:parse("<r>", "xml")"#, &[]);
    let diagnostic = failure
        .diagnostics
        .iter()
        .find(|d| d.code == "cem.ql.import_malformed")
        .unwrap();
    assert_eq!(
        diagnostic.error_name().unwrap().local_name,
        "invalid-source"
    );
    assert!(diagnostic.source_map.is_some());
    assert!(
        diagnostic.details.as_ref().unwrap()["importDiagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["uri"] == "data:parse/source"),
        "{:?}",
        diagnostic.details
    );
    assert!(!ok(
        r#"data:read(source, "json").error"#,
        &[("source", string(r#""\uDEAD""#))]
    )
    .contains(&string("")));
    for input in [
        " ".repeat(32769),
        "<!DOCTYPE r SYSTEM 'https://invalid.test/dtd'><r/>".into(),
    ] {
        let result = run(
            r#"try { data:parse(source, "xml") } catch (code, message) { "hidden" }"#,
            &[("source", string(&input))],
        );
        assert!(result.error.is_some());
        assert!(result.items.is_empty());
    }
    for query in [
        r#"data:parse(1, "xml")"#,
        r#"data:parse("<r/>", ("xml", "xml"))"#,
        r#"data:parse("<r/>", "xml", 1)"#,
    ] {
        assert!(run(query, &[]).error.is_some());
    }
}

#[test]
fn uri_accessors_share_native_metadata_and_enforce_optional_node_arguments() {
    let library = CemtXPathFunctions::compile(
        r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=test.uris @visibility=public @returns=any |
    {param @name=node @type=any @required=true}
    {body | {xpath @context=node @sequence-type="xs:anyURI*" |
        {expression | (base-uri(.), document-uri(.))}}}}}"#,
        "memory:uris.cemt",
    )
    .unwrap();
    let tree = import_data(
        "<r xml:base='https://example.test/base/'><row/></r>",
        "xml",
        "cem",
        "https://example.test/doc.xml",
    )
    .unwrap();
    let node = XPathNativeNode::cem_document(tree.clone());
    for root in [
        imported_cem_tree(tree),
        XPathQueryItem::from_node(node.clone()),
    ] {
        let mut context = EvaluationContext {
            policy_bindings: BTreeMap::from([("root".into(), ItemStream::once(root.clone()))]),
            ..Default::default()
        };
        library
            .install(
                &mut context.native_functions,
                Arc::new(cem_ml::resolver::ResolverRegistry::new()),
                Arc::new(cem_ml::resolver::ResolverPolicy::new()),
            )
            .unwrap();
        let query = compile(
            r#"native:call("test.uris", root)"#,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let xpath = evaluate(&query, &context);
        assert!(xpath.error.is_none(), "{:?}", xpath.diagnostics);
        let native = ok(
            "(data:base_uri(root), data:document_uri(root))",
            &[("root", root.clone())],
        );
        assert_eq!(
            native.iter().map(Item::atom).collect::<Vec<_>>(),
            xpath.items.iter().map(Item::atom).collect::<Vec<_>>()
        );
        assert_eq!(
            ok("data:base_uri(root)", &[("root", root.clone())])
                .first()
                .and_then(Item::atom),
            node.base_uri().map(|s| AtomValue::AnyUri(s.into()))
        );
        assert_eq!(
            ok("data:document_uri(root)", &[("root", root)])
                .first()
                .and_then(Item::atom),
            node.owner()
                .document_uri()
                .map(|s| AtomValue::AnyUri(s.into()))
        );
    }
    let child = node.child_nodes()[0].child_nodes()[0].clone();
    assert_eq!(
        ok(
            "data:base_uri(child)",
            &[("child", XPathQueryItem::from_node(child.clone()))]
        ),
        vec![Item::Atomic(AtomValue::AnyUri(
            "https://example.test/base/".into()
        ))]
    );
    assert!(ok(
        "data:document_uri(child)",
        &[("child", XPathQueryItem::from_node(child))]
    )
    .is_empty());
    for name in ["base_uri", "document_uri"] {
        assert!(ok(&format!("data:{name}(())"), &[]).is_empty());
        for input in ["1", "{kind: \"document\"}", "(1, 2)"] {
            assert!(run(&format!("data:{name}({input})"), &[]).error.is_some());
        }
    }
    assert!(ok(r#"data:document_uri(data:parse("<r/>", "xml"))"#, &[]).is_empty());
    assert_eq!(
        ok(
            r#"data:base_uri(data:parse("<r/>", "xml", {"base-uri": "https://example.test/base/"}))"#,
            &[]
        ),
        vec![Item::Atomic(AtomValue::AnyUri(
            "https://example.test/base/".into()
        ))]
    );
}
