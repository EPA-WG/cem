//! XPATH-CEM-TREE-PARITY and CEM-IMPORT-BOUNDARY.
use cem_ml::{
    diagnostics::Diagnostic,
    import::import_data,
    parser::{
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::*,
};
use std::sync::Arc;

fn strings(query: &str, input: &XPathResultItem) -> Vec<String> {
    eval(query, Some(input.clone()), Default::default())
        .unwrap_or_else(|e| panic!("{query}: {e:?}"))
        .sequence
        .items
        .into_iter()
        .map(|item| match item {
            XPathResultItem::Atomic { value, .. } => value.lexical_value,
            XPathResultItem::Node {
                native_node: Some(node),
                ..
            } => node.string_value(),
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn json_mapping_and_equivalent_xml_have_identical_xpath_semantics() {
    let json = root(
        r#"{"qty":3,"note":null,"fruit":["🍒",true,{},[]]}"#,
        "json",
        "json-to-xml",
    );
    let xml = root(
        r#"<map xmlns="http://www.w3.org/2005/xpath-functions"><number key="qty">3</number><null key="note"/><array key="fruit"><string>🍒</string><boolean>true</boolean><map/><array/></array></map>"#,
        "xml",
        "cem",
    );
    for query in [
        "(/*/*/@key, count(//*), string(.), count(//text()))",
        "/*/Q{http://www.w3.org/2005/xpath-functions}number[@key='qty'] + 2",
        "(exists(/*/*[@key='note']), exists(/*/*[@key='missing']), count(/*/*[@key='fruit']/*))",
        "(/*/*[1] is /*/*[1], /*/*[1] << /*/*[2], local-name(/*/*[2]), namespace-uri(/*))",
        "(/*/*[2]/preceding-sibling::*, /*/*[1]/following-sibling::*, /*/*[2]/parent::*)",
    ] {
        assert_eq!(strings(query, &json), strings(query, &xml), "{query}");
    }
    assert_ne!(json.native_node().unwrap(), xml.native_node().unwrap());
}

#[test]
fn import_vocabulary_preserves_null_empty_duplicate_keys_order_and_lexical_values() {
    let input = root(
        r#"{"":null,"a b":"","dup":1,"dup":2,"a":[null,"",true,1.2300],"cr":"x\ry","unsafe":"\u0000"}"#,
        "json",
        "json-to-xml",
    );
    assert_eq!(strings("(count(/*/*[@key='dup']), /*/*[@key='dup']/text(), exists(/*/*[@key='']), exists(/*/*[@key='absent']), count(/*/*[@key='a b']/text()), /*/*[@key='a']/* ! local-name(.), string(/*/*[@key='a']/*[4]), string(/*/*[@key='cr']), string(/*/*[@key='unsafe']))", &input), ["2", "1", "2", "true", "false", "0", "null", "string", "boolean", "number", "1.2300", "x\ry", "�"]);
}

#[test]
fn all_data_formats_use_generic_navigation_and_value_operations() {
    for (source, format) in [
        ("qty: 3\nnote: null\n", "yaml"),
        (r#"{"qty":3,"note":null}"#, "json"),
    ] {
        let input = root(source, format, "cem");
        assert_eq!(strings("(count(/*/*), string(/*/*[@name='qty']), local-name(/*/*[@name='note']/*), count(/*/*/parent::*))", &input), ["2", "3", "null", "1"]);
    }
    let csv = root("qty,note\n3,hello\n4,world", "csv", "cem");
    assert_eq!(
        strings("(count(/*/*), /*/*/*[@name='qty']/string(.))", &csv),
        ["2", "3", "4"]
    );
}

#[test]
fn lifecycle_imports_preserve_coordinates_and_the_same_data_profile() {
    for (source, format, expected) in [
        ("<r>3</r>", "xml", "3"),
        ("{\n  \"qty\":3\n}", "json", "3"),
        ("qty:\n  - 3\n", "yaml", "3"),
        ("qty\n3", "csv", "3"),
    ] {
        let imported = root(source, format, "cem");
        let retained = cem_ml::import::retain_lifecycle(
            imported.native_node().unwrap().source_owner().unwrap(),
        )
        .unwrap();
        let from_lifecycle =
            XPathResultItem::from_native_node(XPathNativeNode::cem_document(retained));
        assert_eq!(strings("string(.)", &from_lifecycle), [expected]);
        let before = eval("//text()", Some(imported), Default::default()).unwrap();
        let after = eval("//text()", Some(from_lifecycle), Default::default()).unwrap();
        for (before, after) in before.sequence.items.iter().zip(&after.sequence.items) {
            if let (
                XPathResultItem::Node {
                    source_range: a,
                    source_map: am,
                    ..
                },
                XPathResultItem::Node {
                    source_range: b,
                    source_map: bm,
                    ..
                },
            ) = (before, after)
            {
                assert_eq!(a, b, "{format}");
                assert_eq!(am, bm, "{format}");
            } else {
                panic!("expected retained text nodes");
            }
        }
    }
    for projection in ["cem", "json-to-xml"] {
        assert!(import_data(
            &format!("{}0{}", "[".repeat(65), "]".repeat(65)),
            "json",
            projection,
            "memory:deep"
        )
        .is_err());
    }
}

#[test]
fn lifecycle_imports_reject_partial_parser_asts_with_errors() {
    use cem_ml::lifecycle::LoadedInputAstStream;
    use cem_ml::validation::{csv, json, xml, yaml};

    let (xml, xd) = xml::xml_document_ast_from_source_bytes(xml::XmlSourceValidationRequest {
        bytes: b"<p:r/>",
        source_uri: "memory:invalid.xml",
        content_type: Some("application/xml"),
    });
    let (json, jd) = json::json_document_ast_from_source_bytes(json::JsonSourceValidationRequest {
        bytes: b"{",
        source_uri: "memory:invalid.json",
        content_type: Some("application/json"),
    });
    let (yaml, yd) = yaml::yaml_document_ast_from_source_bytes(yaml::YamlSourceValidationRequest {
        bytes: b"[",
        source_uri: "memory:invalid.yaml",
        content_type: Some("application/yaml"),
    });
    let (csv, cd) = csv::csv_document_ast_from_source_bytes(csv::CsvSourceValidationRequest {
        bytes: b"\"broken",
        source_uri: "memory:invalid.csv",
        content_type: Some("text/csv;header=present"),
    });
    // The JSON parser rejects the AST outright; the other parsers retain diagnostics.
    assert!(json.is_none() && jd.iter().any(|d| d.severity.is_hard_violation()));
    let mut accepted = Vec::new();
    for (format, native, diagnostics) in [
        ("xml", LoadedInputAstStream::XmlDocument(xml.unwrap()), xd),
        (
            "yaml",
            LoadedInputAstStream::YamlDocument(yaml.unwrap()),
            yd,
        ),
        ("csv", LoadedInputAstStream::CsvDocument(csv.unwrap()), cd),
    ] {
        assert!(
            diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            "{format}: {diagnostics:?}"
        );
        let native = Arc::new(native);
        if cem_ml::import::retain_lifecycle(native.clone()).is_ok() {
            accepted.push(format);
        }
        if format == "xml" && XPathNativeNode::xml_document(native).is_ok() {
            accepted.push("xml compatibility constructor");
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted invalid parser ASTs: {accepted:?}"
    );
}

#[test]
fn xml_import_normalizes_semantics_without_mutating_the_source_cem_tree() {
    let input = root("<?xml version='1.0'?><r xmlns='urn:r' xmlns:p='urn:p' a='x\t\r\ny&#13;&#9;' p:b='2'>a\r\n<![CDATA[b\rc]]>&#13;<?keep x\r\ny?><!--c\rd--></r>", "xml", "cem");
    assert_eq!(strings("(count(/processing-instruction()), count(/*/@*), string(/*/@a), /*/text(), /*/processing-instruction('keep'), /*/comment())", &input), ["0", "2", "x  y\r\t", "a\nb\nc\r", "x\ny", "c\nd"]);
    let ast = input.native_node().unwrap().owner().ast();
    assert!(ast
        .iter()
        .any(|node| matches!(node, CemAstNode::Cdata { data, .. } if data == "b\rc")));
    assert!(ast.iter().any(|node| matches!(node, CemAstNode::Attribute { expanded_name, value: Some(value), .. } if expanded_name.local_name == "a" && value.contains('\t'))));
}

#[test]
fn opaque_containers_keep_imported_nodes_sources_and_limits() {
    let input = root(r#"{"a":"value","b":null}"#, "json", "json-to-xml");
    let tree = input.native_node().unwrap().owner().clone();
    let native = tree.native_owner().unwrap().clone();
    let weak_tree = Arc::downgrade(&tree);
    let weak_native = Arc::downgrade(&native);
    let packed = eval(
        "map { 'rows': [/*/*] }?rows?1",
        Some(input.clone()),
        Default::default(),
    )
    .unwrap();
    assert_eq!(packed.sequence.items.len(), 2);
    for item in &packed.sequence.items {
        let node = item.native_node().unwrap();
        assert!(Arc::ptr_eq(node.owner(), &tree));
        assert!(!node.source_map().frames.is_empty());
        assert_eq!(node.owner().source_uri(), "memory:source");
    }
    let limited = eval(
        "string(.)",
        Some(input.clone()),
        XPathEvaluationLimits {
            max_text_bytes: Some(4),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        limited
            .iter()
            .any(|e| e.code.contains("text") && e.code.contains("limit")),
        "{limited:?}"
    );
    let limited = eval(
        "count(//*)",
        Some(input.clone()),
        XPathEvaluationLimits {
            max_work_units: Some(1),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        limited
            .iter()
            .any(|e| e.code.contains("work") && e.code.contains("limit")),
        "{limited:?}"
    );
    drop(input);
    drop(tree);
    drop(native);
    assert!(weak_tree.upgrade().is_some());
    assert!(weak_native.upgrade().is_some());
    drop(packed);
    assert!(weak_tree.upgrade().is_none());
    assert!(weak_native.upgrade().is_none());
}

#[test]
fn native_cem_trees_need_no_external_format_owner_and_reject_invalid_graphs() {
    let source = Default::default();
    let valid = || CemDocument {
        nodes: vec![
            CemAstNode::Document {
                node_id: 0,
                root_children: vec![1],
                source: Default::default(),
            },
            CemAstNode::Text {
                node_id: 1,
                data: "native CEM".into(),
                source: Default::default(),
            },
        ],
        ..Default::default()
    };
    let tree = RetainedCemTree::new(valid(), "memory:cem", "", source, None).unwrap();
    let input = XPathResultItem::from_native_node(XPathNativeNode::cem_document(tree));
    assert_eq!(strings("string(.)", &input), ["native CEM"]);
    for children in [vec![0], vec![1, 1], vec![9], vec![]] {
        let mut ast = valid();
        if let CemAstNode::Document { root_children, .. } = &mut ast.nodes[0] {
            *root_children = children;
        }
        assert!(
            RetainedCemTree::new(ast, "memory:invalid", "", CemTreeSemantics::default(), None)
                .is_err()
        );
    }
}

#[test]
fn omitted_source_subtrees_cannot_be_reentered_by_a_native_handle() {
    let ast = CemDocument {
        nodes: vec![
            CemAstNode::Document {
                node_id: 0,
                root_children: vec![1],
                source: Default::default(),
            },
            CemAstNode::Element {
                node_id: 1,
                expanded_name: cem_ml::parser::ExpandedName {
                    namespace_uri: String::new(),
                    local_name: "hidden".into(),
                    schema_id: None,
                },
                children: vec![2],
                attributes: vec![],
                has_explicit_boundary: true,
                source: Default::default(),
            },
            CemAstNode::Text {
                node_id: 2,
                data: "hidden".into(),
                source: Default::default(),
            },
        ],
        ..Default::default()
    };
    let mut semantics = CemTreeSemantics::default();
    semantics.omitted.insert(1);
    let tree = RetainedCemTree::new(ast, "memory:cem", "", semantics, None).unwrap();
    assert!(XPathNativeNode::cem_node(tree.clone(), 2).is_err());
    assert_eq!(
        strings(
            "string(.)",
            &XPathResultItem::from_native_node(XPathNativeNode::cem_document(tree))
        ),
        [""]
    );
}

fn root(source: &str, format: &str, projection: &str) -> XPathResultItem {
    XPathResultItem::from_native_node(XPathNativeNode::cem_document(
        import_data(source, format, projection, "memory:source").unwrap(),
    ))
}
fn eval(
    source: &str,
    item: Option<XPathResultItem>,
    limits: XPathEvaluationLimits,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:containers.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    let hash = cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes());
    let artifact = artifact::XPathCompiledArtifact::compile(&expression, hash.clone()).unwrap();
    let expression = artifact
        .reload(&artifact::XPathArtifactLoadContext {
            expected_source_hash: hash,
            invocation_host: XPathInvocationHost::StandaloneTransform,
        })
        .unwrap();
    CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression: &expression,
        dynamic_context: XPathDynamicContext {
            context_item: item,
            ..Default::default()
        },
        static_context: XPathStaticContext::default(),
        expected_result: None,
        resolver_registry: &ResolverRegistry::new(),
        resolver_policy: &ResolverPolicy::new(),
        evaluation_limits: XPathEvaluationLimits {
            max_sequence_items: limits.max_sequence_items.or(Some(10000)),
            ..limits
        },
        safety_policy_stamp: "container-test",
        module_resolution: None,
    })
}
