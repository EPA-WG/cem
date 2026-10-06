use cem_ml::{
    import::import_xml_ast_with_lexical_scopes,
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{scoping::SchemaSource, vocab::CompiledSchema},
    source_map::TransformKind,
    validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
};
use std::sync::Arc;

fn capture(text: &str) -> cem_ml::import::ScopedXmlCemImport {
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: text.as_bytes(),
        source_uri: "scope.xml",
        content_type: Some("application/xml"),
    });
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core()).unwrap()
}

#[test]
fn xml_aliases_self_closing_switches_and_wrapping_defaults_keep_original_reference_associations() {
    let text = "<root xmlns:r='https://cem.dev/ns/cem-ml/1' xmlns:v='urn:outer' r:schema-src='outer'><section r:schema-src='host'><r:expr>#a</r:expr><r:schema select='missing()' xmlns:v='urn:inner'><r:expr>#b</r:expr></r:schema><r:expr>#c</r:expr><r:schema src='sibling'/><r:expr>#d</r:expr></section><r:expr>#e</r:expr></root>";
    let imported = capture(text);
    let owner = imported.captured.document().clone();
    let occurrences = imported.captured.occurrences().collect::<Vec<_>>();
    let actual = occurrences
        .iter()
        .map(|node| {
            let snapshot = imported.captured.snapshot(&owner, *node).unwrap();
            assert_eq!(
                snapshot.namespaces.binding("r").unwrap().namespace_uri,
                "https://cem.dev/ns/cem-ml/1"
            );
            assert!(!snapshot
                .namespaces
                .binding("v")
                .unwrap()
                .source_map
                .frames
                .is_empty());
            (
                snapshot.schema.active.clone(),
                snapshot
                    .namespaces
                    .binding("v")
                    .unwrap()
                    .namespace_uri
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (SchemaSource::Uri("host".into()), "urn:outer".into()),
            (SchemaSource::Select("missing()".into()), "urn:inner".into()),
            (SchemaSource::Uri("host".into()), "urn:outer".into()),
            (SchemaSource::Uri("sibling".into()), "urn:outer".into()),
            (SchemaSource::Uri("outer".into()), "urn:outer".into()),
        ]
    );
    for node in occurrences {
        assert!(imported.event_nodes.iter().any(|id| *id == Some(node)));
        assert!(matches!(
            owner.get(node),
            Some(CemAstNode::Reference { targets: None, .. })
        ));
    }
    assert!(imported
        .event_nodes
        .iter()
        .flatten()
        .all(|node| owner.get(*node).is_some()));
    let tree =
        RetainedCemTree::from_shared(owner.clone(), "scope.xml", text, imported.semantics, None)
            .unwrap();
    assert!(Arc::ptr_eq(tree.ast_owner(), &owner));
}

#[test]
fn xml_local_alias_rebinding_entity_cdata_provenance_and_foreign_cem_spelling_stay_distinct() {
    let text = "<root xmlns:cem='urn:foreign' xmlns:v='urn:outer' cem:schema-src='foreign'><cem:schema src='foreign'/><cem:expr>#ignored</cem:expr><expr xmlns='https://cem.dev/ns/cem-ml/1' xmlns:r='https://cem.dev/ns/cem-ml/1' xmlns:v='urn:inner'>#lib&#114;<![CDATA[ary]]></expr><r:expr xmlns:r='urn:foreign'>#ignored</r:expr><r:expr xmlns:r='https://cem.dev/ns/cem-ml/1'>#library</r:expr><target value='{#library}'/></root>";
    let imported = capture(text);
    let owner = imported.captured.document();
    let occurrences = imported.captured.occurrences().collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 2);
    for (index, node) in occurrences.iter().enumerate() {
        let snapshot = imported.captured.snapshot(owner, *node).unwrap();
        assert_ne!(snapshot.schema.active, SchemaSource::Uri("foreign".into()));
        assert_eq!(
            snapshot.namespaces.binding("cem").unwrap().namespace_uri,
            "urn:foreign"
        );
        assert_eq!(
            snapshot.namespaces.binding("v").unwrap().namespace_uri,
            if index == 0 { "urn:inner" } else { "urn:outer" }
        );
        assert!(
            matches!(owner.get(*node), Some(CemAstNode::Reference {expression, targets: None, ..}) if expression == "#library")
        );
    }
    let CemAstNode::Reference { source, .. } = owner.get(occurrences[0]).unwrap() else {
        unreachable!()
    };
    assert!(
        source
            .frames
            .iter()
            .filter(|frame| matches!(frame.transform, TransformKind::ExpressionEmbedding { .. }))
            .count()
            >= 3
    );
    assert!(owner.nodes.iter().any(|node| matches!(node, CemAstNode::Attribute {value: Some(value), value_nodes, ..} if value == "{#library}" && value_nodes.is_empty())));
    let invalid = capture("<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:schema src='one' select='missing()'/></root>");
    assert!(invalid
        .captured
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation()));
}

#[test]
fn imported_name_lookup_retains_xml_namespace_rules_and_checks_owner() {
    let imported = capture("<schema xmlns='https://cem.dev/ns/schema/1' name='selected'><element xmlns='urn:other' name='child'/></schema>");
    let owner = imported.captured.document();
    for node in &owner.nodes {
        match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            }
            | CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } => {
                let name = imported.captured.expanded_name(owner, *node_id).unwrap();
                assert_eq!(name.namespace_uri, expanded_name.namespace_uri);
                assert_eq!(name.local_name, expanded_name.local_name);
                if matches!(node, CemAstNode::Attribute { .. }) && name.local_name == "name" {
                    assert!(name.namespace_uri.is_empty());
                }
            }
            _ => {}
        }
    }
    let other = capture("<schema xmlns='https://cem.dev/ns/schema/1' name='selected'><element xmlns='urn:other' name='child'/></schema>");
    assert!(imported
        .captured
        .expanded_name(other.captured.document(), 1)
        .is_none());
}
