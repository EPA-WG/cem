use cem_ml::parser::{document::CemDocument, AstNodeId, CemAstNode};
use cem_ml::validation::json::{
    json_document_ast_from_source_bytes, JsonDocumentAst, JsonSourceValidationRequest,
};
use cem_ml::validation::json_xml::{
    project_json_to_xml, JsonXmlDuplicates, JsonXmlProjectionOptions, JSON_XML_NAMESPACE,
};

fn parse(source: &str) -> JsonDocumentAst {
    let (document, diagnostics) =
        json_document_ast_from_source_bytes(JsonSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture:json-xml",
            content_type: Some("application/json"),
        });
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    document.unwrap()
}

fn children(doc: &CemDocument, id: AstNodeId) -> &[AstNodeId] {
    match doc.get(id).unwrap() {
        CemAstNode::Document { root_children, .. } => root_children,
        CemAstNode::Element { children, .. } => children,
        _ => panic!("expected a document or element"),
    }
}

fn name(doc: &CemDocument, id: AstNodeId) -> &str {
    match doc.get(id).unwrap() {
        CemAstNode::Element { expanded_name, .. } => {
            assert_eq!(expanded_name.namespace_uri, JSON_XML_NAMESPACE);
            &expanded_name.local_name
        }
        _ => panic!("expected element"),
    }
}

fn attr<'a>(doc: &'a CemDocument, id: AstNodeId, key: &str) -> Option<&'a str> {
    let CemAstNode::Element { attributes, .. } = doc.get(id).unwrap() else {
        panic!("expected element");
    };
    attributes
        .iter()
        .find_map(|id| match doc.get(*id).unwrap() {
            CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            } if expanded_name.local_name == key && expanded_name.namespace_uri.is_empty() => {
                value.as_deref()
            }
            _ => None,
        })
}

fn text(doc: &CemDocument, id: AstNodeId) -> String {
    children(doc, id)
        .iter()
        .map(|id| match doc.get(*id).unwrap() {
            CemAstNode::Text { data, .. } => data.as_str(),
            other => panic!("expected scalar text, got {other:?}"),
        })
        .collect()
}

#[test]
fn json_xml_tree_uses_standard_names_keyed_values_and_source_order() {
    let source = r#"{"z":[null,"",false],"a":1.2300e+02,"":{},"z":true}"#;
    let json = parse(source);
    let doc = project_json_to_xml(&json, &JsonXmlProjectionOptions::default()).unwrap();
    assert_eq!(children(&doc, 0).len(), 1);
    let root = children(&doc, 0)[0];
    assert_eq!(name(&doc, root), "map");
    let entries = children(&doc, root);
    assert_eq!(
        entries.iter().map(|id| name(&doc, *id)).collect::<Vec<_>>(),
        ["array", "number", "map", "boolean"]
    );
    assert_eq!(
        entries
            .iter()
            .map(|id| attr(&doc, *id, "key"))
            .collect::<Vec<_>>(),
        [Some("z"), Some("a"), Some(""), Some("z")]
    );
    assert_eq!(text(&doc, entries[1]), "1.2300e+02");
    let members = children(&doc, entries[0]);
    assert_eq!(
        members.iter().map(|id| name(&doc, *id)).collect::<Vec<_>>(),
        ["null", "string", "boolean"]
    );
    assert!(children(&doc, members[0]).is_empty());
    assert!(
        children(&doc, members[1]).is_empty(),
        "empty strings have no zero-length XDM text node"
    );
    assert_eq!(text(&doc, members[2]), "false");
    assert!(members.iter().all(|id| attr(&doc, *id, "key").is_none()));
}

#[test]
fn json_xml_projection_accepts_scalar_and_empty_roots() {
    for (source, expected_name, expected_text) in [
        ("null", "null", ""),
        ("true", "boolean", "true"),
        (r#""🍒""#, "string", "🍒"),
        ("-0", "number", "-0"),
        ("{}", "map", ""),
        ("[]", "array", ""),
    ] {
        let doc =
            project_json_to_xml(&parse(source), &JsonXmlProjectionOptions::default()).unwrap();
        let root = children(&doc, 0)[0];
        assert_eq!(name(&doc, root), expected_name);
        assert_eq!(text(&doc, root), expected_text);
        assert_eq!(attr(&doc, root, "key"), None);
    }
}

#[test]
fn json_xml_projection_retains_original_value_and_key_source_maps() {
    let json = parse("{\n  \"fruit\": \"🍒\"\n}");
    let cem_ml::validation::json::JsonValueAst::Object { members, .. } =
        json.root.as_ref().unwrap()
    else {
        panic!()
    };
    let doc = project_json_to_xml(&json, &JsonXmlProjectionOptions::default()).unwrap();
    let root = children(&doc, 0)[0];
    let value = children(&doc, root)[0];
    let CemAstNode::Element {
        source, attributes, ..
    } = doc.get(value).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        source.origin(),
        members[0].value.range().source_map().origin()
    );
    let CemAstNode::Attribute { source, .. } = doc.get(attributes[0]).unwrap() else {
        panic!()
    };
    assert_eq!(source.origin(), members[0].name_range.source_map().origin());
    assert!(doc
        .nodes
        .iter()
        .all(|node| !matches!(node, CemAstNode::Error { .. })));
}

