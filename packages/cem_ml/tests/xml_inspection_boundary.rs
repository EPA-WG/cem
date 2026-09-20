//! XML-VIEW-1-BOUNDARY: source inspection must survive the typed writer boundary.
//! These tests distinguish retained source CEM nodes from the XPath semantic view.
use std::sync::Arc;

use cem_ml::{
    conversion::{
        direct_cem_output_pipeline,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
    },
    import::{
        import_data, import_string, ImportFailureKind, ImportStringProfile, ImportStringRequest,
    },
    lifecycle::LoadedInputAstStream,
    parser::{document::CemDocument, tree::RetainedCemTree, AstNodeId, CemAstNode},
    projection::{cem_tree_nodes, CemTreeAstNode},
    schema::SchemaRegistry,
    source_map::{FrameSpan, SourceMapStack},
};

const URI: &str = "memory:xml-inspection.xml";

fn imported(source: &str) -> Arc<RetainedCemTree> {
    import_data(source, "xml", "cem", URI).expect("native XML import")
}

fn children(doc: &CemDocument, id: AstNodeId) -> &[AstNodeId] {
    match doc.get(id).unwrap() {
        CemAstNode::Document { root_children, .. } => root_children,
        CemAstNode::Element { children, .. } => children,
        other => panic!("expected a parent, got {other:?}"),
    }
}

fn origin_text<'a>(input: &'a str, source: &SourceMapStack) -> &'a str {
    let FrameSpan::Single(range) = source.origin().expect("original source frame").span else {
        panic!("expected one source range");
    };
    &input[range.start as usize..range.start as usize + range.len as usize]
}

#[test]
fn source_cem_tree_retains_namespace_identity_order_empty_values_and_lexical_owner() {
    let input = concat!(
        "<?xml version='1.0'?>\r\n<?xml-stylesheet href='https://invalid.test/inert.xsl'?>\n",
        "<r xmlns='urn:root' xmlns:a='urn:one' xmlns:b='urn:one' xmlns:c='urn:two' ",
        "empty='' a:item='🍒' priority='100'> before<a:item/> between<b:item/>",
        "<c:item/> \r\n<!--note--><![CDATA[<raw>🍋]]> after&#x1F352; </r>"
    );
    let tree = imported(input);
    let doc = tree.ast();
    let owner = tree.native_owner().unwrap();
    let LoadedInputAstStream::XmlDocument(xml) = owner.downcast_ref().unwrap() else {
        panic!("XML parser owner stays retained at import");
    };
    assert_eq!(xml.source.uri, URI);
    assert_eq!(
        xml.events
            .iter()
            .map(|e| e.lexeme.as_str())
            .collect::<String>(),
        input
    );
    let root = *children(doc, 0).last().unwrap();
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = doc.get(root).unwrap()
    else {
        panic!("document element");
    };
    assert_eq!(expanded_name.namespace_uri, "urn:root");
    // Namespace declarations remain provenance; viewers exclude them from data attributes.
    assert_eq!(attributes.len(), 7);
    let ordinary: Vec<_> = attributes
        .iter()
        .filter_map(|id| match doc.get(*id).unwrap() {
            CemAstNode::Attribute {
                expanded_name,
                value,
                source,
                ..
            } if expanded_name.namespace_uri != "http://www.w3.org/2000/xmlns/" => {
                Some((*id, expanded_name, value.as_deref(), source))
            }
            _ => None,
        })
        .collect();
    assert_eq!(ordinary.len(), 3);
    assert_eq!(ordinary[0].1.local_name, "empty");
    assert_eq!(ordinary[0].2, Some(""));
    assert_eq!(origin_text(input, ordinary[0].3), "");
    assert!(!ordinary
        .iter()
        .any(|(_, name, _, _)| name.local_name == "missing"));
    assert_eq!(ordinary[1].1.namespace_uri, "urn:one");
    assert_eq!(ordinary[1].2, Some("🍒"));
    assert_eq!(origin_text(input, ordinary[1].3), "🍒");
    assert_eq!(ordinary[2].2, Some("100"));
    let mixed = children(doc, root);
    assert_eq!(mixed.len(), 9);
    assert!(matches!(doc.get(mixed[0]), Some(CemAstNode::Text { data, .. }) if data == " before"));
    assert!(matches!(doc.get(mixed[2]), Some(CemAstNode::Text { data, .. }) if data == " between"));
    for (index, namespace, lexeme) in [
        (1, "urn:one", "<a:item/>"),
        (3, "urn:one", "<b:item/>"),
        (4, "urn:two", "<c:item/>"),
    ] {
        let CemAstNode::Element {
            expanded_name,
            source,
            ..
        } = doc.get(mixed[index]).unwrap()
        else {
            panic!("ordered mixed-content element");
        };
        assert_eq!(expanded_name.local_name, "item");
        assert_eq!(expanded_name.namespace_uri, namespace);
        assert_eq!(origin_text(input, source), lexeme);
        assert_eq!(tree.node(mixed[index]).unwrap().range.line, 3);
    }
    assert_ne!(tree.source_key(mixed[1]), tree.source_key(mixed[3]));
    assert_ne!(tree.source_key(mixed[1]), tree.source_key(ordinary[1].0));
    assert!(
        matches!(doc.get(mixed[5]), Some(CemAstNode::Whitespace { data, .. }) if data == " \r\n")
    );
    assert!(
        matches!(doc.get(mixed[6]), Some(CemAstNode::Comment { data, source, .. }) if data == "note" && origin_text(input, source) == "<!--note-->")
    );
    assert!(
        matches!(doc.get(mixed[7]), Some(CemAstNode::Cdata { data, source, .. }) if data == "<raw>🍋" && origin_text(input, source) == "<![CDATA[<raw>🍋]]>")
    );
    assert!(
        matches!(doc.get(mixed[8]), Some(CemAstNode::Text { data, source, .. }) if data == " after🍒 " && source.frames.len() == 3)
    );
    // Query normalization does not mutate the source-oriented CEM arena.
    assert_eq!(tree.node(mixed[5]).unwrap().value, " \n");
}