#[test]
fn json_xml_nodes_feed_the_existing_typed_cem_presentation_directly() {
    use cem_ml::projection::{cem_tree_nodes, CemTreeAstNode};
    let doc = project_json_to_xml(&parse(r#"{"fruit":"🍒"}"#), &Default::default()).unwrap();
    let stream = cem_tree_nodes(&doc);
    let [CemTreeAstNode::Element {
        name,
        children,
        source,
        ..
    }] = stream.as_nodes()
    else {
        panic!()
    };
    assert_eq!(name, &format!("{JSON_XML_NAMESPACE}:map"));
    assert!(source.origin().is_some());
    let [CemTreeAstNode::Element {
        name,
        attributes,
        children,
        source,
        ..
    }] = children.as_slice()
    else {
        panic!()
    };
    assert_eq!(name, &format!("{JSON_XML_NAMESPACE}:string"));
    assert_eq!(attributes[0].name, "key");
    assert_eq!(attributes[0].value.as_deref(), Some("fruit"));
    assert!(source.origin().is_some());
    assert!(matches!(children.as_slice(), [CemTreeAstNode::Text { value, .. }] if value == "🍒"));
}

#[test]
fn json_xml_duplicate_policy_uses_decoded_keys_without_reordering() {
    let json = parse(r#"{"a":1,"\u0061":2,"b":3}"#);
    let options = JsonXmlProjectionOptions {
        duplicates: JsonXmlDuplicates::UseFirst,
        ..Default::default()
    };
    let doc = project_json_to_xml(&json, &options).unwrap();
    let entries = children(&doc, children(&doc, 0)[0]);
    assert_eq!(entries.len(), 2);
    assert_eq!(text(&doc, entries[0]), "1");
    assert_eq!(attr(&doc, entries[1], "key"), Some("b"));
    let options = JsonXmlProjectionOptions {
        duplicates: JsonXmlDuplicates::Reject,
        ..Default::default()
    };
    let error = project_json_to_xml(&json, &options).unwrap_err();
    assert_eq!(error.code, "cem.json.xml_projection.duplicate_key");
    assert!(error.source.origin().is_some());
}

#[test]
fn json_xml_escaping_is_xml_safe_and_marks_only_escaped_values() {
    let json = parse(r#"{"\u0000":"\n\u0000\\\u007f🍒","plain":"\u0025"}"#);
    let doc = project_json_to_xml(&json, &JsonXmlProjectionOptions::default()).unwrap();
    let entries = children(&doc, children(&doc, 0)[0]);
    assert_eq!(attr(&doc, entries[0], "key"), Some("�"));
    assert_eq!(text(&doc, entries[0]), "\n�\\\u{7f}🍒");
    assert_eq!(attr(&doc, entries[0], "escaped"), None);
    assert_eq!(attr(&doc, entries[0], "escaped-key"), None);
    let options = JsonXmlProjectionOptions {
        escape: true,
        ..Default::default()
    };
    let doc = project_json_to_xml(&json, &options).unwrap();
    let entries = children(&doc, children(&doc, 0)[0]);
    assert_eq!(attr(&doc, entries[0], "key"), Some(r#"\u0000"#));
    assert_eq!(attr(&doc, entries[0], "escaped-key"), Some("true"));
    assert_eq!(attr(&doc, entries[0], "escaped"), Some("true"));
    assert_eq!(text(&doc, entries[0]), r#"\n\u0000\\\u007f🍒"#);
    assert_eq!(text(&doc, entries[1]), "%");
    assert_eq!(attr(&doc, entries[1], "escaped"), None);
}

#[test]
fn json_xml_projection_rejects_incomplete_input_and_resource_overflow() {
    let (invalid, _) = json_document_ast_from_source_bytes(JsonSourceValidationRequest {
        bytes: b"{",
        source_uri: "fixture:invalid",
        content_type: Some("application/json"),
    });
    if let Some(doc) = invalid {
        assert!(project_json_to_xml(&doc, &JsonXmlProjectionOptions::default()).is_err());
    }
    let json = parse("[[0]]");
    for options in [
        JsonXmlProjectionOptions {
            max_depth: 1,
            ..Default::default()
        },
        JsonXmlProjectionOptions {
            max_values: 2,
            ..Default::default()
        },
    ] {
        assert_eq!(
            project_json_to_xml(&json, &options).unwrap_err().code,
            "cem.json.xml_projection.limit"
        );
    }
}