#[test]
fn typed_projection_retains_payloads_before_the_writer() {
    let input = "<r empty=''>before<![CDATA[<raw>🍒]]><!--note--><?keep inert?>after</r>";
    let tree = imported(input);
    let stream = cem_tree_nodes(tree.ast());
    let [CemTreeAstNode::Element {
        attributes,
        children: projected_children,
        ..
    }] = stream.as_nodes()
    else {
        panic!("typed element projection");
    };
    assert_eq!(attributes[0].value.as_deref(), Some(""));
    assert!(
        matches!(&projected_children[0], CemTreeAstNode::Text { value, .. } if value == "before")
    );
    assert!(
        matches!(&projected_children[1], CemTreeAstNode::Cdata { data, source } if data == "<raw>🍒" && origin_text(input, source) == "<![CDATA[<raw>🍒]]>")
    );
    assert!(
        matches!(&projected_children[2], CemTreeAstNode::Comment { data, .. } if data == "note")
    );
    // Existing source-view contract: full lexical PI body, fallback target.
    // XML-VIEW-1 must decide how to expose its decoded target without parsing it here.
    assert!(
        matches!(&projected_children[3], CemTreeAstNode::ProcessingInstruction { target, data, source, .. } if target == "xml" && data == "keep inert" && origin_text(input, source) == "<?keep inert?>")
    );
    assert!(
        matches!(&projected_children[4], CemTreeAstNode::Text { value, .. } if value == "after")
    );
    let root = children(tree.ast(), 0)[0];
    let pi = *children(tree.ast(), root)
        .iter()
        .find(|id| {
            matches!(
                tree.ast().get(**id),
                Some(CemAstNode::ProcessingInstruction { .. })
            )
        })
        .unwrap();
    assert_eq!(
        tree.node(pi).unwrap().name.as_ref().unwrap().local_name,
        "keep"
    );
    assert_eq!(tree.node(pi).unwrap().value, "inert");
}

#[test]
fn tabular_writer_probe_exposes_the_remaining_payload_gap() {
    let tree = imported("<r empty=''>before<![CDATA[<raw>🍒]]><!--note--><?keep inert?>after</r>");
    let stream = Arc::new(cem_tree_nodes(tree.ast()));
    let registry = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &registry,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
        &environment,
        &pipeline,
        stream,
        Some(tree.node(0).unwrap().source.clone()),
        vec![],
        "xml-inspection-boundary",
        None,
        Some(URI),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    // This is the named public text-output envelope after all typed stages.
    let output = result
        .output
        .as_ref()
        .and_then(|value| value.as_str())
        .unwrap();
    // Characterization of the open XML-VIEW-1 gap, not an acceptance contract.
    // Replace this assertion with lossless inspection assertions when the
    // shared presentation decision is implemented. The typed input above
    // proves these payloads were available without reading XML again.
    assert_eq!(
        output,
        "{r @empty=\"\" |\n    before\n    {node}\n    {comment @value=note}\n    {xml}\n    after\n}\n"
    );
    assert!(!result.output_spans.is_empty());
}

#[test]
fn malformed_entities_and_resource_limits_fail_at_import() {
    for input in [
        "<r>\n<x></r>",
        "<!DOCTYPE r SYSTEM 'https://invalid.test/external.dtd'><r/>",
        "<!DOCTYPE r [<!ENTITY x 'expanded'>]><r>&x;</r>",
        "<r>&external;</r>",
    ] {
        assert!(import_data(input, "xml", "cem", URI).is_err(), "{input}");
    }
    let malformed = import_string(ImportStringRequest {
        source: "<r>\n<x></r>",
        source_uri: URI,
        base_uri: None,
        profile: ImportStringProfile::Xml,
    })
    .unwrap_err();
    assert_eq!(malformed.kind, ImportFailureKind::Malformed);
    assert!(malformed
        .diagnostics
        .iter()
        .any(|d| d.line == Some(2) && d.column.is_some()));
    for (accepted, rejected) in [
        (
            format!("{}x{}", "<r>".repeat(64), "</r>".repeat(64)),
            format!("{}x{}", "<r>".repeat(65), "</r>".repeat(65)),
        ),
        (
            format!("<r>{}</r>", "<x/>".repeat(4094)),
            format!("<r>{}</r>", "<x/>".repeat(4095)),
        ),
        (
            format!("<r>{}</r>", "x".repeat(32768 - 7)),
            format!("<r>{}</r>", "x".repeat(32769 - 7)),
        ),
    ] {
        assert!(import_data(&accepted, "xml", "cem", URI).is_ok());
        assert!(import_data(&rejected, "xml", "cem", URI).is_err());
        let failure = import_string(ImportStringRequest {
            source: &rejected,
            source_uri: URI,
            base_uri: None,
            profile: ImportStringProfile::Xml,
        })
        .unwrap_err();
        assert_eq!(failure.kind, ImportFailureKind::Limit);
    }
}
